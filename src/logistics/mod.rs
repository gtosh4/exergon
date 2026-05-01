use std::collections::{HashMap, HashSet};

use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::inventory::ItemRegistry;
use crate::machine::{Machine, MachineActivity, MachineScanSet, MachineNetworkChanged, MachineState, MachineUnformed};
use crate::recipe_graph::RecipeGraph;
use crate::world::{BlockChangeKind, BlockChangedEvent};

pub struct LogisticsPlugin;

impl Plugin for LogisticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogisticsData>().add_systems(
            Update,
            (
                ApplyDeferred,
                update_logistics_networks
                    .run_if(resource_exists::<ItemRegistry>),
                ApplyDeferred,
                machine_io_system
                    .run_if(resource_exists::<RecipeGraph>),
            )
                .chain()
                .after(MachineScanSet)
                .in_set(crate::GameSystems::Simulation)
                .run_if(in_state(crate::GameState::Playing)),
        );
    }
}

const LOGISTICS_CABLE_ID: &str = "logistics_cable";
const STORAGE_CRATE_ID: &str = "storage_crate";

const DIRS: [IVec3; 6] = [
    IVec3::new(1, 0, 0),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 0, -1),
];

#[derive(Component)]
#[relationship(relationship_target = LogisticsNetworkMembers)]
pub struct LogisticsMember(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = LogisticsMember)]
pub struct LogisticsNetworkMembers(Vec<Entity>);

#[derive(Component)]
pub struct LogisticsNetwork {
    pub storage_positions: Vec<IVec3>,
}

#[derive(Resource, Default)]
pub struct LogisticsData {
    pub cable_positions: HashSet<IVec3>,
    /// Items stored at each storage crate position. Persists through rebuilds.
    pub storage_blocks: HashMap<IVec3, HashMap<String, u32>>,
    dirty: bool,
}

impl LogisticsData {
    pub fn has_items(&self, storage_positions: &[IVec3], item_id: &str, count: u32) -> bool {
        let available: u32 = storage_positions
            .iter()
            .filter_map(|p| self.storage_blocks.get(p))
            .map(|s| s.get(item_id).copied().unwrap_or(0))
            .sum();
        available >= count
    }

    pub fn take_items(&mut self, storage_positions: &[IVec3], item_id: &str, count: u32) {
        let mut remaining = count;
        for pos in storage_positions {
            if remaining == 0 {
                break;
            }
            if let Some(block) = self.storage_blocks.get_mut(pos) {
                let avail = *block.get(item_id).unwrap_or(&0);
                let take = remaining.min(avail);
                if take > 0 {
                    let v = block.entry(item_id.to_owned()).or_insert(0);
                    *v -= take;
                    if *v == 0 {
                        block.remove(item_id);
                    }
                    remaining -= take;
                }
            }
        }
    }

    pub fn give_items(&mut self, storage_positions: &[IVec3], item_id: &str, count: u32) {
        if let Some(pos) = storage_positions.first() {
            if let Some(block) = self.storage_blocks.get_mut(pos) {
                *block.entry(item_id.to_owned()).or_insert(0) += count;
                return;
            }
        }
        warn!("No storage for network; {item_id} ×{count} lost");
    }
}

fn update_logistics_networks(
    mut commands: Commands,
    mut net_data: ResMut<LogisticsData>,
    existing_nets: Query<Entity, With<LogisticsNetwork>>,
    mut block_events: MessageReader<BlockChangedEvent>,
    mut machine_events: MessageReader<MachineNetworkChanged>,
    item_registry: Res<ItemRegistry>,
    machine_q: Query<(Entity, &Machine), Without<MachineUnformed>>,
) {
    let cable_vox = item_registry.voxel_id(LOGISTICS_CABLE_ID);
    let storage_vox = item_registry.voxel_id(STORAGE_CRATE_ID);

    for ev in block_events.read() {
        let placed = match ev.kind {
            BlockChangeKind::Placed { voxel_id } => Some((None::<u8>, Some(voxel_id))),
            BlockChangeKind::Removed { voxel_id } => Some((Some(voxel_id), None)),
            BlockChangeKind::Replaced { old_voxel_id, new_voxel_id } => {
                Some((Some(old_voxel_id), Some(new_voxel_id)))
            }
        };
        if let Some((removed, added)) = placed {
            if removed.map(|v| Some(v) == cable_vox).unwrap_or(false) {
                net_data.cable_positions.remove(&ev.pos);
                net_data.dirty = true;
            }
            if added.map(|v| Some(v) == cable_vox).unwrap_or(false) {
                net_data.cable_positions.insert(ev.pos);
                net_data.dirty = true;
            }
            if removed.map(|v| Some(v) == storage_vox).unwrap_or(false) {
                net_data.storage_blocks.remove(&ev.pos);
                net_data.dirty = true;
            }
            if added.map(|v| Some(v) == storage_vox).unwrap_or(false) {
                net_data.storage_blocks.entry(ev.pos).or_default();
                net_data.dirty = true;
            }
        }
    }

    for _ in machine_events.read() {
        net_data.dirty = true;
    }

    if !net_data.dirty {
        return;
    }
    net_data.dirty = false;

    for net_entity in &existing_nets {
        commands.entity(net_entity).despawn();
    }

    let cable_positions = net_data.cable_positions.clone();
    let storage_positions: HashSet<IVec3> = net_data.storage_blocks.keys().copied().collect();

    let logistics_io_map: HashMap<IVec3, Entity> = machine_q
        .iter()
        .flat_map(|(e, m)| m.logistics_io_blocks.iter().map(move |&p| (p, e)))
        .collect();

    let mut visited: HashSet<IVec3> = HashSet::new();
    let mut network_count = 0usize;

    for &start in &cable_positions {
        if visited.contains(&start) {
            continue;
        }

        let mut queue = vec![start];
        let mut component: HashSet<IVec3> = HashSet::new();
        visited.insert(start);
        component.insert(start);

        while let Some(pos) = queue.pop() {
            for &dir in &DIRS {
                let n = pos + dir;
                if !visited.contains(&n) && cable_positions.contains(&n) {
                    visited.insert(n);
                    component.insert(n);
                    queue.push(n);
                }
            }
        }

        let mut machine_entities: Vec<Entity> = Vec::new();
        let mut net_storage: Vec<IVec3> = Vec::new();
        let mut seen_machines: HashSet<Entity> = HashSet::new();

        for &cable_pos in &component {
            for &dir in &DIRS {
                let n = cable_pos + dir;
                if let Some(&entity) = logistics_io_map.get(&n) {
                    if seen_machines.insert(entity) {
                        machine_entities.push(entity);
                    }
                }
                if storage_positions.contains(&n) && !net_storage.contains(&n) {
                    net_storage.push(n);
                }
            }
        }

        let net_entity = commands.spawn(LogisticsNetwork { storage_positions: net_storage }).id();
        for machine_entity in machine_entities {
            commands.entity(machine_entity).insert(LogisticsMember(net_entity));
        }
        network_count += 1;
    }

    debug!(
        "Logistics: {} networks, {} cables, {} storage blocks",
        network_count,
        net_data.cable_positions.len(),
        net_data.storage_blocks.len(),
    );
}

fn machine_io_system(
    mut commands: Commands,
    time: Res<Time>,
    mut net_data: ResMut<LogisticsData>,
    recipe_graph: Res<RecipeGraph>,
    net_q: Query<&LogisticsNetwork>,
    mut params: ParamSet<(
        Query<(Entity, &Machine, &MachineState, Option<&MachineActivity>, &LogisticsMember)>,
        Query<&mut MachineActivity>,
    )>,
) {
    let dt = time.delta_secs();

    let mut to_start: Vec<(Entity, String, Entity)> = Vec::new();
    let mut to_finish: Vec<(Entity, Vec<(String, u32)>, Entity)> = Vec::new();
    let mut progress_updates: Vec<(Entity, f32)> = Vec::new();

    {
        let machine_q = params.p0();
        for (entity, machine, state, activity, member) in &machine_q {
            let net_entity = member.0;
            let Ok(network) = net_q.get(net_entity) else { continue; };

            match *state {
                MachineState::Idle => {
                    for recipe in recipe_graph.recipes.values() {
                        if recipe.machine_type != machine.machine_type {
                            continue;
                        }
                        if recipe.machine_tier > machine.tier {
                            continue;
                        }
                        let all_ok = recipe.inputs.iter().all(|input| {
                            net_data.has_items(
                                &network.storage_positions,
                                &input.material,
                                input.quantity as u32,
                            )
                        });
                        if all_ok {
                            to_start.push((entity, recipe.id.clone(), net_entity));
                            break;
                        }
                    }
                }
                MachineState::Running => {
                    if let Some(activity) = activity {
                        let Some(recipe) = recipe_graph.recipes.get(&activity.recipe_id) else {
                            continue;
                        };
                        let new_progress = activity.progress + dt * activity.speed_factor;
                        if new_progress >= recipe.processing_time {
                            let outputs: Vec<(String, u32)> = recipe
                                .outputs
                                .iter()
                                .chain(recipe.byproducts.iter())
                                .map(|o| (o.material.clone(), o.quantity as u32))
                                .collect();
                            to_finish.push((entity, outputs, net_entity));
                        } else {
                            progress_updates.push((entity, new_progress));
                        }
                    }
                }
            }
        }
    }

    {
        let mut activity_q = params.p1();
        for (entity, new_progress) in progress_updates {
            if let Ok(mut act) = activity_q.get_mut(entity) {
                act.progress = new_progress;
            }
        }
    }

    for (entity, recipe_id, net_entity) in to_start {
        let recipe = recipe_graph.recipes.get(&recipe_id).unwrap();
        if let Ok(network) = net_q.get(net_entity) {
            for input in &recipe.inputs {
                net_data.take_items(
                    &network.storage_positions,
                    &input.material,
                    input.quantity as u32,
                );
            }
        }
        commands.entity(entity).insert((
            MachineActivity { recipe_id, progress: 0.0, speed_factor: 1.0 },
            MachineState::Running,
        ));
        info!("Machine {:?} started recipe", entity);
    }

    for (entity, outputs, net_entity) in to_finish {
        if let Ok(network) = net_q.get(net_entity) {
            for (item_id, count) in outputs {
                if count > 0 {
                    net_data.give_items(&network.storage_positions, &item_id, count);
                }
            }
        }
        commands.entity(entity).remove::<MachineActivity>().insert(MachineState::Idle);
        info!("Machine {:?} finished recipe", entity);
    }
}

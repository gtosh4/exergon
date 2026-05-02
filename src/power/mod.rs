use std::collections::{HashMap, HashSet};

use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::inventory::ItemRegistry;
use crate::machine::{
    Machine, MachineActivity, MachineNetworkChanged, MachineScanSet, MachineState, MachineUnformed,
};
use crate::recipe_graph::RecipeGraph;
use crate::world::{BlockChangeKind, BlockChangedMessage};

pub struct PowerPlugin;

impl Plugin for PowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PowerData>().add_systems(
            Update,
            (
                ApplyDeferred,
                update_power_networks.run_if(resource_exists::<ItemRegistry>),
                ApplyDeferred,
                brownout_system.run_if(resource_exists::<RecipeGraph>),
            )
                .chain()
                .after(MachineScanSet)
                .in_set(crate::GameSystems::Simulation)
                .run_if(in_state(crate::GameState::Playing)),
        );
    }
}

const POWER_CABLE_ID: &str = "power_cable";
const GENERATOR_ID: &str = "generator";
const GENERATOR_DEFAULT_WATTS: f32 = 50.0;

const DIRS: [IVec3; 6] = [
    IVec3::new(1, 0, 0),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 0, -1),
];

#[derive(Component)]
#[relationship(relationship_target = PowerNetworkMembers)]
pub struct PowerMember(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = PowerMember)]
pub struct PowerNetworkMembers(Vec<Entity>);

#[derive(Component)]
pub struct PowerNetwork {
    pub capacity_watts: f32,
}

#[derive(Resource, Default)]
pub struct PowerData {
    pub cable_positions: HashSet<IVec3>,
    /// Watts output per generator block position.
    pub generator_blocks: HashMap<IVec3, f32>,
    dirty: bool,
}

fn update_power_networks(
    mut commands: Commands,
    mut power_data: ResMut<PowerData>,
    existing_nets: Query<Entity, With<PowerNetwork>>,
    mut block_events: MessageReader<BlockChangedMessage>,
    mut machine_events: MessageReader<MachineNetworkChanged>,
    item_registry: Res<ItemRegistry>,
    machine_q: Query<(Entity, &Machine), Without<MachineUnformed>>,
) {
    let cable_vox = item_registry.voxel_id(POWER_CABLE_ID);
    let generator_vox = item_registry.voxel_id(GENERATOR_ID);

    for ev in block_events.read() {
        let placed = match ev.kind {
            BlockChangeKind::Placed { voxel_id } => Some((None::<u8>, Some(voxel_id))),
            BlockChangeKind::Removed { voxel_id } => Some((Some(voxel_id), None)),
            BlockChangeKind::Replaced {
                old_voxel_id,
                new_voxel_id,
            } => Some((Some(old_voxel_id), Some(new_voxel_id))),
        };
        if let Some((removed, added)) = placed {
            if removed.is_some_and(|v| Some(v) == cable_vox) {
                power_data.cable_positions.remove(&ev.pos);
                power_data.dirty = true;
            }
            if added.is_some_and(|v| Some(v) == cable_vox) {
                power_data.cable_positions.insert(ev.pos);
                power_data.dirty = true;
            }
            if removed.is_some_and(|v| Some(v) == generator_vox) {
                power_data.generator_blocks.remove(&ev.pos);
                power_data.dirty = true;
            }
            if added.is_some_and(|v| Some(v) == generator_vox) {
                power_data
                    .generator_blocks
                    .insert(ev.pos, GENERATOR_DEFAULT_WATTS);
                power_data.dirty = true;
            }
        }
    }

    for _ in machine_events.read() {
        power_data.dirty = true;
    }

    if !power_data.dirty {
        return;
    }
    power_data.dirty = false;

    for net_entity in &existing_nets {
        commands.entity(net_entity).despawn();
    }

    let cable_positions = power_data.cable_positions.clone();
    let generator_positions: HashSet<IVec3> = power_data.generator_blocks.keys().copied().collect();

    let energy_io_map: HashMap<IVec3, Entity> = machine_q
        .iter()
        .flat_map(|(e, m)| m.energy_io_blocks.iter().map(move |&p| (p, e)))
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
        let mut net_generators: Vec<IVec3> = Vec::new();
        let mut seen_machines: HashSet<Entity> = HashSet::new();

        for &cable_pos in &component {
            for &dir in &DIRS {
                let n = cable_pos + dir;
                if let Some(&entity) = energy_io_map.get(&n)
                    && seen_machines.insert(entity)
                {
                    machine_entities.push(entity);
                }
                if generator_positions.contains(&n) && !net_generators.contains(&n) {
                    net_generators.push(n);
                }
            }
        }

        let capacity_watts: f32 = net_generators
            .iter()
            .filter_map(|p| power_data.generator_blocks.get(p))
            .sum();

        let net_entity = commands.spawn(PowerNetwork { capacity_watts }).id();
        for machine_entity in machine_entities {
            commands
                .entity(machine_entity)
                .insert(PowerMember(net_entity));
        }
        network_count += 1;
    }

    debug!(
        "Power: {} networks, {} cables, {} generators",
        network_count,
        power_data.cable_positions.len(),
        power_data.generator_blocks.len(),
    );
}

fn brownout_system(
    net_q: Query<(&PowerNetwork, &PowerNetworkMembers)>,
    recipe_graph: Res<RecipeGraph>,
    mut params: ParamSet<(
        Query<(&MachineState, Option<&MachineActivity>)>,
        Query<&mut MachineActivity>,
    )>,
) {
    let net_speeds: Vec<(Vec<Entity>, f32)> = {
        let machine_q = params.p0();
        net_q
            .iter()
            .map(|(network, members)| {
                let speed = if network.capacity_watts > 0.0 {
                    let demand: f32 = members
                        .iter()
                        .filter_map(|e| {
                            let (state, activity) = machine_q.get(e).ok()?;
                            if *state != MachineState::Running {
                                return None;
                            }
                            let activity = activity?;
                            let recipe = recipe_graph.recipes.get(&activity.recipe_id)?;
                            Some(recipe.energy_cost / recipe.processing_time)
                        })
                        .sum();
                    if demand > network.capacity_watts {
                        network.capacity_watts / demand
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };
                (members.iter().collect::<Vec<_>>(), speed)
            })
            .collect()
    };

    let mut activity_q = params.p1();
    for (entities, speed) in &net_speeds {
        for &e in entities {
            if let Ok(mut act) = activity_q.get_mut(e) {
                act.speed_factor = *speed;
            }
        }
    }
}

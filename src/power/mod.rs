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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bevy::prelude::*;

    use super::*;
    use crate::inventory::ItemRegistry;
    use crate::machine::{MachineActivity, MachineNetworkChanged, MachineState};
    use crate::recipe_graph::{RecipeDef, RecipeGraph};
    use crate::world::BlockChangedMessage;

    fn power_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<BlockChangedMessage>()
            .add_message::<MachineNetworkChanged>()
            .insert_resource(ItemRegistry::default())
            .init_resource::<PowerData>()
            .add_systems(Update, update_power_networks);
        app
    }

    fn registered_power_app() -> App {
        use crate::inventory::{BlockProps, ItemDef};
        let mut app = power_app();
        {
            let mut reg = app.world_mut().resource_mut::<ItemRegistry>();
            reg.register(ItemDef {
                id: POWER_CABLE_ID.to_string(),
                name: POWER_CABLE_ID.to_string(),
                block: Some(BlockProps {
                    voxel_id: 20,
                    hardness: 1.0,
                }),
            });
            reg.register(ItemDef {
                id: GENERATOR_ID.to_string(),
                name: GENERATOR_ID.to_string(),
                block: Some(BlockProps {
                    voxel_id: 21,
                    hardness: 1.0,
                }),
            });
        }
        app
    }

    fn recipe_graph(energy_cost: f32, processing_time: f32) -> RecipeGraph {
        let mut recipes = HashMap::new();
        recipes.insert(
            "r1".to_string(),
            RecipeDef {
                id: "r1".to_string(),
                inputs: vec![],
                outputs: vec![],
                byproducts: vec![],
                machine_type: "smelter".to_string(),
                machine_tier: 1,
                processing_time,
                energy_cost,
            },
        );
        RecipeGraph {
            materials: HashMap::new(),
            recipes,
            terminal: String::new(),
            producers: HashMap::new(),
            consumers: HashMap::new(),
        }
    }

    #[test]
    fn no_dirty_no_networks_spawned() {
        let mut app = power_app();
        app.update();
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 0);
    }

    #[test]
    fn single_cable_creates_one_network() {
        let mut app = power_app();
        {
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.insert(IVec3::ZERO);
            pd.dirty = true;
        }
        app.update();
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn two_adjacent_cables_one_network() {
        let mut app = power_app();
        {
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.insert(IVec3::ZERO);
            pd.cable_positions.insert(IVec3::new(1, 0, 0));
            pd.dirty = true;
        }
        app.update();
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn two_disconnected_cables_two_networks() {
        let mut app = power_app();
        {
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.insert(IVec3::ZERO);
            pd.cable_positions.insert(IVec3::new(5, 0, 0));
            pd.dirty = true;
        }
        app.update();
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 2);
    }

    #[test]
    fn generator_adjacent_to_cable_adds_capacity() {
        let mut app = power_app();
        {
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.insert(IVec3::ZERO);
            pd.generator_blocks.insert(IVec3::new(1, 0, 0), 50.0);
            pd.dirty = true;
        }
        app.update();
        let world = app.world_mut();
        let caps: Vec<f32> = world
            .query::<&PowerNetwork>()
            .iter(world)
            .map(|n| n.capacity_watts)
            .collect();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0], 50.0);
    }

    #[test]
    fn dirty_rebuild_replaces_old_networks() {
        let mut app = power_app();
        {
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.insert(IVec3::ZERO);
            pd.dirty = true;
        }
        app.update();
        {
            // Remove cable, mark dirty → should clear networks
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.clear();
            pd.dirty = true;
        }
        app.update();
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 0);
    }

    #[test]
    fn brownout_full_speed_when_capacity_exceeds_demand() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(recipe_graph(50.0, 1.0))
            .add_systems(Update, brownout_system);

        let net = app
            .world_mut()
            .spawn(PowerNetwork {
                capacity_watts: 200.0,
            })
            .id();
        app.world_mut().spawn((
            MachineState::Running,
            MachineActivity {
                recipe_id: "r1".to_string(),
                progress: 0.0,
                speed_factor: 1.0,
            },
            PowerMember(net),
        ));

        app.update();

        let world = app.world_mut();
        let speed = world
            .query::<&MachineActivity>()
            .iter(world)
            .next()
            .unwrap()
            .speed_factor;
        assert_eq!(speed, 1.0);
    }

    #[test]
    fn brownout_throttles_when_demand_exceeds_capacity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(recipe_graph(100.0, 1.0))
            .add_systems(Update, brownout_system);

        let net = app
            .world_mut()
            .spawn(PowerNetwork {
                capacity_watts: 50.0,
            })
            .id();
        app.world_mut().spawn((
            MachineState::Running,
            MachineActivity {
                recipe_id: "r1".to_string(),
                progress: 0.0,
                speed_factor: 1.0,
            },
            PowerMember(net),
        ));

        app.update();

        let world = app.world_mut();
        let speed = world
            .query::<&MachineActivity>()
            .iter(world)
            .next()
            .unwrap()
            .speed_factor;
        assert!((speed - 0.5).abs() < 1e-6, "expected 0.5, got {speed}");
    }

    #[test]
    fn block_placed_cable_message_builds_network() {
        let mut app = registered_power_app();
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::ZERO,
            kind: BlockChangeKind::Placed { voxel_id: 20 },
        });
        app.update();
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn block_removed_cable_message_removes_network() {
        let mut app = registered_power_app();
        {
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.insert(IVec3::ZERO);
            pd.dirty = true;
        }
        app.update();
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::ZERO,
            kind: BlockChangeKind::Removed { voxel_id: 20 },
        });
        app.update();
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 0);
    }

    #[test]
    fn block_placed_generator_message_adds_capacity() {
        let mut app = registered_power_app();
        // cable at origin, generator adjacent
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::ZERO,
            kind: BlockChangeKind::Placed { voxel_id: 20 },
        });
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::new(1, 0, 0),
            kind: BlockChangeKind::Placed { voxel_id: 21 },
        });
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&PowerNetwork>();
        let net = q.iter(world).next().unwrap();
        assert!(net.capacity_watts > 0.0);
    }

    #[test]
    fn block_removed_generator_message_clears_capacity() {
        let mut app = registered_power_app();
        {
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.insert(IVec3::ZERO);
            pd.generator_blocks.insert(IVec3::new(1, 0, 0), 50.0);
            pd.dirty = true;
        }
        app.update();
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::new(1, 0, 0),
            kind: BlockChangeKind::Removed { voxel_id: 21 },
        });
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&PowerNetwork>();
        let net = q.iter(world).next().unwrap();
        assert_eq!(net.capacity_watts, 0.0);
    }

    #[test]
    fn machine_network_changed_message_sets_dirty() {
        let mut app = power_app();
        {
            let mut pd = app.world_mut().resource_mut::<PowerData>();
            pd.cable_positions.insert(IVec3::ZERO);
        }
        app.world_mut().write_message(MachineNetworkChanged);
        app.update();
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn brownout_idle_machine_not_counted_in_demand() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(recipe_graph(1000.0, 1.0))
            .add_systems(Update, brownout_system);

        let net = app
            .world_mut()
            .spawn(PowerNetwork {
                capacity_watts: 1.0,
            })
            .id();
        // Idle machine with no activity → contributes 0 demand
        app.world_mut()
            .spawn((MachineState::Idle, PowerMember(net)));

        app.update();

        // No MachineActivity to check speed on, but system should not crash
        // and network should still exist
        let world = app.world_mut();
        let count = world.query::<&PowerNetwork>().iter(world).count();
        assert_eq!(count, 1);
    }
}

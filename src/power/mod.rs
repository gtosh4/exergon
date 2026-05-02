use std::collections::{HashMap, HashSet};

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::inventory::ItemRegistry;
use crate::machine::{Machine, MachineActivity, MachineScanSet, MachineState};
use crate::network::{
    self, DIRS, HasPos, NetworkChanged, NetworkKind, NetworkMemberComponent,
    NetworkMembersComponent, Power,
};
use crate::recipe_graph::RecipeGraph;
use crate::world::{BlockChangeKind, BlockChangedMessage};

pub struct PowerPlugin;

impl Plugin for PowerPlugin {
    fn build(&self, app: &mut App) {
        network::init_network::<Power>(app);
        app.add_systems(
            Update,
            (
                ApplyDeferred,
                network::cable_placed_system::<Power>.run_if(resource_exists::<ItemRegistry>),
                network::cable_removed_system::<Power>.run_if(resource_exists::<ItemRegistry>),
                network::machine_membership_system::<Power>.run_if(resource_exists::<ItemRegistry>),
                ApplyDeferred,
                generator_system.run_if(resource_exists::<ItemRegistry>),
                ApplyDeferred,
                recalc_capacity_system,
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

// -- Components --------------------------------------------------------------

#[derive(Component)]
pub struct PowerCableBlock {
    pub pos: IVec3,
}

impl HasPos for PowerCableBlock {
    fn pos(&self) -> IVec3 {
        self.pos
    }
}

#[derive(Component)]
#[relationship(relationship_target = PowerNetworkMembers)]
pub struct PowerNetworkMember(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = PowerNetworkMember)]
pub struct PowerNetworkMembers(Vec<Entity>);

impl NetworkMemberComponent for PowerNetworkMember {
    fn new(network: Entity) -> Self {
        PowerNetworkMember(network)
    }
    fn network(&self) -> Entity {
        self.0
    }
}

impl NetworkMembersComponent for PowerNetworkMembers {
    fn members(&self) -> &[Entity] {
        &self.0
    }
}

#[derive(Component)]
pub struct GeneratorBlock {
    pub pos: IVec3,
    pub watts: f32,
}

#[derive(Component)]
pub struct PowerNetwork {
    pub capacity_watts: f32,
}

// -- NetworkKind impl --------------------------------------------------------

impl NetworkKind for Power {
    const CABLE_ITEM_ID: &'static str = POWER_CABLE_ID;

    type CableBlock = PowerCableBlock;
    type Member = PowerNetworkMember;
    type Members = PowerNetworkMembers;

    fn io_blocks(machine: &Machine) -> &HashSet<IVec3> {
        &machine.energy_io_blocks
    }

    fn new_cable_block(pos: IVec3) -> PowerCableBlock {
        PowerCableBlock { pos }
    }

    fn spawn_network(commands: &mut Commands) -> Entity {
        commands
            .spawn(PowerNetwork {
                capacity_watts: 0.0,
            })
            .id()
    }
}

// -- Systems -----------------------------------------------------------------

fn generator_system(
    mut commands: Commands,
    mut block_events: MessageReader<BlockChangedMessage>,
    item_registry: Res<ItemRegistry>,
    cable_q: Query<(&PowerCableBlock, &PowerNetworkMember)>,
    gen_q: Query<(Entity, &GeneratorBlock)>,
    mut changed: MessageWriter<NetworkChanged<Power>>,
) {
    let Some(generator_vox) = item_registry.voxel_id(GENERATOR_ID) else {
        return;
    };

    let cable_pos_to_net: HashMap<IVec3, Entity> =
        cable_q.iter().map(|(c, m)| (c.pos, m.0)).collect();

    let mut affected_nets: HashSet<Entity> = HashSet::new();

    for ev in block_events.read() {
        let (removed, added) = match ev.kind {
            BlockChangeKind::Placed { voxel_id } => (None, Some(voxel_id)),
            BlockChangeKind::Removed { voxel_id } => (Some(voxel_id), None),
            BlockChangeKind::Replaced {
                old_voxel_id,
                new_voxel_id,
            } => (Some(old_voxel_id), Some(new_voxel_id)),
        };

        if removed.is_some_and(|v| v == generator_vox)
            && let Some((gen_e, _)) = gen_q.iter().find(|(_, g)| g.pos == ev.pos)
        {
            for &dir in &DIRS {
                if let Some(&net) = cable_pos_to_net.get(&(ev.pos + dir)) {
                    affected_nets.insert(net);
                    break;
                }
            }
            commands.entity(gen_e).despawn();
        }

        if added.is_some_and(|v| v == generator_vox) {
            let gen_e = commands
                .spawn(GeneratorBlock {
                    pos: ev.pos,
                    watts: GENERATOR_DEFAULT_WATTS,
                })
                .id();
            for &dir in &DIRS {
                if let Some(&net) = cable_pos_to_net.get(&(ev.pos + dir)) {
                    commands.entity(gen_e).insert(PowerNetworkMember(net));
                    affected_nets.insert(net);
                    break;
                }
            }
        }
    }

    for net in affected_nets {
        changed.write(NetworkChanged::new(net));
    }
}

fn recalc_capacity_system(
    mut events: MessageReader<NetworkChanged<Power>>,
    mut net_q: Query<(Entity, &mut PowerNetwork, &PowerNetworkMembers)>,
    gen_q: Query<&GeneratorBlock>,
) {
    let affected: HashSet<Entity> = events.read().map(|e| e.network).collect();
    if affected.is_empty() {
        return;
    }
    for (net_entity, mut network, members) in &mut net_q {
        if !affected.contains(&net_entity) {
            continue;
        }
        network.capacity_watts = members
            .0
            .iter()
            .filter_map(|&e| gen_q.get(e).ok())
            .map(|g| g.watts)
            .sum();
    }
}

fn brownout_system(
    mut events: MessageReader<NetworkChanged<Power>>,
    net_q: Query<(Entity, &PowerNetwork, &PowerNetworkMembers)>,
    recipe_graph: Res<RecipeGraph>,
    mut params: ParamSet<(
        Query<(&MachineState, Option<&MachineActivity>)>,
        Query<&mut MachineActivity>,
    )>,
) {
    let affected: HashSet<Entity> = events.read().map(|e| e.network).collect();
    if affected.is_empty() {
        return;
    }

    let net_speeds: Vec<(Vec<Entity>, f32)> = {
        let machine_q = params.p0();
        net_q
            .iter()
            .filter(|(e, _, _)| affected.contains(e))
            .map(|(_, network, members)| {
                let speed = if network.capacity_watts > 0.0 {
                    let demand: f32 = members
                        .0
                        .iter()
                        .filter_map(|&e| {
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
                (members.0.clone(), speed)
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
    use std::collections::HashSet;

    use bevy::prelude::*;

    use super::*;
    use crate::inventory::{BlockProps, ItemDef, ItemRegistry};
    use crate::machine::{
        Machine, MachineNetworkChanged, MachineState, Mirror, Orientation, Rotation,
    };
    use crate::network::NetworkChanged;
    use crate::recipe_graph::{RecipeDef, RecipeGraph};
    use crate::world::{BlockChangeKind, BlockChangedMessage};

    fn power_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<BlockChangedMessage>()
            .add_message::<MachineNetworkChanged>()
            .add_message::<NetworkChanged<Power>>()
            .insert_resource(ItemRegistry::default())
            .add_systems(
                Update,
                (
                    network::cable_placed_system::<Power>,
                    network::cable_removed_system::<Power>,
                    network::machine_membership_system::<Power>,
                    ApplyDeferred,
                    generator_system,
                    ApplyDeferred,
                    recalc_capacity_system,
                )
                    .chain()
                    .run_if(resource_exists::<ItemRegistry>),
            );
        app
    }

    fn registered_power_app() -> App {
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

    fn recipe_graph_with(energy_cost: f32, processing_time: f32) -> RecipeGraph {
        let mut recipes = std::collections::HashMap::new();
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
            materials: std::collections::HashMap::new(),
            recipes,
            terminal: String::new(),
            producers: std::collections::HashMap::new(),
            consumers: std::collections::HashMap::new(),
        }
    }

    fn write_cable_placed(app: &mut App, pos: IVec3) {
        app.world_mut().write_message(BlockChangedMessage {
            pos,
            kind: BlockChangeKind::Placed { voxel_id: 20 },
        });
    }

    fn write_cable_removed(app: &mut App, pos: IVec3) {
        app.world_mut().write_message(BlockChangedMessage {
            pos,
            kind: BlockChangeKind::Removed { voxel_id: 20 },
        });
    }

    fn network_count(app: &mut App) -> usize {
        let world = app.world_mut();
        world
            .query_filtered::<(), With<PowerNetwork>>()
            .iter(world)
            .count()
    }

    #[test]
    fn no_cables_no_networks() {
        let mut app = registered_power_app();
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn single_cable_creates_one_network() {
        let mut app = registered_power_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn two_adjacent_cables_one_network() {
        let mut app = registered_power_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        write_cable_placed(&mut app, IVec3::new(1, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn two_disconnected_cables_two_networks() {
        let mut app = registered_power_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        write_cable_placed(&mut app, IVec3::new(5, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
    }

    #[test]
    fn cable_removed_clears_network() {
        let mut app = registered_power_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        write_cable_removed(&mut app, IVec3::ZERO);
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn middle_cable_removed_splits_network() {
        let mut app = registered_power_app();
        write_cable_placed(&mut app, IVec3::new(0, 0, 0));
        app.update();
        write_cable_placed(&mut app, IVec3::new(1, 0, 0));
        app.update();
        write_cable_placed(&mut app, IVec3::new(2, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
        write_cable_removed(&mut app, IVec3::new(1, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
    }

    #[test]
    fn placing_cable_between_two_merges() {
        let mut app = registered_power_app();
        write_cable_placed(&mut app, IVec3::new(0, 0, 0));
        app.update();
        write_cable_placed(&mut app, IVec3::new(2, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
        write_cable_placed(&mut app, IVec3::new(1, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn generator_adjacent_to_cable_adds_capacity() {
        let mut app = registered_power_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::new(1, 0, 0),
            kind: BlockChangeKind::Placed { voxel_id: 21 },
        });
        app.update();
        let world = app.world_mut();
        let caps: Vec<f32> = world
            .query_filtered::<&PowerNetwork, ()>()
            .iter(world)
            .map(|n| n.capacity_watts)
            .collect();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0], GENERATOR_DEFAULT_WATTS);
    }

    #[test]
    fn generator_removed_clears_capacity() {
        let mut app = registered_power_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::new(1, 0, 0),
            kind: BlockChangeKind::Placed { voxel_id: 21 },
        });
        app.update();
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::new(1, 0, 0),
            kind: BlockChangeKind::Removed { voxel_id: 21 },
        });
        app.update();
        let world = app.world_mut();
        let caps: Vec<f32> = world
            .query_filtered::<&PowerNetwork, ()>()
            .iter(world)
            .map(|n| n.capacity_watts)
            .collect();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0], 0.0);
    }

    #[test]
    fn machine_with_energy_io_adjacent_to_cable_gets_member() {
        let mut app = registered_power_app();
        let cable_pos = IVec3::ZERO;
        let io_pos = IVec3::new(1, 0, 0);
        write_cable_placed(&mut app, cable_pos);
        let machine_entity = app
            .world_mut()
            .spawn((
                Machine {
                    machine_type: "smelter".to_string(),
                    tier: 1,
                    orientation: Orientation {
                        rotation: Rotation::North,
                        mirror: Mirror::Normal,
                    },
                    origin_pos: IVec3::ZERO,
                    blocks: HashSet::new(),
                    energy_io_blocks: [io_pos].into_iter().collect(),
                    logistics_io_blocks: HashSet::new(),
                },
                MachineState::Idle,
            ))
            .id();
        app.world_mut().write_message(MachineNetworkChanged);
        app.update();
        assert!(
            app.world()
                .get::<PowerNetworkMember>(machine_entity)
                .is_some()
        );
    }

    #[test]
    fn brownout_full_speed_when_capacity_exceeds_demand() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(recipe_graph_with(50.0, 1.0))
            .add_message::<NetworkChanged<Power>>()
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
            PowerNetworkMember(net),
        ));
        app.world_mut()
            .write_message(NetworkChanged::<Power>::new(net));
        app.update();

        let world = app.world_mut();
        let speed = world
            .query_filtered::<&MachineActivity, ()>()
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
            .insert_resource(recipe_graph_with(100.0, 1.0))
            .add_message::<NetworkChanged<Power>>()
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
            PowerNetworkMember(net),
        ));
        app.world_mut()
            .write_message(NetworkChanged::<Power>::new(net));
        app.update();

        let world = app.world_mut();
        let speed = world
            .query_filtered::<&MachineActivity, ()>()
            .iter(world)
            .next()
            .unwrap()
            .speed_factor;
        assert!((speed - 0.5).abs() < 1e-6, "expected 0.5, got {speed}");
    }

    #[test]
    fn brownout_idle_machine_not_counted_in_demand() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(recipe_graph_with(1000.0, 1.0))
            .add_message::<NetworkChanged<Power>>()
            .add_systems(Update, brownout_system);

        let net = app
            .world_mut()
            .spawn(PowerNetwork {
                capacity_watts: 1.0,
            })
            .id();
        app.world_mut()
            .spawn((MachineState::Idle, PowerNetworkMember(net)));
        app.world_mut()
            .write_message(NetworkChanged::<Power>::new(net));
        app.update();

        assert_eq!(network_count(&mut app), 1);
    }
}

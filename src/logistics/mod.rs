use std::collections::{HashMap, HashSet};

use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::inventory::ItemRegistry;
use crate::machine::{Machine, MachineActivity, MachineScanSet, MachineState};
use crate::network::{
    self, DIRS, HasPos, Logistics, NetworkKind, NetworkMemberComponent, NetworkMembersComponent,
};
use crate::recipe_graph::RecipeGraph;
use crate::research::{RESEARCH_POINTS_ID, ResearchPool, TechTreeProgress};
use crate::world::{BlockChangeKind, BlockChangedMessage};

pub struct LogisticsPlugin;

impl Plugin for LogisticsPlugin {
    fn build(&self, app: &mut App) {
        network::init_network::<Logistics>(app);
        app.add_message::<NetworkStorageChanged>().add_systems(
            Update,
            (
                ApplyDeferred,
                network::cable_placed_system::<Logistics>.run_if(resource_exists::<ItemRegistry>),
                network::cable_removed_system::<Logistics>.run_if(resource_exists::<ItemRegistry>),
                network::machine_membership_system::<Logistics>
                    .run_if(resource_exists::<ItemRegistry>),
                ApplyDeferred,
                storage_block_system.run_if(resource_exists::<ItemRegistry>),
                ApplyDeferred,
                recipe_start_system.run_if(resource_exists::<RecipeGraph>),
                ApplyDeferred,
                recipe_progress_system.run_if(resource_exists::<RecipeGraph>),
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

// -- Messages ----------------------------------------------------------------

#[derive(Clone)]
pub struct NetworkStorageChanged {
    pub network: Entity,
}

impl Message for NetworkStorageChanged {}

// -- Components --------------------------------------------------------------

#[derive(Component)]
pub struct LogisticsCableBlock {
    pub pos: IVec3,
}

impl HasPos for LogisticsCableBlock {
    fn pos(&self) -> IVec3 {
        self.pos
    }
}

#[derive(Component)]
#[relationship(relationship_target = LogisticsNetworkMembers)]
pub struct LogisticsNetworkMember(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = LogisticsNetworkMember)]
pub struct LogisticsNetworkMembers(Vec<Entity>);

impl NetworkMemberComponent for LogisticsNetworkMember {
    fn new(network: Entity) -> Self {
        LogisticsNetworkMember(network)
    }
    fn network(&self) -> Entity {
        self.0
    }
}

impl NetworkMembersComponent for LogisticsNetworkMembers {
    fn members(&self) -> &[Entity] {
        &self.0
    }
}

#[derive(Component)]
pub struct StorageBlock {
    pub pos: IVec3,
    pub items: HashMap<String, u32>,
}

#[derive(Component)]
pub struct LogisticsNetwork;

// -- NetworkKind impl --------------------------------------------------------

impl NetworkKind for Logistics {
    const CABLE_ITEM_ID: &'static str = LOGISTICS_CABLE_ID;

    type CableBlock = LogisticsCableBlock;
    type Member = LogisticsNetworkMember;
    type Members = LogisticsNetworkMembers;

    fn io_blocks(machine: &Machine) -> &HashSet<IVec3> {
        &machine.logistics_io_blocks
    }

    fn new_cable_block(pos: IVec3) -> LogisticsCableBlock {
        LogisticsCableBlock { pos }
    }

    fn spawn_network(commands: &mut Commands) -> Entity {
        commands.spawn(LogisticsNetwork).id()
    }
}

// -- Item helpers (pure, no ECS) ---------------------------------------------

pub fn has_items(
    members: &LogisticsNetworkMembers,
    storage_q: &Query<&StorageBlock>,
    item_id: &str,
    count: u32,
) -> bool {
    let available: u32 = members
        .0
        .iter()
        .filter_map(|&e| storage_q.get(e).ok())
        .map(|s| s.items.get(item_id).copied().unwrap_or(0))
        .sum();
    available >= count
}

pub fn take_items(
    members: &LogisticsNetworkMembers,
    storage_q: &mut Query<&mut StorageBlock>,
    item_id: &str,
    count: u32,
) {
    let mut remaining = count;
    for &e in &members.0 {
        if remaining == 0 {
            break;
        }
        if let Ok(mut block) = storage_q.get_mut(e) {
            let avail = *block.items.get(item_id).unwrap_or(&0);
            let take = remaining.min(avail);
            if take > 0 {
                let v = block.items.entry(item_id.to_owned()).or_insert(0);
                *v -= take;
                if *v == 0 {
                    block.items.remove(item_id);
                }
                remaining -= take;
            }
        }
    }
}

pub fn give_items(
    members: &LogisticsNetworkMembers,
    storage_q: &mut Query<&mut StorageBlock>,
    item_id: &str,
    count: u32,
) {
    for &e in &members.0 {
        if let Ok(mut block) = storage_q.get_mut(e) {
            *block.items.entry(item_id.to_owned()).or_insert(0) += count;
            return;
        }
    }
    warn!("No storage for network; {item_id} ×{count} lost");
}

// -- Systems -----------------------------------------------------------------

fn storage_block_system(
    mut commands: Commands,
    mut block_events: MessageReader<BlockChangedMessage>,
    item_registry: Res<ItemRegistry>,
    cable_q: Query<(&LogisticsCableBlock, &LogisticsNetworkMember)>,
    storage_q: Query<(Entity, &StorageBlock)>,
    mut changed: MessageWriter<NetworkStorageChanged>,
) {
    let Some(storage_vox) = item_registry.voxel_id(STORAGE_CRATE_ID) else {
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

        if removed.is_some_and(|v| v == storage_vox)
            && let Some((storage_e, _)) = storage_q.iter().find(|(_, s)| s.pos == ev.pos)
        {
            for &dir in &DIRS {
                if let Some(&net) = cable_pos_to_net.get(&(ev.pos + dir)) {
                    affected_nets.insert(net);
                    break;
                }
            }
            commands.entity(storage_e).despawn();
        }

        if added.is_some_and(|v| v == storage_vox) {
            let storage_e = commands
                .spawn(StorageBlock {
                    pos: ev.pos,
                    items: HashMap::new(),
                })
                .id();
            for &dir in &DIRS {
                if let Some(&net) = cable_pos_to_net.get(&(ev.pos + dir)) {
                    commands
                        .entity(storage_e)
                        .insert(LogisticsNetworkMember(net));
                    affected_nets.insert(net);
                    break;
                }
            }
        }
    }

    for net in affected_nets {
        changed.write(NetworkStorageChanged { network: net });
    }
}

fn recipe_start_system(
    mut commands: Commands,
    mut storage_changed: MessageReader<NetworkStorageChanged>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    machine_q: Query<
        (Entity, &Machine, &MachineState, &LogisticsNetworkMember),
        Without<MachineActivity>,
    >,
    recipe_graph: Res<RecipeGraph>,
    progress: Option<Res<TechTreeProgress>>,
    mut storage_params: ParamSet<(Query<&StorageBlock>, Query<&mut StorageBlock>)>,
) {
    let affected: HashSet<Entity> = storage_changed.read().map(|e| e.network).collect();
    if affected.is_empty() {
        return;
    }

    // Collect starts with immutable storage borrow
    let mut to_start: Vec<(Entity, String, Entity)> = Vec::new();
    {
        let storage_q = storage_params.p0();
        for (net_entity, members) in &net_q {
            if !affected.contains(&net_entity) {
                continue;
            }
            for (machine_e, machine, state, member) in &machine_q {
                if member.0 != net_entity || *state != MachineState::Idle {
                    continue;
                }
                for recipe in recipe_graph.recipes.values() {
                    if recipe.machine_type != machine.machine_type
                        || recipe.machine_tier > machine.tier
                    {
                        continue;
                    }
                    if let Some(ref prog) = progress
                        && !prog.unlocked_recipes.contains(&recipe.id)
                    {
                        continue;
                    }
                    let all_ok = recipe.inputs.iter().all(|input| {
                        has_items(members, &storage_q, &input.material, input.quantity as u32)
                    });
                    if all_ok {
                        to_start.push((machine_e, recipe.id.clone(), net_entity));
                        break;
                    }
                }
            }
        }
    }

    // Consume inputs with mutable storage borrow
    {
        let mut storage_q = storage_params.p1();
        for (machine_e, recipe_id, net_entity) in to_start {
            let Some(recipe) = recipe_graph.recipes.get(&recipe_id) else {
                continue;
            };
            if let Ok((_, members)) = net_q.get(net_entity) {
                for input in &recipe.inputs {
                    take_items(
                        members,
                        &mut storage_q,
                        &input.material,
                        input.quantity as u32,
                    );
                }
            }
            commands.entity(machine_e).insert((
                MachineActivity {
                    recipe_id,
                    progress: 0.0,
                    speed_factor: 1.0,
                },
                MachineState::Running,
            ));
            info!("Machine {:?} started recipe", machine_e);
        }
    }
}

fn recipe_progress_system(
    mut commands: Commands,
    time: Res<Time>,
    recipe_graph: Res<RecipeGraph>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    mut machine_q: Query<(
        Entity,
        &mut MachineActivity,
        &MachineState,
        &LogisticsNetworkMember,
    )>,
    mut storage_q: Query<&mut StorageBlock>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
    mut research_pool: Option<ResMut<ResearchPool>>,
) {
    let dt = time.delta_secs();

    let mut to_finish: Vec<(Entity, Vec<(String, u32)>, Entity)> = Vec::new();
    let mut progress_updates: Vec<(Entity, f32)> = Vec::new();

    for (machine_e, mut activity, state, member) in &mut machine_q {
        if *state != MachineState::Running {
            continue;
        }
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
            to_finish.push((machine_e, outputs, member.0));
        } else {
            activity.progress = new_progress;
            progress_updates.push((machine_e, new_progress));
        }
    }
    let _ = progress_updates; // already applied above via &mut activity

    for (machine_e, outputs, net_entity) in to_finish {
        if let Ok((_, members)) = net_q.get(net_entity) {
            for (item_id, count) in outputs {
                if item_id == RESEARCH_POINTS_ID {
                    if let Some(ref mut pool) = research_pool {
                        pool.points += count as f32;
                        info!("Research pool +{} points (total: {})", count, pool.points);
                    }
                } else if count > 0 {
                    give_items(members, &mut storage_q, &item_id, count);
                }
            }
            storage_changed.write(NetworkStorageChanged {
                network: net_entity,
            });
        }
        commands
            .entity(machine_e)
            .remove::<MachineActivity>()
            .insert(MachineState::Idle);
        info!("Machine {:?} finished recipe", machine_e);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::collections::HashSet;

    use bevy::prelude::*;

    use super::*;
    use crate::inventory::{BlockProps, ItemDef, ItemRegistry};
    use crate::machine::{
        Machine, MachineActivity, MachineNetworkChanged, MachineState, Mirror, Orientation,
        Rotation,
    };
    use crate::network::NetworkChanged;
    use crate::recipe_graph::{ItemStack, RecipeDef, RecipeGraph};
    use crate::world::{BlockChangeKind, BlockChangedMessage};

    fn logistics_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<BlockChangedMessage>()
            .add_message::<MachineNetworkChanged>()
            .add_message::<NetworkChanged<Logistics>>()
            .add_message::<NetworkStorageChanged>()
            .insert_resource(ItemRegistry::default())
            .add_systems(
                Update,
                (
                    network::cable_placed_system::<Logistics>,
                    network::cable_removed_system::<Logistics>,
                    network::machine_membership_system::<Logistics>,
                    ApplyDeferred,
                    storage_block_system,
                    ApplyDeferred,
                )
                    .chain()
                    .run_if(resource_exists::<ItemRegistry>),
            );
        app
    }

    fn registered_logistics_app() -> App {
        let mut app = logistics_app();
        {
            let mut reg = app.world_mut().resource_mut::<ItemRegistry>();
            reg.register(ItemDef {
                id: LOGISTICS_CABLE_ID.to_string(),
                name: LOGISTICS_CABLE_ID.to_string(),
                block: Some(BlockProps {
                    voxel_id: 10,
                    hardness: 1.0,
                }),
            });
            reg.register(ItemDef {
                id: STORAGE_CRATE_ID.to_string(),
                name: STORAGE_CRATE_ID.to_string(),
                block: Some(BlockProps {
                    voxel_id: 11,
                    hardness: 1.0,
                }),
            });
        }
        app
    }

    fn network_count(app: &mut App) -> usize {
        let world = app.world_mut();
        world
            .query_filtered::<(), With<LogisticsNetwork>>()
            .iter(world)
            .count()
    }

    fn write_cable_placed(app: &mut App, pos: IVec3) {
        app.world_mut().write_message(BlockChangedMessage {
            pos,
            kind: BlockChangeKind::Placed { voxel_id: 10 },
        });
    }

    fn write_cable_removed(app: &mut App, pos: IVec3) {
        app.world_mut().write_message(BlockChangedMessage {
            pos,
            kind: BlockChangeKind::Removed { voxel_id: 10 },
        });
    }

    #[test]
    fn no_cables_no_networks() {
        let mut app = registered_logistics_app();
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn single_cable_creates_one_network() {
        let mut app = registered_logistics_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn two_adjacent_cables_one_network() {
        let mut app = registered_logistics_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        write_cable_placed(&mut app, IVec3::new(1, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn two_disconnected_cables_two_networks() {
        let mut app = registered_logistics_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        write_cable_placed(&mut app, IVec3::new(5, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
    }

    #[test]
    fn cable_removed_clears_network() {
        let mut app = registered_logistics_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        write_cable_removed(&mut app, IVec3::ZERO);
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn middle_cable_removed_splits_network() {
        let mut app = registered_logistics_app();
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
        let mut app = registered_logistics_app();
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
    fn storage_adjacent_to_cable_becomes_member() {
        let mut app = registered_logistics_app();
        write_cable_placed(&mut app, IVec3::ZERO);
        app.update();
        app.world_mut().write_message(BlockChangedMessage {
            pos: IVec3::new(1, 0, 0),
            kind: BlockChangeKind::Placed { voxel_id: 11 },
        });
        app.update();
        let world = app.world_mut();
        let count = world
            .query_filtered::<(), With<StorageBlock>>()
            .iter(world)
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn machine_with_logistics_io_adjacent_to_cable_gets_member() {
        let mut app = registered_logistics_app();
        let cable_pos = IVec3::ZERO;
        let io_pos = IVec3::new(1, 0, 0);
        write_cable_placed(&mut app, cable_pos);
        let machine_entity = app
            .world_mut()
            .spawn((
                Machine {
                    machine_type: "furnace".to_string(),
                    tier: 1,
                    orientation: Orientation {
                        rotation: Rotation::North,
                        mirror: Mirror::Normal,
                    },
                    origin_pos: IVec3::ZERO,
                    blocks: HashSet::new(),
                    energy_io_blocks: HashSet::new(),
                    logistics_io_blocks: [io_pos].into_iter().collect(),
                },
                MachineState::Idle,
            ))
            .id();
        app.world_mut().write_message(MachineNetworkChanged);
        app.update();
        assert!(
            app.world()
                .get::<LogisticsNetworkMember>(machine_entity)
                .is_some()
        );
    }

    fn test_recipe_def(
        machine_type: &str,
        inputs: &[(&str, f32)],
        outputs: &[(&str, f32)],
    ) -> RecipeDef {
        RecipeDef {
            id: "test_recipe".to_string(),
            inputs: inputs
                .iter()
                .map(|(m, q)| ItemStack {
                    material: m.to_string(),
                    quantity: *q,
                })
                .collect(),
            outputs: outputs
                .iter()
                .map(|(m, q)| ItemStack {
                    material: m.to_string(),
                    quantity: *q,
                })
                .collect(),
            byproducts: vec![],
            machine_type: machine_type.to_string(),
            machine_tier: 1,
            processing_time: 1.0,
            energy_cost: 0.0,
        }
    }

    fn single_recipe_graph(recipe: RecipeDef) -> RecipeGraph {
        let recipe_id = recipe.id.clone();
        let mut recipes = HashMap::new();
        recipes.insert(recipe_id, recipe);
        RecipeGraph {
            materials: HashMap::new(),
            recipes,
            terminal: String::new(),
            producers: HashMap::new(),
            consumers: HashMap::new(),
        }
    }

    fn bare_machine(machine_type: &str) -> Machine {
        Machine {
            machine_type: machine_type.to_string(),
            tier: 1,
            orientation: Orientation {
                rotation: Rotation::North,
                mirror: Mirror::Normal,
            },
            origin_pos: IVec3::ZERO,
            blocks: HashSet::new(),
            energy_io_blocks: HashSet::new(),
            logistics_io_blocks: HashSet::new(),
        }
    }

    fn recipe_io_app(rg: RecipeGraph) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<NetworkChanged<Logistics>>()
            .add_message::<NetworkStorageChanged>()
            .insert_resource(rg)
            .add_systems(
                Update,
                (recipe_start_system, ApplyDeferred, recipe_progress_system).chain(),
            );
        app
    }

    #[test]
    fn idle_machine_starts_recipe_when_inputs_available() {
        let rg = single_recipe_graph(test_recipe_def(
            "furnace",
            &[("iron", 1.0)],
            &[("copper", 1.0)],
        ));
        let mut app = recipe_io_app(rg);

        let net_entity = app.world_mut().spawn(LogisticsNetwork).id();
        let storage_e = app
            .world_mut()
            .spawn((
                StorageBlock {
                    pos: IVec3::ZERO,
                    items: [("iron".to_owned(), 10u32)].into_iter().collect(),
                },
                LogisticsNetworkMember(net_entity),
            ))
            .id();
        let machine_entity = app
            .world_mut()
            .spawn((
                bare_machine("furnace"),
                MachineState::Idle,
                LogisticsNetworkMember(net_entity),
            ))
            .id();

        app.world_mut().write_message(NetworkStorageChanged {
            network: net_entity,
        });
        app.update();

        let world = app.world();
        assert_eq!(
            *world.get::<MachineState>(machine_entity).unwrap(),
            MachineState::Running
        );
        assert!(world.get::<MachineActivity>(machine_entity).is_some());
        let storage = world.get::<StorageBlock>(storage_e).unwrap();
        assert_eq!(storage.items.get("iron").copied().unwrap_or(0), 9);
    }

    #[test]
    fn running_machine_completes_and_stores_output() {
        let rg = single_recipe_graph(test_recipe_def("furnace", &[], &[("copper", 2.0)]));
        let mut app = recipe_io_app(rg);

        let net_entity = app.world_mut().spawn(LogisticsNetwork).id();
        let storage_e = app
            .world_mut()
            .spawn((
                StorageBlock {
                    pos: IVec3::ZERO,
                    items: HashMap::new(),
                },
                LogisticsNetworkMember(net_entity),
            ))
            .id();
        let machine_entity = app
            .world_mut()
            .spawn((
                bare_machine("furnace"),
                MachineState::Running,
                MachineActivity {
                    recipe_id: "test_recipe".to_string(),
                    progress: 10.0,
                    speed_factor: 1.0,
                },
                LogisticsNetworkMember(net_entity),
            ))
            .id();

        app.update();

        let world = app.world();
        assert_eq!(
            *world.get::<MachineState>(machine_entity).unwrap(),
            MachineState::Idle
        );
        assert!(world.get::<MachineActivity>(machine_entity).is_none());
        let storage = world.get::<StorageBlock>(storage_e).unwrap();
        assert_eq!(storage.items.get("copper").copied().unwrap_or(0), 2);
    }

    #[test]
    fn running_machine_updates_progress_when_not_done() {
        let rg = single_recipe_graph(test_recipe_def("furnace", &[], &[]));
        let mut app = recipe_io_app(rg);

        let net_entity = app.world_mut().spawn(LogisticsNetwork).id();
        app.world_mut().spawn((
            StorageBlock {
                pos: IVec3::ZERO,
                items: HashMap::new(),
            },
            LogisticsNetworkMember(net_entity),
        ));
        let machine_entity = app
            .world_mut()
            .spawn((
                bare_machine("furnace"),
                MachineState::Running,
                MachineActivity {
                    recipe_id: "test_recipe".to_string(),
                    progress: 0.5,
                    speed_factor: 1.0,
                },
                LogisticsNetworkMember(net_entity),
            ))
            .id();

        app.update();

        let world = app.world();
        assert_eq!(
            *world.get::<MachineState>(machine_entity).unwrap(),
            MachineState::Running
        );
        assert!(world.get::<MachineActivity>(machine_entity).is_some());
    }
}

use std::collections::{HashMap, HashSet};

use avian3d::prelude::{Collider, RigidBody, Sensor};
use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy::prelude::*;

use rand::SeedableRng;
use rand_pcg::Pcg64;

use crate::drone::{sample_ore, yield_factor};
use crate::machine::{IoPortMarker, Machine, MachineActivity, MachineState, MinerMachine};
use crate::network::{
    HasEndpoints, Logistics, NetworkKind, NetworkMemberComponent, NetworkMembersComponent,
    NetworkPlugin, NetworkSystems, route_avoiding,
};
use crate::recipe_graph::RecipeGraph;
use crate::research::{RESEARCH_POINTS_ID, ResearchPool, TechTreeProgress};
use crate::world::generation::OreDeposit;
use crate::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct LogisticsSimSystems;

/// Simulation-only plugin: no state gates, no visual assets.
/// Suitable for integration tests with `MinimalPlugins`.
pub struct LogisticsSimPlugin;

impl Plugin for LogisticsSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkPlugin::<Logistics>::default());
        app.add_message::<NetworkStorageChanged>().add_systems(
            Update,
            (
                ApplyDeferred, // flush cable entities spawned by NetworkPlugin
                storage_unit_system,
                ApplyDeferred, // flush storage membership inserts
                miner_tick_system,
                recipe_start_system.run_if(resource_exists::<RecipeGraph>),
                recipe_progress_system.run_if(resource_exists::<RecipeGraph>),
            )
                .chain()
                .after(NetworkSystems::of::<Logistics>())
                .in_set(LogisticsSimSystems),
        );
    }
}

pub struct LogisticsPlugin;

impl Plugin for LogisticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LogisticsSimPlugin);
        app.configure_sets(
            Update,
            NetworkSystems::of::<Logistics>().run_if(in_state(crate::GameState::Playing)),
        );
        app.configure_sets(
            Update,
            LogisticsSimSystems
                .in_set(crate::GameSystems::Simulation)
                .run_if(in_state(crate::GameState::Playing)),
        );
        app.add_systems(Startup, setup_logistics_visuals);
        app.add_systems(
            Update,
            (add_cable_visuals, add_storage_visuals, add_storage_port_visuals)
                .in_set(crate::GameSystems::Rendering)
                .run_if(in_state(crate::GameState::Playing)),
        );
    }
}

const LOGISTICS_CABLE_ID: &str = "logistics_cable";
const STORAGE_CRATE_ID: &str = "storage_crate";
const CABLE_RADIUS: f32 = 0.05;

// -- Visual assets -----------------------------------------------------------

#[derive(Resource)]
struct LogisticsVisualAssets {
    tube: Handle<Mesh>,
    joint: Handle<Mesh>,
    cable_material: Handle<StandardMaterial>,
    storage_mesh: Handle<Mesh>,
    storage_material: Handle<StandardMaterial>,
    port_mesh: Handle<Mesh>,
    port_material: Handle<StandardMaterial>,
}

fn setup_logistics_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(LogisticsVisualAssets {
        tube: meshes.add(Cylinder::new(CABLE_RADIUS, 1.0)),
        joint: meshes.add(Sphere::new(CABLE_RADIUS)),
        cable_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.7, 0.3),
            ..default()
        }),
        storage_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        storage_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.6, 0.65),
            ..default()
        }),
        port_mesh: meshes.add(Sphere::new(0.15)),
        port_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.9, 0.2),
            emissive: LinearRgba::new(0.0, 0.5, 0.0, 1.0),
            ..default()
        }),
    });
}

fn add_cable_visuals(
    mut commands: Commands,
    added: Query<(Entity, &LogisticsCableSegment), Added<LogisticsCableSegment>>,
    assets: Option<Res<LogisticsVisualAssets>>,
    machine_q: Query<(&Machine, &Transform)>,
) {
    let Some(assets) = assets else { return };
    for (entity, seg) in &added {
        commands
            .entity(entity)
            .insert((Transform::default(), Visibility::default()))
            .with_children(|parent| {
                for window in seg.path.windows(2) {
                    let [a_pos, b_pos] = window else { continue };
                    let a = a_pos.as_vec3() + Vec3::splat(0.5);
                    let b = b_pos.as_vec3() + Vec3::splat(0.5);
                    let dir = b - a;
                    let rotation = if dir.x.abs() > 0.5 {
                        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
                    } else if dir.z.abs() > 0.5 {
                        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
                    } else {
                        Quat::IDENTITY
                    };
                    parent.spawn((
                        Mesh3d(assets.tube.clone()),
                        MeshMaterial3d(assets.cable_material.clone()),
                        Transform::from_translation((a + b) * 0.5).with_rotation(rotation),
                        Collider::cylinder(CABLE_RADIUS, 1.0),
                        Sensor,
                    ));
                }
                for window in seg.path.windows(3) {
                    let [prev, curr, next] = window else { continue };
                    let prev_dir = *curr - *prev;
                    let next_dir = *next - *curr;
                    if prev_dir != next_dir {
                        parent.spawn((
                            Mesh3d(assets.joint.clone()),
                            MeshMaterial3d(assets.cable_material.clone()),
                            Transform::from_translation(curr.as_vec3() + Vec3::splat(0.5)),
                        ));
                    }
                }
                // Connector tubes from each machine body to its port
                for port in [seg.from, seg.to] {
                    let port_center = port + Vec3::splat(0.5);
                    let port_key = port.round().as_ivec3();
                    if let Some(mpos) = machine_q
                        .iter()
                        .find(|(m, _)| {
                            m.logistics_ports
                                .iter()
                                .any(|p| p.round().as_ivec3() == port_key)
                        })
                        .map(|(_, t)| t.translation)
                    {
                        let diff = port_center - mpos;
                        let length = diff.length();
                        if length > 1e-4 {
                            let rotation = Quat::from_rotation_arc(Vec3::Y, diff / length);
                            parent.spawn((
                                Mesh3d(assets.tube.clone()),
                                MeshMaterial3d(assets.cable_material.clone()),
                                Transform::from_translation((mpos + port_center) * 0.5)
                                    .with_rotation(rotation)
                                    .with_scale(Vec3::new(1.0, length, 1.0)),
                            ));
                        }
                    }
                }
            });
    }
}

fn add_storage_visuals(
    mut commands: Commands,
    added: Query<(Entity, &StorageUnit), Added<StorageUnit>>,
    assets: Option<Res<LogisticsVisualAssets>>,
) {
    let Some(assets) = assets else { return };
    for (entity, unit) in &added {
        commands.entity(entity).insert((
            Mesh3d(assets.storage_mesh.clone()),
            MeshMaterial3d(assets.storage_material.clone()),
            Transform::from_translation(unit.pos + Vec3::splat(0.5)),
            RigidBody::Static,
            Collider::cuboid(0.5, 0.5, 0.5),
        ));
    }
}

fn add_storage_port_visuals(
    mut commands: Commands,
    added: Query<(Entity, &IoPortMarker), Added<IoPortMarker>>,
    storage_q: Query<(), With<StorageUnit>>,
    assets: Option<Res<LogisticsVisualAssets>>,
) {
    let Some(assets) = assets else { return };
    for (port_e, marker) in &added {
        if storage_q.contains(marker.owner) {
            commands.entity(port_e).insert((
                Mesh3d(assets.port_mesh.clone()),
                MeshMaterial3d(assets.port_material.clone()),
                Visibility::default(),
            ));
        }
    }
}

// -- Messages ----------------------------------------------------------------

#[derive(Clone)]
pub struct NetworkStorageChanged {
    pub network: Entity,
}

impl Message for NetworkStorageChanged {}

// -- Components --------------------------------------------------------------

#[derive(Component)]
pub struct LogisticsCableSegment {
    pub from: Vec3,
    pub to: Vec3,
    pub path: Vec<IVec3>,
}

impl HasEndpoints for LogisticsCableSegment {
    fn endpoints(&self) -> [Vec3; 2] {
        [self.from, self.to]
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
pub struct StorageUnit {
    pub pos: Vec3,
    pub items: HashMap<String, u32>,
}

#[derive(Component)]
pub struct LogisticsNetwork;

// -- NetworkKind impl --------------------------------------------------------

impl NetworkKind for Logistics {
    const CABLE_ITEM_ID: &'static str = LOGISTICS_CABLE_ID;

    type CableSegment = LogisticsCableSegment;
    type Member = LogisticsNetworkMember;
    type Members = LogisticsNetworkMembers;

    fn io_ports(machine: &Machine) -> &[Vec3] {
        &machine.logistics_ports
    }

    fn new_cable_segment(
        from: Vec3,
        to: Vec3,
        is_blocked: &dyn Fn(IVec3) -> bool,
    ) -> LogisticsCableSegment {
        LogisticsCableSegment {
            from,
            to,
            path: route_avoiding(from.round().as_ivec3(), to.round().as_ivec3(), is_blocked),
        }
    }

    fn spawn_network(commands: &mut Commands) -> Entity {
        commands.spawn(LogisticsNetwork).id()
    }
}

// -- Item helpers (pure, no ECS) ---------------------------------------------

pub fn has_items(
    members: &LogisticsNetworkMembers,
    storage_q: &Query<&StorageUnit>,
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
    storage_q: &mut Query<&mut StorageUnit>,
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
    storage_q: &mut Query<&mut StorageUnit>,
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

pub fn storage_unit_system(
    mut commands: Commands,
    mut world_events: MessageReader<WorldObjectEvent>,
    mut cable_events: MessageReader<CableConnectionEvent>,
    cable_q: Query<(&LogisticsCableSegment, &LogisticsNetworkMember)>,
    storage_q: Query<(Entity, &StorageUnit)>,
    port_q: Query<(Entity, &IoPortMarker)>,
    mut changed: MessageWriter<NetworkStorageChanged>,
) {
    // endpoint → network (keys are rounded IVec3); includes cables flushed by ApplyDeferred
    let endpoint_to_net: HashMap<IVec3, Entity> = cable_q
        .iter()
        .flat_map(|(seg, m)| seg.endpoints().map(|ep| (ep.round().as_ivec3(), m.0)))
        .collect();

    let mut affected_nets: HashSet<Entity> = HashSet::new();

    for ev in world_events.read() {
        if ev.item_id != STORAGE_CRATE_ID {
            continue;
        }

        let grid_pos = ev.pos.round().as_ivec3();

        if ev.kind == WorldObjectKind::Removed
            && let Some((storage_e, _)) = storage_q
                .iter()
                .find(|(_, s)| s.pos.round().as_ivec3() == grid_pos)
        {
            for &dir in &crate::network::DIRS {
                if let Some(&net) = endpoint_to_net.get(&(grid_pos + dir)) {
                    affected_nets.insert(net);
                    break;
                }
            }
            for (port_e, marker) in &port_q {
                if marker.owner == storage_e {
                    commands.entity(port_e).despawn();
                }
            }
            commands.entity(storage_e).despawn();
        }

        if ev.kind == WorldObjectKind::Placed {
            let storage_e = commands.spawn_empty().id();
            // Spawn logistics port markers at each horizontal face for cable snapping
            for offset in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                commands.spawn((
                    IoPortMarker { owner: storage_e },
                    Transform::from_translation(ev.pos + offset.as_vec3()),
                    Collider::sphere(0.4),
                    Sensor,
                ));
            }
            commands.entity(storage_e).insert(StorageUnit {
                pos: ev.pos,
                items: HashMap::new(),
            });
            for &dir in &crate::network::DIRS {
                if let Some(&net) = endpoint_to_net.get(&(grid_pos + dir)) {
                    commands
                        .entity(storage_e)
                        .insert(LogisticsNetworkMember(net));
                    affected_nets.insert(net);
                    break;
                }
            }
        }
    }

    // When a logistics cable is placed, re-scan all existing storage units for adjacency.
    // ApplyDeferred before this system ensures the new cable entity is visible in cable_q.
    let cable_placed = cable_events
        .read()
        .any(|ev| ev.kind == WorldObjectKind::Placed && ev.item_id == LOGISTICS_CABLE_ID);

    if cable_placed {
        for (storage_e, unit) in &storage_q {
            let grid_pos = unit.pos.round().as_ivec3();
            for &dir in &crate::network::DIRS {
                if let Some(&net) = endpoint_to_net.get(&(grid_pos + dir)) {
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

pub fn recipe_start_system(
    mut commands: Commands,
    mut storage_changed: MessageReader<NetworkStorageChanged>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    machine_q: Query<
        (Entity, &Machine, &MachineState, &LogisticsNetworkMember),
        Without<MachineActivity>,
    >,
    recipe_graph: Res<RecipeGraph>,
    progress: Option<Res<TechTreeProgress>>,
    mut storage_params: ParamSet<(Query<&StorageUnit>, Query<&mut StorageUnit>)>,
) {
    let affected: HashSet<Entity> = storage_changed.read().map(|e| e.network).collect();
    if affected.is_empty() {
        return;
    }

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
                        has_items(members, &storage_q, &input.item, input.quantity as u32)
                    });
                    if all_ok {
                        to_start.push((machine_e, recipe.id.clone(), net_entity));
                        break;
                    }
                }
            }
        }
    }

    {
        let mut storage_q = storage_params.p1();
        for (machine_e, recipe_id, net_entity) in to_start {
            let Some(recipe) = recipe_graph.recipes.get(&recipe_id) else {
                continue;
            };
            if let Ok((_, members)) = net_q.get(net_entity) {
                for input in &recipe.inputs {
                    take_items(members, &mut storage_q, &input.item, input.quantity as u32);
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

pub fn recipe_progress_system(
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
    mut storage_q: Query<&mut StorageUnit>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
    mut research_pool: Option<ResMut<ResearchPool>>,
) {
    let dt = time.delta_secs();

    let mut to_finish: Vec<(Entity, Vec<(String, u32)>, Entity)> = Vec::new();

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
                .map(|o| (o.item.clone(), o.quantity as u32))
                .collect();
            to_finish.push((machine_e, outputs, member.0));
        } else {
            activity.progress = new_progress;
        }
    }

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

// -- Automatic miner ---------------------------------------------------------

/// Advance one miner by `dt` seconds. Returns sampled ore id if a whole item was produced.
fn tick_miner(deposit: &mut OreDeposit, accumulator: &mut f32, dt: f32) -> Option<String> {
    let yf = yield_factor(deposit.total_extracted, deposit.depletion_seed);
    *accumulator += yf * dt;
    if *accumulator >= 1.0 {
        *accumulator -= 1.0;
        let rng_seed = deposit.depletion_seed ^ deposit.total_extracted.to_bits() as u64;
        let mut rng = Pcg64::seed_from_u64(rng_seed);
        if let Some(ore_id) = sample_ore(&deposit.ores, &mut rng) {
            deposit.total_extracted += 1.0;
            return Some(ore_id);
        }
    }
    None
}

fn miner_tick_system(
    time: Res<Time>,
    mut miner_q: Query<(&mut MinerMachine, &LogisticsNetworkMember)>,
    mut deposit_q: Query<&mut OreDeposit>,
    net_q: Query<&LogisticsNetworkMembers>,
    mut storage_q: Query<&mut StorageUnit>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
) {
    let dt = time.delta_secs();
    for (mut miner, member) in &mut miner_q {
        let Ok(mut deposit) = deposit_q.get_mut(miner.deposit) else {
            continue;
        };
        if let Some(ore_id) = tick_miner(&mut deposit, &mut miner.accumulator, dt) {
            let Ok(members) = net_q.get(member.0) else {
                continue;
            };
            give_items(members, &mut storage_q, &ore_id, 1);
            storage_changed.write(NetworkStorageChanged { network: member.0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bevy::prelude::*;

    use super::*;
    use crate::machine::{
        Machine, MachineActivity, MachineNetworkChanged, MachineState, Mirror, Orientation,
        Rotation,
    };
    use crate::network::{NetworkChanged, NetworkPlugin, NetworkSystems};
    use crate::recipe_graph::{ConcreteRecipe, ItemStack, RecipeGraph};
    use crate::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

    fn logistics_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<CableConnectionEvent>()
            .add_message::<MachineNetworkChanged>()
            .add_message::<NetworkStorageChanged>()
            .add_plugins(NetworkPlugin::<Logistics>::default())
            .add_systems(
                Update,
                storage_unit_system.after(NetworkSystems::of::<Logistics>()),
            );
        app
    }

    fn connect_cable(app: &mut App, from: Vec3, to: Vec3) {
        app.world_mut().write_message(CableConnectionEvent {
            from,
            to,
            item_id: LOGISTICS_CABLE_ID.to_string(),
            kind: WorldObjectKind::Placed,
        });
    }

    #[test]
    fn storage_adjacent_to_cable_endpoint_becomes_member() {
        let mut app = logistics_app();
        // Cable endpoint at (0,0,0) — storage at (1,0,0) is adjacent
        connect_cable(&mut app, Vec3::ZERO, Vec3::new(0.0, 0.0, 5.0));
        app.update();
        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.0, 0.0),
            item_id: STORAGE_CRATE_ID.to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();
        let world = app.world_mut();
        let count = world
            .query_filtered::<(), With<StorageUnit>>()
            .iter(world)
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn machine_with_logistics_port_matching_cable_endpoint_gets_member() {
        let mut app = logistics_app();
        let io_pos = Vec3::new(1.0, 0.0, 0.0);
        // Cable endpoint at io_pos — machine with port there should join
        connect_cable(&mut app, io_pos, Vec3::new(5.0, 0.0, 0.0));
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
                    energy_ports: vec![],
                    logistics_ports: vec![io_pos],
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
    ) -> ConcreteRecipe {
        ConcreteRecipe {
            id: "test_recipe".to_string(),
            inputs: inputs
                .iter()
                .map(|(m, q)| ItemStack {
                    item: m.to_string(),
                    quantity: *q,
                })
                .collect(),
            outputs: outputs
                .iter()
                .map(|(m, q)| ItemStack {
                    item: m.to_string(),
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

    fn single_recipe_graph(recipe: ConcreteRecipe) -> RecipeGraph {
        let recipe_id = recipe.id.clone();
        let mut recipes = HashMap::new();
        recipes.insert(recipe_id, recipe);
        RecipeGraph {
            materials: HashMap::new(),
            form_groups: HashMap::new(),
            templates: HashMap::new(),
            items: HashMap::new(),
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
            energy_ports: vec![],
            logistics_ports: vec![],
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
                StorageUnit {
                    pos: Vec3::ZERO,
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
        let storage = world.get::<StorageUnit>(storage_e).unwrap();
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
                StorageUnit {
                    pos: Vec3::ZERO,
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
        let storage = world.get::<StorageUnit>(storage_e).unwrap();
        assert_eq!(storage.items.get("copper").copied().unwrap_or(0), 2);
    }

    #[test]
    fn running_machine_updates_progress_when_not_done() {
        let rg = single_recipe_graph(test_recipe_def("furnace", &[], &[]));
        let mut app = recipe_io_app(rg);

        let net_entity = app.world_mut().spawn(LogisticsNetwork).id();
        app.world_mut().spawn((
            StorageUnit {
                pos: Vec3::ZERO,
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

    #[test]
    fn tick_miner_outputs_ore_when_accumulator_overflows() {
        use crate::world::generation::OreDeposit;
        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("iron_ore".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 0,
        };
        // yield_factor(0.0, 0) = 1.0; acc 0.5 + 1.0*0.6 = 1.1 → outputs
        let mut acc = 0.5;
        let result = tick_miner(&mut deposit, &mut acc, 0.6);
        assert_eq!(result.as_deref(), Some("iron_ore"));
        assert!(acc < 1.0, "accumulator drained after output");
        assert_eq!(deposit.total_extracted, 1.0);
    }

    #[test]
    fn tick_miner_no_output_below_threshold() {
        use crate::world::generation::OreDeposit;
        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("iron_ore".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 0,
        };
        let mut acc = 0.0;
        let result = tick_miner(&mut deposit, &mut acc, 0.5);
        assert!(result.is_none());
        assert_eq!(deposit.total_extracted, 0.0);
    }

    #[test]
    fn tick_miner_yield_floor_still_produces() {
        use crate::world::generation::OreDeposit;
        // After huge extraction, yield ≈ floor (> 0); given enough dt still produces
        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("iron_ore".to_string(), 1.0)],
            total_extracted: 1_000_000.0,
            depletion_seed: 99,
        };
        // floor = 0.1 + 0.099 = 0.199; 10s * 0.199 = 1.99 >= 1.0
        let mut acc = 0.0;
        let result = tick_miner(&mut deposit, &mut acc, 10.0);
        assert!(result.is_some(), "floor yield must still produce over time");
    }
}

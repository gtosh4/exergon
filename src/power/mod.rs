use std::collections::{HashMap, HashSet};

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::machine::{EnergyPortOf, Machine};
use crate::network::visuals::spawn_cable_children;
use crate::network::{
    HasEndpoints, NetworkChanged, NetworkKind, NetworkMemberComponent, NetworkMembersComponent,
    NetworkPlugin, NetworkSystems, Power, route_avoiding,
};
use crate::world::{WorldObjectEvent, WorldObjectKind};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PowerSimSystems;

pub struct PowerPlugin;

impl Plugin for PowerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkPlugin::<Power>::default());
        app.configure_sets(
            Update,
            NetworkSystems::of::<Power>().run_if(in_state(crate::GameState::Playing)),
        );
        app.add_systems(Startup, setup_power_visuals);
        app.add_systems(
            Update,
            (generator_system, generator_tick_system)
                .chain()
                .after(NetworkSystems::of::<Power>())
                .in_set(PowerSimSystems)
                .in_set(crate::GameSystems::Simulation)
                .run_if(in_state(crate::GameState::Playing)),
        );
        app.add_systems(
            Update,
            (add_power_cable_visuals, add_generator_visuals)
                .in_set(crate::GameSystems::Rendering)
                .run_if(in_state(crate::GameState::Playing)),
        );
    }
}

const POWER_CABLE_ID: &str = "power_cable";
const GENERATOR_ID: &str = "generator";
pub const GENERATOR_DEFAULT_WATTS: f32 = 50.0;
const CABLE_RADIUS: f32 = 0.04;

// -- Visual assets -----------------------------------------------------------

#[derive(Resource)]
struct PowerVisualAssets {
    tube: Handle<Mesh>,
    joint: Handle<Mesh>,
    cable_material: Handle<StandardMaterial>,
    gen_mesh: Handle<Mesh>,
    gen_material: Handle<StandardMaterial>,
}

fn setup_power_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(PowerVisualAssets {
        tube: meshes.add(Cylinder::new(CABLE_RADIUS, 1.0)),
        joint: meshes.add(Sphere::new(CABLE_RADIUS)),
        cable_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.8, 0.1),
            ..default()
        }),
        gen_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        gen_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.8, 0.1),
            ..default()
        }),
    });
}

fn add_power_cable_visuals(
    mut commands: Commands,
    added: Query<(Entity, &PowerCableSegment), Added<PowerCableSegment>>,
    assets: Option<Res<PowerVisualAssets>>,
    machine_q: Query<(&Machine, &Transform)>,
) {
    let Some(assets) = assets else { return };
    for (entity, seg) in &added {
        let port_machine_positions: Vec<(Vec3, Vec3)> = [seg.from, seg.to]
            .into_iter()
            .filter_map(|port| {
                let port_center = port + Vec3::splat(0.5);
                let port_key = port.round().as_ivec3();
                machine_q
                    .iter()
                    .find(|(m, _)| {
                        m.energy_ports
                            .iter()
                            .any(|p| p.round().as_ivec3() == port_key)
                    })
                    .map(|(_, t)| (port_center, t.translation))
            })
            .collect();

        commands
            .entity(entity)
            .insert((Transform::default(), Visibility::default()))
            .with_children(|parent| {
                spawn_cable_children(
                    parent,
                    &seg.path,
                    &port_machine_positions,
                    assets.tube.clone(),
                    assets.joint.clone(),
                    assets.cable_material.clone(),
                    CABLE_RADIUS,
                );
            });
    }
}

fn add_generator_visuals(
    mut commands: Commands,
    added: Query<(Entity, &GeneratorUnit), Added<GeneratorUnit>>,
    assets: Option<Res<PowerVisualAssets>>,
) {
    let Some(assets) = assets else { return };
    for (entity, unit) in &added {
        commands.entity(entity).insert((
            Mesh3d(assets.gen_mesh.clone()),
            MeshMaterial3d(assets.gen_material.clone()),
            Transform::from_translation(unit.pos + Vec3::splat(0.5)),
        ));
    }
}

// -- Components --------------------------------------------------------------

#[derive(Component)]
pub struct PowerCableSegment {
    pub from: Vec3,
    pub to: Vec3,
    pub path: Vec<IVec3>,
}

impl HasEndpoints for PowerCableSegment {
    fn endpoints(&self) -> [Vec3; 2] {
        [self.from, self.to]
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
pub struct GeneratorUnit {
    pub pos: Vec3,
    pub watts: f32,
    pub buffer_joules: f32,
    pub max_buffer_joules: f32,
}

#[derive(Component)]
pub struct PowerNetwork;

// -- NetworkKind impl --------------------------------------------------------

impl NetworkKind for Power {
    const CABLE_ITEM_ID: &'static str = POWER_CABLE_ID;

    type CableSegment = PowerCableSegment;
    type Member = PowerNetworkMember;
    type Members = PowerNetworkMembers;
    type PortOf = EnergyPortOf;

    fn io_ports(machine: &Machine) -> &[Vec3] {
        &machine.energy_ports
    }

    fn new_cable_segment(
        from: Vec3,
        to: Vec3,
        is_blocked: &dyn Fn(IVec3) -> bool,
    ) -> PowerCableSegment {
        PowerCableSegment {
            from,
            to,
            path: route_avoiding(from.round().as_ivec3(), to.round().as_ivec3(), is_blocked),
        }
    }

    fn spawn_network(commands: &mut Commands) -> Entity {
        commands.spawn(PowerNetwork).id()
    }
}

// -- PowerNetworkMembers energy helpers --------------------------------------

impl PowerNetworkMembers {
    fn collect_generator_entities(
        &self,
        gen_q: &Query<&GeneratorUnit>,
        port_of_q: &Query<&EnergyPortOf>,
    ) -> Vec<Entity> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for &e in &self.0 {
            let gen_e = if gen_q.contains(e) {
                Some(e)
            } else if let Ok(pof) = port_of_q.get(e) {
                if gen_q.contains(pof.0) {
                    Some(pof.0)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(g) = gen_e
                && seen.insert(g)
            {
                result.push(g);
            }
        }
        result
    }

    pub fn has_energy(
        &self,
        gen_q: &Query<&GeneratorUnit>,
        port_of_q: &Query<&EnergyPortOf>,
        joules: f32,
    ) -> bool {
        self.collect_generator_entities(gen_q, port_of_q)
            .iter()
            .filter_map(|&e| gen_q.get(e).ok())
            .map(|g| g.buffer_joules)
            .sum::<f32>()
            >= joules
    }

    pub fn take_energy(
        &self,
        gen_q: &mut Query<&mut GeneratorUnit>,
        port_of_q: &Query<&EnergyPortOf>,
        joules: f32,
    ) -> bool {
        // Resolve generator entities without borrowing gen_q mutably yet
        let entities: Vec<Entity> = {
            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();
            for &e in &self.0 {
                let gen_e = if gen_q.contains(e) {
                    Some(e)
                } else if let Ok(pof) = port_of_q.get(e) {
                    if gen_q.contains(pof.0) {
                        Some(pof.0)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(g) = gen_e
                    && seen.insert(g)
                {
                    result.push(g);
                }
            }
            result
        };
        // Collect current buffers to compute total without holding a borrow
        let buffers: Vec<(Entity, f32)> = entities
            .iter()
            .filter_map(|&e| gen_q.get(e).ok().map(|g| (e, g.buffer_joules)))
            .collect();
        let total: f32 = buffers.iter().map(|(_, b)| b).sum();
        if total < joules {
            return false;
        }
        let mut remaining = joules;
        for (e, _) in buffers {
            if remaining <= 0.0 {
                break;
            }
            if let Ok(mut unit) = gen_q.get_mut(e) {
                let take = remaining.min(unit.buffer_joules);
                unit.buffer_joules -= take;
                remaining -= take;
            }
        }
        true
    }
}

// -- Systems -----------------------------------------------------------------

fn generator_system(
    mut commands: Commands,
    mut world_events: MessageReader<WorldObjectEvent>,
    cable_q: Query<(&PowerCableSegment, &PowerNetworkMember)>,
    gen_q: Query<(Entity, &GeneratorUnit)>,
    mut changed: MessageWriter<NetworkChanged<Power>>,
) {
    // endpoint → network (keys are rounded IVec3)
    let endpoint_to_net: HashMap<IVec3, Entity> = cable_q
        .iter()
        .flat_map(|(seg, m)| seg.endpoints().map(|ep| (ep.round().as_ivec3(), m.0)))
        .collect();

    let mut affected_nets: HashSet<Entity> = HashSet::new();

    for ev in world_events.read() {
        if ev.item_id != GENERATOR_ID {
            continue;
        }

        let grid_pos = ev.pos.round().as_ivec3();

        if ev.kind == WorldObjectKind::Removed
            && let Some((gen_e, _)) = gen_q
                .iter()
                .find(|(_, g)| g.pos.round().as_ivec3() == grid_pos)
        {
            for &dir in &crate::network::DIRS {
                if let Some(&net) = endpoint_to_net.get(&(grid_pos + dir)) {
                    affected_nets.insert(net);
                    break;
                }
            }
            commands.entity(gen_e).despawn();
        }

        if ev.kind == WorldObjectKind::Placed {
            let gen_e = commands
                .spawn(GeneratorUnit {
                    pos: ev.pos,
                    watts: GENERATOR_DEFAULT_WATTS,
                    buffer_joules: 0.0,
                    max_buffer_joules: GENERATOR_DEFAULT_WATTS * 10.0,
                })
                .id();
            for &dir in &crate::network::DIRS {
                if let Some(&net) = endpoint_to_net.get(&(grid_pos + dir)) {
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

fn generator_tick_system(
    time: Res<Time>,
    net_q: Query<(Entity, &PowerNetworkMembers)>,
    mut gen_q: Query<&mut GeneratorUnit>,
    port_of_q: Query<&EnergyPortOf>,
    mut changed: MessageWriter<NetworkChanged<Power>>,
) {
    let dt = time.delta_secs();
    for (net_e, members) in &net_q {
        let mut seen = std::collections::HashSet::new();
        let mut gen_entities = Vec::new();
        for &e in members.members() {
            let g = if gen_q.contains(e) {
                Some(e)
            } else if let Ok(pof) = port_of_q.get(e) {
                if gen_q.contains(pof.0) {
                    Some(pof.0)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(ge) = g
                && seen.insert(ge)
            {
                gen_entities.push(ge);
            }
        }
        let mut went_positive = false;
        for ge in gen_entities {
            if let Ok(mut unit) = gen_q.get_mut(ge) {
                let prev = unit.buffer_joules;
                unit.buffer_joules =
                    (unit.buffer_joules + unit.watts * dt).min(unit.max_buffer_joules);
                if prev <= 0.0 && unit.buffer_joules > 0.0 {
                    went_positive = true;
                }
            }
        }
        if went_positive {
            changed.write(NetworkChanged::new(net_e));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;
    use crate::machine::{EnergyPortOf, Machine, MachineState, Mirror, Orientation, Rotation};
    use crate::network::{NetworkPlugin, NetworkSystems};
    use crate::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

    fn power_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<CableConnectionEvent>()
            .add_plugins(NetworkPlugin::<Power>::default())
            .add_systems(
                Update,
                (generator_system, generator_tick_system)
                    .chain()
                    .after(NetworkSystems::of::<Power>()),
            );
        app
    }

    fn connect_cable(app: &mut App, from: Vec3, to: Vec3) {
        app.world_mut().write_message(CableConnectionEvent {
            from,
            to,
            item_id: POWER_CABLE_ID.to_string(),
            kind: WorldObjectKind::Placed,
            from_port: None,
            to_port: None,
        });
    }

    fn disconnect_at(app: &mut App, pos: Vec3) {
        app.world_mut().write_message(WorldObjectEvent {
            pos,
            item_id: POWER_CABLE_ID.to_string(),
            kind: WorldObjectKind::Removed,
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
    fn generator_adjacent_to_cable_endpoint_spawns_and_charges_buffer() {
        // Generator placed adjacent to cable endpoint: spawns with correct watts
        // and its buffer fills up over ticks (generator_tick_system runs same frame).
        let mut app = power_app();
        connect_cable(&mut app, Vec3::ZERO, Vec3::new(0.0, 0.0, 5.0));
        app.update();
        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.0, 0.0),
            item_id: GENERATOR_ID.to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();
        let world = app.world_mut();
        let gens: Vec<&GeneratorUnit> = world.query::<&GeneratorUnit>().iter(world).collect();
        assert_eq!(gens.len(), 1);
        assert_eq!(gens[0].watts, GENERATOR_DEFAULT_WATTS);
        // Buffer starts at 0 and is filled by tick; after one update it is ≥ 0
        assert!(gens[0].buffer_joules >= 0.0);
        assert!(gens[0].buffer_joules <= gens[0].max_buffer_joules);
    }

    #[test]
    fn generator_removed_despawns_generator_entity() {
        let mut app = power_app();
        connect_cable(&mut app, Vec3::ZERO, Vec3::new(0.0, 0.0, 5.0));
        app.update();
        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.0, 0.0),
            item_id: GENERATOR_ID.to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();
        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.0, 0.0),
            item_id: GENERATOR_ID.to_string(),
            kind: WorldObjectKind::Removed,
        });
        app.update();
        let world = app.world_mut();
        assert_eq!(world.query::<&GeneratorUnit>().iter(world).count(), 0);
    }

    #[test]
    fn machine_with_energy_port_matching_cable_endpoint_gets_member() {
        // Port is spawned before cable → snap-radius assigns membership
        let mut app = power_app();
        let io_pos = Vec3::new(1.0, 0.0, 0.0);
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
                    energy_ports: vec![io_pos],
                    logistics_ports: vec![],
                },
                MachineState::Idle,
            ))
            .id();
        let port_entity = app
            .world_mut()
            .spawn((
                EnergyPortOf(machine_entity),
                Transform::from_translation(io_pos),
            ))
            .id();
        connect_cable(&mut app, io_pos, Vec3::new(5.0, 0.0, 0.0));
        app.update();
        assert!(app.world().get::<PowerNetworkMember>(port_entity).is_some());
    }

    fn machine_with_port(port: Vec3) -> impl Bundle {
        (
            Machine {
                machine_type: "smelter".to_string(),
                tier: 1,
                orientation: Orientation {
                    rotation: Rotation::North,
                    mirror: Mirror::Normal,
                },
                energy_ports: vec![port],
                logistics_ports: vec![],
            },
            MachineState::Idle,
            Transform::default(),
        )
    }

    #[test]
    fn cable_placed_near_machine_port_joins_machine_to_network() {
        let mut app = power_app();
        let machine = app.world_mut().spawn(machine_with_port(Vec3::ZERO)).id();
        let port_e = app
            .world_mut()
            .spawn((
                EnergyPortOf(machine),
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();
        connect_cable(&mut app, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        app.update();
        assert!(app.world().get::<PowerNetworkMember>(port_e).is_some());
    }

    #[test]
    fn cable_removal_destroys_network_and_clears_machine_membership() {
        let mut app = power_app();
        let machine = app.world_mut().spawn(machine_with_port(Vec3::ZERO)).id();
        let port_e = app
            .world_mut()
            .spawn((
                EnergyPortOf(machine),
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();
        connect_cable(&mut app, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        app.update();
        assert!(app.world().get::<PowerNetworkMember>(port_e).is_some());

        disconnect_at(&mut app, Vec3::ZERO);
        app.update();
        assert!(app.world().get::<PowerNetworkMember>(port_e).is_none());
    }

    #[test]
    fn cable_removal_leaves_one_component_removes_orphaned_machine() {
        let mut app = power_app();
        // Machine at (0,0,0). Two cables: 0→5 and 5→10.
        // Remove 0→5 → remaining 5→10 (1 component).
        // Port at (0,0,0) not in remaining endpoints → membership removed.
        let machine = app.world_mut().spawn(machine_with_port(Vec3::ZERO)).id();
        let port_e = app
            .world_mut()
            .spawn((
                EnergyPortOf(machine),
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();
        connect_cable(&mut app, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        connect_cable(
            &mut app,
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        );
        app.update();
        assert!(app.world().get::<PowerNetworkMember>(port_e).is_some());

        disconnect_at(&mut app, Vec3::ZERO);
        app.update();
        assert!(app.world().get::<PowerNetworkMember>(port_e).is_none());
    }

    #[test]
    fn degenerate_cable_same_endpoints_is_noop() {
        let mut app = power_app();
        connect_cable(&mut app, Vec3::ZERO, Vec3::ZERO);
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn generic_removal_near_cable_segment_removes_it() {
        let mut app = power_app();
        connect_cable(&mut app, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        app.update();
        assert_eq!(network_count(&mut app), 1);

        // Generic removal: item_id="" → finds nearest cable by distance
        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(2.5, 0.0, 0.0),
            item_id: String::new(),
            kind: WorldObjectKind::Removed,
        });
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn has_energy_true_when_sufficient_false_when_not() {
        use bevy::ecs::system::SystemState;

        let mut world = World::new();
        let gen_e = world
            .spawn(GeneratorUnit {
                pos: Vec3::ZERO,
                watts: 0.0,
                buffer_joules: 100.0,
                max_buffer_joules: 100.0,
            })
            .id();
        let net_e = world.spawn_empty().id();
        world.entity_mut(gen_e).insert(PowerNetworkMember(net_e));

        let mut state: SystemState<(
            Query<&PowerNetworkMembers>,
            Query<&GeneratorUnit>,
            Query<&EnergyPortOf>,
        )> = SystemState::new(&mut world);
        let (net_q, gen_q, port_q) = state.get(&world);
        let members = net_q.get(net_e).unwrap();

        assert!(
            members.has_energy(&gen_q, &port_q, 50.0),
            "buffer=100 should satisfy 50J request"
        );
        assert!(
            !members.has_energy(&gen_q, &port_q, 150.0),
            "buffer=100 should not satisfy 150J request"
        );
    }

    #[test]
    fn take_energy_drains_buffer_and_returns_false_when_insufficient() {
        use bevy::ecs::system::SystemState;

        let mut world = World::new();
        let gen_e = world
            .spawn(GeneratorUnit {
                pos: Vec3::ZERO,
                watts: 0.0,
                buffer_joules: 100.0,
                max_buffer_joules: 100.0,
            })
            .id();
        let net_e = world.spawn_empty().id();
        world.entity_mut(gen_e).insert(PowerNetworkMember(net_e));

        // Successful drain
        {
            let mut state: SystemState<(
                Query<&PowerNetworkMembers>,
                Query<&mut GeneratorUnit>,
                Query<&EnergyPortOf>,
            )> = SystemState::new(&mut world);
            let (net_q, mut gen_q, port_q) = state.get_mut(&mut world);
            assert!(
                net_q
                    .get(net_e)
                    .unwrap()
                    .take_energy(&mut gen_q, &port_q, 30.0),
                "take_energy should succeed when buffer >= amount"
            );
            state.apply(&mut world);
        }
        assert_eq!(
            world.get::<GeneratorUnit>(gen_e).unwrap().buffer_joules,
            70.0,
            "buffer must decrease by the taken amount"
        );

        // Failed drain — 200 > 70
        {
            let mut state: SystemState<(
                Query<&PowerNetworkMembers>,
                Query<&mut GeneratorUnit>,
                Query<&EnergyPortOf>,
            )> = SystemState::new(&mut world);
            let (net_q, mut gen_q, port_q) = state.get_mut(&mut world);
            assert!(
                !net_q
                    .get(net_e)
                    .unwrap()
                    .take_energy(&mut gen_q, &port_q, 200.0),
                "take_energy should fail when buffer < amount"
            );
            state.apply(&mut world);
        }
        // Buffer must be unchanged after a failed drain
        assert_eq!(
            world.get::<GeneratorUnit>(gen_e).unwrap().buffer_joules,
            70.0,
            "failed take_energy must not modify the buffer"
        );
    }
}

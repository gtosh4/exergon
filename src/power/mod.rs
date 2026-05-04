use std::collections::{HashMap, HashSet};

use avian3d::prelude::{Collider, Sensor};

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::machine::{Machine, MachineActivity, MachineState};
use crate::network::{
    HasEndpoints, NetworkChanged, NetworkKind, NetworkMemberComponent, NetworkMembersComponent,
    NetworkPlugin, NetworkSystems, Power, route_avoiding,
};
use crate::recipe_graph::RecipeGraph;
use crate::world::{WorldObjectEvent, WorldObjectKind};

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
            (
                generator_system,
                recalc_capacity_system,
                brownout_system.run_if(resource_exists::<RecipeGraph>),
            )
                .chain()
                .after(NetworkSystems::of::<Power>())
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
const GENERATOR_DEFAULT_WATTS: f32 = 50.0;
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
                            m.energy_ports
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
}

#[derive(Component)]
pub struct PowerNetwork {
    pub capacity_watts: f32,
}

// -- NetworkKind impl --------------------------------------------------------

impl NetworkKind for Power {
    const CABLE_ITEM_ID: &'static str = POWER_CABLE_ID;

    type CableSegment = PowerCableSegment;
    type Member = PowerNetworkMember;
    type Members = PowerNetworkMembers;

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

fn recalc_capacity_system(
    mut events: MessageReader<NetworkChanged<Power>>,
    mut net_q: Query<(Entity, &mut PowerNetwork, &PowerNetworkMembers)>,
    gen_q: Query<&GeneratorUnit>,
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
    use bevy::prelude::*;

    use super::*;
    use crate::machine::{
        Machine, MachineNetworkChanged, MachineState, Mirror, Orientation, Rotation,
    };
    use crate::network::{NetworkChanged, NetworkPlugin, NetworkSystems};
    use crate::recipe_graph::{ConcreteRecipe, RecipeGraph};
    use crate::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

    fn power_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<CableConnectionEvent>()
            .add_message::<MachineNetworkChanged>()
            .add_plugins(NetworkPlugin::<Power>::default())
            .add_systems(
                Update,
                (generator_system, recalc_capacity_system)
                    .chain()
                    .after(NetworkSystems::of::<Power>()),
            );
        app
    }

    fn recipe_graph_with(energy_cost: f32, processing_time: f32) -> RecipeGraph {
        let mut recipes = std::collections::HashMap::new();
        recipes.insert(
            "r1".to_string(),
            ConcreteRecipe {
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
            form_groups: std::collections::HashMap::new(),
            templates: std::collections::HashMap::new(),
            items: std::collections::HashMap::new(),
            recipes,
            terminal: String::new(),
            producers: std::collections::HashMap::new(),
            consumers: std::collections::HashMap::new(),
        }
    }

    fn connect_cable(app: &mut App, from: Vec3, to: Vec3) {
        app.world_mut().write_message(CableConnectionEvent {
            from,
            to,
            item_id: POWER_CABLE_ID.to_string(),
            kind: WorldObjectKind::Placed,
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
    fn generator_adjacent_to_cable_endpoint_adds_capacity() {
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
        let caps: Vec<f32> = world
            .query_filtered::<&PowerNetwork, ()>()
            .iter(world)
            .map(|n| n.capacity_watts)
            .collect();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0], 0.0);
    }

    #[test]
    fn machine_with_energy_port_matching_cable_endpoint_gets_member() {
        let mut app = power_app();
        let io_pos = Vec3::new(1.0, 0.0, 0.0);
        connect_cable(&mut app, io_pos, Vec3::new(5.0, 0.0, 0.0));
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

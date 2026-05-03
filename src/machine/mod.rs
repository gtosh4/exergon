use std::collections::HashMap;

use avian3d::prelude::{Collider, RigidBody};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use serde::Deserialize;

use crate::content::load_ron_dir;
use crate::world::{WorldObjectEvent, WorldObjectKind};

/// System set that contains machine placement. Logistics/power run after this.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MachineScanSet;

/// Emitted whenever machines are placed or removed.
/// Listeners (logistics, power) use this to trigger network rebuilds.
#[derive(bevy::ecs::message::Message, Clone, Copy)]
pub struct MachineNetworkChanged;

/// Active recipe processing state on a running machine.
#[derive(Component, Clone)]
pub struct MachineActivity {
    pub recipe_id: String,
    pub progress: f32,
    /// Set by the power brownout system each tick (1.0 = full speed).
    pub speed_factor: f32,
}

pub struct MachinePlugin;

impl Plugin for MachinePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MachineNetworkChanged>()
            .configure_sets(
                Update,
                MachineScanSet
                    .in_set(crate::GameSystems::Simulation)
                    .run_if(resource_exists::<MachineRegistry>)
                    .run_if(in_state(crate::GameState::Playing)),
            )
            .add_systems(Startup, (load_machines, setup_machine_visuals))
            .add_systems(
                Update,
                (
                    place_machine_system,
                    place_platform_system,
                    remove_machine_system,
                )
                    .in_set(MachineScanSet),
            );
    }
}

// ---------------------------------------------------------------------------
// Data types (deserialised from RON)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug, Default)]
pub struct MachineTierDef {
    pub tier: u8,
    #[serde(default)]
    pub energy_io_offsets: Vec<IVec3>,
    #[serde(default)]
    pub logistics_io_offsets: Vec<IVec3>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MachineDef {
    pub id: String,
    pub tiers: Vec<MachineTierDef>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct MachineRegistry {
    machines: Vec<MachineDef>,
}

impl MachineRegistry {
    fn new(machines: Vec<MachineDef>) -> Self {
        Self { machines }
    }

    pub fn machine_def(&self, id: &str) -> Option<&MachineDef> {
        self.machines.iter().find(|m| m.id == id)
    }
}

// ---------------------------------------------------------------------------
// ECS components
// ---------------------------------------------------------------------------

#[derive(Component, Debug)]
pub struct Machine {
    pub machine_type: String,
    pub tier: u8,
    pub orientation: Orientation,
    pub energy_ports: Vec<Vec3>,
    pub logistics_ports: Vec<Vec3>,
}

/// Marker: machine entity exists but is not fully operational (e.g. during removal).
#[derive(Component)]
pub struct MachineUnformed;

/// Spawned at each IO port position when a machine is placed. Despawned with the machine.
#[derive(Component)]
pub struct IoPortMarker {
    pub owner: Entity,
}

/// Flat static platform entity placed on terrain.
#[derive(Component)]
pub struct Platform;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineState {
    Idle,
    Running,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirror {
    Normal,
    Mirrored,
}

#[derive(Clone, Copy, Debug)]
pub struct Orientation {
    pub rotation: Rotation,
    pub mirror: Mirror,
}

impl Orientation {
    fn all() -> [Orientation; 8] {
        use Mirror::{Mirrored, Normal};
        use Rotation::{East, North, South, West};
        [
            Orientation {
                rotation: North,
                mirror: Normal,
            },
            Orientation {
                rotation: East,
                mirror: Normal,
            },
            Orientation {
                rotation: South,
                mirror: Normal,
            },
            Orientation {
                rotation: West,
                mirror: Normal,
            },
            Orientation {
                rotation: North,
                mirror: Mirrored,
            },
            Orientation {
                rotation: East,
                mirror: Mirrored,
            },
            Orientation {
                rotation: South,
                mirror: Mirrored,
            },
            Orientation {
                rotation: West,
                mirror: Mirrored,
            },
        ]
    }

    /// Rotate a canonical-space delta into world-space, applying mirror then rotation.
    pub fn transform(&self, delta: IVec3) -> IVec3 {
        let dx = if self.mirror == Mirror::Mirrored {
            -delta.x
        } else {
            delta.x
        };
        let dy = delta.y;
        let dz = delta.z;
        let (rx, rz) = match self.rotation {
            Rotation::North => (dx, dz),
            Rotation::East => (dz, -dx),
            Rotation::South => (-dx, -dz),
            Rotation::West => (-dz, dx),
        };
        IVec3::new(rx, dy, rz)
    }
}

// ---------------------------------------------------------------------------
// Visual assets
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct MachineVisualAssets {
    mesh: Handle<Mesh>,
    materials: HashMap<String, Handle<StandardMaterial>>,
    fallback: Handle<StandardMaterial>,
    port_mesh: Handle<Mesh>,
    energy_port_mat: Handle<StandardMaterial>,
    logistics_port_mat: Handle<StandardMaterial>,
    platform_mesh: Handle<Mesh>,
    platform_mat: Handle<StandardMaterial>,
}

fn setup_machine_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mut mats: HashMap<String, Handle<StandardMaterial>> = HashMap::new();
    mats.insert(
        "smelter".into(),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.45, 0.1),
            ..default()
        }),
    );
    mats.insert(
        "assembler".into(),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.45, 0.9),
            ..default()
        }),
    );
    mats.insert(
        "analysis_station".into(),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.75, 0.55),
            ..default()
        }),
    );
    let fallback = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.65, 0.65),
        ..default()
    });
    let port_mesh = meshes.add(Sphere::new(0.15));
    let energy_port_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.0),
        unlit: true,
        ..default()
    });
    let logistics_port_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.9, 0.2),
        unlit: true,
        ..default()
    });
    let platform_mesh = meshes.add(Cuboid::new(2.0, 0.25, 2.0));
    let platform_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.55),
        ..default()
    });
    commands.insert_resource(MachineVisualAssets {
        mesh,
        materials: mats,
        fallback,
        port_mesh,
        energy_port_mat,
        logistics_port_mat,
        platform_mesh,
        platform_mat,
    });
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn load_machines(mut commands: Commands) {
    let machines = load_ron_dir::<MachineDef>("assets/machines", "machine");
    info!("Loaded {} machine definitions", machines.len());
    commands.insert_resource(MachineRegistry::new(machines));
}

// ---------------------------------------------------------------------------
// Placement systems
// ---------------------------------------------------------------------------

fn place_machine_system(
    mut commands: Commands,
    mut events: MessageReader<WorldObjectEvent>,
    registry: Res<MachineRegistry>,
    mut network_changed: MessageWriter<MachineNetworkChanged>,
    visuals: Option<Res<MachineVisualAssets>>,
) {
    for ev in events.read() {
        if ev.kind != WorldObjectKind::Placed {
            continue;
        }
        let Some(def) = registry.machine_def(&ev.item_id) else {
            continue;
        };

        let default_tier = MachineTierDef::default();
        let tier_def = def
            .tiers
            .iter()
            .max_by_key(|t| t.tier)
            .unwrap_or(&default_tier);

        let orientation = Orientation {
            rotation: Rotation::North,
            mirror: Mirror::Normal,
        };

        let energy_ports: Vec<Vec3> = tier_def
            .energy_io_offsets
            .iter()
            .map(|&o| ev.pos + orientation.transform(o).as_vec3())
            .collect();
        let logistics_ports: Vec<Vec3> = tier_def
            .logistics_io_offsets
            .iter()
            .map(|&o| ev.pos + orientation.transform(o).as_vec3())
            .collect();

        let machine_entity = commands
            .spawn((
                Machine {
                    machine_type: def.id.clone(),
                    tier: tier_def.tier,
                    orientation,
                    energy_ports: energy_ports.clone(),
                    logistics_ports: logistics_ports.clone(),
                },
                MachineState::Idle,
                Transform::from_translation(ev.pos),
                RigidBody::Static,
                Collider::cuboid(0.5, 0.5, 0.5),
            ))
            .id();

        if let Some(ref v) = visuals {
            let mat = v
                .materials
                .get(&def.id)
                .cloned()
                .unwrap_or_else(|| v.fallback.clone());
            commands
                .entity(machine_entity)
                .insert((Mesh3d(v.mesh.clone()), MeshMaterial3d(mat)));
        }

        for &port_pos in &energy_ports {
            let mut marker_cmd = commands.spawn((
                IoPortMarker {
                    owner: machine_entity,
                },
                Transform::from_translation(port_pos),
            ));
            if let Some(ref v) = visuals {
                marker_cmd.insert((
                    Mesh3d(v.port_mesh.clone()),
                    MeshMaterial3d(v.energy_port_mat.clone()),
                ));
            }
        }
        for &port_pos in &logistics_ports {
            let mut marker_cmd = commands.spawn((
                IoPortMarker {
                    owner: machine_entity,
                },
                Transform::from_translation(port_pos),
            ));
            if let Some(ref v) = visuals {
                marker_cmd.insert((
                    Mesh3d(v.port_mesh.clone()),
                    MeshMaterial3d(v.logistics_port_mat.clone()),
                ));
            }
        }

        network_changed.write(MachineNetworkChanged);
        info!(
            "Machine '{}' tier {} placed at {:?}",
            def.id, tier_def.tier, ev.pos
        );
    }
}

fn place_platform_system(
    mut commands: Commands,
    mut events: MessageReader<WorldObjectEvent>,
    visuals: Option<Res<MachineVisualAssets>>,
) {
    for ev in events.read() {
        if ev.kind != WorldObjectKind::Placed || ev.item_id != "platform" {
            continue;
        }
        let mut entity_cmd = commands.spawn((
            Platform,
            Transform::from_translation(ev.pos),
            RigidBody::Static,
            Collider::cuboid(1.0, 0.125, 1.0),
        ));
        if let Some(ref v) = visuals {
            entity_cmd.insert((
                Mesh3d(v.platform_mesh.clone()),
                MeshMaterial3d(v.platform_mat.clone()),
            ));
        }
        info!("Platform placed at {:?}", ev.pos);
    }
}

fn remove_machine_system(
    mut commands: Commands,
    mut events: MessageReader<WorldObjectEvent>,
    machine_q: Query<(Entity, &Machine, &Transform)>,
    port_marker_q: Query<(Entity, &IoPortMarker)>,
    mut network_changed: MessageWriter<MachineNetworkChanged>,
) {
    for ev in events.read() {
        if ev.kind != WorldObjectKind::Removed || !ev.item_id.is_empty() {
            continue;
        }
        if let Some((entity, machine, _)) = machine_q
            .iter()
            .find(|(_, _, t)| t.translation.distance(ev.pos) < 1.5)
        {
            let machine_type = machine.machine_type.clone();
            for (marker_entity, marker) in port_marker_q.iter() {
                if marker.owner == entity {
                    commands.entity(marker_entity).despawn();
                }
            }
            commands.entity(entity).despawn();
            network_changed.write(MachineNetworkChanged);
            info!("Machine '{}' removed near {:?}", machine_type, ev.pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_all_returns_8() {
        assert_eq!(Orientation::all().len(), 8);
    }

    #[test]
    fn orientation_north_normal_is_identity() {
        let o = Orientation {
            rotation: Rotation::North,
            mirror: Mirror::Normal,
        };
        assert_eq!(o.transform(IVec3::new(1, 2, 3)), IVec3::new(1, 2, 3));
    }

    #[test]
    fn orientation_east_rotates_correctly() {
        let o = Orientation {
            rotation: Rotation::East,
            mirror: Mirror::Normal,
        };
        // East: (dx, dy, dz) -> (dz, dy, -dx)
        assert_eq!(o.transform(IVec3::new(1, 0, 0)), IVec3::new(0, 0, -1));
        assert_eq!(o.transform(IVec3::new(0, 0, 1)), IVec3::new(1, 0, 0));
    }

    #[test]
    fn orientation_mirror_negates_x() {
        let o = Orientation {
            rotation: Rotation::North,
            mirror: Mirror::Mirrored,
        };
        assert_eq!(o.transform(IVec3::new(2, 1, 3)), IVec3::new(-2, 1, 3));
    }

    #[test]
    fn orientation_south_normal() {
        let o = Orientation {
            rotation: Rotation::South,
            mirror: Mirror::Normal,
        };
        // South: (-dx, dy, -dz)
        assert_eq!(o.transform(IVec3::new(1, 0, 0)), IVec3::new(-1, 0, 0));
        assert_eq!(o.transform(IVec3::new(0, 0, 1)), IVec3::new(0, 0, -1));
    }

    #[test]
    fn orientation_west_normal() {
        let o = Orientation {
            rotation: Rotation::West,
            mirror: Mirror::Normal,
        };
        // West: (-dz, dy, dx)
        assert_eq!(o.transform(IVec3::new(1, 0, 0)), IVec3::new(0, 0, 1));
        assert_eq!(o.transform(IVec3::new(0, 0, 1)), IVec3::new(-1, 0, 0));
    }

    #[test]
    fn orientation_y_unchanged_all_variants() {
        for o in Orientation::all() {
            assert_eq!(o.transform(IVec3::new(3, 7, 5)).y, 7);
        }
    }

    fn simple_machine(id: &str) -> MachineDef {
        MachineDef {
            id: id.to_string(),
            tiers: vec![MachineTierDef {
                tier: 1,
                energy_io_offsets: vec![IVec3::new(0, 0, 1)],
                logistics_io_offsets: vec![IVec3::new(1, 0, 0)],
            }],
        }
    }

    #[test]
    fn registry_machine_def_found_and_not_found() {
        let reg = MachineRegistry::new(vec![simple_machine("smelter")]);
        assert!(reg.machine_def("smelter").is_some());
        assert_eq!(reg.machine_def("smelter").unwrap().id, "smelter");
        assert!(reg.machine_def("unknown").is_none());
    }

    #[test]
    fn place_machine_system_creates_entity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>()
            .insert_resource(MachineRegistry::new(vec![simple_machine("smelter")]));

        app.add_systems(Update, place_machine_system);

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::ZERO,
            item_id: "smelter".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();

        let world = app.world_mut();
        let count = world.query::<&Machine>().iter(world).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn place_machine_system_unknown_item_no_spawn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>()
            .insert_resource(MachineRegistry::new(vec![]));

        app.add_systems(Update, place_machine_system);

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::ZERO,
            item_id: "unknown".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();

        let world = app.world_mut();
        assert_eq!(world.query::<&Machine>().iter(world).count(), 0);
    }

    #[test]
    fn place_machine_io_offsets_applied() {
        let def = MachineDef {
            id: "smelter".to_string(),
            tiers: vec![MachineTierDef {
                tier: 1,
                energy_io_offsets: vec![IVec3::new(0, 0, 1)],
                logistics_io_offsets: vec![IVec3::new(1, 0, 0)],
            }],
        };
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>()
            .insert_resource(MachineRegistry::new(vec![def]));

        app.add_systems(Update, place_machine_system);

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(5.0, 0.0, 5.0),
            item_id: "smelter".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();

        let world = app.world_mut();
        let machines: Vec<&Machine> = world.query::<&Machine>().iter(world).collect();
        assert_eq!(machines.len(), 1);
        let m = machines[0];
        assert!(
            m.energy_ports
                .iter()
                .any(|p| p.round().as_ivec3() == IVec3::new(5, 0, 6))
        );
        assert!(
            m.logistics_ports
                .iter()
                .any(|p| p.round().as_ivec3() == IVec3::new(6, 0, 5))
        );
    }

    #[test]
    fn place_machine_spawns_port_markers() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>()
            .insert_resource(MachineRegistry::new(vec![simple_machine("smelter")]));

        app.add_systems(Update, place_machine_system);

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::ZERO,
            item_id: "smelter".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();

        let world = app.world_mut();
        // simple_machine: 1 energy port + 1 logistics port = 2 markers
        let count = world.query::<&IoPortMarker>().iter(world).count();
        assert_eq!(count, 2);
    }

    #[test]
    fn remove_machine_despawns_port_markers() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>()
            .insert_resource(MachineRegistry::new(vec![simple_machine("smelter")]));

        app.add_systems(Update, (place_machine_system, remove_machine_system));

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::ZERO,
            item_id: "smelter".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();

        {
            let world = app.world_mut();
            assert_eq!(world.query::<&Machine>().iter(world).count(), 1);
            assert_eq!(world.query::<&IoPortMarker>().iter(world).count(), 2);
        }

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::ZERO,
            item_id: String::new(),
            kind: WorldObjectKind::Removed,
        });
        app.update();

        {
            let world = app.world_mut();
            assert_eq!(world.query::<&Machine>().iter(world).count(), 0);
            assert_eq!(world.query::<&IoPortMarker>().iter(world).count(), 0);
        }
    }

    #[test]
    fn place_platform_creates_entity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>();

        app.add_systems(Update, place_platform_system);

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.0, 1.0),
            item_id: "platform".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();

        let world = app.world_mut();
        assert_eq!(world.query::<&Platform>().iter(world).count(), 1);
    }
}

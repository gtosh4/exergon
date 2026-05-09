use bevy::prelude::*;

mod placement;
mod registry;
mod visuals;

pub use placement::{MachineBundle, spawn_port_markers};
pub use registry::{MachineDef, MachineRegistry, MachineTierDef};
pub(crate) use visuals::GhostAssets;
pub use visuals::{MachineColliders, MachineVisualAssets};

/// System set that contains machine placement. Logistics/power run after this.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MachineScanSet;

/// Emitted whenever machines are placed or removed.
/// Listeners (logistics, power) use this to trigger network rebuilds.
#[derive(bevy::ecs::message::Message, Clone, Copy)]
pub struct MachineNetworkChanged;

/// Active recipe processing state on a running machine.
// SparseSet: added/removed every recipe cycle; avoids archetype churn from TableStorage moves.
#[derive(Component, Clone)]
#[component(storage = "SparseSet")]
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
            .init_resource::<MachineColliders>()
            .configure_sets(
                Update,
                MachineScanSet
                    .in_set(crate::GameSystems::Simulation)
                    .run_if(resource_exists::<MachineRegistry>)
                    .run_if(in_state(crate::GameState::Playing)),
            )
            .add_systems(
                Startup,
                (registry::load_machines, visuals::setup_machine_visuals),
            )
            .add_systems(
                Startup,
                visuals::setup_ghost_assets.after(visuals::setup_machine_visuals),
            )
            .add_systems(Update, visuals::compute_machine_colliders)
            .add_systems(
                Update,
                (
                    placement::place_machine_system,
                    placement::place_platform_system,
                    placement::remove_placed_objects_system,
                )
                    .in_set(MachineScanSet),
            );
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

pub trait PortOfMachine: Component {
    fn machine_entity(&self) -> Entity;
}

#[derive(Component)]
#[relationship(relationship_target = MachineLogisticsPorts)]
pub struct LogisticsPortOf(pub Entity);

impl PortOfMachine for LogisticsPortOf {
    fn machine_entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
#[relationship_target(relationship = LogisticsPortOf)]
pub struct MachineLogisticsPorts(Vec<Entity>);

impl MachineLogisticsPorts {
    pub fn ports(&self) -> &[Entity] {
        &self.0
    }
}

#[derive(Component)]
#[relationship(relationship_target = MachineEnergyPorts)]
pub struct EnergyPortOf(pub Entity);

impl PortOfMachine for EnergyPortOf {
    fn machine_entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
#[relationship_target(relationship = EnergyPortOf)]
pub struct MachineEnergyPorts(Vec<Entity>);

impl MachineEnergyPorts {
    pub fn ports(&self) -> &[Entity] {
        &self.0
    }
}

/// Flat static platform entity placed on terrain.
#[derive(Component)]
pub struct Platform;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineState {
    Idle,
    Running,
}

/// Automatic miner placed on an ore deposit. Continuously samples ore and outputs to logistics.
#[derive(Component)]
pub struct MinerMachine {
    pub deposit: Entity,
    pub accumulator: f32,
}

// ---------------------------------------------------------------------------
// Orientation
// ---------------------------------------------------------------------------

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

#[cfg(test)]
mod tests {
    use super::placement::{
        place_machine_system, place_platform_system, remove_placed_objects_system,
    };
    use super::*;
    use crate::world::{WorldObjectEvent, WorldObjectKind};

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
        assert_eq!(o.transform(IVec3::new(1, 0, 0)), IVec3::new(-1, 0, 0));
        assert_eq!(o.transform(IVec3::new(0, 0, 1)), IVec3::new(0, 0, -1));
    }

    #[test]
    fn orientation_west_normal() {
        let o = Orientation {
            rotation: Rotation::West,
            mirror: Mirror::Normal,
        };
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

        app.add_systems(Update, (place_machine_system, remove_placed_objects_system));

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

    #[test]
    fn remove_platform_despawns_entity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>();

        app.add_systems(
            Update,
            (place_platform_system, remove_placed_objects_system),
        );

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.0, 1.0),
            item_id: "platform".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();
        {
            let world = app.world_mut();
            assert_eq!(world.query::<&Platform>().iter(world).count(), 1);
        }

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.0, 1.0),
            item_id: String::new(),
            kind: WorldObjectKind::Removed,
        });
        app.update();

        let world = app.world_mut();
        assert_eq!(world.query::<&Platform>().iter(world).count(), 0);
    }

    #[test]
    fn remove_does_not_remove_platform_when_machine_present() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>()
            .insert_resource(MachineRegistry::new(vec![simple_machine("smelter")]));

        app.add_systems(
            Update,
            (
                place_machine_system,
                place_platform_system,
                remove_placed_objects_system,
            ),
        );

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.5, 1.0),
            item_id: "smelter".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.0, 1.0),
            item_id: "platform".to_string(),
            kind: WorldObjectKind::Placed,
        });
        app.update();

        app.world_mut().write_message(WorldObjectEvent {
            pos: Vec3::new(1.0, 0.5, 1.0),
            item_id: String::new(),
            kind: WorldObjectKind::Removed,
        });
        app.update();

        let world = app.world_mut();
        assert_eq!(world.query::<&Machine>().iter(world).count(), 0);
        assert_eq!(world.query::<&Platform>().iter(world).count(), 1);
    }
}

use std::collections::HashMap;

use bevy::ecs::message::Message;
use bevy::prelude::*;

use crate::machine::Machine;
use crate::network::{
    HasEndpoints, Logistics, NetworkKind, NetworkMemberComponent, NetworkMembersComponent,
    NetworkPlugin, NetworkSystems, route_avoiding,
};
use crate::recipe_graph::RecipeGraph;

mod items;
mod miner;
mod recipes;
mod storage;
mod visuals;

pub use items::{give_items, has_items, take_items};

pub(super) const LOGISTICS_CABLE_ID: &str = "logistics_cable";
pub(super) const STORAGE_CRATE_ID: &str = "storage_crate";
pub(super) const CABLE_RADIUS: f32 = 0.05;

// -- Plugins -----------------------------------------------------------------

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
                ApplyDeferred,
                storage::storage_unit_system,
                ApplyDeferred,
                miner::miner_tick_system,
                recipes::recipe_start_system.run_if(resource_exists::<RecipeGraph>),
                recipes::recipe_progress_system.run_if(resource_exists::<RecipeGraph>),
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
        app.add_systems(Startup, visuals::setup_logistics_visuals);
        app.add_systems(
            Update,
            visuals::add_cable_visuals
                .in_set(crate::GameSystems::Rendering)
                .run_if(in_state(crate::GameState::Playing)),
        );
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bevy::prelude::*;

    use super::recipes::{recipe_progress_system, recipe_start_system};
    use super::storage::storage_unit_system;
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
    fn storage_crate_machine_gets_storage_unit() {
        let mut app = logistics_app();
        let entity = app
            .world_mut()
            .spawn(Machine {
                machine_type: STORAGE_CRATE_ID.to_string(),
                tier: 1,
                orientation: Orientation {
                    rotation: Rotation::North,
                    mirror: Mirror::Normal,
                },
                energy_ports: vec![],
                logistics_ports: vec![],
            })
            .id();
        app.update();
        assert!(app.world().get::<StorageUnit>(entity).is_some());
    }

    #[test]
    fn machine_with_logistics_port_matching_cable_endpoint_gets_member() {
        let mut app = logistics_app();
        let io_pos = Vec3::new(1.0, 0.0, 0.0);
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

use std::collections::HashMap;

use bevy::ecs::message::Message;
use bevy::prelude::*;

use crate::machine::{LogisticsPortOf, Machine};
use crate::network::{
    HasEndpoints, Logistics, NetworkKind, NetworkMemberComponent, NetworkMembersComponent,
    NetworkPlugin, NetworkSystems, route_avoiding,
};
use crate::power::PowerSimSystems;
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
        app.add_message::<NetworkStorageChanged>()
            .add_message::<crate::network::NetworkChanged<crate::network::Power>>()
            .add_systems(
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
                .after(PowerSimSystems)
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
    type PortOf = LogisticsPortOf;

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
    use crate::research::{RESEARCH_POINTS_ID, ResearchPool, TechTreeProgress};
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

    fn disconnect_at(app: &mut App, pos: Vec3) {
        app.world_mut().write_message(WorldObjectEvent {
            pos,
            item_id: LOGISTICS_CABLE_ID.to_string(),
            kind: WorldObjectKind::Removed,
        });
    }

    fn network_count(app: &mut App) -> usize {
        let world = app.world_mut();
        world
            .query_filtered::<(), With<LogisticsNetwork>>()
            .iter(world)
            .count()
    }

    #[test]
    fn three_machines_on_separate_ports_join_same_network() {
        // Regression: cable to smelter_port_2 created a new network N2 and
        // reassigned the smelter from N1 to N2, orphaning storage_1.
        let mut app = logistics_app();

        let storage_1_e = app
            .world_mut()
            .spawn((
                Machine {
                    machine_type: "storage_crate".to_string(),
                    tier: 1,
                    orientation: Orientation {
                        rotation: Rotation::North,
                        mirror: Mirror::Normal,
                    },
                    energy_ports: vec![],
                    logistics_ports: vec![Vec3::new(1.0, 0.0, 0.0)],
                },
                Transform::default(),
            ))
            .id();

        let smelter_e = app
            .world_mut()
            .spawn((
                Machine {
                    machine_type: "smelter".to_string(),
                    tier: 1,
                    orientation: Orientation {
                        rotation: Rotation::North,
                        mirror: Mirror::Normal,
                    },
                    energy_ports: vec![],
                    logistics_ports: vec![Vec3::new(4.0, 0.0, 0.0), Vec3::new(6.0, 0.0, 0.0)],
                },
                MachineState::Idle,
                Transform::default(),
            ))
            .id();

        let storage_2_e = app
            .world_mut()
            .spawn((
                Machine {
                    machine_type: "storage_crate".to_string(),
                    tier: 1,
                    orientation: Orientation {
                        rotation: Rotation::North,
                        mirror: Mirror::Normal,
                    },
                    energy_ports: vec![],
                    logistics_ports: vec![Vec3::new(9.0, 0.0, 0.0)],
                },
                Transform::default(),
            ))
            .id();

        // Spawn port entities
        let port_s1 = app
            .world_mut()
            .spawn((
                LogisticsPortOf(storage_1_e),
                Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
            ))
            .id();
        let port_sm_1 = app
            .world_mut()
            .spawn((
                LogisticsPortOf(smelter_e),
                Transform::from_translation(Vec3::new(4.0, 0.0, 0.0)),
            ))
            .id();
        let port_sm_2 = app
            .world_mut()
            .spawn((
                LogisticsPortOf(smelter_e),
                Transform::from_translation(Vec3::new(6.0, 0.0, 0.0)),
            ))
            .id();
        let port_s2 = app
            .world_mut()
            .spawn((
                LogisticsPortOf(storage_2_e),
                Transform::from_translation(Vec3::new(9.0, 0.0, 0.0)),
            ))
            .id();

        connect_cable(&mut app, Vec3::new(1.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 0.0));
        app.update();
        connect_cable(&mut app, Vec3::new(6.0, 0.0, 0.0), Vec3::new(9.0, 0.0, 0.0));
        app.update();

        let net_s1 = app
            .world()
            .get::<LogisticsNetworkMember>(port_s1)
            .map(|m| m.0);
        let net_sm1 = app
            .world()
            .get::<LogisticsNetworkMember>(port_sm_1)
            .map(|m| m.0);
        let net_sm2 = app
            .world()
            .get::<LogisticsNetworkMember>(port_sm_2)
            .map(|m| m.0);
        let net_s2 = app
            .world()
            .get::<LogisticsNetworkMember>(port_s2)
            .map(|m| m.0);

        assert!(net_s1.is_some(), "storage_1 port should be in a network");
        assert!(net_sm1.is_some(), "smelter port 1 should be in a network");
        assert!(net_sm2.is_some(), "smelter port 2 should be in a network");
        assert!(net_s2.is_some(), "storage_2 port should be in a network");
        assert_eq!(
            net_s1, net_sm1,
            "storage_1 and smelter port 1 must share a network"
        );
        assert_eq!(net_sm1, net_sm2, "smelter ports must share a network");
        assert_eq!(
            net_sm2, net_s2,
            "smelter port 2 and storage_2 must share a network"
        );
        assert_eq!(network_count(&mut app), 1, "should be exactly one network");
    }

    #[test]
    fn cable_removal_clears_network() {
        let mut app = logistics_app();
        connect_cable(&mut app, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        app.update();
        assert_eq!(network_count(&mut app), 1);

        disconnect_at(&mut app, Vec3::ZERO);
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn cable_removal_splits_chain_into_two_networks() {
        let mut app = logistics_app();
        connect_cable(&mut app, Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0));
        connect_cable(
            &mut app,
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        );
        connect_cable(
            &mut app,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(15.0, 0.0, 0.0),
        );
        connect_cable(
            &mut app,
            Vec3::new(15.0, 0.0, 0.0),
            Vec3::new(20.0, 0.0, 0.0),
        );
        app.update();
        assert_eq!(network_count(&mut app), 1);

        disconnect_at(&mut app, Vec3::new(10.0, 0.0, 0.0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
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
        let port_entity = app
            .world_mut()
            .spawn((
                LogisticsPortOf(machine_entity),
                Transform::from_translation(io_pos),
            ))
            .id();
        app.world_mut().write_message(MachineNetworkChanged);
        app.update();
        assert!(
            app.world()
                .get::<LogisticsNetworkMember>(port_entity)
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
            .add_message::<NetworkChanged<crate::network::Power>>()
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
        // Storage is a machine entity with StorageUnit; its port is in the network
        let storage_e = app
            .world_mut()
            .spawn(StorageUnit {
                items: [("iron".to_owned(), 10u32)].into_iter().collect(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_e),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));
        let machine_entity = app
            .world_mut()
            .spawn((bare_machine("furnace"), MachineState::Idle))
            .id();
        // Port entity linking machine to network
        app.world_mut().spawn((
            LogisticsPortOf(machine_entity),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));

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
            .spawn(StorageUnit {
                items: HashMap::new(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_e),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));
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
            ))
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(machine_entity),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));

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
        let storage_e = app
            .world_mut()
            .spawn(StorageUnit {
                items: HashMap::new(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_e),
            Transform::default(),
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
            ))
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(machine_entity),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));

        app.update();

        let world = app.world();
        assert_eq!(
            *world.get::<MachineState>(machine_entity).unwrap(),
            MachineState::Running
        );
        assert!(world.get::<MachineActivity>(machine_entity).is_some());
    }

    #[test]
    fn locked_recipe_not_started_with_progress_resource() {
        let rg = single_recipe_graph(test_recipe_def(
            "furnace",
            &[("iron", 1.0)],
            &[("copper", 1.0)],
        ));
        let mut app = recipe_io_app(rg);
        // TechTreeProgress present but recipe not in unlocked_recipes → locked
        app.insert_resource(TechTreeProgress::default());

        let net_entity = app.world_mut().spawn(LogisticsNetwork).id();
        let storage_e = app
            .world_mut()
            .spawn(StorageUnit {
                items: [("iron".to_owned(), 10u32)].into_iter().collect(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_e),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));
        let machine_entity = app
            .world_mut()
            .spawn((bare_machine("furnace"), MachineState::Idle))
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(machine_entity),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));

        app.world_mut().write_message(NetworkStorageChanged {
            network: net_entity,
        });
        app.update();

        assert_eq!(
            *app.world().get::<MachineState>(machine_entity).unwrap(),
            MachineState::Idle
        );
    }

    #[test]
    fn recipe_completion_adds_research_points() {
        let rg = single_recipe_graph(test_recipe_def(
            "furnace",
            &[],
            &[(RESEARCH_POINTS_ID, 5.0)],
        ));
        let mut app = recipe_io_app(rg);
        app.insert_resource(ResearchPool::default());

        let net_entity = app.world_mut().spawn(LogisticsNetwork).id();
        let storage_e = app
            .world_mut()
            .spawn(StorageUnit {
                items: HashMap::new(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_e),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));
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
            ))
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(machine_entity),
            Transform::default(),
            LogisticsNetworkMember(net_entity),
        ));

        app.update();

        assert_eq!(app.world().resource::<ResearchPool>().points, 5.0);
    }

    #[test]
    fn idle_machine_starts_recipe_when_power_network_gains_capacity() {
        use crate::machine::EnergyPortOf;
        use crate::network::{NetworkChanged, Power};
        use crate::power::{PowerNetwork, PowerNetworkMember};

        let mut recipe = test_recipe_def("smelter", &[("iron_ore", 1.0)], &[("iron_ingot", 1.0)]);
        recipe.energy_cost = 10.0;
        let rg = single_recipe_graph(recipe);
        let mut app = recipe_io_app(rg);

        let net_e = app.world_mut().spawn(LogisticsNetwork).id();
        let storage_e = app
            .world_mut()
            .spawn(StorageUnit {
                items: [("iron_ore".to_owned(), 5u32)].into_iter().collect(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_e),
            Transform::default(),
            LogisticsNetworkMember(net_e),
        ));
        let machine_e = app
            .world_mut()
            .spawn((bare_machine("smelter"), MachineState::Idle))
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(machine_e),
            Transform::default(),
            LogisticsNetworkMember(net_e),
        ));

        let power_net_e = app
            .world_mut()
            .spawn(PowerNetwork {
                capacity_watts: 50.0,
            })
            .id();
        app.world_mut()
            .spawn((EnergyPortOf(machine_e), PowerNetworkMember(power_net_e)));

        // Trigger via power network change (not storage change) — regression for bug where
        // connecting a generator via power cable never re-triggered recipe_start_system
        app.world_mut()
            .write_message(NetworkChanged::<Power>::new(power_net_e));
        app.update();

        assert_eq!(
            *app.world().get::<MachineState>(machine_e).unwrap(),
            MachineState::Running,
            "smelter should start when power network gains capacity"
        );
    }
}

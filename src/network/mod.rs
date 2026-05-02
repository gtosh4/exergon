use std::collections::HashSet;
use std::marker::PhantomData;

use bevy::ecs::message::Message;
use bevy::prelude::*;

use crate::inventory::ItemRegistry;
use crate::machine::{Machine, MachineScanSet};

pub mod bfs;
pub mod membership;

pub use membership::{cable_placed_system, cable_removed_system, machine_membership_system};

/// The six axis-aligned directions in 3D voxel space.
pub const DIRS: [IVec3; 6] = [
    IVec3::new(1, 0, 0),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 0, -1),
];

// -- Network kind marker types -----------------------------------------------

pub struct Power;
pub struct Logistics;

// -- Helper traits -----------------------------------------------------------

/// Implemented by cable block components to expose their world position.
pub trait HasPos {
    fn pos(&self) -> IVec3;
}

/// Marks an entity as belonging to a single network of kind `N`.
pub trait NetworkMemberComponent: Component {
    fn new(network: Entity) -> Self;
    fn network(&self) -> Entity;
}

/// Stores the full member list on the network entity itself.
pub trait NetworkMembersComponent: Component {
    fn members(&self) -> &[Entity];
}

// -- NetworkKind trait -------------------------------------------------------

/// Parameterises the generic network systems over a concrete network type (e.g. [`Power`], [`Logistics`]).
pub trait NetworkKind: Send + Sync + 'static {
    const CABLE_ITEM_ID: &'static str;

    type CableBlock: Component + HasPos;
    type Member: NetworkMemberComponent;
    type Members: NetworkMembersComponent;

    fn io_blocks(machine: &Machine) -> &HashSet<IVec3>;
    fn new_cable_block(pos: IVec3) -> Self::CableBlock;
    fn spawn_network(commands: &mut Commands) -> Entity;
}

// -- Generic message ---------------------------------------------------------

/// Sent whenever a network's membership changes (cables placed/removed, machines joined/left).
pub struct NetworkChanged<N: Send + Sync + 'static> {
    pub network: Entity,
    _n: PhantomData<N>,
}

impl<N: Send + Sync + 'static> NetworkChanged<N> {
    pub fn new(network: Entity) -> Self {
        NetworkChanged {
            network,
            _n: PhantomData,
        }
    }
}

impl<N: Send + Sync + 'static> Clone for NetworkChanged<N> {
    fn clone(&self) -> Self {
        NetworkChanged {
            network: self.network,
            _n: PhantomData,
        }
    }
}

impl<N: Send + Sync + 'static> Message for NetworkChanged<N> {}

// -- SystemSet & Plugin ------------------------------------------------------

/// SystemSet for the generic cable/membership systems of a network kind.
/// Identified by the `TypeId` of the kind marker (e.g. `Power`, `Logistics`).
/// Use `NetworkSystems::of::<N>()` to refer to the set for kind `N`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkSystems(std::any::TypeId);

impl NetworkSystems {
    pub fn of<N: NetworkKind>() -> Self {
        Self(std::any::TypeId::of::<N>())
    }
}

pub struct NetworkPlugin<N: NetworkKind>(PhantomData<N>);

impl<N: NetworkKind> Default for NetworkPlugin<N> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<N: NetworkKind> Plugin for NetworkPlugin<N> {
    fn build(&self, app: &mut App) {
        app.add_message::<NetworkChanged<N>>();
        app.configure_sets(
            Update,
            NetworkSystems::of::<N>()
                .after(MachineScanSet)
                .in_set(crate::GameSystems::Simulation),
        );
        app.add_systems(
            Update,
            (
                cable_placed_system::<N>.run_if(resource_exists::<ItemRegistry>),
                cable_removed_system::<N>.run_if(resource_exists::<ItemRegistry>),
                machine_membership_system::<N>.run_if(resource_exists::<ItemRegistry>),
            )
                .chain()
                .in_set(NetworkSystems::of::<N>()),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bevy::prelude::*;

    use super::*;
    use crate::inventory::{BlockProps, ItemDef, ItemRegistry};
    use crate::machine::{Machine, MachineNetworkChanged};
    use crate::world::{BlockChangeKind, BlockChangedMessage};

    struct TestNetwork;

    const TEST_CABLE_ID: &str = "test_cable";
    const TEST_CABLE_VOXEL: u8 = 1;

    #[derive(Component)]
    struct TestCableBlock {
        pos: IVec3,
    }

    impl HasPos for TestCableBlock {
        fn pos(&self) -> IVec3 {
            self.pos
        }
    }

    #[derive(Component)]
    #[relationship(relationship_target = TestNetworkMembers)]
    struct TestNetworkMember(Entity);

    #[derive(Component)]
    #[relationship_target(relationship = TestNetworkMember)]
    struct TestNetworkMembers(Vec<Entity>);

    impl NetworkMemberComponent for TestNetworkMember {
        fn new(network: Entity) -> Self {
            TestNetworkMember(network)
        }
        fn network(&self) -> Entity {
            self.0
        }
    }

    impl NetworkMembersComponent for TestNetworkMembers {
        fn members(&self) -> &[Entity] {
            &self.0
        }
    }

    #[derive(Component)]
    struct TestNetworkMarker;

    impl NetworkKind for TestNetwork {
        const CABLE_ITEM_ID: &'static str = TEST_CABLE_ID;
        type CableBlock = TestCableBlock;
        type Member = TestNetworkMember;
        type Members = TestNetworkMembers;

        fn io_blocks(machine: &Machine) -> &HashSet<IVec3> {
            &machine.energy_io_blocks
        }

        fn new_cable_block(pos: IVec3) -> TestCableBlock {
            TestCableBlock { pos }
        }

        fn spawn_network(commands: &mut Commands) -> Entity {
            commands.spawn(TestNetworkMarker).id()
        }
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<BlockChangedMessage>()
            .add_message::<MachineNetworkChanged>()
            .insert_resource(ItemRegistry::default())
            .add_plugins(NetworkPlugin::<TestNetwork>::default());
        {
            let mut reg = app.world_mut().resource_mut::<ItemRegistry>();
            reg.register(ItemDef {
                id: TEST_CABLE_ID.to_string(),
                name: TEST_CABLE_ID.to_string(),
                block: Some(BlockProps {
                    voxel_id: TEST_CABLE_VOXEL,
                    hardness: 1.0,
                }),
            });
        }
        app
    }

    fn network_count(app: &mut App) -> usize {
        let world = app.world_mut();
        world
            .query_filtered::<(), With<TestNetworkMarker>>()
            .iter(world)
            .count()
    }

    fn place_cable(app: &mut App, pos: IVec3) {
        app.world_mut().write_message(BlockChangedMessage {
            pos,
            kind: BlockChangeKind::Placed {
                voxel_id: TEST_CABLE_VOXEL,
            },
        });
    }

    fn remove_cable(app: &mut App, pos: IVec3) {
        app.world_mut().write_message(BlockChangedMessage {
            pos,
            kind: BlockChangeKind::Removed {
                voxel_id: TEST_CABLE_VOXEL,
            },
        });
    }

    #[test]
    fn no_cables_no_networks() {
        let mut app = test_app();
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn single_cable_creates_one_network() {
        let mut app = test_app();
        place_cable(&mut app, IVec3::ZERO);
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn two_adjacent_cables_one_network() {
        let mut app = test_app();
        place_cable(&mut app, IVec3::ZERO);
        app.update();
        place_cable(&mut app, IVec3::new(1, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn two_disconnected_cables_two_networks() {
        let mut app = test_app();
        place_cable(&mut app, IVec3::ZERO);
        app.update();
        place_cable(&mut app, IVec3::new(5, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
    }

    #[test]
    fn cable_removed_clears_network() {
        let mut app = test_app();
        place_cable(&mut app, IVec3::ZERO);
        app.update();
        remove_cable(&mut app, IVec3::ZERO);
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn middle_cable_removed_splits_network() {
        let mut app = test_app();
        place_cable(&mut app, IVec3::new(0, 0, 0));
        app.update();
        place_cable(&mut app, IVec3::new(1, 0, 0));
        app.update();
        place_cable(&mut app, IVec3::new(2, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
        remove_cable(&mut app, IVec3::new(1, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
    }

    #[test]
    fn placing_cable_between_two_merges_networks() {
        let mut app = test_app();
        place_cable(&mut app, IVec3::new(0, 0, 0));
        app.update();
        place_cable(&mut app, IVec3::new(2, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
        place_cable(&mut app, IVec3::new(1, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }
}

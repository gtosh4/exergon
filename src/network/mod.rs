use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::marker::PhantomData;

use bevy::ecs::message::Message;
use bevy::prelude::*;

use crate::machine::{Machine, MachineScanSet};
use crate::world::CableConnectionEvent;

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

/// Exposes the two IO-port endpoints of a cable segment component.
pub trait HasEndpoints {
    fn endpoints(&self) -> [IVec3; 2];
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

/// Parameterises the generic network systems over a concrete network type
/// (e.g. [`Power`], [`Logistics`]).
pub trait NetworkKind: Send + Sync + 'static {
    const CABLE_ITEM_ID: &'static str;

    type CableSegment: Component + HasEndpoints;
    type Member: NetworkMemberComponent;
    type Members: NetworkMembersComponent;

    fn io_ports(machine: &Machine) -> &HashSet<IVec3>;
    fn new_cable_segment(from: IVec3, to: IVec3, blocked: &HashSet<IVec3>) -> Self::CableSegment;
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

// -- Auto-routing ------------------------------------------------------------

/// Returns the Manhattan path from `from` to `to`, stepping X → Y → Z.
pub fn auto_route(from: IVec3, to: IVec3) -> Vec<IVec3> {
    let mut path = vec![from];
    let mut cur = from;
    while cur.x != to.x {
        cur.x += (to.x - cur.x).signum();
        path.push(cur);
    }
    while cur.y != to.y {
        cur.y += (to.y - cur.y).signum();
        path.push(cur);
    }
    while cur.z != to.z {
        cur.z += (to.z - cur.z).signum();
        path.push(cur);
    }
    path
}

/// A* path from `from` to `to` avoiding `blocked` voxels, preferring straight runs.
/// Falls back to [`auto_route`] when no path is found.
pub fn route_avoiding(from: IVec3, to: IVec3, blocked: &HashSet<IVec3>) -> Vec<IVec3> {
    if from == to {
        return vec![from];
    }

    #[derive(Copy, Clone, Eq, PartialEq)]
    struct Node {
        f: u32,
        g: u32,
        pos: IVec3,
        dir: IVec3,
    }

    impl Ord for Node {
        fn cmp(&self, other: &Self) -> Ordering {
            other.f.cmp(&self.f)
        }
    }

    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    const TURN_PENALTY: u32 = 3;

    let h = |p: IVec3| -> u32 {
        let d = (to - p).abs();
        (d.x + d.y + d.z) as u32
    };

    let max_nodes = h(from) * 30 + 500;
    let mut explored: u32 = 0;

    let mut g_scores: HashMap<(IVec3, IVec3), u32> = HashMap::new();
    let mut came_from: HashMap<(IVec3, IVec3), (IVec3, IVec3)> = HashMap::new();
    let mut heap: BinaryHeap<Node> = BinaryHeap::new();

    g_scores.insert((from, IVec3::ZERO), 0);
    heap.push(Node {
        f: h(from),
        g: 0,
        pos: from,
        dir: IVec3::ZERO,
    });

    while let Some(Node { g, pos, dir, .. }) = heap.pop() {
        explored += 1;
        if explored > max_nodes {
            break;
        }

        if pos == to {
            let mut path = vec![pos];
            let mut state = (pos, dir);
            while let Some(&prev) = came_from.get(&state) {
                path.push(prev.0);
                state = prev;
            }
            path.reverse();
            return path;
        }

        if g_scores.get(&(pos, dir)).copied().unwrap_or(u32::MAX) < g {
            continue;
        }

        for &d in &DIRS {
            let next = pos + d;
            if blocked.contains(&next) && next != to {
                continue;
            }
            let turn_cost = if dir != IVec3::ZERO && d != dir {
                TURN_PENALTY
            } else {
                0
            };
            let new_g = g + 1 + turn_cost;
            let state = (next, d);
            if new_g < g_scores.get(&state).copied().unwrap_or(u32::MAX) {
                g_scores.insert(state, new_g);
                came_from.insert(state, (pos, dir));
                heap.push(Node {
                    f: new_g + h(next),
                    g: new_g,
                    pos: next,
                    dir: d,
                });
            }
        }
    }

    auto_route(from, to)
}

// -- SystemSet & Plugin ------------------------------------------------------

/// SystemSet for the generic cable/membership systems of a network kind.
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
        app.add_message::<CableConnectionEvent>();
        app.configure_sets(
            Update,
            NetworkSystems::of::<N>()
                .after(MachineScanSet)
                .in_set(crate::GameSystems::Simulation),
        );
        app.add_systems(
            Update,
            (
                cable_placed_system::<N>,
                cable_removed_system::<N>,
                machine_membership_system::<N>,
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
    use crate::machine::{Machine, MachineNetworkChanged};
    use crate::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

    struct TestNetwork;

    const TEST_CABLE_ID: &str = "test_cable";

    #[derive(Component)]
    struct TestCableSegment {
        from: IVec3,
        to: IVec3,
        path: Vec<IVec3>,
    }

    impl HasEndpoints for TestCableSegment {
        fn endpoints(&self) -> [IVec3; 2] {
            [self.from, self.to]
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
        type CableSegment = TestCableSegment;
        type Member = TestNetworkMember;
        type Members = TestNetworkMembers;

        fn io_ports(machine: &Machine) -> &HashSet<IVec3> {
            &machine.energy_ports
        }

        fn new_cable_segment(
            from: IVec3,
            to: IVec3,
            _blocked: &HashSet<IVec3>,
        ) -> TestCableSegment {
            TestCableSegment {
                from,
                to,
                path: auto_route(from, to),
            }
        }

        fn spawn_network(commands: &mut Commands) -> Entity {
            commands.spawn(TestNetworkMarker).id()
        }
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<CableConnectionEvent>()
            .add_message::<MachineNetworkChanged>()
            .add_plugins(NetworkPlugin::<TestNetwork>::default());
        app
    }

    fn network_count(app: &mut App) -> usize {
        let world = app.world_mut();
        world
            .query_filtered::<(), With<TestNetworkMarker>>()
            .iter(world)
            .count()
    }

    fn connect(app: &mut App, from: IVec3, to: IVec3) {
        app.world_mut().write_message(CableConnectionEvent {
            from,
            to,
            item_id: TEST_CABLE_ID.to_string(),
            kind: WorldObjectKind::Placed,
        });
    }

    fn disconnect_at(app: &mut App, pos: IVec3) {
        app.world_mut().write_message(WorldObjectEvent {
            pos: pos.as_vec3(),
            item_id: TEST_CABLE_ID.to_string(),
            kind: WorldObjectKind::Removed,
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
        connect(&mut app, IVec3::new(0, 0, 0), IVec3::new(5, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn two_cables_sharing_endpoint_one_network() {
        let mut app = test_app();
        connect(&mut app, IVec3::new(0, 0, 0), IVec3::new(5, 0, 0));
        app.update();
        connect(&mut app, IVec3::new(5, 0, 0), IVec3::new(10, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }

    #[test]
    fn two_cables_no_shared_endpoint_two_networks() {
        let mut app = test_app();
        connect(&mut app, IVec3::new(0, 0, 0), IVec3::new(5, 0, 0));
        app.update();
        connect(&mut app, IVec3::new(20, 0, 0), IVec3::new(25, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
    }

    #[test]
    fn cable_removed_clears_network() {
        let mut app = test_app();
        connect(&mut app, IVec3::new(0, 0, 0), IVec3::new(5, 0, 0));
        app.update();
        disconnect_at(&mut app, IVec3::new(0, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 0);
    }

    #[test]
    fn removing_bridge_cable_splits_network() {
        let mut app = test_app();
        // A-B and B-C: B is the shared endpoint
        connect(&mut app, IVec3::new(0, 0, 0), IVec3::new(5, 0, 0));
        app.update();
        connect(&mut app, IVec3::new(5, 0, 0), IVec3::new(10, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
        // Remove cable whose endpoint is B=(5,0,0)
        disconnect_at(&mut app, IVec3::new(5, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 0); // both removed since both had B as endpoint
    }

    #[test]
    fn placing_cable_between_two_networks_merges_them() {
        let mut app = test_app();
        connect(&mut app, IVec3::new(0, 0, 0), IVec3::new(5, 0, 0));
        app.update();
        connect(&mut app, IVec3::new(20, 0, 0), IVec3::new(25, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 2);
        // Connect the two networks via a shared endpoint
        connect(&mut app, IVec3::new(5, 0, 0), IVec3::new(20, 0, 0));
        app.update();
        assert_eq!(network_count(&mut app), 1);
    }
}

#[cfg(test)]
mod auto_route_tests {
    use bevy::prelude::IVec3;

    use super::auto_route;

    #[test]
    fn same_point_returns_single_element() {
        let path = auto_route(IVec3::ZERO, IVec3::ZERO);
        assert_eq!(path, vec![IVec3::ZERO]);
    }

    #[test]
    fn along_x_axis() {
        let path = auto_route(IVec3::new(0, 0, 0), IVec3::new(3, 0, 0));
        assert_eq!(
            path,
            vec![
                IVec3::new(0, 0, 0),
                IVec3::new(1, 0, 0),
                IVec3::new(2, 0, 0),
                IVec3::new(3, 0, 0),
            ]
        );
    }

    #[test]
    fn xyz_order() {
        let path = auto_route(IVec3::new(0, 0, 0), IVec3::new(2, 1, 1));
        // X first: 0→2, then Y: 0→1, then Z: 0→1
        assert_eq!(path[0], IVec3::new(0, 0, 0));
        assert_eq!(*path.last().unwrap(), IVec3::new(2, 1, 1));
        // All steps differ by exactly 1
        for w in path.windows(2) {
            let diff = w[1] - w[0];
            assert_eq!(diff.abs().element_sum(), 1);
        }
    }
}

#[cfg(test)]
mod route_avoiding_tests {
    use std::collections::HashSet;

    use bevy::prelude::IVec3;

    use super::{DIRS, route_avoiding};

    #[test]
    fn straight_line_no_obstacles() {
        let path = route_avoiding(IVec3::ZERO, IVec3::new(4, 0, 0), &HashSet::new());
        assert_eq!(path.first(), Some(&IVec3::ZERO));
        assert_eq!(path.last(), Some(&IVec3::new(4, 0, 0)));
        assert_eq!(path.len(), 5);
        let has_turn = path.windows(3).any(|w| (w[1] - w[0]) != (w[2] - w[1]));
        assert!(!has_turn, "straight line should have no turns");
    }

    #[test]
    fn routes_around_single_obstacle() {
        let mut blocked = HashSet::new();
        blocked.insert(IVec3::new(1, 0, 0));
        let path = route_avoiding(IVec3::ZERO, IVec3::new(2, 0, 0), &blocked);
        assert_eq!(*path.first().unwrap(), IVec3::ZERO);
        assert_eq!(*path.last().unwrap(), IVec3::new(2, 0, 0));
        assert!(!path.contains(&IVec3::new(1, 0, 0)));
    }

    #[test]
    fn fallback_when_all_neighbors_blocked() {
        let blocked: HashSet<IVec3> = DIRS.iter().map(|&d| IVec3::ZERO + d).collect();
        let to = IVec3::new(3, 0, 0);
        let path = route_avoiding(IVec3::ZERO, to, &blocked);
        assert_eq!(*path.first().unwrap(), IVec3::ZERO);
        assert_eq!(*path.last().unwrap(), to);
    }
}

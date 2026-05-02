use std::collections::HashSet;
use std::marker::PhantomData;

use bevy::ecs::message::Message;
use bevy::prelude::*;

use crate::machine::Machine;

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

// -- Registration ------------------------------------------------------------

/// Register message type and system-set ordering for network type `N`.
/// Call this from each type-specific plugin's `build` method.
pub fn init_network<N: NetworkKind>(app: &mut App) {
    app.add_message::<NetworkChanged<N>>();
}

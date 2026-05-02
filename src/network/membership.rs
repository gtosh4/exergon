use std::collections::{HashMap, HashSet};

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::inventory::ItemRegistry;
use crate::machine::{Machine, MachineNetworkChanged, MachineUnformed};
use crate::world::{BlockChangeKind, BlockChangedMessage};

use super::bfs::find_components;
use super::{
    DIRS, HasPos, NetworkChanged, NetworkKind, NetworkMemberComponent, NetworkMembersComponent,
};

/// Spawns cable entities and merges/assigns networks when cable blocks are placed.
pub fn cable_placed_system<N: NetworkKind>(
    mut commands: Commands,
    mut block_events: MessageReader<BlockChangedMessage>,
    item_registry: Res<ItemRegistry>,
    cable_q: Query<(Entity, &N::CableBlock, &N::Member)>,
    net_members_q: Query<&N::Members>,
    machine_q: Query<(Entity, &Machine), Without<MachineUnformed>>,
    mut changed: MessageWriter<NetworkChanged<N>>,
) {
    let Some(cable_vox) = item_registry.voxel_id(N::CABLE_ITEM_ID) else {
        return;
    };

    let mut new_positions: Vec<IVec3> = Vec::new();
    for ev in block_events.read() {
        let (removed, added) = match ev.kind {
            BlockChangeKind::Placed { voxel_id } => (None, Some(voxel_id)),
            BlockChangeKind::Removed { voxel_id } => (Some(voxel_id), None),
            BlockChangeKind::Replaced {
                old_voxel_id,
                new_voxel_id,
            } => (Some(old_voxel_id), Some(new_voxel_id)),
        };
        let is_added = added.is_some_and(|v| v == cable_vox);
        let is_removed = removed.is_some_and(|v| v == cable_vox);
        if is_added && !is_removed {
            new_positions.push(ev.pos);
        }
    }

    if new_positions.is_empty() {
        return;
    }

    // Build mutable pos_map so new cables added this frame are visible to later iterations
    let mut pos_map: HashMap<IVec3, (Entity, Entity)> = cable_q
        .iter()
        .map(|(e, c, m)| (c.pos(), (e, m.network())))
        .collect();

    for pos in new_positions {
        let adjacent_nets: HashSet<Entity> = DIRS
            .iter()
            .filter_map(|&d| pos_map.get(&(pos + d)))
            .map(|&(_, net)| net)
            .collect();

        let new_cable_e = commands.spawn(N::new_cable_block(pos)).id();

        let target_net = if adjacent_nets.is_empty() {
            // No neighbors: spawn fresh network
            N::spawn_network(&mut commands)
        } else {
            // Merge into largest; for len==1 the loop is a no-op
            let Some(&survivor) = adjacent_nets.iter().max_by_key(|&&net| {
                net_members_q
                    .get(net)
                    .map(|m| m.members().len())
                    .unwrap_or(0)
            }) else {
                continue;
            };

            for &absorbed in adjacent_nets.iter().filter(|&&n| n != survivor) {
                if let Ok(members) = net_members_q.get(absorbed) {
                    for &member_e in members.members() {
                        commands.entity(member_e).insert(N::Member::new(survivor));
                    }
                }
                commands.entity(absorbed).despawn();
            }
            survivor
        };

        commands
            .entity(new_cable_e)
            .insert(N::Member::new(target_net));

        // Assign machines adjacent to pos
        for (machine_e, machine) in &machine_q {
            let adjacent = N::io_blocks(machine)
                .iter()
                .any(|&io_pos| DIRS.iter().any(|&d| io_pos + d == pos));
            if adjacent {
                commands
                    .entity(machine_e)
                    .insert(N::Member::new(target_net));
            }
        }

        pos_map.insert(pos, (new_cable_e, target_net));
        changed.write(NetworkChanged::new(target_net));
    }
}

/// Despawns cable entities and splits/reassigns networks when cable blocks are removed.
pub fn cable_removed_system<N: NetworkKind>(
    mut commands: Commands,
    mut block_events: MessageReader<BlockChangedMessage>,
    item_registry: Res<ItemRegistry>,
    cable_q: Query<(Entity, &N::CableBlock, &N::Member)>,
    machine_q: Query<(Entity, &Machine, Option<&N::Member>), Without<MachineUnformed>>,
    unformed_q: Query<Entity, (With<MachineUnformed>, With<N::Member>)>,
    mut changed: MessageWriter<NetworkChanged<N>>,
) {
    let Some(cable_vox) = item_registry.voxel_id(N::CABLE_ITEM_ID) else {
        return;
    };

    let mut removed_positions: Vec<IVec3> = Vec::new();
    for ev in block_events.read() {
        let (removed, added) = match ev.kind {
            BlockChangeKind::Placed { voxel_id } => (None, Some(voxel_id)),
            BlockChangeKind::Removed { voxel_id } => (Some(voxel_id), None),
            BlockChangeKind::Replaced {
                old_voxel_id,
                new_voxel_id,
            } => (Some(old_voxel_id), Some(new_voxel_id)),
        };
        let is_removed = removed.is_some_and(|v| v == cable_vox);
        let is_added = added.is_some_and(|v| v == cable_vox);
        if is_removed && !is_added {
            removed_positions.push(ev.pos);
        }
    }

    // Remove N::Member from unformed machines (they lost their network membership)
    for e in &unformed_q {
        commands.entity(e).remove::<N::Member>();
    }

    if removed_positions.is_empty() {
        return;
    }

    // Snapshot all cables before any despawns (deferred, but we need stable data)
    let all_cables: Vec<(Entity, IVec3, Entity)> = cable_q
        .iter()
        .map(|(e, c, m)| (e, c.pos(), m.network()))
        .collect();

    for pos in removed_positions {
        let Some((cable_e, _, net)) = all_cables.iter().find(|(_, p, _)| *p == pos).copied() else {
            continue;
        };

        let remaining: HashMap<IVec3, Entity> = all_cables
            .iter()
            .filter(|(e, _, _)| *e != cable_e)
            .map(|(e, p, _)| (*p, *e))
            .collect();

        commands.entity(cable_e).despawn();

        if remaining.is_empty() {
            // Last cable in this network — remove machine memberships adjacent to pos
            for (machine_e, _, maybe_member) in &machine_q {
                if maybe_member.is_some_and(|m| m.network() == net) {
                    commands.entity(machine_e).remove::<N::Member>();
                }
            }
            commands.entity(net).despawn();
            return;
        }

        let components = find_components(remaining, pos);

        if components.len() == 1 {
            // No split; just clean up machine memberships near removed cable
            if let Some(comp) = components.first() {
                update_machines_near_pos::<N>(&mut commands, pos, net, &machine_q, comp);
            }
            changed.write(NetworkChanged::new(net));
            return;
        }

        // Real split: re-use net for largest component, spawn new nets for rest
        let mut sorted: Vec<HashMap<IVec3, Entity>> = components;
        sorted.sort_unstable_by_key(|c| std::cmp::Reverse(c.len()));

        // components[0] stays on `net`
        let main_component = sorted.remove(0);

        let mut all_nets_and_components: Vec<(Entity, HashMap<IVec3, Entity>)> =
            vec![(net, main_component)];

        for component in sorted {
            let new_net = N::spawn_network(&mut commands);
            for &cable_e_i in component.values() {
                commands.entity(cable_e_i).insert(N::Member::new(new_net));
            }
            all_nets_and_components.push((new_net, component));
        }

        // Re-assign machines based on which component their io_blocks are adjacent to
        reassign_machines::<N>(&mut commands, net, &all_nets_and_components, &machine_q);

        for (new_net, _) in &all_nets_and_components {
            changed.write(NetworkChanged::new(*new_net));
        }
    }
}

/// Recomputes machine→network membership whenever machines are formed or unformed.
pub fn machine_membership_system<N: NetworkKind>(
    mut events: MessageReader<MachineNetworkChanged>,
    cable_q: Query<(Entity, &N::CableBlock, &N::Member)>,
    machine_q: Query<(Entity, &Machine), Without<MachineUnformed>>,
    unformed_q: Query<Entity, (With<MachineUnformed>, With<N::Member>)>,
    mut commands: Commands,
    mut changed: MessageWriter<NetworkChanged<N>>,
) {
    if events.read().count() == 0 {
        return;
    }

    // Remove membership from unformed machines
    for e in &unformed_q {
        commands.entity(e).remove::<N::Member>();
    }

    let cable_pos_to_net: HashMap<IVec3, Entity> = cable_q
        .iter()
        .map(|(_, c, m)| (c.pos(), m.network()))
        .collect();

    let mut affected_nets: HashSet<Entity> = HashSet::new();

    for (machine_e, machine) in &machine_q {
        let new_net = N::io_blocks(machine)
            .iter()
            .flat_map(|&io_pos| DIRS.iter().map(move |&d| io_pos + d))
            .find_map(|cable_pos| cable_pos_to_net.get(&cable_pos).copied());

        match new_net {
            Some(net) => {
                commands.entity(machine_e).insert(N::Member::new(net));
                affected_nets.insert(net);
            }
            None => {
                commands.entity(machine_e).remove::<N::Member>();
            }
        }
    }

    for net in affected_nets {
        changed.write(NetworkChanged::new(net));
    }
}

fn update_machines_near_pos<N: NetworkKind>(
    commands: &mut Commands,
    _removed_pos: IVec3,
    net: Entity,
    machine_q: &Query<(Entity, &Machine, Option<&N::Member>), Without<MachineUnformed>>,
    remaining: &HashMap<IVec3, Entity>,
) {
    for (machine_e, machine, maybe_member) in machine_q.iter() {
        if maybe_member.is_none_or(|m| m.network() != net) {
            continue;
        }
        // If the machine's only adjacent cable was removed_pos, check for remaining cables
        let still_adjacent = N::io_blocks(machine)
            .iter()
            .any(|&io_pos| DIRS.iter().any(|&d| remaining.contains_key(&(io_pos + d))));
        if !still_adjacent {
            commands.entity(machine_e).remove::<N::Member>();
        }
    }
}

fn reassign_machines<N: NetworkKind>(
    commands: &mut Commands,
    old_net: Entity,
    nets_and_components: &[(Entity, HashMap<IVec3, Entity>)],
    machine_q: &Query<(Entity, &Machine, Option<&N::Member>), Without<MachineUnformed>>,
) {
    // Build pos → new_net map from all components
    let pos_to_net: HashMap<IVec3, Entity> = nets_and_components
        .iter()
        .flat_map(|(net, comp)| comp.keys().map(|&p| (p, *net)))
        .collect();

    for (machine_e, machine, maybe_member) in machine_q.iter() {
        if maybe_member.is_none_or(|m| m.network() != old_net) {
            continue;
        }
        let new_net = N::io_blocks(machine)
            .iter()
            .flat_map(|&io_pos| DIRS.iter().map(move |&d| io_pos + d))
            .find_map(|cable_pos| pos_to_net.get(&cable_pos).copied());

        match new_net {
            Some(n) => {
                commands.entity(machine_e).insert(N::Member::new(n));
            }
            None => {
                commands.entity(machine_e).remove::<N::Member>();
            }
        }
    }
}

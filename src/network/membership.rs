use std::collections::{HashMap, HashSet};

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::machine::{Machine, MachineNetworkChanged, MachineUnformed};
use crate::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

use super::bfs::find_segment_components;
use super::{
    HasEndpoints, NetworkChanged, NetworkKind, NetworkMemberComponent, NetworkMembersComponent,
};

fn key(v: Vec3) -> IVec3 {
    v.round().as_ivec3()
}

fn port_matches_key(ports: &[Vec3], k: IVec3) -> bool {
    ports.iter().any(|p| key(*p) == k)
}

/// Spawns cable segment entities and merges/assigns networks when cable connections are made.
pub fn cable_placed_system<N: NetworkKind>(
    mut commands: Commands,
    mut cable_events: MessageReader<CableConnectionEvent>,
    cable_q: Query<(Entity, &N::CableSegment, &N::Member)>,
    net_members_q: Query<&N::Members>,
    machine_q: Query<(Entity, &Machine, &Transform), Without<MachineUnformed>>,
    mut changed: MessageWriter<NetworkChanged<N>>,
) {
    let new_connections: Vec<(Vec3, Vec3)> = cable_events
        .read()
        .filter(|ev| ev.kind == WorldObjectKind::Placed && ev.item_id == N::CABLE_ITEM_ID)
        .map(|ev| (ev.from, ev.to))
        .collect();

    if new_connections.is_empty() {
        return;
    }

    // endpoint → network, keys are rounded IVec3 for fast lookup
    let mut endpoint_to_net: HashMap<IVec3, Entity> = cable_q
        .iter()
        .flat_map(|(_, seg, m)| seg.endpoints().map(|ep| (key(ep), m.network())))
        .collect();

    let blocked: HashSet<IVec3> = machine_q
        .iter()
        .map(|(_, _, t)| t.translation.round().as_ivec3())
        .collect();

    for (from, to) in new_connections {
        if from.distance(to) < 0.01 {
            continue;
        }

        let from_k = key(from);
        let to_k = key(to);

        let adjacent_nets: HashSet<Entity> = [from_k, to_k]
            .iter()
            .filter_map(|k| endpoint_to_net.get(k).copied())
            .collect();

        let target_net = if adjacent_nets.is_empty() {
            N::spawn_network(&mut commands)
        } else {
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

        commands.spawn((
            N::new_cable_segment(from, to, &blocked),
            N::Member::new(target_net),
        ));

        for (machine_e, machine, _) in &machine_q {
            let ports = N::io_ports(machine);
            if port_matches_key(ports, from_k) || port_matches_key(ports, to_k) {
                commands
                    .entity(machine_e)
                    .insert(N::Member::new(target_net));
            }
        }

        endpoint_to_net.insert(from_k, target_net);
        endpoint_to_net.insert(to_k, target_net);

        changed.write(NetworkChanged::new(target_net));
    }
}

/// Despawns cable segments and splits/reassigns networks when IO-port positions are removed.
pub fn cable_removed_system<N: NetworkKind>(
    mut commands: Commands,
    mut world_events: MessageReader<WorldObjectEvent>,
    cable_q: Query<(Entity, &N::CableSegment, &N::Member)>,
    machine_q: Query<(Entity, &Machine, Option<&N::Member>), Without<MachineUnformed>>,
    unformed_q: Query<Entity, (With<MachineUnformed>, With<N::Member>)>,
    mut changed: MessageWriter<NetworkChanged<N>>,
) {
    for e in &unformed_q {
        commands.entity(e).remove::<N::Member>();
    }

    let removed_positions: Vec<IVec3> = world_events
        .read()
        .filter(|ev| ev.kind == WorldObjectKind::Removed && ev.item_id == N::CABLE_ITEM_ID)
        .map(|ev| ev.pos.round().as_ivec3())
        .collect();

    if removed_positions.is_empty() {
        return;
    }

    // Store Vec3 endpoints for new_cable_segment, IVec3 keys for BFS/HashMap
    let all_cables: Vec<(Entity, [Vec3; 2], Entity)> = cable_q
        .iter()
        .map(|(e, seg, m)| (e, seg.endpoints(), m.network()))
        .collect();

    for removed_pos in removed_positions {
        let to_remove: Vec<(Entity, [Vec3; 2], Entity)> = all_cables
            .iter()
            .filter(|(_, eps, _)| key(eps[0]) == removed_pos || key(eps[1]) == removed_pos)
            .cloned()
            .collect();

        if to_remove.is_empty() {
            continue;
        }

        let Some(first) = to_remove.first() else {
            continue;
        };
        let net = first.2;
        let removed_entities: HashSet<Entity> = to_remove.iter().map(|(e, _, _)| *e).collect();

        for &cable_e in &removed_entities {
            commands.entity(cable_e).despawn();
        }

        // Convert Vec3 endpoints to IVec3 for BFS
        let remaining: HashMap<Entity, [IVec3; 2]> = all_cables
            .iter()
            .filter(|(e, _, _)| !removed_entities.contains(e))
            .map(|(e, eps, _)| (*e, [key(eps[0]), key(eps[1])]))
            .collect();

        if remaining.is_empty() {
            for (machine_e, _, maybe_member) in &machine_q {
                if maybe_member.is_some_and(|m| m.network() == net) {
                    commands.entity(machine_e).remove::<N::Member>();
                }
            }
            commands.entity(net).despawn();
            continue;
        }

        let components = find_segment_components(remaining);

        if components.len() == 1 {
            let remaining_endpoints: HashSet<IVec3> = components
                .first()
                .into_iter()
                .flat_map(|c| c.values().flat_map(|&eps| eps))
                .collect();
            for (machine_e, machine, maybe_member) in &machine_q {
                if maybe_member.is_some_and(|m| m.network() == net)
                    && !N::io_ports(machine)
                        .iter()
                        .any(|p| remaining_endpoints.contains(&key(*p)))
                {
                    commands.entity(machine_e).remove::<N::Member>();
                }
            }
            changed.write(NetworkChanged::new(net));
            continue;
        }

        // Real split
        let mut sorted: Vec<HashMap<Entity, [IVec3; 2]>> = components;
        sorted.sort_unstable_by_key(|c| std::cmp::Reverse(c.len()));

        let main = sorted.remove(0);
        let mut all_nets: Vec<(Entity, HashMap<Entity, [IVec3; 2]>)> = vec![(net, main)];

        for comp in sorted {
            let new_net = N::spawn_network(&mut commands);
            for &cable_e_i in comp.keys() {
                commands.entity(cable_e_i).insert(N::Member::new(new_net));
            }
            all_nets.push((new_net, comp));
        }

        reassign_machines::<N>(&mut commands, net, &all_nets, &machine_q);

        for (new_net, _) in &all_nets {
            changed.write(NetworkChanged::new(*new_net));
        }
    }
}

/// Recomputes machine→network membership whenever machines are placed or removed.
pub fn machine_membership_system<N: NetworkKind>(
    mut events: MessageReader<MachineNetworkChanged>,
    cable_q: Query<(Entity, &N::CableSegment, &N::Member)>,
    machine_q: Query<(Entity, &Machine), Without<MachineUnformed>>,
    unformed_q: Query<Entity, (With<MachineUnformed>, With<N::Member>)>,
    mut commands: Commands,
    mut changed: MessageWriter<NetworkChanged<N>>,
) {
    if events.read().count() == 0 {
        return;
    }

    for e in &unformed_q {
        commands.entity(e).remove::<N::Member>();
    }

    let endpoint_to_net: HashMap<IVec3, Entity> = cable_q
        .iter()
        .flat_map(|(_, seg, m)| seg.endpoints().map(|ep| (key(ep), m.network())))
        .collect();

    let mut affected_nets: HashSet<Entity> = HashSet::new();

    for (machine_e, machine) in &machine_q {
        let new_net = N::io_ports(machine)
            .iter()
            .find_map(|port| endpoint_to_net.get(&key(*port)).copied());

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

fn reassign_machines<N: NetworkKind>(
    commands: &mut Commands,
    old_net: Entity,
    nets_and_components: &[(Entity, HashMap<Entity, [IVec3; 2]>)],
    machine_q: &Query<(Entity, &Machine, Option<&N::Member>), Without<MachineUnformed>>,
) {
    let endpoint_to_net: HashMap<IVec3, Entity> = nets_and_components
        .iter()
        .flat_map(|(net, comp)| comp.values().flat_map(|&eps| eps.map(|ep| (ep, *net))))
        .collect();

    for (machine_e, machine, maybe_member) in machine_q.iter() {
        if maybe_member.is_none_or(|m| m.network() != old_net) {
            continue;
        }
        let new_net = N::io_ports(machine)
            .iter()
            .find_map(|port| endpoint_to_net.get(&key(*port)).copied());

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

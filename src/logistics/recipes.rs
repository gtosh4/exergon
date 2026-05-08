use std::collections::HashSet;

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::machine::{EnergyPortOf, Machine};
use crate::machine::{MachineActivity, MachineEnergyPorts, MachineLogisticsPorts, MachineState};
use crate::network::{NetworkChanged, Power};
use crate::power::{PowerNetwork, PowerNetworkMember, PowerNetworkMembers};
use crate::recipe_graph::RecipeGraph;
use crate::research::{RESEARCH_POINTS_ID, ResearchPool, TechTreeProgress};

use super::items::{give_items, has_items, take_items};
use crate::network::NetworkMembersComponent;

use super::{LogisticsNetworkMember, LogisticsNetworkMembers, NetworkStorageChanged, StorageUnit};
use crate::machine::LogisticsPortOf;

pub(super) fn recipe_start_system(
    mut commands: Commands,
    mut storage_changed: MessageReader<NetworkStorageChanged>,
    mut power_changed: MessageReader<NetworkChanged<Power>>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    machine_q: Query<
        (Entity, &Machine, &MachineState, &MachineLogisticsPorts),
        Without<MachineActivity>,
    >,
    port_net_q: Query<&LogisticsNetworkMember>,
    energy_ports_q: Query<&MachineEnergyPorts>,
    port_power_q: Query<&PowerNetworkMember>,
    power_net_q: Query<&PowerNetwork>,
    power_members_q: Query<&PowerNetworkMembers>,
    energy_port_of_q: Query<&EnergyPortOf>,
    recipe_graph: Res<RecipeGraph>,
    progress: Option<Res<TechTreeProgress>>,
    mut storage_params: ParamSet<(Query<&StorageUnit>, Query<&mut StorageUnit>)>,
    port_of_q: Query<&LogisticsPortOf>,
) {
    let mut affected: HashSet<Entity> = storage_changed.read().map(|e| e.network).collect();

    // Power network changes can unblock recipes that were waiting for power
    for pnet_e in power_changed.read().map(|e| e.network) {
        let Ok(members) = power_members_q.get(pnet_e) else {
            continue;
        };
        for &port_e in members.members() {
            let Ok(energy_port) = energy_port_of_q.get(port_e) else {
                continue;
            };
            if let Ok((_, _, _, logistics_ports)) = machine_q.get(energy_port.0) {
                for &lport_e in logistics_ports.ports() {
                    if let Ok(member) = port_net_q.get(lport_e) {
                        affected.insert(member.0);
                    }
                }
            }
        }
    }

    if affected.is_empty() {
        return;
    }
    debug!(
        "recipe_start: NetworkStorageChanged for networks {:?}",
        affected
    );

    // Collect networks the machine is connected to via its port entities
    let machine_connected_nets = |logistics_ports: &MachineLogisticsPorts| -> Vec<Entity> {
        logistics_ports
            .ports()
            .iter()
            .filter_map(|&port_e| port_net_q.get(port_e).ok().map(|m| m.0))
            .collect()
    };

    let mut to_start: Vec<(Entity, String, Entity)> = Vec::new();
    {
        let storage_q = storage_params.p0();
        for (net_entity, members) in &net_q {
            if !affected.contains(&net_entity) {
                continue;
            }
            debug!(
                "recipe_start: checking network {:?} ({} members)",
                net_entity,
                members.members().len()
            );
            for (machine_e, machine, state, logistics_ports) in &machine_q {
                let connected_nets = machine_connected_nets(logistics_ports);
                if !connected_nets.contains(&net_entity) {
                    continue;
                }
                if *state != MachineState::Idle {
                    debug!(
                        "recipe_start: machine {:?} ({}) skipped — state={:?}",
                        machine_e, machine.machine_type, state
                    );
                    continue;
                }
                let matching_recipes: Vec<_> = recipe_graph
                    .recipes
                    .values()
                    .filter(|r| {
                        r.machine_type == machine.machine_type && r.machine_tier <= machine.tier
                    })
                    .collect();
                if matching_recipes.is_empty() {
                    debug!(
                        "recipe_start: machine {:?} ({}) — no recipes for this machine type",
                        machine_e, machine.machine_type
                    );
                    continue;
                }
                let mut started = false;
                for recipe in &matching_recipes {
                    if let Some(ref prog) = progress
                        && !prog.unlocked_recipes.contains(&recipe.id)
                    {
                        debug!(
                            "recipe_start: machine {:?} recipe {} locked",
                            machine_e, recipe.id
                        );
                        continue;
                    }
                    // Power check: if recipe has energy cost, require a connected power network with capacity
                    if recipe.energy_cost > 0.0 {
                        let has_power = energy_ports_q
                            .get(machine_e)
                            .ok()
                            .map(|energy_ports| {
                                energy_ports.ports().iter().any(|&energy_port_e| {
                                    port_power_q
                                        .get(energy_port_e)
                                        .ok()
                                        .and_then(|pm| power_net_q.get(pm.0).ok())
                                        .is_some_and(|pn| pn.capacity_watts > 0.0)
                                })
                            })
                            .unwrap_or(false);
                        if !has_power {
                            debug!(
                                "recipe_start: machine {:?} recipe {} requires power but none available",
                                machine_e, recipe.id
                            );
                            continue;
                        }
                    }
                    // Check inputs across any connected network that has them
                    let missing: Vec<_> = recipe
                        .inputs
                        .iter()
                        .filter(|input| {
                            // Check across all connected networks
                            !connected_nets.iter().any(|&cn| {
                                if let Ok((_, net_members)) = net_q.get(cn) {
                                    has_items(
                                        net_members,
                                        &storage_q,
                                        &port_of_q,
                                        &input.item,
                                        input.quantity as u32,
                                    )
                                } else {
                                    false
                                }
                            })
                        })
                        .collect();
                    if missing.is_empty() {
                        to_start.push((machine_e, recipe.id.clone(), net_entity));
                        started = true;
                        break;
                    } else {
                        debug!(
                            "recipe_start: machine {:?} recipe {} missing inputs: {:?}",
                            machine_e,
                            recipe.id,
                            missing
                                .iter()
                                .map(|i| (&i.item, i.quantity))
                                .collect::<Vec<_>>()
                        );
                    }
                }
                if !started {
                    debug!(
                        "recipe_start: machine {:?} ({}) — no startable recipe found",
                        machine_e, machine.machine_type
                    );
                }
            }
        }
    }

    {
        let mut storage_q = storage_params.p1();
        for (machine_e, recipe_id, net_entity) in to_start {
            let Some(recipe) = recipe_graph.recipes.get(&recipe_id) else {
                continue;
            };
            if let Ok((_, members)) = net_q.get(net_entity) {
                for input in &recipe.inputs {
                    take_items(
                        members,
                        &mut storage_q,
                        &port_of_q,
                        &input.item,
                        input.quantity as u32,
                    );
                }
            }
            commands.entity(machine_e).insert((
                MachineActivity {
                    recipe_id,
                    progress: 0.0,
                    speed_factor: 1.0,
                },
                MachineState::Running,
            ));
            info!("Machine {:?} started recipe", machine_e);
        }
    }
}

pub(super) fn recipe_progress_system(
    mut commands: Commands,
    time: Res<Time>,
    recipe_graph: Res<RecipeGraph>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    mut machine_q: Query<(
        Entity,
        &mut MachineActivity,
        &MachineState,
        &MachineLogisticsPorts,
    )>,
    mut storage_q: Query<&mut StorageUnit>,
    port_of_q: Query<&LogisticsPortOf>,
    port_net_q: Query<&LogisticsNetworkMember>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
    mut research_pool: Option<ResMut<ResearchPool>>,
) {
    let dt = time.delta_secs();

    let mut to_finish: Vec<(Entity, Vec<(String, u32)>, Option<Entity>)> = Vec::new();

    for (machine_e, mut activity, state, logistics_ports) in &mut machine_q {
        if *state != MachineState::Running {
            continue;
        }
        let Some(recipe) = recipe_graph.recipes.get(&activity.recipe_id) else {
            continue;
        };
        let new_progress = activity.progress + dt * activity.speed_factor;
        if new_progress >= recipe.processing_time {
            let outputs: Vec<(String, u32)> = recipe
                .outputs
                .iter()
                .chain(recipe.byproducts.iter())
                .map(|o| (o.item.clone(), o.quantity as u32))
                .collect();
            // Find first connected network
            let net = logistics_ports
                .ports()
                .iter()
                .find_map(|&port_e| port_net_q.get(port_e).ok().map(|m| m.0));
            to_finish.push((machine_e, outputs, net));
        } else {
            activity.progress = new_progress;
        }
    }

    for (machine_e, outputs, maybe_net) in to_finish {
        if let Some(net_entity) = maybe_net {
            if let Ok((_, members)) = net_q.get(net_entity) {
                for (item_id, count) in &outputs {
                    if *item_id == RESEARCH_POINTS_ID {
                        if let Some(ref mut pool) = research_pool {
                            pool.points += *count as f32;
                            info!("Research pool +{} points (total: {})", count, pool.points);
                        }
                    } else if *count > 0 {
                        give_items(members, &mut storage_q, &port_of_q, item_id, *count);
                    }
                }
                storage_changed.write(NetworkStorageChanged {
                    network: net_entity,
                });
            }
        } else {
            warn!(
                "Machine {:?} finished recipe but has no connected logistics network; outputs lost",
                machine_e
            );
        }
        commands
            .entity(machine_e)
            .remove::<MachineActivity>()
            .insert(MachineState::Idle);
        info!("Machine {:?} finished recipe", machine_e);
    }
}

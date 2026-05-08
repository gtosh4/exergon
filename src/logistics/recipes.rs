use std::collections::HashSet;

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::machine::{EnergyPortOf, Machine};
use crate::machine::{MachineActivity, MachineEnergyPorts, MachineLogisticsPorts, MachineState};
use crate::network::{NetworkChanged, Power};
use crate::power::{GeneratorUnit, PowerNetworkMember, PowerNetworkMembers};
use crate::recipe_graph::RecipeGraph;
use crate::research::{RESEARCH_POINTS_ID, ResearchPool, TechTreeProgress};

use super::items::{give_items, has_items, take_items};
use crate::network::NetworkMembersComponent;

use super::{
    LogisticsNetworkMember, LogisticsNetworkMembers, NetworkStorageChanged, PortPolicy, StorageUnit,
};
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
    power_members_q: Query<&PowerNetworkMembers>,
    energy_port_of_q: Query<&EnergyPortOf>,
    gen_q: Query<&GeneratorUnit>,
    recipe_graph: Res<RecipeGraph>,
    progress: Option<Res<TechTreeProgress>>,
    mut storage_params: ParamSet<(Query<&StorageUnit>, Query<&mut StorageUnit>)>,
    port_of_q: Query<&LogisticsPortOf>,
    policy_q: Query<&PortPolicy>,
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
                    // Power check: if recipe has energy cost, require buffer energy
                    if recipe.energy_cost > 0.0 {
                        let energy_per_tick = recipe.energy_cost / recipe.processing_time * 0.016;
                        let has_power = energy_ports_q
                            .get(machine_e)
                            .ok()
                            .map(|energy_ports| {
                                energy_ports.ports().iter().any(|&energy_port_e| {
                                    port_power_q
                                        .get(energy_port_e)
                                        .ok()
                                        .and_then(|pm| power_members_q.get(pm.0).ok())
                                        .is_some_and(|members| {
                                            members.has_energy(
                                                &gen_q,
                                                &energy_port_of_q,
                                                energy_per_tick,
                                            )
                                        })
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
                    // Input check: for each input, find networks of input-eligible ports
                    let missing: Vec<_> = recipe
                        .inputs
                        .iter()
                        .filter(|input| {
                            let input_nets: Vec<Entity> = logistics_ports
                                .ports()
                                .iter()
                                .filter_map(|&port_e| {
                                    let policy_ok = policy_q
                                        .get(port_e)
                                        .map(|p| p.allows_input(&input.item))
                                        .unwrap_or(true);
                                    if !policy_ok {
                                        return None;
                                    }
                                    port_net_q.get(port_e).ok().map(|m| m.0)
                                })
                                .collect();
                            !input_nets.iter().any(|&net_e| {
                                net_q.get(net_e).ok().is_some_and(|(_, net_members)| {
                                    has_items(
                                        net_members,
                                        &storage_q,
                                        &port_of_q,
                                        &input.item,
                                        input.quantity as u32,
                                    )
                                })
                            })
                        })
                        .collect();
                    if !missing.is_empty() {
                        debug!(
                            "recipe_start: machine {:?} recipe {} missing inputs: {:?}",
                            machine_e,
                            recipe.id,
                            missing
                                .iter()
                                .map(|i| (&i.item, i.quantity))
                                .collect::<Vec<_>>()
                        );
                        continue;
                    }
                    // Output check: verify at least one output-eligible port with a network
                    // for each non-research output
                    let outputs_blocked = recipe
                        .outputs
                        .iter()
                        .chain(recipe.byproducts.iter())
                        .any(|out| {
                            if out.item == RESEARCH_POINTS_ID {
                                return false;
                            }
                            let has_destination = logistics_ports.ports().iter().any(|&port_e| {
                                let policy_ok = policy_q
                                    .get(port_e)
                                    .map(|p| p.allows_output(&out.item))
                                    .unwrap_or(true);
                                policy_ok && port_net_q.get(port_e).is_ok()
                            });
                            !has_destination
                        });
                    if outputs_blocked {
                        debug!(
                            "recipe_start: machine {:?} recipe {} output has no destination",
                            machine_e, recipe.id
                        );
                        continue;
                    }
                    to_start.push((machine_e, recipe.id.clone(), net_entity));
                    started = true;
                    break;
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
                    // Take from input-eligible ports' networks
                    let input_nets: Vec<Entity> = {
                        let Ok((_, _, _, logistics_ports)) = machine_q.get(machine_e) else {
                            continue;
                        };
                        logistics_ports
                            .ports()
                            .iter()
                            .filter_map(|&port_e| {
                                let policy_ok = policy_q
                                    .get(port_e)
                                    .map(|p| p.allows_input(&input.item))
                                    .unwrap_or(true);
                                if !policy_ok {
                                    return None;
                                }
                                port_net_q.get(port_e).ok().map(|m| m.0)
                            })
                            .collect()
                    };
                    let take_net = input_nets.first().copied().unwrap_or(net_entity);
                    if let Ok((_, net_members)) = net_q.get(take_net) {
                        take_items(
                            net_members,
                            &mut storage_q,
                            &port_of_q,
                            &input.item,
                            input.quantity as u32,
                        );
                    } else if let Ok((_, net_members)) = net_q.get(net_entity) {
                        take_items(
                            net_members,
                            &mut storage_q,
                            &port_of_q,
                            &input.item,
                            input.quantity as u32,
                        );
                    }
                    // Fallback: if no input net found, use the members we already have
                    if input_nets.is_empty() {
                        // already handled above
                    }
                }
                // For zero-input recipes, we still need to use the original members variable
                // but we've already handled it in the loop above. Handle zero-input case:
                if recipe.inputs.is_empty() {
                    // nothing to take
                    let _ = members;
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
    energy_ports_q: Query<&MachineEnergyPorts>,
    port_power_q: Query<&PowerNetworkMember>,
    power_members_q: Query<&PowerNetworkMembers>,
    energy_port_of_q: Query<&EnergyPortOf>,
    mut gen_q: Query<&mut GeneratorUnit>,
    policy_q: Query<&PortPolicy>,
) {
    let dt = time.delta_secs();

    let mut to_finish: Vec<(Entity, Vec<(String, u32)>, Vec<Entity>)> = Vec::new();

    for (machine_e, mut activity, state, logistics_ports) in &mut machine_q {
        if *state != MachineState::Running {
            continue;
        }
        let Some(recipe) = recipe_graph.recipes.get(&activity.recipe_id) else {
            continue;
        };

        // Per-tick energy withdrawal
        if recipe.energy_cost > 0.0 {
            let energy_per_tick = recipe.energy_cost / recipe.processing_time * dt;
            let took_energy = energy_ports_q
                .get(machine_e)
                .ok()
                .map(|eps| {
                    eps.ports().iter().any(|&ep| {
                        port_power_q
                            .get(ep)
                            .ok()
                            .and_then(|pm| power_members_q.get(pm.0).ok())
                            .is_some_and(|members| {
                                members.take_energy(&mut gen_q, &energy_port_of_q, energy_per_tick)
                            })
                    })
                })
                .unwrap_or(false);
            if !took_energy {
                // Pause: skip progress this tick
                continue;
            }
        }

        let new_progress = activity.progress + dt * activity.speed_factor;
        if new_progress >= recipe.processing_time {
            let outputs: Vec<(String, u32)> = recipe
                .outputs
                .iter()
                .chain(recipe.byproducts.iter())
                .map(|o| (o.item.clone(), o.quantity as u32))
                .collect();
            // Collect port entities for output routing
            let port_entities: Vec<Entity> = logistics_ports.ports().to_vec();
            to_finish.push((machine_e, outputs, port_entities));
        } else {
            activity.progress = new_progress;
        }
    }

    for (machine_e, outputs, port_entities) in to_finish {
        for (item_id, count) in &outputs {
            if *item_id == RESEARCH_POINTS_ID {
                if let Some(ref mut pool) = research_pool {
                    pool.points += *count as f32;
                    info!("Research pool +{} points (total: {})", count, pool.points);
                }
                continue;
            }
            if *count == 0 {
                continue;
            }
            // Find output-eligible port's network for this item
            let target_net = port_entities.iter().find_map(|&port_e| {
                let policy_ok = policy_q
                    .get(port_e)
                    .map(|p| p.allows_output(item_id))
                    .unwrap_or(true);
                if !policy_ok {
                    return None;
                }
                port_net_q.get(port_e).ok().map(|m| m.0)
            });
            if let Some(net_e) = target_net {
                if let Ok((_, members)) = net_q.get(net_e) {
                    give_items(members, &mut storage_q, &port_of_q, item_id, *count);
                    storage_changed.write(NetworkStorageChanged { network: net_e });
                }
            } else {
                warn!(
                    "Machine {:?} finished recipe but output '{}' has no eligible port network; output lost",
                    machine_e, item_id
                );
            }
        }
        commands
            .entity(machine_e)
            .remove::<MachineActivity>()
            .insert(MachineState::Idle);
        info!("Machine {:?} finished recipe", machine_e);
    }
}

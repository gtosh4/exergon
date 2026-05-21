use std::collections::HashSet;

use bevy::ecs::message::{MessageReader, MessageWriter, Messages};
use bevy::prelude::*;

use crate::machine::{EnergyPortOf, Machine};
use crate::machine::{MachineActivity, MachineEnergyPorts, MachineLogisticsPorts, MachineState};
use crate::network::{NetworkChanged, Power};
use crate::power::{GeneratorUnit, PowerNetworkMember, PowerNetworkMembers};
use crate::recipe_graph::{ConcreteRecipe, RecipeGraph};
use crate::research::{RESEARCH_POINTS_ID, ResearchPool, TechTreeProgress};

use super::items::{give_items, has_items, take_items};
use crate::network::NetworkMembersComponent;

use super::{
    JobComplete, LogisticsNetworkMember, LogisticsNetworkMembers, NetworkStorageChanged,
    PortPolicy, StorageUnit,
};
use crate::machine::LogisticsPortOf;

// -- Intra-frame cascade messages --------------------------------------------

#[derive(Clone, Message)]
pub(super) struct RecipeToStart {
    pub machine: Entity,
    pub recipe_id: String,
    pub net: Entity,
}

#[derive(Clone, Message)]
pub(super) struct RecipeCompleted {
    pub machine: Entity,
    pub recipe_id: String,
    pub outputs: Vec<(String, u32)>,
    pub port_entities: Vec<Entity>,
    pub energy_output: f32,
}

// -- Feasibility helpers (pure, no mutations) --------------------------------

fn machine_has_power(
    machine_e: Entity,
    energy_per_tick: f32,
    energy_ports_q: &Query<&MachineEnergyPorts>,
    port_power_q: &Query<&PowerNetworkMember>,
    power_members_q: &Query<&PowerNetworkMembers>,
    gen_q: &Query<&GeneratorUnit>,
    energy_port_of_q: &Query<&EnergyPortOf>,
) -> bool {
    energy_ports_q
        .get(machine_e)
        .ok()
        .map(|energy_ports| {
            energy_ports.ports().iter().any(|&energy_port_e| {
                port_power_q
                    .get(energy_port_e)
                    .ok()
                    .and_then(|pm| power_members_q.get(pm.0).ok())
                    .is_some_and(|members| {
                        members.has_energy(gen_q, energy_port_of_q, energy_per_tick)
                    })
            })
        })
        .unwrap_or(false)
}

fn recipe_inputs_available(
    recipe: &ConcreteRecipe,
    logistics_ports: &MachineLogisticsPorts,
    net_q: &Query<(Entity, &LogisticsNetworkMembers)>,
    port_net_q: &Query<&LogisticsNetworkMember>,
    storage_q: &Query<&StorageUnit>,
    port_of_q: &Query<&LogisticsPortOf>,
    policy_q: &Query<&PortPolicy>,
) -> bool {
    recipe.inputs.iter().all(|input| {
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
        input_nets.iter().any(|&net_e| {
            net_q.get(net_e).ok().is_some_and(|(_, net_members)| {
                has_items(
                    net_members,
                    storage_q,
                    port_of_q,
                    &input.item,
                    input.quantity as u32,
                )
            })
        })
    })
}

fn recipe_outputs_routable(
    recipe: &ConcreteRecipe,
    logistics_ports: &MachineLogisticsPorts,
    port_net_q: &Query<&LogisticsNetworkMember>,
    policy_q: &Query<&PortPolicy>,
) -> bool {
    !recipe
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
        })
}

// -- Systems -----------------------------------------------------------------

pub(super) fn recipe_check_system(
    mut to_start: MessageWriter<RecipeToStart>,
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
    storage_q: Query<&StorageUnit>,
    port_of_q: Query<&LogisticsPortOf>,
    policy_q: Query<&PortPolicy>,
) {
    let mut affected: HashSet<Entity> = storage_changed.read().map(|e| e.network).collect();

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
        "recipe_check: NetworkStorageChanged for networks {:?}",
        affected
    );

    let machine_connected_nets = |logistics_ports: &MachineLogisticsPorts| -> Vec<Entity> {
        logistics_ports
            .ports()
            .iter()
            .filter_map(|&port_e| port_net_q.get(port_e).ok().map(|m| m.0))
            .collect()
    };

    for (net_entity, _) in &net_q {
        if !affected.contains(&net_entity) {
            continue;
        }
        debug!("recipe_check: checking network {:?}", net_entity);
        for (machine_e, machine, state, logistics_ports) in &machine_q {
            let connected_nets = machine_connected_nets(logistics_ports);
            if !connected_nets.contains(&net_entity) {
                continue;
            }
            if *state != MachineState::Idle {
                debug!(
                    "recipe_check: machine {:?} ({}) skipped — state={:?}",
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
                    "recipe_check: machine {:?} ({}) — no recipes for this machine type",
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
                        "recipe_check: machine {:?} recipe {} locked",
                        machine_e, recipe.id
                    );
                    continue;
                }
                if recipe.energy_cost > 0.0 {
                    let energy_per_tick = recipe.energy_cost / recipe.processing_time * 0.016;
                    if !machine_has_power(
                        machine_e,
                        energy_per_tick,
                        &energy_ports_q,
                        &port_power_q,
                        &power_members_q,
                        &gen_q,
                        &energy_port_of_q,
                    ) {
                        debug!(
                            "recipe_check: machine {:?} recipe {} requires power but none available",
                            machine_e, recipe.id
                        );
                        continue;
                    }
                }
                if !recipe_inputs_available(
                    recipe,
                    logistics_ports,
                    &net_q,
                    &port_net_q,
                    &storage_q,
                    &port_of_q,
                    &policy_q,
                ) {
                    debug!(
                        "recipe_check: machine {:?} recipe {} missing inputs",
                        machine_e, recipe.id
                    );
                    continue;
                }
                if !recipe_outputs_routable(recipe, logistics_ports, &port_net_q, &policy_q) {
                    debug!(
                        "recipe_check: machine {:?} recipe {} output has no destination",
                        machine_e, recipe.id
                    );
                    continue;
                }
                to_start.write(RecipeToStart {
                    machine: machine_e,
                    recipe_id: recipe.id.clone(),
                    net: net_entity,
                });
                started = true;
                break;
            }
            if !started {
                debug!(
                    "recipe_check: machine {:?} ({}) — no startable recipe found",
                    machine_e, machine.machine_type
                );
            }
        }
    }
}

pub(super) fn recipe_execute_system(
    mut commands: Commands,
    mut to_start: MessageReader<RecipeToStart>,
    recipe_graph: Res<RecipeGraph>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    machine_q: Query<&MachineLogisticsPorts, Without<MachineActivity>>,
    mut storage_q: Query<&mut StorageUnit>,
    port_of_q: Query<&LogisticsPortOf>,
    policy_q: Query<&PortPolicy>,
    port_net_q: Query<&LogisticsNetworkMember>,
) {
    let mut started: HashSet<Entity> = HashSet::new();
    for msg in to_start.read() {
        if !started.insert(msg.machine) {
            continue;
        }
        let Some(recipe) = recipe_graph.recipes.get(&msg.recipe_id) else {
            continue;
        };
        let Ok(logistics_ports) = machine_q.get(msg.machine) else {
            continue;
        };
        for input in &recipe.inputs {
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
            let take_net = input_nets.first().copied().unwrap_or(msg.net);
            if let Ok((_, net_members)) = net_q.get(take_net) {
                take_items(
                    net_members,
                    &mut storage_q,
                    &port_of_q,
                    &input.item,
                    input.quantity as u32,
                );
            } else if let Ok((_, net_members)) = net_q.get(msg.net) {
                take_items(
                    net_members,
                    &mut storage_q,
                    &port_of_q,
                    &input.item,
                    input.quantity as u32,
                );
            }
        }
        commands.entity(msg.machine).insert((
            MachineActivity {
                recipe_id: msg.recipe_id.clone(),
                progress: 0.0,
                speed_factor: 1.0,
            },
            MachineState::Running,
        ));
        info!("Machine {:?} started recipe {}", msg.machine, msg.recipe_id);
    }
}

pub(super) fn recipe_advance_system(
    mut to_finish: MessageWriter<RecipeCompleted>,
    time: Res<Time>,
    recipe_graph: Res<RecipeGraph>,
    mut machine_q: Query<(
        Entity,
        &mut MachineActivity,
        &MachineState,
        &MachineLogisticsPorts,
    )>,
    energy_ports_q: Query<&MachineEnergyPorts>,
    port_power_q: Query<&PowerNetworkMember>,
    power_members_q: Query<&PowerNetworkMembers>,
    energy_port_of_q: Query<&EnergyPortOf>,
    mut gen_q: Query<&mut GeneratorUnit>,
) {
    let dt = time.delta_secs();

    for (machine_e, mut activity, state, logistics_ports) in &mut machine_q {
        if *state != MachineState::Running {
            continue;
        }
        let Some(recipe) = recipe_graph.recipes.get(&activity.recipe_id) else {
            continue;
        };

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
            let port_entities: Vec<Entity> = logistics_ports.ports().to_vec();
            to_finish.write(RecipeCompleted {
                machine: machine_e,
                recipe_id: activity.recipe_id.clone(),
                outputs,
                port_entities,
                energy_output: recipe.energy_output,
            });
        } else {
            activity.progress = new_progress;
        }
    }
}

pub(super) fn recipe_finish_system(
    mut commands: Commands,
    mut to_finish: MessageReader<RecipeCompleted>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    mut storage_q: Query<&mut StorageUnit>,
    port_of_q: Query<&LogisticsPortOf>,
    port_net_q: Query<&LogisticsNetworkMember>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
    mut research_pool: Option<ResMut<ResearchPool>>,
    mut gen_q: Query<&mut GeneratorUnit>,
    policy_q: Query<&PortPolicy>,
) {
    for completion in to_finish.read() {
        for (item_id, count) in &completion.outputs {
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
            let target_net = completion.port_entities.iter().find_map(|&port_e| {
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
                    completion.machine, item_id
                );
            }
        }
        if completion.energy_output > 0.0
            && let Ok(mut generator) = gen_q.get_mut(completion.machine)
        {
            generator.buffer_joules = (generator.buffer_joules + completion.energy_output)
                .min(generator.max_buffer_joules);
        }
        commands
            .entity(completion.machine)
            .remove::<MachineActivity>()
            .insert(MachineState::Idle);
        let recipe_id = completion.recipe_id.clone();
        let machine = completion.machine;
        commands.queue(move |world: &mut World| {
            if let Some(mut msgs) = world.get_resource_mut::<Messages<JobComplete>>() {
                msgs.write(JobComplete { machine, recipe_id });
            }
        });
        info!(
            "Machine {:?} finished recipe {}",
            completion.machine, completion.recipe_id
        );
    }
}

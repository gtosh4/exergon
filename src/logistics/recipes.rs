use std::collections::HashSet;

use bevy::ecs::message::{MessageReader, MessageWriter, Messages};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::logistics::job_queue::NetworkCraftQueue;
use crate::machine::{EnergyPortOf, Machine, ManualCraftOnly};
use crate::machine::{MachineActivity, MachineEnergyPorts, MachineLogisticsPorts, MachineState};
use crate::network::{NetworkChanged, Power};
use crate::power::{GeneratorUnit, PowerNetworkMember, PowerNetworkMembers};
use crate::recipe_graph::{ConcreteRecipe, RecipeGraph};
use crate::research::{ProductionTally, ResearchPool, TechTreeProgress, research_theme_of};

use super::items::{give_items, has_items, take_items};
use crate::network::NetworkMembersComponent;

use super::{
    JobComplete, LogisticsNetworkMember, LogisticsNetworkMembers, NetworkStorageChanged,
    PortPolicy, StorageUnit,
};
use crate::machine::LogisticsPortOf;

// -- Intra-frame cascade messages --------------------------------------------

/// Sent by explicit player action to allow ManualCraftOnly machines to run one cycle.
/// Superseded by `CraftJobQueue` for multi-step crafting; kept as a direct override.
#[derive(Clone, Message)]
pub struct ManualCraftTrigger;

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

#[derive(SystemParam)]
pub(crate) struct PowerChecker<'w, 's> {
    energy_ports_q: Query<'w, 's, &'static MachineEnergyPorts>,
    port_power_q: Query<'w, 's, &'static PowerNetworkMember>,
    power_members_q: Query<'w, 's, &'static PowerNetworkMembers>,
    energy_port_of_q: Query<'w, 's, &'static EnergyPortOf>,
    gen_q: Query<'w, 's, &'static GeneratorUnit>,
}

fn machine_has_power(machine_e: Entity, energy_per_tick: f32, p: &PowerChecker) -> bool {
    p.energy_ports_q
        .get(machine_e)
        .ok()
        .map(|energy_ports| {
            energy_ports.ports().iter().any(|&energy_port_e| {
                p.port_power_q
                    .get(energy_port_e)
                    .ok()
                    .and_then(|pm| p.power_members_q.get(pm.0).ok())
                    .is_some_and(|members| {
                        members.has_energy(&p.gen_q, &p.energy_port_of_q, energy_per_tick)
                    })
            })
        })
        .unwrap_or(false)
}

/// Check whether `recipe`'s inputs are available in the network.
/// `reserved` is added to the required quantity so that auto-craft machines cannot
/// consume items earmarked for queued jobs. Pass an empty map for queued-job checks
/// (those jobs are allowed to use reserved items — the items are reserved *for* them).
fn recipe_inputs_available(
    recipe: &ConcreteRecipe,
    logistics_ports: &MachineLogisticsPorts,
    net_q: &Query<(Entity, &LogisticsNetworkMembers)>,
    port_net_q: &Query<&LogisticsNetworkMember>,
    storage_q: &Query<&StorageUnit>,
    port_of_q: &Query<&LogisticsPortOf>,
    policy_q: &Query<&PortPolicy>,
    reserved: &std::collections::HashMap<String, u32>,
) -> bool {
    recipe.inputs.iter().all(|input| {
        let extra = reserved.get(&input.item).copied().unwrap_or(0);
        let needed = (input.quantity as u32).saturating_add(extra);
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
                has_items(net_members, storage_q, port_of_q, &input.item, needed)
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
            if research_theme_of(&out.item).is_some() {
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
    mut manual_trigger: MessageReader<ManualCraftTrigger>,
    mut queue_q: Query<&mut NetworkCraftQueue>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    machine_q: Query<
        (
            Entity,
            &Machine,
            &MachineState,
            &MachineLogisticsPorts,
            Option<&crate::power::PodPowered>,
            Option<&ManualCraftOnly>,
        ),
        Without<MachineActivity>,
    >,
    port_net_q: Query<&LogisticsNetworkMember>,
    power: PowerChecker,
    recipe_graph: Res<RecipeGraph>,
    progress: Option<Res<TechTreeProgress>>,
    storage_q: Query<&StorageUnit>,
    port_of_q: Query<&LogisticsPortOf>,
    policy_q: Query<&PortPolicy>,
) {
    let manual = manual_trigger.read().next().is_some();
    let mut affected: HashSet<Entity> = storage_changed.read().map(|e| e.network).collect();

    for pnet_e in power_changed.read().map(|e| e.network) {
        let Ok(members) = power.power_members_q.get(pnet_e) else {
            continue;
        };
        for &port_e in members.members() {
            let Ok(energy_port) = power.energy_port_of_q.get(port_e) else {
                continue;
            };
            if let Ok((_, _, _, logistics_ports, _, _)) = machine_q.get(energy_port.0) {
                for &lport_e in logistics_ports.ports() {
                    if let Ok(member) = port_net_q.get(lport_e) {
                        affected.insert(member.0);
                    }
                }
            }
        }
    }

    let any_queue_nonempty = queue_q.iter().any(|q| !q.jobs.is_empty());
    if affected.is_empty() && !manual && !any_queue_nonempty {
        return;
    }

    let machine_connected_nets = |logistics_ports: &MachineLogisticsPorts| -> Vec<Entity> {
        logistics_ports
            .ports()
            .iter()
            .filter_map(|&port_e| port_net_q.get(port_e).ok().map(|m| m.0))
            .collect()
    };

    // Prevent double-dispatch when a machine spans multiple networks.
    let mut dispatched: HashSet<Entity> = HashSet::new();

    // Collect network entities first so we can get_mut queue_q inside the loop
    // without conflicting borrows on net_q.
    let net_entities: Vec<Entity> = net_q.iter().map(|(e, _)| e).collect();

    for net_entity in net_entities {
        let has_queued = queue_q
            .get(net_entity)
            .map(|q| !q.jobs.is_empty())
            .unwrap_or(false);
        if !manual && !has_queued && !affected.contains(&net_entity) {
            continue;
        }
        debug!("recipe_check: checking network {:?}", net_entity);

        for (machine_e, machine, state, logistics_ports, pod_powered, manual_only) in &machine_q {
            if dispatched.contains(&machine_e) {
                continue;
            }
            let connected_nets = machine_connected_nets(logistics_ports);
            if !connected_nets.contains(&net_entity) {
                continue;
            }
            if *state != MachineState::Idle {
                continue;
            }

            // --- queue-driven dispatch -------------------------------------------
            // Queued jobs can use the full storage (reserved items are FOR them).
            let empty_reserved: std::collections::HashMap<String, u32> = Default::default();
            let mut queue_pick: Option<(usize, String)> = None;
            if let Ok(net_queue) = queue_q.get(net_entity) {
                for (i, job) in net_queue.jobs.iter().enumerate() {
                    let Some(recipe) = recipe_graph.recipes.get(&job.recipe_id) else {
                        continue;
                    };
                    if recipe.machine_type != machine.machine_type
                        || recipe.machine_tier > machine.tier
                    {
                        continue;
                    }
                    if let Some(ref prog) = progress
                        && !prog.unlocked_recipes.contains(&recipe.id)
                    {
                        continue;
                    }
                    if recipe.energy_cost > 0.0 && pod_powered.is_none() {
                        let energy_per_tick = recipe.energy_cost / recipe.processing_time * 0.016;
                        if !machine_has_power(machine_e, energy_per_tick, &power) {
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
                        &empty_reserved,
                    ) {
                        continue;
                    }
                    if !recipe_outputs_routable(recipe, logistics_ports, &port_net_q, &policy_q) {
                        continue;
                    }
                    queue_pick = Some((i, job.recipe_id.clone()));
                    break;
                }
            }

            if let Some((idx, recipe_id)) = queue_pick {
                if let Ok(mut net_queue) = queue_q.get_mut(net_entity) {
                    net_queue.jobs.remove(idx);
                    // Release the reservation for this job's inputs — they are about
                    // to be consumed by recipe_execute_system.
                    if let Some(recipe) = recipe_graph.recipes.get(&recipe_id) {
                        for input in &recipe.inputs {
                            let r = net_queue.reserved.entry(input.item.clone()).or_insert(0);
                            *r = r.saturating_sub(input.quantity as u32);
                        }
                    }
                }
                to_start.write(RecipeToStart {
                    machine: machine_e,
                    recipe_id,
                    net: net_entity,
                });
                dispatched.insert(machine_e);
                continue;
            }

            // ManualCraftOnly machines only run from the queue (or direct manual trigger).
            if manual_only.is_some() && !manual {
                continue;
            }

            // --- auto-craft fallback (non-ManualCraftOnly) -----------------------
            // Auto-craft must not consume items reserved for queued jobs.
            let auto_reserved: std::collections::HashMap<String, u32> = queue_q
                .get(net_entity)
                .map(|q| q.reserved.clone())
                .unwrap_or_default();

            let matching_recipes: Vec<_> = recipe_graph
                .recipes
                .values()
                .filter(|r| {
                    r.machine_type == machine.machine_type && r.machine_tier <= machine.tier
                })
                .collect();

            for recipe in &matching_recipes {
                if let Some(ref prog) = progress
                    && !prog.unlocked_recipes.contains(&recipe.id)
                {
                    continue;
                }
                if recipe.energy_cost > 0.0 && pod_powered.is_none() {
                    let energy_per_tick = recipe.energy_cost / recipe.processing_time * 0.016;
                    if !machine_has_power(machine_e, energy_per_tick, &power) {
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
                    &auto_reserved,
                ) {
                    continue;
                }
                if !recipe_outputs_routable(recipe, logistics_ports, &port_net_q, &policy_q) {
                    continue;
                }
                to_start.write(RecipeToStart {
                    machine: machine_e,
                    recipe_id: recipe.id.clone(),
                    net: net_entity,
                });
                dispatched.insert(machine_e);
                break;
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
    mut tally: Option<ResMut<ProductionTally>>,
    mut gen_q: Query<&mut GeneratorUnit>,
    policy_q: Query<&PortPolicy>,
    mut queue_q: Query<&mut NetworkCraftQueue>,
    recipe_graph: Res<RecipeGraph>,
) {
    for completion in to_finish.read() {
        for (item_id, count) in &completion.outputs {
            if let Some(theme) = research_theme_of(item_id) {
                if let Some(ref mut pool) = research_pool {
                    pool.add(theme, *count as f32);
                }
                continue;
            }
            if *count == 0 {
                continue;
            }
            // Count genuine production (outputs + byproducts) toward ProductionMilestone.
            if let Some(ref mut tally) = tally {
                tally.record(item_id, *count as f32);
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
                // Reserve this output for any remaining queued job that needs it,
                // preventing auto-craft from consuming intermediate products.
                if let Ok(mut queue) = queue_q.get_mut(net_e) {
                    let still_needed = queue.inputs_still_needed(&recipe_graph);
                    if let Some(&needed) = still_needed.get(item_id) {
                        let to_reserve = (*count).min(needed);
                        if to_reserve > 0 {
                            *queue.reserved.entry(item_id.clone()).or_insert(0) += to_reserve;
                        }
                    }
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

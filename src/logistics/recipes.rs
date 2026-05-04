use std::collections::HashSet;

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::machine::{Machine, MachineActivity, MachineState};
use crate::recipe_graph::RecipeGraph;
use crate::research::{RESEARCH_POINTS_ID, ResearchPool, TechTreeProgress};

use super::items::{give_items, has_items, take_items};
use crate::network::NetworkMembersComponent;

use super::{LogisticsNetworkMember, LogisticsNetworkMembers, NetworkStorageChanged, StorageUnit};

pub(super) fn recipe_start_system(
    mut commands: Commands,
    mut storage_changed: MessageReader<NetworkStorageChanged>,
    net_q: Query<(Entity, &LogisticsNetworkMembers)>,
    machine_q: Query<
        (Entity, &Machine, &MachineState, &LogisticsNetworkMember),
        Without<MachineActivity>,
    >,
    recipe_graph: Res<RecipeGraph>,
    progress: Option<Res<TechTreeProgress>>,
    mut storage_params: ParamSet<(Query<&StorageUnit>, Query<&mut StorageUnit>)>,
) {
    let affected: HashSet<Entity> = storage_changed.read().map(|e| e.network).collect();
    if affected.is_empty() {
        return;
    }
    debug!(
        "recipe_start: NetworkStorageChanged for networks {:?}",
        affected
    );

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
            for (machine_e, machine, state, member) in &machine_q {
                if member.0 != net_entity {
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
                    let missing: Vec<_> = recipe
                        .inputs
                        .iter()
                        .filter(|input| {
                            !has_items(members, &storage_q, &input.item, input.quantity as u32)
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
                    take_items(members, &mut storage_q, &input.item, input.quantity as u32);
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
        &LogisticsNetworkMember,
    )>,
    mut storage_q: Query<&mut StorageUnit>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
    mut research_pool: Option<ResMut<ResearchPool>>,
) {
    let dt = time.delta_secs();

    let mut to_finish: Vec<(Entity, Vec<(String, u32)>, Entity)> = Vec::new();

    for (machine_e, mut activity, state, member) in &mut machine_q {
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
            to_finish.push((machine_e, outputs, member.0));
        } else {
            activity.progress = new_progress;
        }
    }

    for (machine_e, outputs, net_entity) in to_finish {
        if let Ok((_, members)) = net_q.get(net_entity) {
            for (item_id, count) in outputs {
                if item_id == RESEARCH_POINTS_ID {
                    if let Some(ref mut pool) = research_pool {
                        pool.points += count as f32;
                        info!("Research pool +{} points (total: {})", count, pool.points);
                    }
                } else if count > 0 {
                    give_items(members, &mut storage_q, &item_id, count);
                }
            }
            storage_changed.write(NetworkStorageChanged {
                network: net_entity,
            });
        }
        commands
            .entity(machine_e)
            .remove::<MachineActivity>()
            .insert(MachineState::Idle);
        info!("Machine {:?} finished recipe", machine_e);
    }
}

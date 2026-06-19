use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use crate::recipe_graph::RecipeGraph;

#[derive(Clone, Debug)]
pub struct QueuedJob {
    pub recipe_id: String,
}

/// Per-network crafting queue. Lives on `LogisticsNetwork` entities (added automatically via
/// `#[require]`). Jobs are stored in topological order — dependencies before dependents.
/// `reserved` tracks items earmarked for pending jobs so auto-craft machines cannot consume
/// them. Auto-craft feasibility check uses `available = storage - reserved`; queued jobs
/// ignore `reserved` and see the full storage (the reserved items belong to them).
#[derive(Component, Default)]
pub struct NetworkCraftQueue {
    pub jobs: VecDeque<QueuedJob>,
    /// Items in this network's storage that are earmarked for pending jobs.
    pub reserved: HashMap<String, u32>,
}

impl NetworkCraftQueue {
    /// Build the full dependency graph for `item_id × quantity` and append the resulting
    /// jobs (leaves first) to this queue. Items already in `storage` are reserved so
    /// auto-craft cannot steal them; intermediate outputs are reserved when each job
    /// completes (see `recipe_finish_system`).
    pub fn enqueue_item(
        &mut self,
        item_id: &str,
        quantity: u32,
        recipe_graph: &RecipeGraph,
        storage: &HashMap<String, u32>,
    ) {
        let mut available: HashMap<String, u32> = storage.clone();
        let mut new_jobs = Vec::new();
        let mut to_reserve: HashMap<String, u32> = Default::default();
        build_recursive(
            item_id,
            quantity,
            recipe_graph,
            &mut available,
            &mut new_jobs,
            &mut to_reserve,
        );
        for job in new_jobs {
            self.jobs.push_back(job);
        }
        for (item, qty) in to_reserve {
            *self.reserved.entry(item).or_insert(0) += qty;
        }
    }

    /// Sum of all inputs still needed by remaining jobs (used when a job completes to
    /// decide which outputs should be added to `reserved`).
    pub fn inputs_still_needed(&self, recipe_graph: &RecipeGraph) -> HashMap<String, u32> {
        let mut needed: HashMap<String, u32> = Default::default();
        for job in &self.jobs {
            if let Some(recipe) = recipe_graph.recipes.get(&job.recipe_id) {
                for input in &recipe.inputs {
                    *needed.entry(input.item.clone()).or_insert(0) += input.quantity as u32;
                }
            }
        }
        needed
    }
}

fn build_recursive(
    item_id: &str,
    quantity: u32,
    recipe_graph: &RecipeGraph,
    available: &mut HashMap<String, u32>,
    jobs: &mut Vec<QueuedJob>,
    reserved: &mut HashMap<String, u32>,
) {
    let have = available.get(item_id).copied().unwrap_or(0);
    if have >= quantity {
        if let Some(a) = available.get_mut(item_id) {
            *a -= quantity;
        }
        // All of `quantity` comes from current storage — reserve it.
        *reserved.entry(item_id.to_string()).or_insert(0) += quantity;
        return;
    }

    // Partial amount from storage; the rest must come from a crafting job.
    let from_storage = have;
    let still_needed = quantity - have;
    *available.entry(item_id.to_string()).or_insert(0) = 0;
    if from_storage > 0 {
        *reserved.entry(item_id.to_string()).or_insert(0) += from_storage;
    }

    let Some(recipe_id) = recipe_graph.producers.get(item_id).and_then(|v| v.first()) else {
        return; // raw material with no recipe — assumed present or unobtainable
    };
    let Some(recipe) = recipe_graph.recipes.get(recipe_id) else {
        return;
    };

    let output_qty = recipe
        .outputs
        .iter()
        .find(|o| o.item == item_id)
        .map(|o| o.quantity as u32)
        .unwrap_or(1);
    let runs = still_needed.div_ceil(output_qty);

    for _ in 0..runs {
        // Push sub-jobs (dependency inputs) before this job.
        for input in &recipe.inputs {
            build_recursive(
                &input.item,
                input.quantity as u32,
                recipe_graph,
                available,
                jobs,
                reserved,
            );
        }
        jobs.push(QueuedJob {
            recipe_id: recipe_id.clone(),
        });
        // Track excess output so subsequent runs don't over-craft.
        *available.entry(item_id.to_string()).or_insert(0) += output_qty;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe_graph::{ConcreteRecipe, ItemStack, RecipeGraph};

    fn recipe(
        id: &str,
        machine: &str,
        inputs: &[(&str, f32)],
        outputs: &[(&str, f32)],
    ) -> ConcreteRecipe {
        ConcreteRecipe {
            id: id.to_string(),
            inputs: inputs
                .iter()
                .map(|(i, q)| ItemStack {
                    item: i.to_string(),
                    quantity: *q,
                })
                .collect(),
            outputs: outputs
                .iter()
                .map(|(o, q)| ItemStack {
                    item: o.to_string(),
                    quantity: *q,
                })
                .collect(),
            byproducts: vec![],
            machine_type: machine.to_string(),
            machine_tier: 1,
            processing_time: 1.0,
            energy_cost: 0.0,
            energy_output: 0.0,
            template_id: None,
        }
    }

    fn graph(recipes: Vec<ConcreteRecipe>) -> RecipeGraph {
        RecipeGraph::from_vecs(vec![], vec![], vec![], recipes, vec![])
    }

    #[test]
    fn single_craftable_item_enqueues_one_job() {
        let rg = graph(vec![recipe(
            "make_smelter",
            "assembler",
            &[("stone", 20.0), ("iron_ore", 10.0)],
            &[("smelter", 1.0)],
        )]);
        let mut q = NetworkCraftQueue::default();
        q.enqueue_item("smelter", 1, &rg, &HashMap::new());
        assert_eq!(q.jobs.len(), 1);
        assert_eq!(q.jobs[0].recipe_id, "make_smelter");
    }

    #[test]
    fn dep_enqueued_before_dependent() {
        let rg = graph(vec![
            recipe(
                "smelt_iron",
                "smelter",
                &[("iron_ore", 2.0)],
                &[("iron_ingot", 1.0)],
            ),
            recipe(
                "make_circuit",
                "assembler",
                &[("iron_ingot", 1.0)],
                &[("circuit", 1.0)],
            ),
        ]);
        let mut q = NetworkCraftQueue::default();
        q.enqueue_item("circuit", 1, &rg, &HashMap::new());
        assert_eq!(q.jobs.len(), 2);
        assert_eq!(q.jobs[0].recipe_id, "smelt_iron");
        assert_eq!(q.jobs[1].recipe_id, "make_circuit");
    }

    #[test]
    fn storage_satisfies_dep_skips_its_job() {
        let rg = graph(vec![
            recipe(
                "smelt_iron",
                "smelter",
                &[("iron_ore", 2.0)],
                &[("iron_ingot", 1.0)],
            ),
            recipe(
                "make_circuit",
                "assembler",
                &[("iron_ingot", 1.0)],
                &[("circuit", 1.0)],
            ),
        ]);
        let mut q = NetworkCraftQueue::default();
        let storage: HashMap<String, u32> = [("iron_ingot".to_string(), 5)].into_iter().collect();
        q.enqueue_item("circuit", 1, &rg, &storage);
        assert_eq!(q.jobs.len(), 1);
        assert_eq!(q.jobs[0].recipe_id, "make_circuit");
    }

    #[test]
    fn storage_satisifes_dep_reserves_item() {
        let rg = graph(vec![
            recipe(
                "smelt_iron",
                "smelter",
                &[("iron_ore", 2.0)],
                &[("iron_ingot", 1.0)],
            ),
            recipe(
                "make_circuit",
                "assembler",
                &[("iron_ingot", 1.0)],
                &[("circuit", 1.0)],
            ),
        ]);
        let mut q = NetworkCraftQueue::default();
        let storage: HashMap<String, u32> = [("iron_ingot".to_string(), 5)].into_iter().collect();
        q.enqueue_item("circuit", 1, &rg, &storage);
        // iron_ingot x1 consumed from storage → should be reserved
        assert_eq!(q.reserved.get("iron_ingot").copied().unwrap_or(0), 1);
    }

    #[test]
    fn quantity_two_enqueues_two_runs() {
        let rg = graph(vec![recipe(
            "make_smelter",
            "assembler",
            &[("stone", 20.0)],
            &[("smelter", 1.0)],
        )]);
        let mut q = NetworkCraftQueue::default();
        q.enqueue_item("smelter", 2, &rg, &HashMap::new());
        assert_eq!(q.jobs.len(), 2);
    }

    #[test]
    fn raw_material_with_no_recipe_not_enqueued() {
        let rg = graph(vec![]);
        let mut q = NetworkCraftQueue::default();
        q.enqueue_item("iron_ore", 5, &rg, &HashMap::new());
        assert_eq!(q.jobs.len(), 0);
    }

    #[test]
    fn inputs_still_needed_sums_across_remaining_jobs() {
        let rg = graph(vec![
            recipe("job_a", "m", &[("x", 2.0)], &[("out_a", 1.0)]),
            recipe("job_b", "m", &[("x", 1.0), ("y", 3.0)], &[("out_b", 1.0)]),
        ]);
        let mut q = NetworkCraftQueue::default();
        q.jobs.push_back(QueuedJob {
            recipe_id: "job_a".into(),
        });
        q.jobs.push_back(QueuedJob {
            recipe_id: "job_b".into(),
        });
        let needed = q.inputs_still_needed(&rg);
        assert_eq!(needed.get("x").copied().unwrap_or(0), 3);
        assert_eq!(needed.get("y").copied().unwrap_or(0), 3);
    }
}

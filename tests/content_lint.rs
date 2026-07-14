//! Whole-tree content integrity lint. Loads every tech node + the full recipe graph
//! (template expansion included) and asserts the progression web is internally consistent:
//! no dangling prerequisites, no tier inversions, and every recipe/item/template a node
//! points at actually exists and is reachable. `smoke_test` proves one item at a time; this
//! sweeps the whole graph so orphaned or mistyped content can't slip in.
//!
//! Reuses the production load path via `scenario_runner::load_registries` (repo-root CWD, same
//! as the other e2e tests). Any failure is a real content gap — fix the RON, not the test.

use std::collections::HashSet;

use exergon::research::research_theme_of;
use exergon::tech_tree::{NodeEffect, UnlockVector};
use scenario_runner::load_registries;

#[test]
fn content_is_consistent() {
    let (tree, graph) = load_registries();
    let mut issues: Vec<String> = Vec::new();

    // Recipes reachable through some node effect (direct list or template expansion).
    let mut unlockable: HashSet<&str> = HashSet::new();

    for node in tree.nodes.values() {
        // A1/A2: prerequisites exist and never come from a higher tier.
        for prereq in &node.prerequisites {
            match tree.nodes.get(prereq) {
                None => issues.push(format!(
                    "node `{}`: prerequisite `{}` does not exist",
                    node.id, prereq
                )),
                Some(p) if p.tier > node.tier => issues.push(format!(
                    "node `{}` (tier {}): prerequisite `{}` is a higher tier ({})",
                    node.id, node.tier, prereq, p.tier
                )),
                Some(_) => {}
            }
        }

        // A3/A4: effect targets exist; templates expand to something.
        for effect in &node.effects {
            match effect {
                NodeEffect::UnlockRecipes(ids) => {
                    for id in ids {
                        if !graph.recipes.contains_key(id) {
                            issues.push(format!(
                                "node `{}`: unlocks unknown recipe `{}`",
                                node.id, id
                            ));
                        }
                        unlockable.insert(id.as_str());
                    }
                }
                NodeEffect::UnlockRecipeTemplate(template) => {
                    match graph.template_recipes.get(template) {
                        Some(recipes) if !recipes.is_empty() => {
                            unlockable.extend(recipes.iter().map(String::as_str));
                        }
                        _ => issues.push(format!(
                            "node `{}`: template `{}` expands to no recipes",
                            node.id, template
                        )),
                    }
                }
                NodeEffect::UnlockMachine(_) => {}
            }
        }

        // A7: a production-milestone gate must name a real produced thing.
        if let UnlockVector::ProductionMilestone { material, .. } = &node.primary_unlock
            && !graph.items.contains_key(material)
            && !graph.materials.contains_key(material)
        {
            issues.push(format!(
                "node `{}`: ProductionMilestone material `{}` is neither a known item nor material",
                node.id, material
            ));
        }
    }

    // A5: every recipe must be unlockable by some node — no starter-recipe mechanism exists,
    // so an unreachable recipe is dead content.
    let mut orphans: Vec<&str> = graph
        .recipes
        .keys()
        .map(String::as_str)
        .filter(|id| !unlockable.contains(id))
        .collect();
    orphans.sort_unstable();
    for id in orphans {
        issues.push(format!(
            "recipe `{id}` is not unlocked by any tech node (unreachable)"
        ));
    }

    // A6: every item a recipe touches must be a defined item or a research-currency id.
    for recipe in graph.recipes.values() {
        for stack in recipe
            .inputs
            .iter()
            .chain(&recipe.outputs)
            .chain(&recipe.byproducts)
        {
            if !graph.items.contains_key(&stack.item) && research_theme_of(&stack.item).is_none() {
                issues.push(format!(
                    "recipe `{}`: references unknown item `{}`",
                    recipe.id, stack.item
                ));
            }
        }
    }

    issues.sort();
    issues.dedup();
    assert!(
        issues.is_empty(),
        "content lint found {} issue(s):\n{}",
        issues.len(),
        issues.join("\n")
    );
}

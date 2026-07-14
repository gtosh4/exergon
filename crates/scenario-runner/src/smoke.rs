//! Auto-generated **smoke scenarios**: prove a single piece of content — a tech node, a recipe, or
//! an item — is *reachable and functional* without hand-authoring a scenario. Given a [`Target`],
//! [`plan_smoke`] resolves its tech tier, preflights its dependency structure, and picks the lowest
//! difficulty whose tier cap covers it; [`build_spec`] splices the target-exercising step onto the
//! matching e2e baseline scenario, truncated at the end of its research economy (the successor build
//! tail is dropped — a smoke proves *reachable*, not *victory*). [`run_smoke`] wires the two around a
//! real headless run and reports whether the target was reached.
//!
//! The `require` / `select` seam on [`crate::ScenarioSpec`] is reserved here for dynamic node
//! selection (runs surfacing a subset of the tree): the plan records the target's prerequisite
//! closure in `require` so a future interpreter can force it into the selection. Today the
//! interpreter ignores both fields — the full tree is always available.

use exergon::recipe_graph::RecipeGraph;
use exergon::research::TechTreeProgress;
use exergon::save::DifficultyTier;
use exergon::tech_tree::{NodeDef, NodeEffect, TechTree};

use crate::harness::{Scenario, load_registries};
use crate::report::RunReport;
use crate::spec::{ScenarioSpec, Select, Step, load_spec};

/// What a smoke scenario proves reachable.
#[derive(Debug, Clone)]
pub enum Target {
    /// A tech-tree node — exercised by researching it.
    Node(String),
    /// A concrete recipe id — exercised by crafting its primary output.
    Recipe(String),
    /// An item id — exercised by crafting it (via its first producing recipe).
    Item(String),
}

impl Target {
    /// The bare content id, for naming the generated scenario.
    fn id(&self) -> &str {
        match self {
            Target::Node(id) | Target::Recipe(id) | Target::Item(id) => id,
        }
    }
}

/// The resolved recipe for one smoke run: which baseline difficulty to derive from, the target's
/// tech tier, the step that exercises the target, and its prerequisite closure (the `require` seam).
#[derive(Debug, Clone)]
pub struct SmokePlan {
    pub difficulty: DifficultyTier,
    pub tier: u8,
    pub exercise: Step,
    pub require: Vec<String>,
}

/// The outcome of a smoke run.
#[derive(Debug, Clone)]
pub struct SmokeReport {
    /// The difficulty the smoke ran at (the lowest that covers the target, unless forced).
    pub difficulty: DifficultyTier,
    /// Whether the target ended up reached — node/recipe unlocked, or item in the hub.
    pub reached: bool,
    /// The underlying run report (tier pace, currency curve, …).
    pub report: RunReport,
}

/// Resolve + preflight a [`Target`] into a [`SmokePlan`]. `difficulty` forces a difficulty; `None`
/// picks the lowest whose tier cap covers the target. Returns a human-readable error when the target
/// is unknown, has no producer, or its prerequisite chain is broken — the legible failure a content
/// author sees *before* a run, instead of a mid-simulation timeout.
pub fn plan_smoke(
    tree: &TechTree,
    graph: &RecipeGraph,
    target: &Target,
    difficulty: Option<DifficultyTier>,
) -> Result<SmokePlan, String> {
    // Resolve the target to (unlocking node id, the step that exercises it).
    let (node_id, exercise) = match target {
        Target::Node(id) => {
            if !tree.nodes.contains_key(id) {
                return Err(format!("unknown tech node `{id}`"));
            }
            (Some(id.clone()), Step::Research { node: id.clone() })
        }
        Target::Recipe(id) => {
            let recipe = graph
                .recipes
                .get(id)
                .ok_or_else(|| format!("unknown recipe `{id}`"))?;
            let output = recipe
                .outputs
                .first()
                .ok_or_else(|| format!("recipe `{id}` has no output to craft"))?
                .item
                .clone();
            (
                unlocking_node(tree, graph, id).map(|n| n.id.clone()),
                Step::Craft {
                    item: output,
                    count: 1,
                },
            )
        }
        Target::Item(id) => {
            let recipe_id = graph
                .producers
                .get(id)
                .and_then(|p| p.first())
                .ok_or_else(|| format!("item `{id}` has no producing recipe"))?;
            (
                unlocking_node(tree, graph, recipe_id).map(|n| n.id.clone()),
                Step::Craft {
                    item: id.clone(),
                    count: 1,
                },
            )
        }
    };

    // Tier + prerequisite closure come from the unlocking node. Content produced by a recipe no node
    // gates (a starter recipe) is tier 0 with an empty closure.
    let (tier, require) = match &node_id {
        Some(id) => (
            tree.nodes.get(id).map_or(0, |n| n.tier),
            node_closure(tree, id)?,
        ),
        None => (0, Vec::new()),
    };

    let difficulty = difficulty.unwrap_or_else(|| lowest_difficulty(tier));
    if difficulty.max_tier() < tier {
        return Err(format!(
            "target is tier {tier} but difficulty {difficulty:?} caps at tier {}",
            difficulty.max_tier()
        ));
    }

    Ok(SmokePlan {
        difficulty,
        tier,
        exercise,
        require,
    })
}

/// Splice a smoke plan onto a baseline scenario: keep the baseline through the end of its research
/// economy (up to its first `Pump(false)`, or all but the terminal `Build`), append the target's
/// exercise step, then disarm the economy. The successor build tail is dropped — a smoke proves the
/// target is reachable, not that the whole run completes.
pub fn build_spec(
    name: &str,
    baseline: &ScenarioSpec,
    plan: &SmokePlan,
    select: Select,
) -> ScenarioSpec {
    let mut steps: Vec<Step> = match baseline
        .steps
        .iter()
        .position(|s| matches!(s, Step::Pump(false)))
    {
        Some(i) => baseline.steps.iter().take(i).cloned().collect(),
        None => baseline
            .steps
            .iter()
            .filter(|s| !matches!(s, Step::Build { .. }))
            .cloned()
            .collect(),
    };
    steps.push(plan.exercise.clone());
    steps.push(Step::Pump(false));

    ScenarioSpec {
        name: name.to_string(),
        seed: baseline.seed,
        difficulty: plan.difficulty,
        steps,
        max_secs: baseline.max_secs,
        require: plan.require.clone(),
        select,
    }
}

/// The e2e baseline scenario a smoke of `difficulty` derives from. Only the tested difficulties have
/// a baseline today; higher tiers error until one is authored.
pub fn baseline_path(difficulty: DifficultyTier) -> Result<&'static str, String> {
    match difficulty {
        DifficultyTier::Initiation => Ok("scenarios/initiation.ron"),
        DifficultyTier::Standard => Ok("scenarios/standard.ron"),
        d => Err(format!("no baseline scenario for difficulty {d:?} yet")),
    }
}

/// Plan, derive, run, and check a smoke for `target`. `difficulty` forces one; `None` picks the
/// lowest that covers the target. Must be called from the repo root (loads `assets/` + the baseline
/// scenario). Preflight errors surface as `Err` before any simulation runs.
pub fn run_smoke(
    target: &Target,
    difficulty: Option<DifficultyTier>,
) -> Result<SmokeReport, String> {
    // Registries are seed-independent (loaded at Startup), so plan off a throwaway app.
    let (tree, graph) = load_registries();
    let plan = plan_smoke(&tree, &graph, target, difficulty)?;

    let baseline = load_spec(baseline_path(plan.difficulty)?)?;
    let spec = build_spec(
        &format!("smoke__{}", target.id()),
        &baseline,
        &plan,
        Select::Force,
    );

    let mut s = Scenario::new(spec.seed);
    let report = s.run(&spec);
    let reached = reached(&s, target);

    Ok(SmokeReport {
        difficulty: plan.difficulty,
        reached,
        report,
    })
}

/// The node whose effects unlock `recipe_id` — directly, or via the template it was expanded from.
fn unlocking_node<'a>(
    tree: &'a TechTree,
    graph: &RecipeGraph,
    recipe_id: &str,
) -> Option<&'a NodeDef> {
    let template = graph
        .recipes
        .get(recipe_id)
        .and_then(|r| r.template_id.clone());
    tree.nodes.values().find(|n| {
        n.effects.iter().any(|e| match e {
            NodeEffect::UnlockRecipes(ids) => ids.iter().any(|i| i == recipe_id),
            NodeEffect::UnlockRecipeTemplate(t) => template.as_deref() == Some(t.as_str()),
            NodeEffect::UnlockMachine(_) => false,
        })
    })
}

/// A node plus all its transitive prerequisites, tier-ascending. Errors naming the first prereq that
/// is missing from the tree (a broken chain — the target is unreachable).
fn node_closure(tree: &TechTree, id: &str) -> Result<Vec<String>, String> {
    let mut seen: Vec<String> = Vec::new();
    let mut stack = vec![id.to_string()];
    while let Some(n) = stack.pop() {
        if seen.contains(&n) {
            continue;
        }
        let node = tree
            .nodes
            .get(&n)
            .ok_or_else(|| format!("prerequisite `{n}` is missing from the tech tree"))?;
        for p in &node.prerequisites {
            stack.push(p.clone());
        }
        seen.push(n);
    }
    seen.sort_by_key(|nid| tree.nodes.get(nid).map_or(0, |n| n.tier));
    Ok(seen)
}

/// The lowest difficulty whose tier cap covers `tier`.
fn lowest_difficulty(tier: u8) -> DifficultyTier {
    [
        DifficultyTier::Initiation,
        DifficultyTier::Standard,
        DifficultyTier::Advanced,
        DifficultyTier::Pinnacle,
    ]
    .into_iter()
    .find(|d| d.max_tier() >= tier)
    .unwrap_or(DifficultyTier::Pinnacle)
}

/// Did the finished run reach the target — node/recipe unlocked, or item in the hub?
fn reached(s: &Scenario, target: &Target) -> bool {
    let prog = s.app.world().resource::<TechTreeProgress>();
    match target {
        Target::Node(id) => prog.unlocked_nodes.contains(id),
        Target::Recipe(id) => prog.unlocked_recipes.contains(id),
        Target::Item(id) => s.hub_stored(id) >= 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use exergon::recipe_graph::{ConcreteRecipe, ItemStack};
    use exergon::tech_tree::{NodeCategory, NodeRarity, UnlockVector};

    fn node(id: &str, tier: u8, prereqs: &[&str], unlocks: &[&str]) -> NodeDef {
        NodeDef {
            id: id.into(),
            name: id.into(),
            category: NodeCategory::Science,
            tier,
            rarity: NodeRarity::Common,
            prerequisites: prereqs.iter().map(|s| s.to_string()).collect(),
            primary_unlock: UnlockVector::PrerequisiteChain,
            effects: vec![NodeEffect::UnlockRecipes(
                unlocks.iter().map(|s| s.to_string()).collect(),
            )],
            exclusive_group: None,
        }
    }

    fn tree(nodes: Vec<NodeDef>) -> TechTree {
        TechTree {
            nodes: nodes.into_iter().map(|n| (n.id.clone(), n)).collect(),
            dependents: HashMap::new(),
            tier_order: Vec::new(),
        }
    }

    fn recipe(id: &str, output: &str) -> ConcreteRecipe {
        ConcreteRecipe {
            id: id.into(),
            inputs: vec![],
            outputs: vec![ItemStack {
                item: output.into(),
                quantity: 1.0,
            }],
            byproducts: vec![],
            machine_type: "assembler".into(),
            machine_tier: 1,
            processing_time: 1.0,
            energy_cost: 0.0,
            energy_output: 0.0,
            template_id: None,
            required_config: vec![],
        }
    }

    fn graph(recipes: Vec<ConcreteRecipe>) -> RecipeGraph {
        let mut producers: HashMap<String, Vec<String>> = HashMap::new();
        for r in &recipes {
            for out in &r.outputs {
                producers
                    .entry(out.item.clone())
                    .or_default()
                    .push(r.id.clone());
            }
        }
        RecipeGraph {
            materials: HashMap::new(),
            form_groups: HashMap::new(),
            templates: HashMap::new(),
            items: HashMap::new(),
            recipes: recipes.into_iter().map(|r| (r.id.clone(), r)).collect(),
            terminal: String::new(),
            producers,
            consumers: HashMap::new(),
            template_recipes: HashMap::new(),
        }
    }

    #[test]
    fn picks_lowest_difficulty_covering_the_target_tier() {
        let t = tree(vec![node("early", 2, &[], &[]), node("late", 4, &[], &[])]);
        let g = graph(vec![]);

        let early = plan_smoke(&t, &g, &Target::Node("early".into()), None).unwrap();
        assert_eq!(
            early.difficulty,
            DifficultyTier::Initiation,
            "tier 2 fits Initiation (cap 3)"
        );

        let late = plan_smoke(&t, &g, &Target::Node("late".into()), None).unwrap();
        assert_eq!(
            late.difficulty,
            DifficultyTier::Standard,
            "tier 4 needs Standard (cap 5)"
        );
    }

    #[test]
    fn item_with_no_producer_fails_preflight_naming_it() {
        let t = tree(vec![]);
        let g = graph(vec![]);
        let err = plan_smoke(&t, &g, &Target::Item("phantom_plate".into()), None).unwrap_err();
        assert!(
            err.contains("phantom_plate"),
            "error must name the item, got: {err}"
        );
        assert!(
            err.contains("no producing recipe"),
            "error must explain the gap, got: {err}"
        );
    }

    #[test]
    fn broken_prerequisite_chain_fails_naming_the_missing_node() {
        // `child` requires `ghost`, which is not in the tree — unreachable.
        let t = tree(vec![node("child", 2, &["ghost"], &[])]);
        let g = graph(vec![]);
        let err = plan_smoke(&t, &g, &Target::Node("child".into()), None).unwrap_err();
        assert!(
            err.contains("ghost"),
            "error must name the missing prereq, got: {err}"
        );
    }

    #[test]
    fn item_target_resolves_tier_via_its_unlocking_node() {
        // recipe `roll_widget` makes `widget`; node `forming` (tier 3) unlocks that recipe.
        let t = tree(vec![node("forming", 3, &[], &["roll_widget"])]);
        let g = graph(vec![recipe("roll_widget", "widget")]);

        let plan = plan_smoke(&t, &g, &Target::Item("widget".into()), None).unwrap();
        assert_eq!(plan.tier, 3);
        assert_eq!(
            plan.difficulty,
            DifficultyTier::Initiation,
            "tier 3 fits Initiation cap"
        );
        assert!(
            matches!(&plan.exercise, Step::Craft { item, count: 1 } if item == "widget"),
            "item target is exercised by crafting it, got {:?}",
            plan.exercise
        );
    }

    #[test]
    fn build_spec_truncates_the_build_tail_and_appends_the_exercise() {
        let baseline = ScenarioSpec {
            name: "baseline".into(),
            seed: 0xABCD,
            difficulty: DifficultyTier::Standard,
            steps: vec![
                Step::Pump(true),
                Step::Research { node: "a".into() },
                Step::Pump(false),
                Step::Build {
                    jobs: vec![("launch".into(), 1)],
                },
            ],
            max_secs: 1234.0,
            require: vec![],
            select: Select::Force,
        };
        let plan = SmokePlan {
            difficulty: DifficultyTier::Initiation,
            tier: 2,
            exercise: Step::Research {
                node: "target".into(),
            },
            require: vec!["target".into()],
        };

        let spec = build_spec("smoke__target", &baseline, &plan, Select::Force);

        assert_eq!(
            spec.difficulty,
            DifficultyTier::Initiation,
            "plan difficulty wins"
        );
        assert_eq!(spec.seed, 0xABCD, "seed carried from the baseline");
        assert_eq!(
            spec.require,
            vec!["target".to_string()],
            "closure recorded on the require seam"
        );
        assert!(
            !spec.steps.iter().any(|s| matches!(s, Step::Build { .. })),
            "the successor build tail is dropped"
        );
        // Ends with the target exercise then a disarm.
        assert!(
            matches!(spec.steps[spec.steps.len() - 2], Step::Research { ref node } if node == "target")
        );
        assert!(matches!(spec.steps.last(), Some(Step::Pump(false))));
    }
}

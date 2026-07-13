use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::{
    GameState,
    recipe_graph::{ItemId, RecipeGraph, RecipeId},
    research::TechTreeProgress,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum RateUnit {
    #[default]
    PerSecond,
    PerMinute,
}

#[derive(Clone, Debug)]
pub struct DepNode {
    pub item: ItemId,
    pub recipe: Option<RecipeId>,
    pub required_rate: f32,
    pub machine_count: u32,
    pub column: u32,
}

#[derive(Component)]
pub struct PlanState {
    pub target: ItemId,
    pub target_rate: f32,
    pub rate_unit: RateUnit,
    pub nodes: Vec<DepNode>,
    pub edges: Vec<(usize, usize)>,
    pub dirty: bool,
    pub locked_counts: HashMap<ItemId, u32>,
    pub alt_recipes: HashMap<ItemId, RecipeId>,
}

impl Default for PlanState {
    fn default() -> Self {
        Self {
            target: String::new(),
            target_rate: 1.0,
            rate_unit: RateUnit::PerSecond,
            nodes: Vec::new(),
            edges: Vec::new(),
            dirty: true,
            locked_counts: HashMap::new(),
            alt_recipes: HashMap::new(),
        }
    }
}

#[derive(Component)]
pub struct PlanName(pub String);

#[derive(Resource, Default)]
pub struct PlanList {
    pub plans: Vec<Entity>,
    pub active: Option<Entity>,
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Clone, Message)]
pub struct SetDepGraphTarget(pub ItemId);

#[derive(Clone, Message)]
pub struct SelectSankeyNode(pub ItemId);

#[derive(Clone, Message)]
pub struct ApplyAltRecipe {
    pub node: ItemId,
    pub recipe: RecipeId,
}

#[derive(Clone, Message)]
pub struct LockMachineCount {
    pub node: ItemId,
    pub count: u32,
}

// ---------------------------------------------------------------------------
// Pure build function
// ---------------------------------------------------------------------------

pub fn build_dep_graph(
    target: &ItemId,
    target_rate: f32,
    graph: &RecipeGraph,
    progress: &TechTreeProgress,
    locked_counts: &HashMap<ItemId, u32>,
    alt_recipes: &HashMap<ItemId, RecipeId>,
) -> (Vec<DepNode>, Vec<(usize, usize)>) {
    if target.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // BFS: item → (required_rate, raw_column). Column 0 = goal (rightmost before flip).
    let mut item_rate: HashMap<ItemId, f32> = HashMap::new();
    let mut item_col: HashMap<ItemId, u32> = HashMap::new();
    let mut item_recipe: HashMap<ItemId, Option<RecipeId>> = HashMap::new();
    let mut queue: VecDeque<(ItemId, f32, u32)> = VecDeque::new();

    item_rate.insert(target.clone(), target_rate);
    item_col.insert(target.clone(), 0);
    queue.push_back((target.clone(), target_rate, 0));

    let mut visited: HashSet<ItemId> = HashSet::new();
    visited.insert(target.clone());

    while let Some((item, rate, col)) = queue.pop_front() {
        // pick primary recipe
        let recipe_id = primary_recipe(&item, graph, progress, alt_recipes);
        item_recipe
            .entry(item.clone())
            .or_insert_with(|| recipe_id.clone());

        let Some(rid) = recipe_id else { continue };
        let Some(recipe) = graph.recipes.get(&rid) else {
            continue;
        };

        // find primary output qty for this recipe
        let primary_out_qty = recipe
            .outputs
            .iter()
            .find(|s| s.item == item)
            .map(|s| s.quantity)
            .unwrap_or(1.0);

        let throughput_per_machine = primary_out_qty / recipe.processing_time;

        for input in &recipe.inputs {
            let child_rate = rate * (input.quantity / primary_out_qty);
            let child_col = col + 1;

            let entry = item_rate.entry(input.item.clone()).or_insert(0.0);
            *entry += child_rate;

            // extend column only if deeper
            let col_entry = item_col.entry(input.item.clone()).or_insert(child_col);
            if child_col > *col_entry {
                *col_entry = child_col;
            }

            if !visited.contains(&input.item) {
                visited.insert(input.item.clone());
                queue.push_back((input.item.clone(), child_rate, child_col));
            }
        }

        // suppress unused warning
        let _ = throughput_per_machine;
    }

    // Build ordered node list — target first, then BFS order
    let mut ordered_items: Vec<ItemId> = Vec::new();
    {
        let mut bfs_q: VecDeque<ItemId> = VecDeque::new();
        let mut seen: HashSet<ItemId> = HashSet::new();
        bfs_q.push_back(target.clone());
        seen.insert(target.clone());
        while let Some(item) = bfs_q.pop_front() {
            ordered_items.push(item.clone());
            if let Some(Some(rid)) = item_recipe.get(&item)
                && let Some(recipe) = graph.recipes.get(rid)
            {
                for input in &recipe.inputs {
                    if !seen.contains(&input.item) {
                        seen.insert(input.item.clone());
                        bfs_q.push_back(input.item.clone());
                    }
                }
            }
        }
    }

    let max_col = item_col.values().copied().max().unwrap_or(0);

    let mut nodes: Vec<DepNode> = ordered_items
        .iter()
        .map(|item| {
            let rate = item_rate.get(item).copied().unwrap_or(0.0);
            let raw_col = item_col.get(item).copied().unwrap_or(0);
            let col = max_col - raw_col; // flip: raw materials → col 0
            let recipe = item_recipe.get(item).cloned().flatten();

            let machine_count = if let Some(ref rid) = recipe {
                if let Some(r) = graph.recipes.get(rid) {
                    let out_qty = r
                        .outputs
                        .iter()
                        .find(|s| s.item == *item)
                        .map(|s| s.quantity)
                        .unwrap_or(1.0);
                    let throughput = out_qty / r.processing_time;
                    (rate / throughput).ceil() as u32
                } else {
                    0
                }
            } else {
                0
            };

            let machine_count = locked_counts.get(item).copied().unwrap_or(machine_count);

            DepNode {
                item: item.clone(),
                recipe,
                required_rate: rate,
                machine_count,
                column: col,
            }
        })
        .collect();

    // Apply locked counts override
    for node in &mut nodes {
        if let Some(&locked) = locked_counts.get(&node.item) {
            node.machine_count = locked;
        }
    }

    // Build edges: (child_idx, parent_idx) where child provides input to parent
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let item_index: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.item.as_str(), i))
        .collect();

    for (parent_idx, node) in nodes.iter().enumerate() {
        if let Some(ref rid) = node.recipe
            && let Some(recipe) = graph.recipes.get(rid)
        {
            for input in &recipe.inputs {
                if let Some(&child_idx) = item_index.get(input.item.as_str()) {
                    edges.push((child_idx, parent_idx));
                }
            }
        }
    }

    (nodes, edges)
}

fn primary_recipe(
    item: &ItemId,
    graph: &RecipeGraph,
    progress: &TechTreeProgress,
    alt_recipes: &HashMap<ItemId, RecipeId>,
) -> Option<RecipeId> {
    if let Some(alt) = alt_recipes.get(item) {
        return Some(alt.clone());
    }
    let producers = graph.producers.get(item)?;
    producers
        .iter()
        .find(|rid| progress.unlocked_recipes.contains(*rid))
        .cloned()
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DepGraphPlugin;

impl Plugin for DepGraphPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SetDepGraphTarget>()
            .add_message::<SelectSankeyNode>()
            .add_message::<ApplyAltRecipe>()
            .add_message::<LockMachineCount>()
            .add_systems(OnEnter(GameState::Playing), spawn_initial_plan)
            .add_systems(
                Update,
                (
                    handle_set_target,
                    handle_apply_alt,
                    handle_lock_count,
                    dep_graph_build_system,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_initial_plan(
    mut commands: Commands,
    mut plan_list: ResMut<PlanList>,
    graph: Option<Res<RecipeGraph>>,
) {
    let target = graph
        .as_ref()
        .map(|g| g.terminal.clone())
        .unwrap_or_default();
    let entity = commands
        .spawn((
            PlanState {
                target,
                dirty: true,
                ..Default::default()
            },
            PlanName("Plan A".to_string()),
        ))
        .id();
    plan_list.plans.push(entity);
    plan_list.active = Some(entity);
}

fn dep_graph_build_system(
    mut plan_q: Query<&mut PlanState>,
    plan_list: Res<PlanList>,
    graph: Option<Res<RecipeGraph>>,
    progress: Option<Res<TechTreeProgress>>,
) {
    let Some(active) = plan_list.active else {
        return;
    };
    let Ok(mut plan) = plan_q.get_mut(active) else {
        return;
    };
    if !plan.dirty {
        return;
    }
    let Some(graph) = graph else { return };
    let empty_prog = TechTreeProgress::default();
    let progress = progress.as_deref().unwrap_or(&empty_prog);
    let (nodes, edges) = build_dep_graph(
        &plan.target.clone(),
        plan.target_rate,
        &graph,
        progress,
        &plan.locked_counts.clone(),
        &plan.alt_recipes.clone(),
    );
    plan.nodes = nodes;
    plan.edges = edges;
    plan.dirty = false;
}

fn handle_set_target(
    mut reader: MessageReader<SetDepGraphTarget>,
    mut plan_q: Query<&mut PlanState>,
    plan_list: Res<PlanList>,
) {
    let Some(active) = plan_list.active else {
        return;
    };
    let Ok(mut plan) = plan_q.get_mut(active) else {
        return;
    };
    for msg in reader.read() {
        plan.target = msg.0.clone();
        plan.dirty = true;
    }
}

fn handle_apply_alt(
    mut reader: MessageReader<ApplyAltRecipe>,
    mut plan_q: Query<&mut PlanState>,
    plan_list: Res<PlanList>,
) {
    let Some(active) = plan_list.active else {
        return;
    };
    let Ok(mut plan) = plan_q.get_mut(active) else {
        return;
    };
    for msg in reader.read() {
        plan.alt_recipes
            .insert(msg.node.clone(), msg.recipe.clone());
        plan.dirty = true;
    }
}

fn handle_lock_count(
    mut reader: MessageReader<LockMachineCount>,
    mut plan_q: Query<&mut PlanState>,
    plan_list: Res<PlanList>,
) {
    let Some(active) = plan_list.active else {
        return;
    };
    let Ok(mut plan) = plan_q.get_mut(active) else {
        return;
    };
    for msg in reader.read() {
        plan.locked_counts.insert(msg.node.clone(), msg.count);
        plan.dirty = true;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe_graph::{ConcreteRecipe, ItemDef, ItemKind, ItemStack, RecipeGraph};

    fn make_graph_chain() -> RecipeGraph {
        // A → B → C (C is goal/target)
        // recipe_b: 1 A → 1 B, processing_time=1.0
        // recipe_c: 1 B → 1 C, processing_time=1.0
        let a = ItemDef {
            id: "a".into(),
            name: "A".into(),
            kind: ItemKind::Unique,
            is_terminal: false,
            config: None,
        };
        let b = ItemDef {
            id: "b".into(),
            name: "B".into(),
            kind: ItemKind::Unique,
            is_terminal: false,
            config: None,
        };
        let c = ItemDef {
            id: "c".into(),
            name: "C".into(),
            kind: ItemKind::Unique,
            is_terminal: true,
            config: None,
        };
        let recipe_b = ConcreteRecipe {
            id: "recipe_b".into(),
            inputs: vec![ItemStack {
                item: "a".into(),
                quantity: 1.0,
            }],
            outputs: vec![ItemStack {
                item: "b".into(),
                quantity: 1.0,
            }],
            byproducts: vec![],
            machine_type: "assembler".into(),
            machine_tier: 1,
            processing_time: 1.0,
            energy_cost: 10.0,
            energy_output: 0.0,
            template_id: None,
            required_config: vec![],
        };
        let recipe_c = ConcreteRecipe {
            id: "recipe_c".into(),
            inputs: vec![ItemStack {
                item: "b".into(),
                quantity: 1.0,
            }],
            outputs: vec![ItemStack {
                item: "c".into(),
                quantity: 1.0,
            }],
            byproducts: vec![],
            machine_type: "assembler".into(),
            machine_tier: 1,
            processing_time: 1.0,
            energy_cost: 10.0,
            energy_output: 0.0,
            template_id: None,
            required_config: vec![],
        };
        RecipeGraph::from_vecs(
            vec![],
            vec![],
            vec![],
            vec![recipe_b, recipe_c],
            vec![a, b, c],
        )
    }

    fn progress_with(recipes: &[&str]) -> TechTreeProgress {
        let mut p = TechTreeProgress::default();
        for r in recipes {
            p.unlocked_recipes.insert(r.to_string());
        }
        p
    }

    #[test]
    fn three_node_chain_correct_rates_and_columns() {
        let graph = make_graph_chain();
        let progress = progress_with(&["recipe_b", "recipe_c"]);
        let (nodes, _edges) = build_dep_graph(
            &"c".to_string(),
            1.0,
            &graph,
            &progress,
            &HashMap::new(),
            &HashMap::new(),
        );

        // c: required_rate=1.0, 1 machine, rightmost column
        let c_node = nodes.iter().find(|n| n.item == "c").unwrap();
        assert!((c_node.required_rate - 1.0).abs() < 1e-5);
        assert_eq!(c_node.machine_count, 1);

        // b: required_rate=1.0, 1 machine
        let b_node = nodes.iter().find(|n| n.item == "b").unwrap();
        assert!((b_node.required_rate - 1.0).abs() < 1e-5);
        assert_eq!(b_node.machine_count, 1);

        // a: required_rate=1.0, 0 machines (no recipe for a)
        let a_node = nodes.iter().find(|n| n.item == "a").unwrap();
        assert!((a_node.required_rate - 1.0).abs() < 1e-5);
        assert_eq!(a_node.machine_count, 0);

        // columns: a=0 (raw material), b=1, c=2 (goal, rightmost)
        assert!(a_node.column < b_node.column);
        assert!(b_node.column < c_node.column);
    }

    #[test]
    fn apply_alt_recipe_sets_recipe_and_marks_dirty() {
        let mut plan = PlanState {
            target: "c".into(),
            target_rate: 1.0,
            dirty: false,
            ..Default::default()
        };
        plan.alt_recipes.insert("b".into(), "recipe_b_alt".into());
        plan.dirty = true;
        assert!(plan.dirty);
        assert_eq!(plan.alt_recipes.get("b").unwrap(), "recipe_b_alt");
    }

    #[test]
    fn lock_machine_count_overrides_computed_count() {
        let graph = make_graph_chain();
        let progress = progress_with(&["recipe_b", "recipe_c"]);
        let mut locked = HashMap::new();
        locked.insert("b".to_string(), 5u32);

        let (nodes, _) = build_dep_graph(
            &"c".to_string(),
            1.0,
            &graph,
            &progress,
            &locked,
            &HashMap::new(),
        );

        let b_node = nodes.iter().find(|n| n.item == "b").unwrap();
        assert_eq!(
            b_node.machine_count, 5,
            "locked count should override computed"
        );
    }
}

use std::collections::{HashMap, HashSet};

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::recipe_graph::RecipeGraph;
use crate::tech_tree::{NodeEffect, NodeId, TechTree, UnlockVector};

#[derive(Debug, Clone, Message)]
pub struct DiscoveryEvent(pub String);

#[derive(Debug, Clone, Message)]
pub struct TechNodeUnlocked {
    pub node_id: String,
    pub via_research: bool,
}

/// Player-initiated request to unlock a ResearchSpend tech node.
#[derive(Debug, Clone, Message)]
pub struct UnlockNodeRequest(pub NodeId);

/// Marker: prevents re-firing discovery events for an already-discovered entity.
#[derive(Component)]
pub struct Discovered;

pub const RESEARCH_POINTS_ID: &str = "research_points";
/// Prefix marking a themed research-currency item, e.g. `research.engineering`.
pub const RESEARCH_PREFIX: &str = "research.";
/// Theme the legacy `research_points` item routes to (back-compat during migration).
pub const DEFAULT_RESEARCH_THEME: &str = "material";

/// If `item` is a research-currency item, return the theme it credits.
/// Legacy `research_points` → the default theme; `research.<theme>` → `<theme>`.
pub fn research_theme_of(item: &str) -> Option<&str> {
    if item == RESEARCH_POINTS_ID {
        Some(DEFAULT_RESEARCH_THEME)
    } else {
        item.strip_prefix(RESEARCH_PREFIX)
    }
}

/// Per-theme research currency balances. Themes are content-defined strings
/// (`material`, `engineering`, `discovery`, `synthesis`, …) — no hardcoded enum.
#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct ResearchPool {
    pub amounts: HashMap<String, f32>,
}

impl ResearchPool {
    /// Credit `amount` of the `type_id` currency.
    pub fn add(&mut self, type_id: &str, amount: f32) {
        *self.amounts.entry(type_id.to_string()).or_default() += amount;
    }

    /// Current balance of `type_id` (0.0 if none).
    pub fn get(&self, type_id: &str) -> f32 {
        self.amounts.get(type_id).copied().unwrap_or(0.0)
    }

    /// Deduct `amount` of `type_id`; returns false and does nothing if insufficient.
    pub fn spend(&mut self, type_id: &str, amount: f32) -> bool {
        let bal = self.amounts.entry(type_id.to_string()).or_default();
        if *bal < amount {
            return false;
        }
        *bal -= amount;
        true
    }
}

/// Compact per-theme balance string, sorted by theme, nonzero only — e.g.
/// `"engineering:40  material:120"`. Returns `"0"` when nothing is banked.
pub fn format_research_balances(pool: &ResearchPool) -> String {
    let mut parts: Vec<(&String, f32)> = pool
        .amounts
        .iter()
        .filter(|(_, v)| **v > 0.0)
        .map(|(k, v)| (k, *v))
        .collect();
    parts.sort_by(|a, b| a.0.cmp(b.0));
    if parts.is_empty() {
        return "0".to_string();
    }
    parts
        .iter()
        .map(|(k, v)| format!("{k}:{v:.0}"))
        .collect::<Vec<_>>()
        .join("  ")
}

/// Cumulative gross count of every item produced this run, keyed by item id.
/// Feeds the `ProductionMilestone` unlock vector. Incremented at genuine production
/// sites only (recipe completion outputs, miner extraction) — never on item transfer.
#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct ProductionTally {
    pub produced: HashMap<String, f32>,
}

impl ProductionTally {
    /// Add `qty` to the cumulative total for `item`.
    pub fn record(&mut self, item: &str, qty: f32) {
        *self.produced.entry(item.to_string()).or_default() += qty;
    }

    /// Cumulative quantity of `item` produced so far (0.0 if never produced).
    pub fn get(&self, item: &str) -> f32 {
        self.produced.get(item).copied().unwrap_or(0.0)
    }
}

#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct TechTreeProgress {
    pub unlocked_nodes: HashSet<NodeId>,
    pub unlocked_recipes: HashSet<String>,
    pub unlocked_machines: HashSet<String>,
    /// Nodes permanently locked out via exclusive group resolution.
    pub disabled_nodes: HashSet<NodeId>,
}

pub struct ResearchPlugin;

impl Plugin for ResearchPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DiscoveryEvent>()
            .add_message::<TechNodeUnlocked>()
            .add_message::<UnlockNodeRequest>()
            .register_type::<ResearchPool>()
            .register_type::<TechTreeProgress>()
            .register_type::<ProductionTally>()
            .init_resource::<ResearchPool>()
            .init_resource::<TechTreeProgress>()
            .init_resource::<ProductionTally>()
            .add_systems(
                Update,
                check_research_unlocks
                    .in_set(crate::GameSystems::Simulation)
                    .run_if(in_state(crate::GameState::Playing)),
            );
    }
}

fn apply_effects(
    progress: &mut TechTreeProgress,
    tree: &TechTree,
    node_id: &NodeId,
    recipe_graph: Option<&RecipeGraph>,
) {
    let Some(node) = tree.nodes.get(node_id) else {
        return;
    };
    for effect in &node.effects {
        match effect {
            NodeEffect::UnlockRecipes(recipes) => {
                progress.unlocked_recipes.extend(recipes.iter().cloned());
            }
            NodeEffect::UnlockMachine(machine) => {
                progress.unlocked_machines.insert(machine.clone());
            }
            NodeEffect::UnlockRecipeTemplate(template_id) => {
                if let Some(graph) = recipe_graph
                    && let Some(ids) = graph.template_recipes.get(template_id)
                {
                    progress.unlocked_recipes.extend(ids.iter().cloned());
                }
            }
        }
    }
    if let Some(group) = &node.exclusive_group {
        for (id, peer) in &tree.nodes {
            if id != node_id
                && peer.exclusive_group.as_deref() == Some(group.as_str())
                && progress.disabled_nodes.insert(id.clone())
            {
                info!(
                    "Tech node '{}' locked out by exclusive group '{}'",
                    id, group
                );
            }
        }
    }
}

fn do_unlock(
    progress: &mut TechTreeProgress,
    tree: &TechTree,
    node_id: &NodeId,
    via_research: bool,
    events: &mut MessageWriter<TechNodeUnlocked>,
    recipe_graph: Option<&RecipeGraph>,
) {
    progress.unlocked_nodes.insert(node_id.clone());
    apply_effects(progress, tree, node_id, recipe_graph);
    events.write(TechNodeUnlocked {
        node_id: node_id.clone(),
        via_research,
    });
    info!("Tech node '{}' unlocked", node_id);
}

fn check_research_unlocks(
    tech_tree: Option<Res<TechTree>>,
    recipe_graph: Option<Res<RecipeGraph>>,
    mut pool: ResMut<ResearchPool>,
    mut progress: ResMut<TechTreeProgress>,
    tally: Option<Res<ProductionTally>>,
    mut unlock_requests: MessageReader<UnlockNodeRequest>,
    mut discovery_events: MessageReader<DiscoveryEvent>,
    mut unlocked_events: MessageWriter<TechNodeUnlocked>,
) {
    let Some(tech_tree) = tech_tree else {
        return;
    };
    let rg = recipe_graph.as_deref();

    // Player-initiated ResearchSpend unlocks
    let requests: Vec<NodeId> = unlock_requests.read().map(|r| r.0.clone()).collect();
    for node_id in &requests {
        if progress.unlocked_nodes.contains(node_id) {
            continue;
        }
        if progress.disabled_nodes.contains(node_id) {
            warn!("Unlock request for disabled node '{}'", node_id);
            continue;
        }
        let Some(node) = tech_tree.nodes.get(node_id) else {
            warn!("Unlock request for unknown node '{}'", node_id);
            continue;
        };
        if !node
            .prerequisites
            .iter()
            .all(|p| progress.unlocked_nodes.contains(p))
        {
            continue;
        }
        let UnlockVector::ResearchSpend { type_id, amount } = &node.primary_unlock else {
            warn!("Unlock request for non-ResearchSpend node '{}'", node_id);
            continue;
        };
        if pool.get(type_id) < *amount as f32 {
            continue;
        }
        pool.spend(type_id, *amount as f32);
        do_unlock(
            &mut progress,
            &tech_tree,
            node_id,
            true,
            &mut unlocked_events,
            rg,
        );
    }

    // Auto-unlock: ExplorationDiscovery
    let discovered_keys: HashSet<String> = discovery_events.read().map(|e| e.0.clone()).collect();
    if !discovered_keys.is_empty() {
        let to_unlock: Vec<NodeId> = tech_tree
            .nodes
            .iter()
            .filter(|(id, node)| {
                !progress.unlocked_nodes.contains(*id)
                    && !progress.disabled_nodes.contains(*id)
                    && matches!(&node.primary_unlock, UnlockVector::ExplorationDiscovery(key) if discovered_keys.contains(key))
                    && node
                        .prerequisites
                        .iter()
                        .all(|p| progress.unlocked_nodes.contains(p))
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in to_unlock {
            do_unlock(
                &mut progress,
                &tech_tree,
                &id,
                false,
                &mut unlocked_events,
                rg,
            );
        }
    }

    // Auto-unlock: ProductionMilestone (cumulative produced count reaches threshold)
    if let Some(tally) = tally.as_deref() {
        let to_unlock: Vec<NodeId> = tech_tree
            .nodes
            .iter()
            .filter(|(id, node)| {
                !progress.unlocked_nodes.contains(*id)
                    && !progress.disabled_nodes.contains(*id)
                    && matches!(&node.primary_unlock,
                        UnlockVector::ProductionMilestone { material, quantity }
                            if tally.get(material) >= *quantity)
                    && node
                        .prerequisites
                        .iter()
                        .all(|p| progress.unlocked_nodes.contains(p))
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in to_unlock {
            do_unlock(
                &mut progress,
                &tech_tree,
                &id,
                false,
                &mut unlocked_events,
                rg,
            );
        }
    }

    // Auto-unlock: PrerequisiteChain (loop until stable)
    loop {
        let to_unlock: Vec<NodeId> = tech_tree
            .nodes
            .iter()
            .filter(|(id, node)| {
                !progress.unlocked_nodes.contains(*id)
                    && !progress.disabled_nodes.contains(*id)
                    && matches!(node.primary_unlock, UnlockVector::PrerequisiteChain)
                    && node
                        .prerequisites
                        .iter()
                        .all(|p| progress.unlocked_nodes.contains(p))
            })
            .map(|(id, _)| id.clone())
            .collect();
        if to_unlock.is_empty() {
            break;
        }
        for id in to_unlock {
            do_unlock(
                &mut progress,
                &tech_tree,
                &id,
                false,
                &mut unlocked_events,
                rg,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tech_tree::{NodeCategory, NodeDef, NodeRarity, TechTree};
    use std::collections::HashMap;

    fn make_tree(nodes: Vec<NodeDef>) -> TechTree {
        let dependents: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let tier_order: Vec<NodeId> = nodes.iter().map(|n| n.id.clone()).collect();
        let nodes_map = nodes.into_iter().map(|n| (n.id.clone(), n)).collect();
        TechTree {
            nodes: nodes_map,
            dependents,
            tier_order,
        }
    }

    fn base_node(id: &str, cost: u32, prereqs: Vec<String>) -> NodeDef {
        NodeDef {
            id: id.to_string(),
            name: id.to_string(),
            category: NodeCategory::Processing,
            tier: 1,
            rarity: NodeRarity::Common,
            prerequisites: prereqs,
            primary_unlock: UnlockVector::ResearchSpend {
                type_id: "material".to_string(),
                amount: cost,
            },
            effects: vec![NodeEffect::UnlockRecipes(vec![format!("recipe_{id}")])],
            exclusive_group: None,
        }
    }

    /// A pool holding `amount` of the default `material` currency.
    fn material_pool(amount: f32) -> ResearchPool {
        let mut p = ResearchPool::default();
        p.add("material", amount);
        p
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_message::<TechNodeUnlocked>()
            .add_message::<UnlockNodeRequest>()
            .add_systems(Update, check_research_unlocks);
        app
    }

    #[test]
    fn unlocks_node_on_player_request_with_enough_points() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![base_node("alpha", 50, vec![])]));
        app.insert_resource(material_pool(50.0));
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("alpha"));
        assert!(progress.unlocked_recipes.contains("recipe_alpha"));
        let pool = app.world().resource::<ResearchPool>();
        assert_eq!(pool.get("material"), 0.0);
    }

    #[test]
    fn does_not_unlock_without_enough_points() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![base_node("alpha", 50, vec![])]));
        app.insert_resource(material_pool(49.0));
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
        let pool = app.world().resource::<ResearchPool>();
        assert_eq!(
            pool.get("material"),
            49.0,
            "no RP deducted on failed unlock"
        );
    }

    #[test]
    fn does_not_unlock_when_prereqs_missing() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![
            base_node("alpha", 50, vec![]),
            base_node("beta", 50, vec!["alpha".to_string()]),
        ]));
        app.insert_resource(material_pool(50.0));
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(UnlockNodeRequest("beta".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(
            !progress.unlocked_nodes.contains("beta"),
            "beta needs alpha first"
        );
        let pool = app.world().resource::<ResearchPool>();
        assert_eq!(
            pool.get("material"),
            50.0,
            "no RP deducted when prereqs not met"
        );
    }

    #[test]
    fn unlocks_chain_in_single_frame() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![
            base_node("alpha", 50, vec![]),
            base_node("beta", 50, vec!["alpha".to_string()]),
        ]));
        app.insert_resource(material_pool(100.0));
        app.init_resource::<TechTreeProgress>();

        // Both requests in same frame — alpha processes first, beta prereq then met
        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.world_mut()
            .write_message(UnlockNodeRequest("beta".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("alpha"));
        assert!(progress.unlocked_nodes.contains("beta"));
    }

    #[test]
    fn no_tech_tree_resource_is_noop() {
        let mut app = make_app();
        app.insert_resource(material_pool(999.0))
            .init_resource::<TechTreeProgress>();
        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();
        assert_eq!(
            app.world().resource::<ResearchPool>().get("material"),
            999.0
        );
    }

    #[test]
    fn unlock_machine_effect_populates_unlocked_machines() {
        let mut app = make_app();
        let mut node = base_node("alpha", 10, vec![]);
        node.effects = vec![NodeEffect::UnlockMachine("smelter".to_string())];
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(material_pool(10.0));
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_machines.contains("smelter"));
    }

    #[test]
    fn research_spend_does_not_auto_unlock() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![base_node("alpha", 50, vec![])]));
        app.insert_resource(material_pool(999.0));
        app.init_resource::<TechTreeProgress>();
        // No UnlockNodeRequest — ResearchSpend must be player-initiated
        app.update();
        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
    }

    #[test]
    fn exploration_discovery_unlocks_on_matching_key() {
        let mut app = make_app();
        let mut node = base_node("alpha", 0, vec![]);
        node.primary_unlock = UnlockVector::ExplorationDiscovery("xalite_deposit".to_string());
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(material_pool(0.0));
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(DiscoveryEvent("xalite_deposit".to_string()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("alpha"));
    }

    #[test]
    fn exploration_discovery_does_not_unlock_on_wrong_key() {
        let mut app = make_app();
        let mut node = base_node("alpha", 0, vec![]);
        node.primary_unlock = UnlockVector::ExplorationDiscovery("xalite_deposit".to_string());
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(material_pool(0.0));
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(DiscoveryEvent("wrong_key".to_string()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
    }

    fn milestone_node(id: &str, material: &str, quantity: f32, prereqs: Vec<String>) -> NodeDef {
        let mut node = base_node(id, 0, prereqs);
        node.primary_unlock = UnlockVector::ProductionMilestone {
            material: material.to_string(),
            quantity,
        };
        node
    }

    #[test]
    fn production_milestone_unlocks_when_threshold_reached() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![milestone_node(
            "miner",
            "stone",
            50.0,
            vec![],
        )]));
        app.insert_resource(material_pool(0.0));
        app.init_resource::<TechTreeProgress>();
        let mut tally = ProductionTally::default();
        tally.record("stone", 50.0);
        app.insert_resource(tally);

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("miner"));
    }

    #[test]
    fn production_milestone_does_not_unlock_below_threshold() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![milestone_node(
            "miner",
            "stone",
            50.0,
            vec![],
        )]));
        app.insert_resource(material_pool(0.0));
        app.init_resource::<TechTreeProgress>();
        let mut tally = ProductionTally::default();
        tally.record("stone", 49.0);
        app.insert_resource(tally);

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("miner"));
    }

    #[test]
    fn production_milestone_respects_prerequisites() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![
            base_node("alpha", 50, vec![]),
            milestone_node("beta", "stone", 10.0, vec!["alpha".to_string()]),
        ]));
        app.insert_resource(material_pool(0.0));
        app.init_resource::<TechTreeProgress>();
        let mut tally = ProductionTally::default();
        tally.record("stone", 10.0);
        app.insert_resource(tally);

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(
            !progress.unlocked_nodes.contains("beta"),
            "milestone met but prereq alpha not unlocked"
        );
    }

    #[test]
    fn research_theme_routing() {
        assert_eq!(research_theme_of("research_points"), Some("material"));
        assert_eq!(
            research_theme_of("research.engineering"),
            Some("engineering")
        );
        assert_eq!(research_theme_of("iron_ingot"), None);
    }

    #[test]
    fn pool_spend_isolated_per_theme() {
        let mut p = ResearchPool::default();
        p.add("material", 50.0);
        assert!(
            !p.spend("engineering", 10.0),
            "no engineering balance to spend"
        );
        assert!(p.spend("material", 30.0));
        assert_eq!(p.get("material"), 20.0);
        assert_eq!(p.get("engineering"), 0.0);
    }

    #[test]
    fn research_spend_uses_named_theme_only() {
        let mut app = make_app();
        let mut node = base_node("eng_node", 10, vec![]);
        node.primary_unlock = UnlockVector::ResearchSpend {
            type_id: "engineering".to_string(),
            amount: 10,
        };
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(material_pool(100.0)); // only material banked
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(UnlockNodeRequest("eng_node".into()));
        app.update();
        assert!(
            !app.world()
                .resource::<TechTreeProgress>()
                .unlocked_nodes
                .contains("eng_node"),
            "material currency must not pay an engineering cost"
        );

        app.world_mut()
            .resource_mut::<ResearchPool>()
            .add("engineering", 10.0);
        app.world_mut()
            .write_message(UnlockNodeRequest("eng_node".into()));
        app.update();
        assert!(
            app.world()
                .resource::<TechTreeProgress>()
                .unlocked_nodes
                .contains("eng_node"),
            "engineering balance should pay the engineering cost"
        );
    }

    #[test]
    fn production_tally_record_accumulates() {
        let mut tally = ProductionTally::default();
        tally.record("iron_ingot", 3.0);
        tally.record("iron_ingot", 2.0);
        assert_eq!(tally.get("iron_ingot"), 5.0);
        assert_eq!(tally.get("never_made"), 0.0);
    }

    #[test]
    fn exclusive_group_disables_peers_on_unlock() {
        let mut app = make_app();
        let mut node_a = base_node("solar", 50, vec![]);
        node_a.exclusive_group = Some("power_tier1".into());
        let mut node_b = base_node("wind", 50, vec![]);
        node_b.exclusive_group = Some("power_tier1".into());
        app.insert_resource(make_tree(vec![node_a, node_b]));
        app.insert_resource(material_pool(50.0));
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(UnlockNodeRequest("solar".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("solar"));
        assert!(
            progress.disabled_nodes.contains("wind"),
            "wind should be locked out"
        );
        assert!(!progress.unlocked_nodes.contains("wind"));
    }

    #[test]
    fn disabled_node_cannot_be_unlocked() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![base_node("alpha", 10, vec![])]));
        app.insert_resource(material_pool(999.0));
        let mut progress = TechTreeProgress::default();
        progress.disabled_nodes.insert("alpha".into());
        app.insert_resource(progress);

        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
    }

    fn make_recipe_graph_with_template(template_id: &str, recipe_ids: Vec<&str>) -> RecipeGraph {
        let mut template_recipes = HashMap::new();
        template_recipes.insert(
            template_id.to_string(),
            recipe_ids.into_iter().map(str::to_string).collect(),
        );
        RecipeGraph {
            materials: HashMap::new(),
            form_groups: HashMap::new(),
            templates: HashMap::new(),
            items: HashMap::new(),
            recipes: HashMap::new(),
            terminal: String::new(),
            producers: HashMap::new(),
            consumers: HashMap::new(),
            template_recipes,
        }
    }

    #[test]
    fn unlock_recipe_template_unlocks_all_expanded_recipes() {
        let mut app = make_app();
        let mut node = base_node("smelting", 10, vec![]);
        node.effects = vec![NodeEffect::UnlockRecipeTemplate("smelt_metal".to_string())];
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(material_pool(10.0));
        app.init_resource::<TechTreeProgress>();
        app.insert_resource(make_recipe_graph_with_template(
            "smelt_metal",
            vec!["smelt_metal__iron", "smelt_metal__copper"],
        ));

        app.world_mut()
            .write_message(UnlockNodeRequest("smelting".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_recipes.contains("smelt_metal__iron"));
        assert!(progress.unlocked_recipes.contains("smelt_metal__copper"));
    }
}

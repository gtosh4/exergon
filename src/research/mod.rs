use std::collections::HashSet;

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

/// Fired by the interaction system when the player completes a hand scan.
#[derive(Debug, Clone, Message)]
pub struct HandScanComplete {
    pub item_id: String,
}

pub const HAND_SCANNER_YIELD: f32 = 5.0;

/// Marker: prevents re-firing discovery events for an already-discovered entity.
#[derive(Component)]
pub struct Discovered;

pub const RESEARCH_POINTS_ID: &str = "research_points";

#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct ResearchPool {
    pub points: f32,
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
            .add_message::<HandScanComplete>()
            .register_type::<ResearchPool>()
            .register_type::<TechTreeProgress>()
            .init_resource::<ResearchPool>()
            .init_resource::<TechTreeProgress>()
            .add_systems(
                Update,
                (hand_scanner_system, check_research_unlocks)
                    .chain()
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

fn hand_scanner_system(
    mut events: MessageReader<HandScanComplete>,
    mut pool: ResMut<ResearchPool>,
) {
    for event in events.read() {
        pool.points += HAND_SCANNER_YIELD;
        info!(
            "Hand scan: +{HAND_SCANNER_YIELD} RP (item: {})",
            event.item_id
        );
    }
}

fn check_research_unlocks(
    tech_tree: Option<Res<TechTree>>,
    recipe_graph: Option<Res<RecipeGraph>>,
    mut pool: ResMut<ResearchPool>,
    mut progress: ResMut<TechTreeProgress>,
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
        let UnlockVector::ResearchSpend(cost) = &node.primary_unlock else {
            warn!("Unlock request for non-ResearchSpend node '{}'", node_id);
            continue;
        };
        if pool.points < *cost as f32 {
            continue;
        }
        pool.points -= *cost as f32;
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
            primary_unlock: UnlockVector::ResearchSpend(cost),
            effects: vec![NodeEffect::UnlockRecipes(vec![format!("recipe_{id}")])],
            exclusive_group: None,
        }
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
        app.insert_resource(ResearchPool { points: 50.0 });
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("alpha"));
        assert!(progress.unlocked_recipes.contains("recipe_alpha"));
        let pool = app.world().resource::<ResearchPool>();
        assert_eq!(pool.points, 0.0);
    }

    #[test]
    fn does_not_unlock_without_enough_points() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![base_node("alpha", 50, vec![])]));
        app.insert_resource(ResearchPool { points: 49.0 });
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
        let pool = app.world().resource::<ResearchPool>();
        assert_eq!(pool.points, 49.0, "no RP deducted on failed unlock");
    }

    #[test]
    fn does_not_unlock_when_prereqs_missing() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![
            base_node("alpha", 50, vec![]),
            base_node("beta", 50, vec!["alpha".to_string()]),
        ]));
        app.insert_resource(ResearchPool { points: 50.0 });
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
        assert_eq!(pool.points, 50.0, "no RP deducted when prereqs not met");
    }

    #[test]
    fn unlocks_chain_in_single_frame() {
        let mut app = make_app();
        app.insert_resource(make_tree(vec![
            base_node("alpha", 50, vec![]),
            base_node("beta", 50, vec!["alpha".to_string()]),
        ]));
        app.insert_resource(ResearchPool { points: 100.0 });
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
        app.insert_resource(ResearchPool { points: 999.0 })
            .init_resource::<TechTreeProgress>();
        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();
        assert_eq!(app.world().resource::<ResearchPool>().points, 999.0);
    }

    #[test]
    fn unlock_machine_effect_populates_unlocked_machines() {
        let mut app = make_app();
        let mut node = base_node("alpha", 10, vec![]);
        node.effects = vec![NodeEffect::UnlockMachine("smelter".to_string())];
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(ResearchPool { points: 10.0 });
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
        app.insert_resource(ResearchPool { points: 999.0 });
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
        app.insert_resource(ResearchPool { points: 0.0 });
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
        app.insert_resource(ResearchPool { points: 0.0 });
        app.init_resource::<TechTreeProgress>();

        app.world_mut()
            .write_message(DiscoveryEvent("wrong_key".to_string()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
    }

    #[test]
    fn exclusive_group_disables_peers_on_unlock() {
        let mut app = make_app();
        let mut node_a = base_node("solar", 50, vec![]);
        node_a.exclusive_group = Some("power_tier1".into());
        let mut node_b = base_node("wind", 50, vec![]);
        node_b.exclusive_group = Some("power_tier1".into());
        app.insert_resource(make_tree(vec![node_a, node_b]));
        app.insert_resource(ResearchPool { points: 50.0 });
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
        app.insert_resource(ResearchPool { points: 999.0 });
        let mut progress = TechTreeProgress::default();
        progress.disabled_nodes.insert("alpha".into());
        app.insert_resource(progress);

        app.world_mut()
            .write_message(UnlockNodeRequest("alpha".into()));
        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
    }

    #[test]
    fn hand_scan_adds_research_points() {
        let mut app = App::new();
        app.add_message::<HandScanComplete>()
            .add_systems(Update, hand_scanner_system)
            .init_resource::<ResearchPool>();

        app.world_mut().write_message(HandScanComplete {
            item_id: "iron_ore".to_string(),
        });
        app.update();

        let pool = app.world().resource::<ResearchPool>();
        assert_eq!(pool.points, HAND_SCANNER_YIELD);
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
        app.insert_resource(ResearchPool { points: 10.0 });
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

    #[test]
    fn hand_scan_accumulates_across_multiple_scans() {
        let mut app = App::new();
        app.add_message::<HandScanComplete>()
            .add_systems(Update, hand_scanner_system)
            .init_resource::<ResearchPool>();

        app.world_mut().write_message(HandScanComplete {
            item_id: "iron_ore".to_string(),
        });
        app.world_mut().write_message(HandScanComplete {
            item_id: "copper_ore".to_string(),
        });
        app.update();

        let pool = app.world().resource::<ResearchPool>();
        assert_eq!(pool.points, HAND_SCANNER_YIELD * 2.0);
    }
}

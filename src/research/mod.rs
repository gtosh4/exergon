use std::collections::HashSet;

use bevy::ecs::message::{Message, MessageReader};
use bevy::prelude::*;

use crate::tech_tree::{NodeEffect, NodeId, TechTree, UnlockVector};

#[derive(Debug, Clone)]
pub struct DiscoveryEvent(pub String);
impl Message for DiscoveryEvent {}

/// Marker: prevents re-firing discovery events for an already-discovered entity.
#[derive(Component)]
pub struct Discovered;

pub const RESEARCH_POINTS_ID: &str = "research_points";

#[derive(Resource, Default, Debug)]
pub struct ResearchPool {
    pub points: f32,
}

#[derive(Resource, Default, Debug)]
pub struct TechTreeProgress {
    pub unlocked_nodes: HashSet<NodeId>,
    pub unlocked_recipes: HashSet<String>,
    pub unlocked_machines: HashSet<String>,
}

pub struct ResearchPlugin;

impl Plugin for ResearchPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DiscoveryEvent>()
            .init_resource::<ResearchPool>()
            .init_resource::<TechTreeProgress>()
            .add_systems(
                Update,
                check_research_unlocks
                    .in_set(crate::GameSystems::Simulation)
                    .run_if(in_state(crate::GameState::Playing)),
            );
    }
}

fn check_research_unlocks(
    tech_tree: Option<Res<TechTree>>,
    mut pool: ResMut<ResearchPool>,
    mut progress: ResMut<TechTreeProgress>,
    mut discovery_events: MessageReader<DiscoveryEvent>,
) {
    let Some(tech_tree) = tech_tree else {
        return;
    };

    let discovered_keys: HashSet<String> = discovery_events.read().map(|e| e.0.clone()).collect();

    loop {
        let mut any_unlocked = false;
        for (id, node) in &tech_tree.nodes {
            if progress.unlocked_nodes.contains(id) {
                continue;
            }
            if !node
                .prerequisites
                .iter()
                .all(|p| progress.unlocked_nodes.contains(p))
            {
                continue;
            }
            let unlocked = match &node.primary_unlock {
                UnlockVector::ResearchSpend(cost) => {
                    if pool.points >= *cost as f32 {
                        pool.points -= *cost as f32;
                        true
                    } else {
                        false
                    }
                }
                UnlockVector::ExplorationDiscovery(key) => discovered_keys.contains(key),
                _ => false,
            };
            if unlocked {
                progress.unlocked_nodes.insert(id.clone());
                for effect in &node.effects {
                    match effect {
                        NodeEffect::UnlockRecipes(recipes) => {
                            progress.unlocked_recipes.extend(recipes.iter().cloned());
                        }
                        NodeEffect::UnlockMachine(machine) => {
                            progress.unlocked_machines.insert(machine.clone());
                        }
                    }
                }
                info!("Tech node '{}' unlocked", id);
                any_unlocked = true;
            }
        }
        if !any_unlocked {
            break;
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
        }
    }

    #[test]
    fn unlocks_node_when_enough_points() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks);
        app.insert_resource(make_tree(vec![base_node("alpha", 50, vec![])]));
        app.insert_resource(ResearchPool { points: 50.0 });
        app.init_resource::<TechTreeProgress>();

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("alpha"));
        assert!(progress.unlocked_recipes.contains("recipe_alpha"));
        let pool = app.world().resource::<ResearchPool>();
        assert_eq!(pool.points, 0.0);
    }

    #[test]
    fn does_not_unlock_without_enough_points() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks);
        app.insert_resource(make_tree(vec![base_node("alpha", 50, vec![])]));
        app.insert_resource(ResearchPool { points: 49.0 });
        app.init_resource::<TechTreeProgress>();

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
    }

    #[test]
    fn does_not_unlock_when_prereqs_missing() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks);
        app.insert_resource(make_tree(vec![
            base_node("alpha", 50, vec![]),
            base_node("beta", 50, vec!["alpha".to_string()]),
        ]));
        app.insert_resource(ResearchPool { points: 50.0 });
        app.init_resource::<TechTreeProgress>();

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("alpha"));
        assert!(
            !progress.unlocked_nodes.contains("beta"),
            "beta needs alpha first"
        );
    }

    #[test]
    fn unlocks_chain_in_single_frame() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks);
        app.insert_resource(make_tree(vec![
            base_node("alpha", 50, vec![]),
            base_node("beta", 50, vec!["alpha".to_string()]),
        ]));
        app.insert_resource(ResearchPool { points: 100.0 });
        app.init_resource::<TechTreeProgress>();

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("alpha"));
        assert!(progress.unlocked_nodes.contains("beta"));
    }

    #[test]
    fn no_tech_tree_resource_is_noop() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks)
            .insert_resource(ResearchPool { points: 999.0 })
            .init_resource::<TechTreeProgress>();
        // No TechTree resource — early return branch
        app.update();
        assert_eq!(app.world().resource::<ResearchPool>().points, 999.0);
    }

    #[test]
    fn unlock_machine_effect_populates_unlocked_machines() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks);
        let mut node = base_node("alpha", 10, vec![]);
        node.effects = vec![NodeEffect::UnlockMachine("smelter".to_string())];
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(ResearchPool { points: 10.0 });
        app.init_resource::<TechTreeProgress>();

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_machines.contains("smelter"));
    }

    #[test]
    fn skips_non_research_spend_unlock_vectors() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks);
        let mut node = base_node("alpha", 50, vec![]);
        node.primary_unlock = UnlockVector::ExplorationDiscovery("somewhere".to_string());
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(ResearchPool { points: 999.0 });
        app.init_resource::<TechTreeProgress>();

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
    }

    #[test]
    fn exploration_discovery_unlocks_on_matching_key() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks);
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
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, check_research_unlocks);
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
}

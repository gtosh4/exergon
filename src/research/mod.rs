use std::collections::HashSet;

use bevy::prelude::*;

use crate::tech_tree::{NodeEffect, NodeId, TechTree, UnlockVector};

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
        app.init_resource::<ResearchPool>()
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
) {
    let Some(tech_tree) = tech_tree else {
        return;
    };

    loop {
        let mut any_unlocked = false;
        for (id, node) in &tech_tree.nodes {
            if progress.unlocked_nodes.contains(id) {
                continue;
            }
            if !node.prerequisites.iter().all(|p| progress.unlocked_nodes.contains(p)) {
                continue;
            }
            let UnlockVector::ResearchSpend(cost) = node.primary_unlock else {
                continue;
            };
            if pool.points >= cost as f32 {
                pool.points -= cost as f32;
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
        TechTree { nodes: nodes_map, dependents, tier_order }
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
        app.add_systems(Update, check_research_unlocks);
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
        app.add_systems(Update, check_research_unlocks);
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
        app.add_systems(Update, check_research_unlocks);
        app.insert_resource(make_tree(vec![
            base_node("alpha", 50, vec![]),
            base_node("beta", 50, vec!["alpha".to_string()]),
        ]));
        app.insert_resource(ResearchPool { points: 50.0 });
        app.init_resource::<TechTreeProgress>();

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(progress.unlocked_nodes.contains("alpha"));
        assert!(!progress.unlocked_nodes.contains("beta"), "beta needs alpha first");
    }

    #[test]
    fn unlocks_chain_in_single_frame() {
        let mut app = App::new();
        app.add_systems(Update, check_research_unlocks);
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
    fn skips_non_research_spend_unlock_vectors() {
        let mut app = App::new();
        app.add_systems(Update, check_research_unlocks);
        let mut node = base_node("alpha", 50, vec![]);
        node.primary_unlock = UnlockVector::ExplorationDiscovery("somewhere".to_string());
        app.insert_resource(make_tree(vec![node]));
        app.insert_resource(ResearchPool { points: 999.0 });
        app.init_resource::<TechTreeProgress>();

        app.update();

        let progress = app.world().resource::<TechTreeProgress>();
        assert!(!progress.unlocked_nodes.contains("alpha"));
    }
}

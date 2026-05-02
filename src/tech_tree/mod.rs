use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use crate::content::load_ron_dir;

pub struct TechTreePlugin;

impl Plugin for TechTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_tech_tree);
    }
}

pub type NodeId = String;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum NodeCategory {
    Power,
    Processing,
    Logistics,
    Science,
    Exploration,
    Escape,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeRarity {
    Common,
    Uncommon,
    Rare,
    Unique,
}

#[derive(Deserialize, Clone, Debug)]
pub enum UnlockVector {
    ResearchSpend(u32),
    PrerequisiteChain,
    ProductionMilestone { material: String, quantity: f32 },
    ExplorationDiscovery(String),
    Observation(String),
}

#[derive(Deserialize, Clone, Debug)]
pub enum NodeEffect {
    UnlockRecipes(Vec<String>),
    UnlockMachine(String),
}

#[derive(Deserialize, Clone, Debug)]
pub struct NodeDef {
    pub id: NodeId,
    pub name: String,
    pub category: NodeCategory,
    pub tier: u8,
    pub rarity: NodeRarity,
    pub prerequisites: Vec<NodeId>,
    pub primary_unlock: UnlockVector,
    pub effects: Vec<NodeEffect>,
}

#[derive(Resource, Clone, Debug)]
pub struct TechTree {
    pub nodes: HashMap<NodeId, NodeDef>,
    /// node → node IDs that depend on it
    pub dependents: HashMap<NodeId, Vec<NodeId>>,
    /// node IDs ordered by tier then original insertion order
    pub tier_order: Vec<NodeId>,
}

impl TechTree {
    fn from_nodes(nodes: Vec<NodeDef>) -> Self {
        let mut dependents: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for node in &nodes {
            for prereq in &node.prerequisites {
                dependents
                    .entry(prereq.clone())
                    .or_default()
                    .push(node.id.clone());
            }
        }

        let tier_map: HashMap<NodeId, u8> = nodes.iter().map(|n| (n.id.clone(), n.tier)).collect();
        let mut tier_order: Vec<NodeId> = nodes.iter().map(|n| n.id.clone()).collect();
        tier_order.sort_by_key(|id| tier_map.get(id).copied().unwrap_or(0));

        let nodes = nodes.into_iter().map(|n| (n.id.clone(), n)).collect();

        Self {
            nodes,
            dependents,
            tier_order,
        }
    }
}

fn load_tech_tree(mut commands: Commands) {
    let nodes = load_ron_dir::<NodeDef>("assets/tech_nodes", "tech node");
    if nodes.is_empty() {
        warn!("No tech node definitions found in assets/tech_nodes/");
        return;
    }
    let tree = TechTree::from_nodes(nodes);
    let max_tier = tree.nodes.values().map(|n| n.tier).max().unwrap_or(0);
    info!(
        "Loaded tech tree: {} nodes, {} tiers",
        tree.nodes.len(),
        max_tier
    );
    commands.insert_resource(tree);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, tier: u8, prereqs: Vec<&str>) -> NodeDef {
        NodeDef {
            id: id.to_string(),
            name: id.to_string(),
            category: NodeCategory::Science,
            tier,
            rarity: NodeRarity::Common,
            prerequisites: prereqs.into_iter().map(str::to_string).collect(),
            primary_unlock: UnlockVector::ResearchSpend(1),
            effects: vec![],
        }
    }

    #[test]
    fn empty_nodes_gives_empty_tree() {
        let tree = TechTree::from_nodes(vec![]);
        assert!(tree.nodes.is_empty());
        assert!(tree.dependents.is_empty());
        assert!(tree.tier_order.is_empty());
    }

    #[test]
    fn single_node_no_dependents() {
        let tree = TechTree::from_nodes(vec![node("a", 1, vec![])]);
        assert_eq!(tree.nodes.len(), 1);
        assert!(tree.dependents.is_empty());
        assert_eq!(tree.tier_order, vec!["a"]);
    }

    #[test]
    fn dependents_populated_from_prerequisites() {
        let nodes = vec![node("a", 1, vec![]), node("b", 2, vec!["a"])];
        let tree = TechTree::from_nodes(nodes);
        let deps = tree.dependents.get("a").unwrap();
        assert_eq!(deps, &vec!["b"]);
        assert!(!tree.dependents.contains_key("b"));
    }

    #[test]
    fn tier_order_sorted_ascending() {
        let nodes = vec![
            node("c", 3, vec![]),
            node("a", 1, vec![]),
            node("b", 2, vec![]),
        ];
        let tree = TechTree::from_nodes(nodes);
        assert_eq!(tree.tier_order[0], "a");
        assert_eq!(tree.tier_order[1], "b");
        assert_eq!(tree.tier_order[2], "c");
    }

    #[test]
    fn multiple_dependents_on_one_node() {
        let nodes = vec![
            node("root", 1, vec![]),
            node("x", 2, vec!["root"]),
            node("y", 2, vec!["root"]),
        ];
        let tree = TechTree::from_nodes(nodes);
        let mut deps = tree.dependents.get("root").unwrap().clone();
        deps.sort();
        assert_eq!(deps, vec!["x", "y"]);
    }
}

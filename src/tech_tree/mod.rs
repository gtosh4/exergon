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

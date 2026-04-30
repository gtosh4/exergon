use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use crate::content::load_ron_dir;

pub struct RecipeGraphPlugin;

impl Plugin for RecipeGraphPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_recipe_graph);
    }
}

pub type MaterialId = String;
pub type RecipeId = String;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum MaterialKind {
    Base,
    Alien,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MaterialDef {
    pub id: MaterialId,
    pub name: String,
    pub kind: MaterialKind,
    /// Exactly one material per run should be true — the escape artifact.
    #[serde(default)]
    pub is_terminal: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ItemStack {
    pub material: MaterialId,
    pub quantity: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RecipeDef {
    pub id: RecipeId,
    pub inputs: Vec<ItemStack>,
    pub outputs: Vec<ItemStack>,
    pub byproducts: Vec<ItemStack>,
    pub machine_tier: u8,
    pub processing_time: f32,
    pub energy_cost: f32,
}

#[derive(Resource, Clone, Debug)]
pub struct RecipeGraph {
    pub materials: HashMap<MaterialId, MaterialDef>,
    pub recipes: HashMap<RecipeId, RecipeDef>,
    pub terminal: MaterialId,
    /// material → recipe IDs that produce it (including byproducts)
    pub producers: HashMap<MaterialId, Vec<RecipeId>>,
    /// material → recipe IDs that consume it
    pub consumers: HashMap<MaterialId, Vec<RecipeId>>,
}

impl RecipeGraph {
    fn from_vecs(materials: Vec<MaterialDef>, recipes: Vec<RecipeDef>, terminal: MaterialId) -> Self {
        let mut producers: HashMap<MaterialId, Vec<RecipeId>> = HashMap::new();
        let mut consumers: HashMap<MaterialId, Vec<RecipeId>> = HashMap::new();

        for recipe in &recipes {
            for stack in recipe.outputs.iter().chain(recipe.byproducts.iter()) {
                producers.entry(stack.material.clone()).or_default().push(recipe.id.clone());
            }
            for stack in &recipe.inputs {
                consumers.entry(stack.material.clone()).or_default().push(recipe.id.clone());
            }
        }

        let materials = materials.into_iter().map(|m| (m.id.clone(), m)).collect();
        let recipes = recipes.into_iter().map(|r| (r.id.clone(), r)).collect();

        Self { materials, recipes, terminal, producers, consumers }
    }
}

fn load_recipe_graph(mut commands: Commands) {
    let materials = load_ron_dir::<MaterialDef>("assets/materials", "material");
    let recipes = load_ron_dir::<RecipeDef>("assets/recipes", "recipe");

    let terminal = materials
        .iter()
        .find(|m| m.is_terminal)
        .map(|m| m.id.clone())
        .unwrap_or_else(|| {
            warn!("No terminal material defined (is_terminal: true); run may be unwinnable");
            String::new()
        });

    let graph = RecipeGraph::from_vecs(materials, recipes, terminal);
    info!(
        "Loaded recipe graph: {} materials, {} recipes, terminal={}",
        graph.materials.len(),
        graph.recipes.len(),
        graph.terminal
    );
    commands.insert_resource(graph);
}

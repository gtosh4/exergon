use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use crate::content::load_ron_dir;
use crate::inventory::ItemRegistry;

pub struct RecipeGraphPlugin;

impl Plugin for RecipeGraphPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_recipe_graph);
    }
}

pub type MaterialId = String;
pub type FormGroupId = String;
pub type FormId = String;
pub type TemplateId = String;
pub type ItemId = String;
pub type RecipeId = String;
pub type MachineTypeId = String;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum MaterialKind {
    Base,
    Alien,
}

#[derive(Deserialize, Clone, Debug)]
pub struct FormGroup {
    pub id: FormGroupId,
    pub forms: Vec<FormId>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MaterialDef {
    pub id: MaterialId,
    pub name: String,
    pub kind: MaterialKind,
    #[serde(default)]
    pub form_groups: Vec<FormGroupId>,
}

#[derive(Deserialize, Clone, Debug)]
pub enum ItemKind {
    Derived { material: MaterialId, form: FormId },
    Composite { template: Option<TemplateId> },
    Unique,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: String,
    pub kind: ItemKind,
    #[serde(default)]
    pub is_terminal: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RecipeTemplate {
    pub id: TemplateId,
    pub input_forms: Vec<(FormId, f32)>,
    pub output_form: FormId,
    pub group: FormGroupId,
    pub machine_type: MachineTypeId,
    pub base_time: f32,
    pub base_energy: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ItemStack {
    pub item: ItemId,
    pub quantity: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ConcreteRecipe {
    pub id: RecipeId,
    pub inputs: Vec<ItemStack>,
    pub outputs: Vec<ItemStack>,
    pub byproducts: Vec<ItemStack>,
    pub machine_type: MachineTypeId,
    pub machine_tier: u8,
    pub processing_time: f32,
    pub energy_cost: f32,
}

#[derive(Resource, Clone, Debug)]
pub struct RecipeGraph {
    pub materials: HashMap<MaterialId, MaterialDef>,
    pub form_groups: HashMap<FormGroupId, FormGroup>,
    pub templates: HashMap<TemplateId, RecipeTemplate>,
    pub items: HashMap<ItemId, ItemDef>,
    pub recipes: HashMap<RecipeId, ConcreteRecipe>,
    pub terminal: ItemId,
    /// item → recipe IDs that produce it (including byproducts)
    pub producers: HashMap<ItemId, Vec<RecipeId>>,
    /// item → recipe IDs that consume it
    pub consumers: HashMap<ItemId, Vec<RecipeId>>,
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub(crate) fn derive_items(
    materials: &HashMap<MaterialId, MaterialDef>,
    form_groups: &HashMap<FormGroupId, FormGroup>,
) -> Vec<ItemDef> {
    let mut items = Vec::new();
    for mat in materials.values() {
        for group_id in &mat.form_groups {
            let Some(group) = form_groups.get(group_id) else {
                continue;
            };
            for form in &group.forms {
                items.push(ItemDef {
                    id: format!("{}_{}", mat.id, form),
                    name: format!("{} {}", mat.name, capitalize(form)),
                    kind: ItemKind::Derived {
                        material: mat.id.clone(),
                        form: form.clone(),
                    },
                    is_terminal: false,
                });
            }
        }
    }
    items
}

pub(crate) fn expand_templates(
    materials: &HashMap<MaterialId, MaterialDef>,
    form_groups: &HashMap<FormGroupId, FormGroup>,
    templates: &[RecipeTemplate],
) -> Vec<ConcreteRecipe> {
    let mut recipes = Vec::new();
    for template in templates {
        let Some(group) = form_groups.get(&template.group) else {
            continue;
        };
        let all_forms_valid = template
            .input_forms
            .iter()
            .all(|(f, _)| group.forms.contains(f))
            && group.forms.contains(&template.output_form);
        if !all_forms_valid {
            continue;
        }
        for (mat_id, mat) in materials {
            if !mat.form_groups.contains(&template.group) {
                continue;
            }
            recipes.push(ConcreteRecipe {
                id: format!("{}__{}", template.id, mat_id),
                inputs: template
                    .input_forms
                    .iter()
                    .map(|(form, qty)| ItemStack {
                        item: format!("{}_{}", mat_id, form),
                        quantity: *qty,
                    })
                    .collect(),
                outputs: vec![ItemStack {
                    item: format!("{}_{}", mat_id, &template.output_form),
                    quantity: 1.0,
                }],
                byproducts: vec![],
                machine_type: template.machine_type.clone(),
                machine_tier: 1,
                processing_time: template.base_time,
                energy_cost: template.base_energy,
            });
        }
    }
    recipes
}

impl RecipeGraph {
    pub fn from_vecs(
        form_group_defs: Vec<FormGroup>,
        material_defs: Vec<MaterialDef>,
        template_defs: Vec<RecipeTemplate>,
        concrete_recipe_defs: Vec<ConcreteRecipe>,
        item_defs: Vec<ItemDef>,
    ) -> Self {
        let form_groups: HashMap<FormGroupId, FormGroup> = form_group_defs
            .into_iter()
            .map(|g| (g.id.clone(), g))
            .collect();

        let materials: HashMap<MaterialId, MaterialDef> = material_defs
            .into_iter()
            .map(|m| (m.id.clone(), m))
            .collect();

        let derived = derive_items(&materials, &form_groups);
        let expanded = expand_templates(&materials, &form_groups, &template_defs);

        let mut items: HashMap<ItemId, ItemDef> = HashMap::new();
        for item in derived.into_iter().chain(item_defs.into_iter()) {
            items.insert(item.id.clone(), item);
        }

        let mut recipes: HashMap<RecipeId, ConcreteRecipe> = HashMap::new();
        for r in concrete_recipe_defs.into_iter().chain(expanded.into_iter()) {
            recipes.insert(r.id.clone(), r);
        }

        let templates: HashMap<TemplateId, RecipeTemplate> = template_defs
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect();

        let mut producers: HashMap<ItemId, Vec<RecipeId>> = HashMap::new();
        let mut consumers: HashMap<ItemId, Vec<RecipeId>> = HashMap::new();
        for recipe in recipes.values() {
            for stack in recipe.outputs.iter().chain(recipe.byproducts.iter()) {
                producers
                    .entry(stack.item.clone())
                    .or_default()
                    .push(recipe.id.clone());
            }
            for stack in &recipe.inputs {
                consumers
                    .entry(stack.item.clone())
                    .or_default()
                    .push(recipe.id.clone());
            }
        }

        let terminal = items.values().find(|i| i.is_terminal).map_or_else(
            || {
                warn!("No terminal item defined (is_terminal: true); run may be unwinnable");
                String::new()
            },
            |i| i.id.clone(),
        );

        Self {
            materials,
            form_groups,
            templates,
            items,
            recipes,
            terminal,
            producers,
            consumers,
        }
    }
}

fn load_recipe_graph(mut commands: Commands) {
    let form_groups = load_ron_dir::<FormGroup>("assets/form_groups", "form_group");
    let materials = load_ron_dir::<MaterialDef>("assets/materials", "material");
    let templates = load_ron_dir::<RecipeTemplate>("assets/recipe_templates", "recipe_template");
    let concrete_recipes = load_ron_dir::<ConcreteRecipe>("assets/recipes", "recipe");
    let item_defs = load_ron_dir::<ItemDef>("assets/items", "item");

    let graph = RecipeGraph::from_vecs(
        form_groups,
        materials,
        templates,
        concrete_recipes,
        item_defs,
    );

    let mut item_registry = ItemRegistry::default();
    for item in graph.items.values() {
        item_registry.register(item.clone());
    }

    info!(
        "Loaded recipe graph: {} materials, {} form_groups, {} templates, {} items, {} recipes, terminal={}",
        graph.materials.len(),
        graph.form_groups.len(),
        graph.templates.len(),
        graph.items.len(),
        graph.recipes.len(),
        graph.terminal,
    );
    commands.insert_resource(graph);
    commands.insert_resource(item_registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mat(id: &str) -> MaterialDef {
        MaterialDef {
            id: id.to_string(),
            name: id.to_string(),
            kind: MaterialKind::Base,
            form_groups: vec![],
        }
    }

    fn mat_with_group(id: &str, group: &str) -> MaterialDef {
        MaterialDef {
            id: id.to_string(),
            name: capitalize(id),
            kind: MaterialKind::Base,
            form_groups: vec![group.to_string()],
        }
    }

    fn stack(item: &str, qty: f32) -> ItemStack {
        ItemStack {
            item: item.to_string(),
            quantity: qty,
        }
    }

    fn recipe(
        id: &str,
        inputs: Vec<ItemStack>,
        outputs: Vec<ItemStack>,
        byproducts: Vec<ItemStack>,
    ) -> ConcreteRecipe {
        ConcreteRecipe {
            id: id.to_string(),
            inputs,
            outputs,
            byproducts,
            machine_type: "furnace".to_string(),
            machine_tier: 1,
            processing_time: 1.0,
            energy_cost: 10.0,
        }
    }

    fn metal_group() -> FormGroup {
        FormGroup {
            id: "metal".to_string(),
            forms: vec!["ore".to_string(), "ingot".to_string()],
        }
    }

    fn terminal_item(id: &str) -> ItemDef {
        ItemDef {
            id: id.to_string(),
            name: id.to_string(),
            kind: ItemKind::Unique,
            is_terminal: true,
        }
    }

    #[test]
    fn empty_graph() {
        let g = RecipeGraph::from_vecs(vec![], vec![], vec![], vec![], vec![]);
        assert!(g.materials.is_empty());
        assert!(g.recipes.is_empty());
        assert!(g.producers.is_empty());
        assert!(g.consumers.is_empty());
        assert!(g.items.is_empty());
        assert_eq!(g.terminal, "");
    }

    #[test]
    fn output_creates_producer() {
        let r = recipe("r1", vec![], vec![stack("iron", 1.0)], vec![]);
        let g = RecipeGraph::from_vecs(vec![], vec![], vec![], vec![r], vec![]);
        assert!(g.producers.get("iron").unwrap().contains(&"r1".to_string()));
    }

    #[test]
    fn byproduct_creates_producer() {
        let r = recipe("r1", vec![], vec![], vec![stack("slag", 0.5)]);
        let g = RecipeGraph::from_vecs(vec![], vec![], vec![], vec![r], vec![]);
        assert!(g.producers.get("slag").unwrap().contains(&"r1".to_string()));
    }

    #[test]
    fn input_creates_consumer() {
        let r = recipe("r1", vec![stack("coal", 2.0)], vec![], vec![]);
        let g = RecipeGraph::from_vecs(vec![], vec![], vec![], vec![r], vec![]);
        assert!(g.consumers.get("coal").unwrap().contains(&"r1".to_string()));
    }

    #[test]
    fn terminal_from_terminal_item() {
        let g = RecipeGraph::from_vecs(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![terminal_item("crystal")],
        );
        assert_eq!(g.terminal, "crystal");
    }

    #[test]
    fn materials_indexed_by_id() {
        let g = RecipeGraph::from_vecs(
            vec![],
            vec![mat("iron"), mat("gold")],
            vec![],
            vec![],
            vec![],
        );
        assert!(g.materials.contains_key("iron"));
        assert!(g.materials.contains_key("gold"));
    }

    #[test]
    fn derive_items_generates_ids_and_names() {
        let mut materials = HashMap::new();
        let m = mat_with_group("copper", "metal");
        materials.insert(m.id.clone(), m);
        let mut form_groups = HashMap::new();
        let g = metal_group();
        form_groups.insert(g.id.clone(), g);

        let items = derive_items(&materials, &form_groups);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(
            ids.contains(&"copper_ore"),
            "expected copper_ore in {ids:?}"
        );
        assert!(
            ids.contains(&"copper_ingot"),
            "expected copper_ingot in {ids:?}"
        );
        let ore = items.iter().find(|i| i.id == "copper_ore").unwrap();
        assert_eq!(ore.name, "Copper Ore");
    }

    #[test]
    fn expand_templates_produces_recipe_for_each_material_in_group() {
        let mut materials = HashMap::new();
        for name in ["copper", "iron"] {
            let m = mat_with_group(name, "metal");
            materials.insert(m.id.clone(), m);
        }
        let mut form_groups = HashMap::new();
        let g = metal_group();
        form_groups.insert(g.id.clone(), g);
        let template = RecipeTemplate {
            id: "smelt".to_string(),
            input_forms: vec![("ore".to_string(), 1.0)],
            output_form: "ingot".to_string(),
            group: "metal".to_string(),
            machine_type: "furnace".to_string(),
            base_time: 5.0,
            base_energy: 20.0,
        };

        let recipes = expand_templates(&materials, &form_groups, &[template]);
        assert_eq!(recipes.len(), 2);
        let ids: Vec<&str> = recipes.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.iter().any(|id| id.contains("smelt")));
        // Verify input/output item IDs follow {material}_{form} pattern
        for r in &recipes {
            assert!(r.inputs[0].item.ends_with("_ore"));
            assert!(r.outputs[0].item.ends_with("_ingot"));
        }
    }

    #[test]
    fn expand_templates_skips_material_without_group() {
        let mut materials = HashMap::new();
        let plain = mat("stone"); // no form groups
        let metal = mat_with_group("iron", "metal");
        materials.insert(plain.id.clone(), plain);
        materials.insert(metal.id.clone(), metal);
        let mut form_groups = HashMap::new();
        let g = metal_group();
        form_groups.insert(g.id.clone(), g);
        let template = RecipeTemplate {
            id: "smelt".to_string(),
            input_forms: vec![("ore".to_string(), 1.0)],
            output_form: "ingot".to_string(),
            group: "metal".to_string(),
            machine_type: "furnace".to_string(),
            base_time: 5.0,
            base_energy: 20.0,
        };

        let recipes = expand_templates(&materials, &form_groups, &[template]);
        assert_eq!(
            recipes.len(),
            1,
            "only iron should produce a recipe, not stone"
        );
        assert!(recipes[0].inputs[0].item.starts_with("iron"));
    }

    #[test]
    fn graph_contains_derived_items_after_construction() {
        let fg = metal_group();
        let mat = mat_with_group("copper", "metal");
        let g = RecipeGraph::from_vecs(vec![fg], vec![mat], vec![], vec![], vec![]);
        assert!(g.items.contains_key("copper_ore"));
        assert!(g.items.contains_key("copper_ingot"));
    }
}

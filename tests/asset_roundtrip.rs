//! Round-trip regression for every RON asset kind: load through the real deserializer,
//! serialize back with the canonical writer, reload, and assert the *typed value* is
//! unchanged. This guards the field-drop hazard — if a kind were (de)serialized through a
//! partial struct (e.g. the reduced `MachineDef`/`MachineItemEntry` instead of the complete
//! `MachineFileDef`), a field present in the file would vanish on the round-trip and this
//! test would fail (either the reparse errors on a now-missing required field, or the
//! re-serialized value's JSON differs).
//!
//! Comparison is via `serde_json::Value` (which the domain types don't need to derive
//! `PartialEq` for), so it tolerates the canonical writer making `#[serde(default)]` fields
//! explicit while still catching any actual data loss.

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use exergon::asset_store::{from_ron, load_all, to_ron};
use exergon::content::{BiomeDef, DepositDef, LayerDef, VeinDef};
use exergon::machine::{MachineFileDef, PlaceableDef};
use exergon::planet::PlanetArchetypeDef;
use exergon::recipe_graph::{ConcreteRecipe, FormGroup, ItemDef, MaterialDef, RecipeTemplate};
use exergon::seed::CuratedSeedEntry;
use exergon::tech_tree::NodeDef;

/// Assert every file under `dir` survives a load -> serialize -> reload round-trip unchanged,
/// and that the directory is non-empty (catches a wrong path).
fn check_dir<T: Serialize + DeserializeOwned>(dir: &str) {
    let items = load_all::<T>(Path::new(dir)).unwrap_or_else(|e| panic!("load {dir}: {e}"));
    assert!(!items.is_empty(), "no assets loaded from {dir}");
    for (path, original) in &items {
        check_value(original, &path.display().to_string());
    }
}

fn check_value<T: Serialize + DeserializeOwned>(original: &T, label: &str) {
    let ron = to_ron(original).unwrap_or_else(|e| panic!("serialize {label}: {e}"));
    let reparsed: T =
        from_ron(&ron).unwrap_or_else(|e| panic!("reparse {label} failed (dropped field?): {e}"));
    let before = serde_json::to_value(original).unwrap();
    let after = serde_json::to_value(&reparsed).unwrap();
    assert_eq!(before, after, "round-trip changed {label}");
}

#[test]
fn all_id_kinds_roundtrip() {
    check_dir::<ConcreteRecipe>("assets/recipes");
    check_dir::<NodeDef>("assets/tech_nodes");
    check_dir::<ItemDef>("assets/items");
    check_dir::<MaterialDef>("assets/materials");
    check_dir::<FormGroup>("assets/form_groups");
    check_dir::<RecipeTemplate>("assets/recipe_templates");
    check_dir::<VeinDef>("assets/veins");
    check_dir::<LayerDef>("assets/layers");
    check_dir::<BiomeDef>("assets/biomes");
    check_dir::<DepositDef>("assets/deposits");
    check_dir::<MachineFileDef>("assets/machines");
    check_dir::<PlaceableDef>("assets/placeables");
    check_dir::<PlanetArchetypeDef>("assets/planet/archetypes");
}

#[test]
fn curated_seeds_roundtrip() {
    // Single list file rather than one-entity-per-file.
    let seeds: Vec<CuratedSeedEntry> =
        from_ron(&std::fs::read_to_string("assets/seeds/curated.ron").unwrap()).unwrap();
    assert!(!seeds.is_empty(), "no curated seeds loaded");
    check_value(&seeds, "assets/seeds/curated.ron");
}

#[test]
fn texture_manifest_roundtrip() {
    // Bare string list.
    let manifest: Vec<String> =
        from_ron(&std::fs::read_to_string("assets/textures/blocks/manifest.ron").unwrap()).unwrap();
    assert!(!manifest.is_empty(), "no texture entries loaded");
    check_value(&manifest, "assets/textures/blocks/manifest.ron");
}

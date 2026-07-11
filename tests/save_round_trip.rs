//! Integration test for Phase 0.1 save/load round-trip.
//!
//! Covers `docs/technical/save.md §10` invariants 1–7 that apply within
//! current VS scope: Run entity components, header status, run_id, seed and
//! domain seed round-trip, and `TechTreeProgress` round-trip via
//! `include_resource`. Gameplay entity (Machine, cables, etc.) round-trip is
//! deferred until those types are tagged `Save` + made `Reflect`-friendly.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use exergon::GameState;
use exergon::research::{ResearchPlugin, ResearchPool, TechTreeProgress};
use exergon::save::{
    DifficultyTier, LoadRunEvent, NewRunEvent, Run, RunSaveHeader, RunStatus, SavePlugin, SaveRoot,
    trigger_run_save,
};
use exergon::seed::{DomainSeeds, RunSeed, SeedPlugin, hash_text};

fn tmp_root(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("exergon_save_test_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

fn make_app(root: PathBuf) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        StatesPlugin,
        ScenePlugin,
    ))
    .insert_resource(SaveRoot(root))
    .add_plugins((SeedPlugin, SavePlugin, ResearchPlugin))
    .init_state::<GameState>();
    app
}

#[test]
fn new_run_event_spawns_run_entity_with_expected_components() {
    let root = tmp_root("new_run");
    let mut app = make_app(root);

    app.world_mut()
        .resource_mut::<Messages<NewRunEvent>>()
        .write(NewRunEvent {
            seed_text: "alpha-beta".into(),
            test_mode: false,
        });
    app.update();

    let world = app.world_mut();
    let mut q = world.query_filtered::<(&RunSaveHeader, &RunSeed, &DomainSeeds), With<Run>>();
    let (header, seed, domains) = q.single(world).expect("run entity should exist");

    assert_eq!(header.status, RunStatus::InProgress);
    assert_eq!(header.difficulty, DifficultyTier::Initiation);
    assert!(!header.run_id.is_empty());
    assert_eq!(header.seed_text, "alpha-beta");
    assert_eq!(seed.text, "alpha-beta");
    assert_eq!(seed.hash, hash_text("alpha-beta"));
    let expected = DomainSeeds::from_master(seed.hash);
    assert_eq!(domains.world, expected.world);
    assert_eq!(domains.tech_tree, expected.tech_tree);
}

#[test]
fn save_then_load_round_trip_preserves_run_state() {
    let root = tmp_root("round_trip");
    let mut app = make_app(root.clone());

    // Spawn a run, populate research state.
    app.world_mut()
        .resource_mut::<Messages<NewRunEvent>>()
        .write(NewRunEvent {
            seed_text: "round-trip".into(),
            test_mode: false,
        });
    app.update();

    let original_run_id = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&RunSaveHeader, With<Run>>();
        q.single(world).unwrap().run_id.clone()
    };

    {
        let world = app.world_mut();
        let mut pool = world.resource_mut::<ResearchPool>();
        pool.add("material", 42.0);
    }
    {
        let world = app.world_mut();
        let mut progress = world.resource_mut::<TechTreeProgress>();
        progress.unlocked_nodes.insert("alpha".into());
        progress.unlocked_recipes.insert("smelt_iron".into());
    }

    // Trigger save explicitly (avoid waiting AUTO_SAVE_SECS).
    let save_run_id = original_run_id.clone();
    app.world_mut()
        .run_system_cached_with(
            move |In(run_id): In<String>, mut commands: Commands, save_root: Res<SaveRoot>| {
                trigger_run_save(&mut commands, &save_root, &run_id);
            },
            save_run_id,
        )
        .unwrap();
    // Let the save observer write the file.
    app.update();

    let path = SaveRoot(root.clone()).run_save_path(&original_run_id);
    assert!(path.is_file(), "run.ron must be written to {path:?}");

    // Fresh app: load, verify state restored.
    let mut app2 = make_app(root.clone());
    app2.world_mut()
        .resource_mut::<Messages<LoadRunEvent>>()
        .write(LoadRunEvent {
            run_id: original_run_id.clone(),
        });
    app2.update(); // dispatch load trigger
    app2.update(); // process load observer

    let world = app2.world_mut();
    let mut q = world.query_filtered::<(&RunSaveHeader, &RunSeed, &DomainSeeds), With<Run>>();
    let (header, seed, domains) = q.single(world).expect("run entity should exist after load");
    assert_eq!(header.run_id, original_run_id);
    assert_eq!(header.seed_text, "round-trip");
    assert_eq!(seed.text, "round-trip");
    assert_eq!(domains.world, DomainSeeds::from_master(seed.hash).world);

    let pool = world.resource::<ResearchPool>();
    assert_eq!(
        pool.get("material"),
        42.0,
        "ResearchPool must round-trip per-theme"
    );
    let progress = world.resource::<TechTreeProgress>();
    assert!(progress.unlocked_nodes.contains("alpha"));
    assert!(progress.unlocked_recipes.contains("smelt_iron"));
}

#[test]
fn missing_file_load_is_handled_not_panic() {
    let root = tmp_root("missing");
    let mut app = make_app(root);
    app.world_mut()
        .resource_mut::<Messages<LoadRunEvent>>()
        .write(LoadRunEvent {
            run_id: "does_not_exist".into(),
        });
    // Two updates: event reader, then observer handle.
    app.update();
    app.update();
    // No panic; no Run entity materialized.
    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<Run>>();
    assert_eq!(q.iter(world).count(), 0);
}

#[test]
fn list_run_ids_returns_run_dirs_with_save_file() {
    let root = tmp_root("list");
    let save_root = SaveRoot(root.clone());

    // Create two runs on disk.
    for id in &["run_a", "run_b"] {
        let dir = save_root.run_dir(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(save_root.run_save_path(id), "()\n").unwrap();
    }
    // Stray dir without run.ron should be filtered.
    std::fs::create_dir_all(save_root.run_dir("orphan")).unwrap();

    let mut ids = exergon::save::list_run_ids(&save_root);
    ids.sort();
    assert_eq!(ids, vec!["run_a".to_string(), "run_b".to_string()]);
}

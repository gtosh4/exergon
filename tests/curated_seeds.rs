//! Phase 10 seed validation: every curated seed must play through an **Insight Run** —
//! land → generate world → place & wire the starter kit → mine → complete the first
//! research unlock → reveal the planet's atmospheric properties. This is the shortest
//! playtest milestone (VS §5); a seed that cannot reach it is unshippable.
//!
//! The seeds are loaded from `assets/seeds/curated.ron` through the same deserializer the
//! game uses, and each seed text is hashed → master seed exactly as `save::spawn_run` does
//! (`hash_text` → `DomainSeeds::from_master`). The mine→analyse→research loop runs on
//! simulated time (see `advance_until`) — no hand-poked internal state — so a pass means
//! the real systems carried that seed to first insight.
//!
//! This mirrors Stage 0–1b of `landing_to_first_research.rs` (see that file for the
//! full-fidelity walkthrough); here the same milestone is swept across all curated seeds.

use std::time::Duration;

use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldAsset;

use exergon::content::ContentPlugin;
use exergon::drone::FogCellRevealedEvent;
use exergon::logistics::{LogisticsSimPlugin, StorageUnit};
use exergon::machine::{
    Machine, MachinePlugin, MachinePortLayout, MachinePortLayouts, MachineState, MinerMachine,
};
use exergon::planet::{Planet, PlanetPlugin, PlanetPropertyVisibility, PropertyVisibility};
use exergon::power::PowerPlugin;
use exergon::recipe_graph::RecipeGraphPlugin;
use exergon::research::{ResearchPlugin, ResearchPool, TechTreeProgress, UnlockNodeRequest};
use exergon::seed::{CuratedSeedEntry, DomainSeeds, hash_text};
use exergon::tech_tree::TechTreePlugin;
use exergon::world::{
    CableConnectionEvent, MainCamera, OreDeposit, WorldObjectEvent, WorldObjectKind, WorldgenPlugin,
};
use exergon::{GameState, PlayMode};

const PORT_OFFSET: Vec3 = Vec3::new(1.0, 0.0, 0.0);

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        StatesPlugin,
        ScenePlugin,
    ));
    app.init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Gltf>()
        .init_asset::<GltfMesh>()
        .init_asset::<GltfNode>()
        .init_asset::<WorldAsset>();
    app.add_message::<WorldObjectEvent>()
        .add_message::<CableConnectionEvent>()
        .add_message::<FogCellRevealedEvent>()
        .init_state::<GameState>()
        .add_sub_state::<PlayMode>()
        .add_plugins((
            ContentPlugin,
            WorldgenPlugin,
            RecipeGraphPlugin,
            TechTreePlugin,
            MachinePlugin,
            LogisticsSimPlugin,
            PowerPlugin,
            ResearchPlugin,
            PlanetPlugin,
        ));
    app
}

fn place(app: &mut App, item_id: &str, pos: Vec3) {
    app.world_mut().write_message(WorldObjectEvent {
        transform: Transform::from_translation(pos),
        item_id: item_id.to_string(),
        kind: WorldObjectKind::Placed,
    });
}

fn connect(app: &mut App, from: Vec3, to: Vec3) {
    app.world_mut().write_message(CableConnectionEvent {
        from,
        to,
        item_id: "logistics_cable".to_string(),
        kind: WorldObjectKind::Placed,
        from_port: None,
        to_port: None,
    });
}

fn machine_entity(app: &mut App, machine_type: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Machine)>();
    q.iter(app.world())
        .find(|(_, m)| m.machine_type == machine_type)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("no placed machine of type {machine_type}"))
}

fn research_points(app: &App) -> f32 {
    app.world().resource::<ResearchPool>().points
}

fn origin_deposit(app: &mut App) -> (Entity, Vec3, Vec<(String, f32)>) {
    let mut q = app.world_mut().query::<(Entity, &Transform, &OreDeposit)>();
    q.iter(app.world())
        .find(|(_, _, d)| d.chunk_pos == IVec2::ZERO)
        .map(|(e, t, d)| (e, t.translation, d.ores.clone()))
        .expect("world generation must place a deposit on the origin chunk")
}

fn advance_until(app: &mut App, dt: f32, max_secs: f32, mut done: impl FnMut(&App) -> bool) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        dt,
    )));
    let mut elapsed = 0.0;
    while elapsed < max_secs {
        if done(app) {
            return;
        }
        app.update();
        elapsed += dt;
    }
    panic!("advance_until: condition not met within {max_secs}s of simulated time");
}

/// Drive one curated seed from landing to first insight. Panics (naming the seed) on any
/// milestone it fails to reach.
fn run_insight_run(seed_text: &str) {
    let master = hash_text(seed_text);
    let mut app = build_app();

    app.world_mut()
        .spawn((exergon::save::Run, DomainSeeds::from_master(master)));
    app.world_mut().spawn((Transform::default(), MainCamera));
    app.update();

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Loading);
    for _ in 0..4 {
        app.update();
    }

    let (deposit_e, deposit_pos, ores) = origin_deposit(&mut app);
    assert!(
        ores.iter().any(|(id, _)| id == "stone"),
        "[{seed_text}] origin deposit must yield stone to bootstrap research, got {ores:?}"
    );

    app.world_mut()
        .resource_mut::<TechTreeProgress>()
        .unlocked_recipes
        .insert("basic_analysis".to_string());

    {
        let mut layouts = app.world_mut().resource_mut::<MachinePortLayouts>();
        for id in ["storage_crate", "miner", "analysis_station"] {
            layouts.by_machine.insert(
                id.to_string(),
                MachinePortLayout {
                    energy: vec![],
                    logistics: vec![PORT_OFFSET],
                },
            );
        }
    }

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    let miner_pos = deposit_pos;
    let storage_pos = deposit_pos + Vec3::new(4.0, 0.0, 0.0);
    let station_pos = deposit_pos + Vec3::new(8.0, 0.0, 0.0);
    place(&mut app, "storage_crate", storage_pos);
    place(&mut app, "miner", miner_pos);
    place(&mut app, "analysis_station", station_pos);
    app.update();

    let miner_e = machine_entity(&mut app, "miner");
    assert_eq!(
        app.world().get::<MinerMachine>(miner_e).map(|m| m.deposit),
        Some(deposit_e),
        "[{seed_text}] placed miner should latch onto the generated origin deposit"
    );

    let storage_e = machine_entity(&mut app, "storage_crate");
    app.world_mut().entity_mut(storage_e).insert(StorageUnit {
        items: Default::default(),
    });

    connect(&mut app, storage_pos + PORT_OFFSET, miner_pos + PORT_OFFSET);
    connect(
        &mut app,
        storage_pos + PORT_OFFSET,
        station_pos + PORT_OFFSET,
    );
    app.update();

    // Mine → analyse under simulated time until the first ResearchSpend node is affordable.
    let station_e = machine_entity(&mut app, "analysis_station");
    let mut station_ran = false;
    advance_until(&mut app, 0.5, 1_000.0, |app| {
        if app.world().get::<MachineState>(station_e).copied() == Some(MachineState::Running) {
            station_ran = true;
        }
        research_points(app) >= 30.0
    });
    assert!(
        station_ran,
        "[{seed_text}] analysis station must actually run the recipe (not just be granted points)"
    );

    app.world_mut()
        .write_message(UnlockNodeRequest("ore_extraction".into()));
    app.update();
    assert!(
        app.world()
            .resource::<TechTreeProgress>()
            .unlocked_nodes
            .contains("ore_extraction"),
        "[{seed_text}] first research node should unlock after spending research points"
    );

    // Insight beat: the first research spend reveals both atmospheric properties.
    app.update();
    let vis = {
        let mut q = app
            .world_mut()
            .query_filtered::<&PlanetPropertyVisibility, With<Planet>>();
        q.single(app.world())
            .cloned()
            .expect("a planet must have been generated for this seed")
    };
    assert_eq!(
        vis.atmospheric_oxygen,
        PropertyVisibility::Revealed,
        "[{seed_text}] first research spend must reveal atmospheric oxygen"
    );
    assert_eq!(
        vis.atmospheric_pressure,
        PropertyVisibility::Revealed,
        "[{seed_text}] first research spend must reveal atmospheric pressure"
    );
}

#[test]
fn every_curated_seed_reaches_insight_run() {
    let ron =
        std::fs::read_to_string("assets/seeds/curated.ron").expect("curated seed file must exist");
    let entries: Vec<CuratedSeedEntry> = ron::from_str(&ron).expect("curated.ron must deserialize");
    assert_eq!(entries.len(), 5, "VS §5 curates exactly 5 seeds");

    for entry in &entries {
        run_insight_run(&entry.seed);
    }
}

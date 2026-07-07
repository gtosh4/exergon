//! End-to-end integration test for the lander bootstrap loop, exercised through the
//! real world-generation + placement + logistics + recipe + research systems from a
//! fixed seed (not hand-built entities):
//!
//!   fixed seed → generate terrain + surface ore deposits → place machines
//!   (miner, storage, analysis station) on the generated stone deposit → wire them
//!   with cables → mine stone → analyse stone into research points → unlock the
//!   first research node with those points.
//!
//! World generation runs through the real `WorldgenPlugin` and the real
//! `DepositRegistry` loaded from `assets/deposits/`, so the origin chunk's stone
//! deposit is placed deterministically — the run is reproducible for a fixed seed.
//! Machines are "placed" by emitting the same `WorldObjectEvent::Placed` the input
//! layer produces, so `place_machine_system` (port spawning, miner→deposit latching)
//! runs for real. GLTF-derived port layouts are stubbed (headless has no renderer),
//! and storage provisioning is injected — both are data, not the logic under test.

use std::time::Duration;

use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldAsset;

use exergon::GameState;
use exergon::content::ContentPlugin;
use exergon::logistics::{LogisticsNetworkMember, LogisticsSimPlugin, StorageUnit};
use exergon::machine::{
    Machine, MachinePlugin, MachinePortLayout, MachinePortLayouts, MachineState, MinerMachine,
};
use exergon::recipe_graph::RecipeGraphPlugin;
use exergon::research::{ResearchPlugin, ResearchPool, TechTreeProgress, UnlockNodeRequest};
use exergon::seed::DomainSeeds;
use exergon::tech_tree::TechTreePlugin;
use exergon::world::{
    CableConnectionEvent, MainCamera, OreDeposit, WorldObjectEvent, WorldObjectKind, WorldgenPlugin,
};

/// Fixed master seed for this run — makes terrain + deposit placement reproducible.
const MASTER_SEED: u64 = 0xE7E6_0007;
const PORT_OFFSET: Vec3 = Vec3::new(1.0, 0.0, 0.0);

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        StatesPlugin,
        ScenePlugin,
    ));
    // Asset stores the machine visual/port-layout startup systems expect (no renderer here).
    app.init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Gltf>()
        .init_asset::<GltfMesh>()
        .init_asset::<GltfNode>()
        .init_asset::<WorldAsset>();
    app.add_message::<WorldObjectEvent>()
        .add_message::<CableConnectionEvent>()
        .init_state::<GameState>()
        .add_plugins((
            ContentPlugin,
            WorldgenPlugin,
            RecipeGraphPlugin,
            TechTreePlugin,
            MachinePlugin,
            LogisticsSimPlugin,
            ResearchPlugin,
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

/// Finds the ore deposit generated for the origin chunk, which the `DepositRegistry`
/// guarantees is a stone-bearing starter deposit regardless of seed.
fn origin_deposit(app: &mut App) -> (Entity, Transform, Vec<(String, f32)>) {
    let mut q = app.world_mut().query::<(Entity, &Transform, &OreDeposit)>();
    q.iter(app.world())
        .find(|(_, _, d)| d.chunk_pos == IVec2::ZERO)
        .map(|(e, t, d)| (e, *t, d.ores.clone()))
        .expect("world generation must place a deposit on the origin chunk")
}

/// Advances simulated time deterministically until `done` holds, in fixed `dt` steps.
///
/// `TimeUpdateStrategy::ManualDuration` makes every `app.update()` advance the clock by
/// exactly `dt`, independent of wall-clock, so the rate-integrating systems (mining,
/// recipe progress, power, …) actually progress. Panics if `done` is not satisfied within
/// `max_secs` of simulated time — a built-in runaway guard. `done` is polled before each
/// step, so it can also observe transient state (e.g. a machine being mid-recipe).
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

#[test]
fn land_generate_place_wire_mine_and_complete_first_research() {
    let mut app = build_app();

    // Provide a run seed the way the real game does: a Run entity carrying the
    // per-domain seeds derived from the master seed. `setup_world_config` reads it.
    app.world_mut()
        .spawn((exergon::save::Run, DomainSeeds::from_master(MASTER_SEED)));

    // The world generator spawns terrain chunks around the camera; place it at origin.
    app.world_mut().spawn((Transform::default(), MainCamera));

    // Startup: load content (deposit/recipe/machine registries).
    app.update();

    // Land: enter Loading so `setup_world_config` activates generation, then let the
    // chunk + deposit systems settle (deposits spawn the frame after their chunk).
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Loading);
    for _ in 0..4 {
        app.update();
    }

    // World generation must have produced the deterministic origin stone deposit.
    let (deposit_e, deposit_tf, ores) = origin_deposit(&mut app);
    let deposit_pos = deposit_tf.translation;
    assert_eq!(
        (deposit_pos.x, deposit_pos.z),
        (32.0, 32.0),
        "origin chunk deposit sits at the chunk centre — reproducible for a fixed seed"
    );
    assert!(
        ores.iter().any(|(id, _)| id == "stone"),
        "origin deposit must yield stone to bootstrap research, got {ores:?}"
    );

    // basic_analysis auto-unlocks via science_basics on a real run; ensure it here too.
    app.world_mut()
        .resource_mut::<TechTreeProgress>()
        .unlocked_recipes
        .insert("basic_analysis".to_string());

    // Stub port layouts for the machines we place: one logistics port each at PORT_OFFSET.
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

    // Enter Playing so placement (gated on GameState::Playing) is live.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    // Place the three starting-kit machines via the real placement path. The miner
    // goes onto the generated stone deposit; storage and station sit alongside it.
    let miner_pos = deposit_pos;
    let storage_pos = deposit_pos + Vec3::new(4.0, 0.0, 0.0);
    let station_pos = deposit_pos + Vec3::new(8.0, 0.0, 0.0);
    place(&mut app, "storage_crate", storage_pos);
    place(&mut app, "miner", miner_pos);
    place(&mut app, "analysis_station", station_pos);
    app.update();

    // The placed miner must have latched onto the generated deposit.
    let miner_e = machine_entity(&mut app, "miner");
    assert_eq!(
        app.world().get::<MinerMachine>(miner_e).map(|m| m.deposit),
        Some(deposit_e),
        "placed miner should latch onto the generated origin deposit"
    );

    // Storage machines don't carry a StorageUnit from placement; provision it.
    let storage_e = machine_entity(&mut app, "storage_crate");
    app.world_mut().entity_mut(storage_e).insert(StorageUnit {
        items: Default::default(),
    });

    // Wire everything onto one logistics network: storage↔miner and storage↔station.
    connect(&mut app, storage_pos + PORT_OFFSET, miner_pos + PORT_OFFSET);
    connect(
        &mut app,
        storage_pos + PORT_OFFSET,
        station_pos + PORT_OFFSET,
    );
    app.update();

    // All three machines' ports must share a network.
    let net_of = |app: &mut App, machine: Entity| -> Entity {
        let ports: Vec<Entity> = {
            let mut q = app
                .world_mut()
                .query::<(Entity, &exergon::machine::LogisticsPortOf)>();
            q.iter(app.world())
                .filter(|(_, p)| p.0 == machine)
                .map(|(e, _)| e)
                .collect()
        };
        ports
            .iter()
            .find_map(|&p| app.world().get::<LogisticsNetworkMember>(p).map(|m| m.0))
            .expect("machine port should have joined a network")
    };
    let station_e = machine_entity(&mut app, "analysis_station");
    let net = net_of(&mut app, storage_e);
    assert_eq!(
        net,
        net_of(&mut app, miner_e),
        "miner shares storage network"
    );
    assert_eq!(
        net,
        net_of(&mut app, station_e),
        "station shares storage network"
    );

    // Stage 1 — bootstrap research. Drive the mine→analyse loop under real simulated time
    // until there is enough research to afford the first ResearchSpend node (ore_extraction,
    // cost 30). No hand-poking of internal state — the rate-integrating systems do the work:
    //   * miner_tick_system    : accumulator += yield * dt  → emits ore + NetworkStorageChanged
    //   * recipe_advance_system : progress    += dt         → completes the analysis recipe
    // The miner's per-ore NetworkStorageChanged is what starts the analysis recipe once 4
    // stone accrue, so the loop self-sustains through the 3 completions (10 pts each) needed
    // to clear the 30-pt bar. basic_analysis has energy_cost 0, so no power wiring.
    let mut station_ran = false;
    advance_until(&mut app, 0.5, 1_000.0, |app| {
        if app.world().get::<MachineState>(station_e).copied() == Some(MachineState::Running) {
            station_ran = true;
        }
        research_points(app) >= 30.0
    });
    assert!(
        station_ran,
        "station must actually run the analysis recipe (not just be granted points)"
    );

    // Complete the first research: spend accumulated points to unlock ore_extraction.
    let points_before = research_points(&app);
    app.world_mut()
        .write_message(UnlockNodeRequest("ore_extraction".into()));
    app.update();

    let progress = app.world().resource::<TechTreeProgress>();
    assert!(
        progress.unlocked_nodes.contains("ore_extraction"),
        "first research node should be unlocked after spending research points"
    );
    assert!(
        research_points(&app) < points_before,
        "unlocking a ResearchSpend node must deduct research points"
    );

    // Stage 2 — sustained grind + a second, sequential node unlock. This is the scaling
    // proof for a full landing→victory test: the exact same time-driven loop must keep the
    // factory producing research over a much longer haul (basic_processing costs 150 → ~15
    // more analysis completions off the same deposit, whose yield decays toward its floor)
    // and chain into the next tier. basic_smelting is basic_processing's only prerequisite;
    // inject it the way the test already injects unlocked content (it is gated on upstream
    // nodes not exercised here — the mechanic under test is the research loop, not gating).
    app.world_mut()
        .resource_mut::<TechTreeProgress>()
        .unlocked_nodes
        .insert("basic_smelting".to_string());

    advance_until(&mut app, 0.5, 4_000.0, |app| research_points(app) >= 150.0);

    let points_before = research_points(&app);
    app.world_mut()
        .write_message(UnlockNodeRequest("basic_processing".into()));
    app.update();

    let progress = app.world().resource::<TechTreeProgress>();
    assert!(
        progress.unlocked_nodes.contains("basic_processing"),
        "second-tier node should unlock after a sustained grind to 150 research points"
    );
    assert_eq!(
        research_points(&app),
        points_before - 150.0,
        "unlocking basic_processing must deduct its 150-point cost"
    );
}

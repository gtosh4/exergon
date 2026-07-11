//! End-to-end integration test for a full Standard run — landing all the way to launching
//! the successor vehicle — exercised through the actual world-generation + placement +
//! logistics + recipe + research + power systems from a fixed seed (not hand-built
//! entities), driven on simulated time (see `advance_until`). Each stage below is a
//! milestone on the landing→victory path; new stages are appended as the systems they need
//! land (see `docs/technical/testing.md §3`).
//!
//!   Stage 0: fixed seed → generate terrain + surface ore deposits → place miner, storage,
//!            analysis station on the generated deposit → wire them → mine → analyse.
//!   Stage 1: accumulate research and unlock the first node (ore_extraction).
//!   Stage 1b: planet reveal loop — the first research spend reveals both atmospheric
//!            properties through the real property_reveal_system.
//!   Stage 2: sustained grind → unlock a second, higher-tier node (basic_processing).
//!   Stage 3: bring power online — a solar generator charges its buffer over time and a
//!            wired smelter draws that energy to smelt mined iron_ore → iron_ingot.
//!   Stage 4: craft through the NetworkCraftQueue — an assembler crafts a power_cell under
//!            power from queued ingot inputs.
//!   Stage 5: drone scan (fog reveal in DronePilot) reveals geological activity.
//!   Stage 6: the full Standard victory — mine EVERY raw material from real surface deposits
//!            (iron/copper/stone, aluminum, titanium, coal, fluxite/resonite/cryophase
//!            shards) and drive the real recipe/power systems through the whole successor
//!            tree — smelting/crushing/washing/wire-drawing the raw ore into ingots/plates/
//!            wire/dust, refining, sub-assemblies, the five successor components, 20
//!            exotic_fuel, and the 180s launch_successor on a launch_site — until completion
//!            fires EscapeEvent / RunState::Completed, then read the accumulated simulated
//!            Time as the "virtual time to complete the Standard run". Nothing refined is
//!            injected: every ingot, shard, plate, and intermediate is produced by a real
//!            machine from real mined ore.
//!
//! World generation runs through the real `WorldgenPlugin` and the real `DepositRegistry`
//! loaded from `assets/deposits/`, so the origin chunk's deposit is placed deterministically
//! — the run is reproducible for a fixed seed. Machines are "placed" by emitting the same
//! `WorldObjectEvent::Placed` the input layer produces, so `place_machine_system` (port
//! spawning, miner→deposit latching) runs for real; miners for the off-origin ores are put
//! onto the real worldgen-spawned `OreDeposit` entities located by `nearest_vein`. GLTF-
//! derived port layouts are stubbed (headless has no renderer), and storage / generator
//! provisioning, tech-tree unlocks, and exotic-deposit discovery events are injected — data
//! and gating, not the logic under test.

use std::time::Duration;

use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldAsset;

use exergon::content::ContentPlugin;
use exergon::drone::FogCellRevealedEvent;
use exergon::escape::{EscapeObjective, EscapePlugin, RunState};
use exergon::logistics::{
    LogisticsNetworkMember, LogisticsSimPlugin, ManualCraftTrigger, NetworkCraftQueue, QueuedJob,
    StorageUnit,
};
use exergon::machine::{
    EnvSource, Machine, MachinePlugin, MachinePortLayout, MachinePortLayouts, MachineState,
    ManualCraftOnly, MinerMachine,
};
use exergon::planet::{Planet, PlanetPlugin, PlanetPropertyVisibility, PropertyVisibility};
use exergon::power::{GeneratorUnit, PowerPlugin};
use exergon::recipe_graph::{RecipeGraph, RecipeGraphPlugin};
use exergon::research::{
    DiscoveryEvent, ResearchPlugin, ResearchPool, TechTreeProgress, UnlockNodeRequest,
};
use exergon::seed::DomainSeeds;
use exergon::tech_tree::TechTreePlugin;
use exergon::world::{
    CableConnectionEvent, MainCamera, OreDeposit, WorldObjectEvent, WorldObjectKind, WorldgenPlugin,
};
use exergon::{GameState, PlayMode};

/// Fixed master seed for this run — makes terrain + deposit placement reproducible.
const MASTER_SEED: u64 = 0xE7E6_0007;
const PORT_OFFSET: Vec3 = Vec3::new(1.0, 0.0, 0.0);
/// Energy ports sit on the opposite side from logistics ports so a power cable and a
/// logistics cable to the same machine snap to different port entities.
const ENERGY_OFFSET: Vec3 = Vec3::new(-1.0, 0.0, 0.0);

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
        // Fog reveal events come from DronePlugin in the real game; it is not in this
        // headless test (avian physics), so register the message the reveal system reads.
        .add_message::<FogCellRevealedEvent>()
        // EscapePlugin writes RunEndEvent (normally registered by SavePlugin, which we omit
        // here to avoid its persistence deps); register the message it needs directly.
        .add_message::<exergon::save::RunEndEvent>()
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
            EscapePlugin,
        ));
    app
}

/// The generated planet's per-property visibility (the reveal system mutates this in place).
fn planet_vis(app: &mut App) -> PlanetPropertyVisibility {
    let mut q = app
        .world_mut()
        .query_filtered::<&PlanetPropertyVisibility, With<Planet>>();
    q.single(app.world())
        .cloned()
        .expect("generate_planet_properties must have spawned the run's planet")
}

fn place(app: &mut App, item_id: &str, pos: Vec3) {
    app.world_mut().write_message(WorldObjectEvent {
        transform: Transform::from_translation(pos),
        item_id: item_id.to_string(),
        kind: WorldObjectKind::Placed,
    });
}

fn connect(app: &mut App, from: Vec3, to: Vec3) {
    cable(app, "logistics_cable", from, to);
}

fn connect_power(app: &mut App, from: Vec3, to: Vec3) {
    cable(app, "power_cable", from, to);
}

fn cable(app: &mut App, item_id: &str, from: Vec3, to: Vec3) {
    app.world_mut().write_message(CableConnectionEvent {
        from,
        to,
        item_id: item_id.to_string(),
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
    app.world().resource::<ResearchPool>().get("material")
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

/// Advances simulated time until a crafting/mining target is reached, then returns. A thin
/// wrapper over `advance_until` that reads clearer at call sites sequencing the mine→smelt→
/// form→assemble chain: "run the factory until `ready` holds". `ready` is any predicate over
/// the app — typically "≥ N of an item sits in network storage" via `stored()`.
fn wait_for_recipe(app: &mut App, dt: f32, max_secs: f32, ready: impl FnMut(&App) -> bool) {
    advance_until(app, dt, max_secs, ready);
}

/// How many of `item` sit in a machine's `StorageUnit`. Used to poll mining/craft progress.
fn stored(app: &App, storage: Entity, item: &str) -> u32 {
    app.world()
        .get::<StorageUnit>(storage)
        .and_then(|s| s.items.get(item).copied())
        .unwrap_or(0)
}

/// Locates the nearest surface deposit that yields `ore_id` among the deposits the real
/// world generator has actually spawned around the camera, returning `(entity, world pos)`
/// of the one closest to origin. Nothing is hand-spawned — this only reads the real
/// `OreDeposit` entities `spawn_deposit_markers` produced for the loaded chunks (an 81-chunk
/// area at origin, SPAWN_DIST=4, which for this seed contains every ore type). `skip_origin`
/// excludes the (0,0) starter deposit so a caller can mine a *fresh*, un-depleted vein of the
/// same ore (Stage 0 mines the origin deposit down over the research grind).
fn nearest_vein(app: &mut App, ore_id: &str, skip_origin: bool) -> (Entity, Vec3) {
    let mut q = app.world_mut().query::<(Entity, &Transform, &OreDeposit)>();
    q.iter(app.world())
        .filter(|(_, _, d)| !(skip_origin && d.chunk_pos == IVec2::ZERO))
        .filter(|(_, _, d)| d.ores.iter().any(|(id, _)| id == ore_id))
        .map(|(e, t, _)| (e, t.translation))
        .min_by(|(_, a), (_, b)| {
            a.length_squared()
                .partial_cmp(&b.length_squared())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or_else(|| panic!("world generation produced no loaded deposit yielding {ore_id}"))
}

/// Stands up a real mining operation for `ore_id`: finds a worldgen-spawned deposit via
/// `nearest_vein` and drops `miners` miner machines onto it — each latching to the deposit
/// through the real `place_machine_system` path — then wires every miner's logistics port
/// onto the shared network at `storage_pos`. Returns the deposit entity so the caller can
/// assert extraction. Multiple miners multiply throughput on the scarce ores (copper is only
/// ~23% of an iron_copper deposit; cryophase feeds all 20 exotic_fuel, 60 shards).
fn mine_deposit(
    app: &mut App,
    ore_id: &str,
    storage_pos: Vec3,
    miners: usize,
    skip_origin: bool,
) -> Entity {
    let (deposit_e, pos) = nearest_vein(app, ore_id, skip_origin);
    // Place the miners (offset so their logistics ports are distinct, all within the 8.0
    // deposit-latch range) — the real placement path latches each onto the deposit.
    for i in 0..miners {
        place(app, "miner", pos + Vec3::new(i as f32 * 2.0, 0.0, 0.0));
    }
    app.update();
    // Wire each miner onto the shared logistics network at storage_pos.
    for i in 0..miners {
        connect(
            app,
            storage_pos + PORT_OFFSET,
            pos + Vec3::new(i as f32 * 2.0, 0.0, 0.0) + PORT_OFFSET,
        );
    }
    app.update();
    deposit_e
}

#[test]
fn standard_run_lands_mines_and_launches_successor() {
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

    // Stub port layouts for the machines we place. Logistics-only machines get one
    // logistics port at PORT_OFFSET; the smelter also needs an energy port to draw power;
    // the solar generator is energy-only.
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
        layouts.by_machine.insert(
            "smelter".to_string(),
            MachinePortLayout {
                energy: vec![ENERGY_OFFSET],
                logistics: vec![PORT_OFFSET],
            },
        );
        layouts.by_machine.insert(
            "solar_generator".to_string(),
            MachinePortLayout {
                energy: vec![ENERGY_OFFSET],
                logistics: vec![],
            },
        );
        layouts.by_machine.insert(
            "assembler".to_string(),
            MachinePortLayout {
                energy: vec![ENERGY_OFFSET],
                logistics: vec![PORT_OFFSET],
            },
        );
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

    // Stage 1b — planet property reveal (research-spend trigger). The real
    // `property_reveal_system` watches the same `TechNodeUnlocked { via_research }` the
    // unlock above emits — the atmospheric-sample-analysis proxy — and advances both
    // atmospheric properties Hidden→Revealed. Geological activity stays Hidden until a
    // drone scan (Stage 5). One more update lets the reveal system observe the event.
    app.update();
    let vis = planet_vis(&mut app);
    assert_eq!(
        vis.atmospheric_oxygen,
        PropertyVisibility::Revealed,
        "first research spend must reveal atmospheric oxygen through property_reveal_system"
    );
    assert_eq!(
        vis.atmospheric_pressure,
        PropertyVisibility::Revealed,
        "first research spend must reveal atmospheric pressure"
    );
    assert_eq!(
        vis.geological_activity,
        PropertyVisibility::Hidden,
        "geological activity stays hidden until a drone scan"
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

    // Stage 3 — power comes online and drives an energy-gated recipe. Everything past basic
    // smelting costs energy, so this proves the whole power path under simulated time: a
    // solar generator charges its buffer passively (buffer += watts*dt), and a smelter wired
    // to it on a power network draws that energy per tick to smelt mined iron_ore → iron_ingot.
    // basic_smelting (unlocked in stage 2) grants the smelt template; unlock the iron variant.
    app.world_mut()
        .resource_mut::<TechTreeProgress>()
        .unlocked_recipes
        .insert("smelt_metal__iron".to_string());

    // Place a smelter (logistics + energy ports) and a solar generator (energy-only).
    let smelter_pos = deposit_pos + Vec3::new(12.0, 0.0, 0.0);
    let generator_pos = deposit_pos + Vec3::new(16.0, 0.0, 0.0);
    place(&mut app, "smelter", smelter_pos);
    place(&mut app, "solar_generator", generator_pos);
    app.update();

    // Placement spawns the Machine + ports but not the generator runtime component in this
    // headless setup; provision GeneratorUnit like StorageUnit. Solar charges via generator_tick.
    let generator_e = machine_entity(&mut app, "solar_generator");
    app.world_mut()
        .entity_mut(generator_e)
        .insert(GeneratorUnit {
            pos: generator_pos,
            watts: 500.0,
            buffer_joules: 0.0,
            max_buffer_joules: 10_000.0,
            env_source: EnvSource::Solar,
        });

    // Wire the smelter onto the existing logistics network (pull iron_ore, push iron_ingot)
    // and onto a power network shared with the generator.
    connect(
        &mut app,
        storage_pos + PORT_OFFSET,
        smelter_pos + PORT_OFFSET,
    );
    connect_power(
        &mut app,
        generator_pos + ENERGY_OFFSET,
        smelter_pos + ENERGY_OFFSET,
    );
    app.update();

    let smelter_e = machine_entity(&mut app, "smelter");
    let iron_ingot = |app: &App| -> u32 {
        app.world()
            .get::<StorageUnit>(storage_e)
            .and_then(|s| s.items.get("iron_ingot").copied())
            .unwrap_or(0)
    };

    // Advance until the smelter deposits its first iron_ingot. Completes only if the miner
    // supplied iron_ore, the generator charged buffer, and the smelter drew energy per tick.
    let mut smelter_ran = false;
    advance_until(&mut app, 0.25, 2_000.0, |app| {
        if app.world().get::<MachineState>(smelter_e).copied() == Some(MachineState::Running) {
            smelter_ran = true;
        }
        iron_ingot(app) >= 1
    });
    assert!(
        smelter_ran,
        "smelter must actually run the energy-gated smelt recipe"
    );
    assert!(
        app.world()
            .get::<GeneratorUnit>(generator_e)
            .is_some_and(|g| g.buffer_joules > 0.0),
        "solar generator must have charged its buffer over simulated time"
    );
    assert!(
        iron_ingot(&app) >= 1,
        "smelter must have smelted mined iron_ore into iron_ingot"
    );

    // Stage 4 — assembler crafting through the NetworkCraftQueue under power. This is the
    // craft-queue path the real game uses for escape components: enqueue a target item and
    // recipe_check dispatches its job onto a matching idle machine, drawing energy per tick.
    // We craft a power_cell (2 iron_ingot + 1 copper_ingot, assembler tier 2, 60 energy).
    // The ingot inputs are injected — smelting is proven in stage 3, so re-deriving copper
    // here adds wiring, not coverage. The xalite→resonite→gateway_key→escape branch past
    // this is blocked on unauthored content (exotic-form processing + coal); see tasks/milestones.
    app.world_mut()
        .resource_mut::<TechTreeProgress>()
        .unlocked_recipes
        .insert("make_power_cell".to_string());

    // Place an assembler and force it to tier 2 (make_power_cell is a tier-2 recipe).
    let assembler_pos = deposit_pos + Vec3::new(20.0, 0.0, 0.0);
    place(&mut app, "assembler", assembler_pos);
    app.update();
    let assembler_e = machine_entity(&mut app, "assembler");
    app.world_mut()
        .get_mut::<Machine>(assembler_e)
        .unwrap()
        .tier = 2;

    // Wire the assembler onto the logistics network (pull ingots, push power_cell) and onto
    // the generator's power network.
    connect(
        &mut app,
        storage_pos + PORT_OFFSET,
        assembler_pos + PORT_OFFSET,
    );
    connect_power(
        &mut app,
        generator_pos + ENERGY_OFFSET,
        assembler_pos + ENERGY_OFFSET,
    );
    app.update();

    // Supply the ingot inputs (upstream smelting covered in stage 3), then enqueue the craft.
    {
        let mut storage = app.world_mut().get_mut::<StorageUnit>(storage_e).unwrap();
        *storage.items.entry("iron_ingot".into()).or_insert(0) += 2;
        *storage.items.entry("copper_ingot".into()).or_insert(0) += 1;
    }
    let craft_net = net_of(&mut app, storage_e);
    let storage_items = app
        .world()
        .get::<StorageUnit>(storage_e)
        .unwrap()
        .items
        .clone();
    app.world_mut()
        .resource_scope(|world, rg: Mut<RecipeGraph>| {
            world
                .get_mut::<NetworkCraftQueue>(craft_net)
                .expect("logistics network carries a craft queue")
                .enqueue_item("power_cell", 1, &rg, &storage_items);
        });
    app.world_mut().write_message(ManualCraftTrigger);

    // Advance until the assembler crafts the power_cell into network storage.
    let power_cell = |app: &App| -> u32 {
        app.world()
            .get::<StorageUnit>(storage_e)
            .and_then(|s| s.items.get("power_cell").copied())
            .unwrap_or(0)
    };
    let mut assembler_ran = false;
    advance_until(&mut app, 0.25, 2_000.0, |app| {
        if app.world().get::<MachineState>(assembler_e).copied() == Some(MachineState::Running) {
            assembler_ran = true;
        }
        power_cell(app) >= 1
    });
    assert!(
        assembler_ran,
        "assembler must actually run the queued make_power_cell recipe"
    );
    assert!(
        power_cell(&app) >= 1,
        "assembler must craft a power_cell from the queued job under power"
    );

    // Stage 5 — drone scan reveals geological activity. Entering DronePilot and revealing a
    // fog cell (the scout action) drives the real `property_reveal_system` to advance
    // geological_activity Hidden→Qualitative — the exploration-side reveal trigger.
    app.world_mut()
        .resource_mut::<NextState<PlayMode>>()
        .set(PlayMode::DronePilot);
    app.update();
    app.world_mut()
        .write_message(FogCellRevealedEvent { cell: IVec2::ZERO });
    app.update();

    assert_eq!(
        planet_vis(&mut app).geological_activity,
        PropertyVisibility::Qualitative,
        "a drone scan (fog reveal in DronePilot) must reveal geological activity"
    );

    // Stage 6 — the full Standard victory: MINE every raw material and craft the successor
    // from scratch, then launch it. This is the payoff — it mines from real surface deposits
    // and drives the real recipe/power systems through the whole successor tree to
    // `launch_successor` on a `launch_site`, whose completion fires `EscapeEvent` (via the
    // auto-tagged `EscapeObjective`) and sets `RunState::Completed`. We then read the
    // accumulated simulated `Time` as "virtual time to complete the run" — a number that is
    // now meaningful because it includes real mining and smelting, not just top-tier assembly.
    //
    // What is DRIVEN for real (this is what consumes the measured virtual time):
    //   * MINING every raw input from a real `DepositRegistry` deposit — iron/copper/stone
    //     from iron_copper, plus aluminum, titanium, coal, and the fluxite/resonite/cryophase
    //     shards — via miners latched onto deposits found by `nearest_vein` (Stage 0's origin
    //     miner covers the starter deposit; the rest get fresh off-origin veins).
    //   * REFINING that raw ore up the tree with real machines: smelting ore→ingot, crushing
    //     stone/aluminum, washing aluminum_crushed→dust, drawing copper wire, making circuit
    //     boards, refining silicon/fluxite_lattice/vitreite/exotic_fuel, rolling plates, the
    //     sub-assemblies, the five successor components, and the 180s launch — each a real
    //     queued job dispatched by `recipe_check_system` (which scans the whole queue for the
    //     first feasible job, so mining and crafting overlap) onto an idle machine, drawing
    //     energy per tick and integrating time in `recipe_advance_system`. Machines are
    //     `ManualCraftOnly` so *only* queued jobs run (no auto-craft noise) — deterministic.
    //
    // What is INJECTED (gating / provisioning, not the logic under measurement):
    //   * tech-node/recipe unlocks (same as earlier stages) + `DiscoveryEvent`s for the exotic
    //     keys (deposit discovery is hardcoded to xalite in worldgen, so fluxite/cryophase/
    //     derelict caches are surfaced via `DiscoveryEvent` the way real recon would). Nothing
    //     REFINED is injected — every ingot/shard/plate/wire/dust is machine-produced from ore.
    //   * generator sizing (as stage 3 provisions one) — cranked so power is genuinely drawn
    //     every tick but never the bottleneck, so the measured time is the mine+craft path.
    //
    // Content dependencies (from `assets path launch_successor` / `assets recipe <id>`):
    //   launch_successor = 1 successor_{core,chassis,drive,sensor} + 1 provisioning_module
    //   + 20 exotic_fuel, on launch_site (tier 2), 180s / 8000 energy. The full leaf→root job
    //   list and per-ore mining targets below are computed from the tree; if a recipe changes
    //   machine/inputs, update those — the milestone (RunState::Completed) stays fixed.

    // Exotic-material discovery gating (represents drone recon / precursor survey finds).
    for key in [
        "xalite_deposit",
        "fluxite_relic_cache",
        "cryophase_deposit",
        "derelict_ship",
    ] {
        app.world_mut()
            .write_message(DiscoveryEvent(key.to_string()));
    }
    app.update();

    // Unlock every recipe the driven tree executes (gating is not what's under test here).
    // Includes the raw-refining leaves now driven for real: the non-iron smelts, the ore
    // crushers, the aluminum washer, copper wire drawing, and circuit-board assembly.
    {
        let mut progress = app.world_mut().resource_mut::<TechTreeProgress>();
        for r in [
            // raw-material processing (mined ore → basic intermediates)
            "smelt_metal__copper",
            "smelt_metal__aluminum",
            "smelt_metal__titanium",
            "crush_stone",
            "crush_aluminum",
            "wash_aluminum",
            "draw_metal__copper",
            "make_circuit",
            // refining + forming up to the successor components
            "refine_silicon",
            "refine_fluxite",
            "synth_vitreite",
            "refine_exotic_fuel__raw",
            "roll_iron_plate",
            "roll_aluminum_plate",
            "roll_titanium_plate",
            "form_silicon_chip",
            "make_resonite_circuit",
            "form_resonite_lattice",
            "make_fluxite_coil",
            "make_power_cell",
            "make_miner_kit",
            "make_generator_kit",
            "make_assembler_kit",
            "make_successor_core",
            "make_successor_chassis",
            "make_successor_drive",
            "make_successor_sensor",
            "make_provisioning_module",
            "launch_successor",
        ] {
            progress.unlocked_recipes.insert(r.to_string());
        }
    }

    // Stub port layouts for the new machine types (logistics + energy ports). Adds the raw-
    // processing machines (crusher/washer/wire_drawer) alongside the assembly-tree machines.
    {
        let mut layouts = app.world_mut().resource_mut::<MachinePortLayouts>();
        for id in [
            "crusher",
            "washer",
            "wire_drawer",
            "refinery",
            "plate_roller",
            "advanced_assembler",
            "launch_site",
        ] {
            layouts.by_machine.insert(
                id.to_string(),
                MachinePortLayout {
                    energy: vec![ENERGY_OFFSET],
                    logistics: vec![PORT_OFFSET],
                },
            );
        }
    }

    // Place the machine types the tree needs (smelter + assembler tier-2 are reused from
    // stages 3–4). Placement auto-assigns each its max defined tier: crusher/washer/
    // wire_drawer/refinery/plate_roller=1, advanced_assembler/launch_site=2 — matching the
    // recipes' required tiers.
    let crusher_pos = deposit_pos + Vec3::new(24.0, 0.0, 0.0);
    let washer_pos = deposit_pos + Vec3::new(28.0, 0.0, 0.0);
    let drawer_pos = deposit_pos + Vec3::new(32.0, 0.0, 0.0);
    let refinery_pos = deposit_pos + Vec3::new(36.0, 0.0, 0.0);
    let roller_pos = deposit_pos + Vec3::new(40.0, 0.0, 0.0);
    let adv_asm_pos = deposit_pos + Vec3::new(44.0, 0.0, 0.0);
    let launch_pos = deposit_pos + Vec3::new(48.0, 0.0, 0.0);
    place(&mut app, "crusher", crusher_pos);
    place(&mut app, "washer", washer_pos);
    place(&mut app, "wire_drawer", drawer_pos);
    place(&mut app, "refinery", refinery_pos);
    place(&mut app, "plate_roller", roller_pos);
    place(&mut app, "advanced_assembler", adv_asm_pos);
    place(&mut app, "launch_site", launch_pos);
    app.update();
    // tag_escape_machines_system observes `Added<Machine>` the frame after the placement
    // command flushes — one more update lets it tag the launch_site.
    app.update();

    // The placed launch_site must have been auto-tagged as the escape objective.
    let launch_e = machine_entity(&mut app, "launch_site");
    assert!(
        app.world().get::<EscapeObjective>(launch_e).is_some(),
        "a placed launch_site must be tagged EscapeObjective so its recipe fires the win"
    );

    // Wire each new machine onto the shared logistics + power networks, and force all
    // crafting machines (incl. the stage-3/4 smelter + assembler) to ManualCraftOnly so the
    // queue is the sole scheduler — deterministic, no auto-craft consuming raw leaves.
    for pos in [
        crusher_pos,
        washer_pos,
        drawer_pos,
        refinery_pos,
        roller_pos,
        adv_asm_pos,
        launch_pos,
    ] {
        connect(&mut app, storage_pos + PORT_OFFSET, pos + PORT_OFFSET);
        connect_power(&mut app, generator_pos + ENERGY_OFFSET, pos + ENERGY_OFFSET);
    }
    app.update();
    for machine in [
        smelter_e,
        assembler_e,
        machine_entity(&mut app, "crusher"),
        machine_entity(&mut app, "washer"),
        machine_entity(&mut app, "wire_drawer"),
        machine_entity(&mut app, "refinery"),
        machine_entity(&mut app, "plate_roller"),
        machine_entity(&mut app, "advanced_assembler"),
        launch_e,
    ] {
        app.world_mut().entity_mut(machine).insert(ManualCraftOnly);
    }

    // Size the generator so 8000-energy launch + the whole tree never stalls on power (power
    // is still drawn every tick — this just makes crafting time, not charging, the bottleneck).
    {
        let mut generator = app
            .world_mut()
            .get_mut::<GeneratorUnit>(generator_e)
            .unwrap();
        generator.watts = 1.0e7;
        generator.buffer_joules = 1.0e8;
        generator.max_buffer_joules = 1.0e9;
    }

    // Mine every raw material for real. Each `mine_deposit` locates a fresh off-origin
    // worldgen deposit (`nearest_vein`) and drops miners onto it wired to the shared
    // network — the miners then feed raw ore into storage every tick for the whole run, so
    // the queue's smelt/crush/wash/draw jobs become feasible as ore arrives (mining and
    // crafting overlap). Miner counts scale with the tree's per-ore demand and each ore's
    // deposit weight (from `assets recipe`/deposit RON): iron_copper is the scarce-copper
    // source (~23% copper), and cryophase feeds all 20 exotic_fuel (60 shards).
    //   iron_copper → iron_ore(10) + copper_ore(18) + stone(8);  aluminum_ore(10),
    //   titanium_ore(2), coal(4), fluxite_shard(8), resonite_shard(4), cryophase_shard(60).
    // iron/copper/stone: mine a FRESH off-origin iron_copper vein (the origin deposit is
    // depleted from the Stage 1–2 research grind) — Stage 0's origin miner also keeps running.
    let iron_copper_deposit = mine_deposit(&mut app, "copper_ore", storage_pos, 4, true);
    mine_deposit(&mut app, "aluminum_ore", storage_pos, 2, false);
    mine_deposit(&mut app, "titanium_ore", storage_pos, 1, false);
    mine_deposit(&mut app, "coal", storage_pos, 1, false);
    mine_deposit(&mut app, "fluxite_shard", storage_pos, 2, false);
    mine_deposit(&mut app, "resonite_shard", storage_pos, 2, false);
    let cryophase_deposit = mine_deposit(&mut app, "cryophase_shard", storage_pos, 3, false);

    // Enqueue the full driven job list, leaves→root, starting from the mined ore. We push
    // explicit `QueuedJob`s (`launch_successor` has no output item so `enqueue_item` can't
    // resolve it); the order is only a hint — `recipe_check_system` scans the whole queue and
    // gates each job on real input feasibility, so leaf jobs fire as mined ore arrives and
    // upper jobs fire as their intermediates are produced. Multiplicities are the exact tree
    // demand (mass-balanced): e.g. 18 copper smelts → 17 copper_wire draws + 1 power_cell;
    // 8 crush_stone → 4 silicon; 60 cryophase (mined) → 20 exotic_fuel. See the per-recipe
    // arithmetic in `assets recipe <id>`.
    let craft_net = net_of(&mut app, storage_e);
    {
        let mut queue = app
            .world_mut()
            .get_mut::<NetworkCraftQueue>(craft_net)
            .expect("logistics network carries a craft queue");
        let mut push = |recipe: &str, n: usize| {
            for _ in 0..n {
                queue.jobs.push_back(QueuedJob {
                    recipe_id: recipe.to_string(),
                });
            }
        };
        // Raw ore → basic intermediates (smelt / crush / wash / draw / circuit).
        push("crush_stone", 8);
        push("crush_aluminum", 6);
        push("smelt_metal__iron", 10);
        push("smelt_metal__copper", 18);
        push("smelt_metal__aluminum", 4);
        push("smelt_metal__titanium", 2);
        push("draw_metal__copper", 17);
        push("wash_aluminum", 2);
        push("make_circuit", 4);
        // Refining + rolling (kept ahead of exotic_fuel so the single refinery works the
        // shorter jobs first, then churns the 20× exotic_fuel while cryophase is mined).
        push("refine_silicon", 4);
        push("refine_fluxite", 4);
        push("synth_vitreite", 2);
        push("roll_iron_plate", 2);
        push("roll_aluminum_plate", 2);
        push("roll_titanium_plate", 1);
        // Sub-assemblies.
        push("form_silicon_chip", 4);
        push("make_resonite_circuit", 2);
        push("form_resonite_lattice", 1);
        push("make_fluxite_coil", 2);
        push("make_power_cell", 1);
        push("make_miner_kit", 1);
        push("make_generator_kit", 1);
        push("make_assembler_kit", 1);
        // Exotic fuel (long serial pole) then the five components and the launch.
        push("refine_exotic_fuel__raw", 20);
        push("make_successor_core", 1);
        push("make_successor_sensor", 1);
        push("make_successor_chassis", 1);
        push("make_successor_drive", 1);
        push("make_provisioning_module", 1);
        push("launch_successor", 1);
    }

    // Checkpoint — prove the raw mine→smelt→draw chain runs for real before the long haul:
    // copper_wire can only appear if copper_ore was mined, smelted to copper_ingot, and drawn
    // into wire, all through real machines. `wait_for_recipe` sequences the factory to this
    // milestone off the same simulated clock.
    wait_for_recipe(&mut app, 0.5, 3_000.0, |app| {
        stored(app, storage_e, "copper_wire") >= 1
    });
    assert!(
        stored(&app, storage_e, "copper_wire") >= 1,
        "mined copper_ore must smelt→draw into copper_wire through real machines"
    );

    // Drive real time until the run completes. dt=0.5 is below the shortest recipe (4s crush)
    // so no recipe edge is skipped. The critical path is the single refinery's 20× exotic_fuel
    // (~600s) plus downstream assembly and the 180s launch — and now also the up-front mining
    // ramp — so 8000s is ample headroom.
    let launch_ran = std::cell::Cell::new(false);
    advance_until(&mut app, 0.5, 8_000.0, |app| {
        if app.world().get::<MachineState>(launch_e).copied() == Some(MachineState::Running) {
            launch_ran.set(true);
        }
        *app.world().resource::<RunState>() == RunState::Completed
    });

    let virtual_secs = app.world().resource::<Time>().elapsed_secs();
    let virtual_hours = virtual_secs / 3600.0;
    println!(
        "\n=== Standard run complete: virtual time to victory = {virtual_secs:.1}s ({virtual_hours:.2}h) ===\n"
    );

    assert!(
        launch_ran.get(),
        "launch_site must actually run the launch_successor recipe (not just be granted the win)"
    );
    // The victory must have been fed by REAL mining, not injected refined items. The
    // cryophase deposit alone supplies all 60 shards for the 20 exotic_fuel, and the fresh
    // iron_copper vein supplies the copper/iron: assert both were extracted from for real.
    let extracted = |app: &App, deposit: Entity| -> f32 {
        app.world()
            .get::<OreDeposit>(deposit)
            .map(|d| d.total_extracted)
            .unwrap_or(0.0)
    };
    assert!(
        extracted(&app, cryophase_deposit) >= 60.0,
        "cryophase deposit must have been mined for ≥60 shards (the 20 exotic_fuel), \
         got {}",
        extracted(&app, cryophase_deposit)
    );
    assert!(
        extracted(&app, iron_copper_deposit) >= 18.0,
        "fresh iron_copper vein must have been mined for real to feed the smelters, got {}",
        extracted(&app, iron_copper_deposit)
    );
    assert_eq!(
        *app.world().resource::<RunState>(),
        RunState::Completed,
        "completing launch_successor on the launch_site must set RunState::Completed \
         (measured virtual time to victory = {virtual_secs:.1}s / {virtual_hours:.2}h)"
    );
    // Sanity bound only (the number itself is unvalidated content balance): a real run's
    // crafting critical path is well under a simulated day but not trivially instant.
    assert!(
        (100.0..86_400.0).contains(&virtual_secs),
        "virtual time to victory {virtual_secs:.1}s outside the sane [100s, 24h) bound"
    );
}

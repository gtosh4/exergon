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
//!   Stage 4: bootstrap the engineering economy through the NetworkCraftQueue — mined
//!            copper_ore/iron_ore → smelt → draw wire → assemble a circuit_board, every step
//!            a real queued job on a ManualCraftOnly machine under power.
//!   Stage 5: drone scan (fog reveal in DronePilot) reveals geological activity.
//!   Stage 6: the full Standard victory with EARNED research — mine EVERY raw material from
//!            real surface deposits (iron/copper/stone, aluminum, titanium, coal, fluxite/
//!            resonite/cryophase shards), convert them into the four research currencies via
//!            real analysis recipes, and spend those through real UnlockNodeRequests to walk
//!            the whole launch_successor tech closure (25 ResearchSpend nodes + auto milestone/
//!            chain nodes + drone-recon discoveries). Then build the successor from scratch —
//!            smelting/crushing/washing/wire-drawing, refining, sub-assemblies, the five
//!            components, 20 exotic_fuel, the 180s launch_successor on a launch_site — until
//!            EscapeEvent / RunState::Completed. Nothing refined OR researched is injected:
//!            every ingot/shard/plate and every tech node is produced/earned by real systems.
//!
//! World generation runs through the real `WorldgenPlugin` and the real `DepositRegistry`
//! loaded from `assets/deposits/`, so the origin chunk's deposit is placed deterministically
//! — the run is reproducible for a fixed seed. Machines are "placed" by emitting the same
//! `WorldObjectEvent::Placed` the input layer produces, so `place_machine_system` (port
//! spawning, miner→deposit latching) runs for real; miners for the off-origin ores are put
//! onto the real worldgen-spawned `OreDeposit` entities located by `nearest_vein`.
//!
//! Research is EARNED, not injected: no tech node or recipe is inserted into
//! `TechTreeProgress` by hand. The four research currencies (material/engineering/discovery/
//! synthesis) are produced by real analysis recipes off mined + refined inputs; the
//! grind-driver loop spends them through real `UnlockNodeRequest`s (which `check_research_unlocks`
//! honors only when prereqs are met AND the pool can pay), and node effects cascade-unlock the
//! successor-tree recipes. PrerequisiteChain nodes (science_basics/basic_smelting/power_basics)
//! auto-unlock; ProductionMilestone nodes (ore_crusher/ore_washer/plate_roller) auto-unlock off
//! the real `ProductionTally`; ExplorationDiscovery nodes unlock from real drone recon. The only
//! headless "cheats" left are simulated-time fast-forward, instant machine placement via
//! `WorldObjectEvent`, `StorageUnit`/`GeneratorUnit` provisioning, `MachinePortLayout` stubs,
//! GLTF asset stubs, and the drone spawn for recon — provisioning, not the logic under test.

use std::time::Duration;

use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldAsset;

use exergon::content::ContentPlugin;
use exergon::drone::{Drone, FogCellRevealedEvent, deposit_discovery_system};
use exergon::escape::{EscapeObjective, EscapePlugin, RunState};
use exergon::logistics::{
    LogisticsNetworkMember, LogisticsSimPlugin, NetworkCraftQueue, QueuedJob, StorageUnit,
};
use exergon::machine::{
    EnvSource, Machine, MachinePlugin, MachinePortLayout, MachinePortLayouts, MachineState,
    ManualCraftOnly, MinerMachine,
};
use exergon::planet::{Planet, PlanetPlugin, PlanetPropertyVisibility, PropertyVisibility};
use exergon::power::{GeneratorUnit, PowerPlugin};
use exergon::recipe_graph::RecipeGraphPlugin;
use exergon::research::{
    Discovered, ProductionTally, ResearchPlugin, ResearchPool, TechTreeProgress, UnlockNodeRequest,
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
        ))
        // Real exotic-site recon: the full DronePlugin pulls avian physics we can't run
        // headless, so register just the discovery system it owns (gated on DronePilot, same as
        // the game) — a piloted drone near a special deposit fires the real DiscoveryEvent.
        .add_systems(
            Update,
            deposit_discovery_system.run_if(in_state(PlayMode::DronePilot)),
        );
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

/// Places one real `solar_generator` at `pos`, wires it onto the power network anchored at
/// `grid_pos`, and provisions its runtime `GeneratorUnit` at the RON's 100 W (headless
/// placement spawns the Machine + ports but not the generator component). Panels start with an
/// empty buffer and charge passively via `generator_tick` (buffer += watts·dt) — a farm of
/// these is how the run powers the successor tree for real, no hand-fed joules.
fn place_solar_panel(app: &mut App, pos: Vec3, grid_pos: Vec3) -> Entity {
    place(app, "solar_generator", pos);
    app.update();
    // place_machine_system already inserts a GeneratorUnit for generators, but its watts are
    // scaled by this world's solar modifier (unpredictable per seed). Find the panel just placed
    // at `pos` and pin it to a known 100 W / empty buffer so the farm's sizing is deterministic.
    let entity = {
        let mut q = app.world_mut().query::<(Entity, &Machine, &Transform)>();
        q.iter(app.world())
            .find(|(_, m, t)| {
                m.machine_type == "solar_generator" && t.translation.distance(pos) < 0.5
            })
            .map(|(e, _, _)| e)
            .expect("just-placed solar_generator at pos")
    };
    app.world_mut().entity_mut(entity).insert(GeneratorUnit {
        pos,
        watts: 100.0,
        buffer_joules: 0.0,
        max_buffer_joules: 10_000.0,
        env_source: EnvSource::Solar,
    });
    connect_power(app, grid_pos + ENERGY_OFFSET, pos + ENERGY_OFFSET);
    app.update();
    entity
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

/// Pilots the drone to the deposit carrying `signature_ore` and runs one frame so the real
/// `deposit_discovery_system` fires its `DiscoveryEvent`. Requires `PlayMode::DronePilot` and a
/// spawned `Drone` (reused across calls by moving its transform). Returns the deposit entity so
/// the caller can assert it was marked `Discovered`. NB: a deposit fires exactly once (then is
/// `Discovered` forever) and the research systems only honor the event in the frame it fires —
/// so a site must be reconned only once its tech node's prerequisites are already met.
fn recon_deposit(app: &mut App, signature_ore: &str) -> Entity {
    let (deposit_e, pos) = nearest_vein(app, signature_ore, false);
    let existing = {
        let mut q = app.world_mut().query_filtered::<Entity, With<Drone>>();
        q.iter(app.world()).next()
    };
    match existing {
        Some(e) => {
            app.world_mut()
                .entity_mut(e)
                .insert(Transform::from_translation(pos));
        }
        None => {
            app.world_mut()
                .spawn((Drone, Transform::from_translation(pos)));
        }
    }
    app.update();
    deposit_e
}

/// Places `count` machines of `machine_type` spread along -Z from `base`, wires each onto the
/// shared logistics network at `storage_pos` and the power network at `generator_pos`, marks
/// each `ManualCraftOnly` (so the `NetworkCraftQueue` is the sole scheduler), and returns their
/// entities. The port layout for `machine_type` must already be stubbed. This is how the Stage-6
/// research/build factory is stood up at scale — several stations of each type so the long grind
/// parallelises instead of serialising on one machine.
fn place_factory(
    app: &mut App,
    machine_type: &str,
    base: Vec3,
    count: usize,
    storage_pos: Vec3,
    generator_pos: Vec3,
) -> Vec<Entity> {
    let positions: Vec<Vec3> = (0..count)
        .map(|i| base + Vec3::new(0.0, 0.0, -3.0 * i as f32))
        .collect();
    for &p in &positions {
        place(app, machine_type, p);
    }
    app.update();
    for &p in &positions {
        connect(app, storage_pos + PORT_OFFSET, p + PORT_OFFSET);
        connect_power(app, generator_pos + ENERGY_OFFSET, p + ENERGY_OFFSET);
    }
    app.update();
    let mut ents = Vec::new();
    for &p in &positions {
        let e = {
            let mut q = app.world_mut().query::<(Entity, &Machine, &Transform)>();
            q.iter(app.world())
                .find(|(_, m, t)| m.machine_type == machine_type && t.translation.distance(p) < 0.5)
                .map(|(e, _, _)| e)
                .expect("just-placed factory machine")
        };
        app.world_mut().entity_mut(e).insert(ManualCraftOnly);
        ents.push(e);
    }
    ents
}

/// True once every node id in `nodes` is in `unlocked_nodes` — a theme's grind is "done".
fn nodes_unlocked(app: &App, nodes: &[&str]) -> bool {
    let prog = app.world().resource::<TechTreeProgress>();
    nodes.iter().all(|n| prog.unlocked_nodes.contains(*n))
}

fn recipe_unlocked(app: &App, recipe: &str) -> bool {
    app.world()
        .resource::<TechTreeProgress>()
        .unlocked_recipes
        .contains(recipe)
}

fn node_unlocked(app: &App, node: &str) -> bool {
    app.world()
        .resource::<TechTreeProgress>()
        .unlocked_nodes
        .contains(node)
}

fn produced(app: &App, item: &str) -> f32 {
    app.world().resource::<ProductionTally>().get(item)
}

/// Tops the queue up so it holds at least `target` jobs of `recipe` (counts current, pushes the
/// deficit). The grind-driver calls this each frame for the analysis chain so machines never
/// starve of candidate jobs; over-queuing is harmless (infeasible jobs wait, surplus points/
/// intermediates are reused by the build phase).
fn ensure_jobs(queue: &mut NetworkCraftQueue, recipe: &str, target: usize) {
    let have = queue.jobs.iter().filter(|j| j.recipe_id == recipe).count();
    for _ in have..target {
        queue.jobs.push_back(QueuedJob {
            recipe_id: recipe.to_string(),
        });
    }
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

    // basic_analysis is NOT injected — science_basics is a zero-prereq PrerequisiteChain node,
    // so `check_research_unlocks` auto-unlocks it (and analysis_station) the first Playing frame,
    // cascading its `basic_analysis` recipe. Likewise basic_smelting + power_basics auto-unlock,
    // so the smelt_metal template and solar_generator are available with no injection.

    // Stub port layouts for the machines we place. Logistics-only machines get one
    // logistics port at PORT_OFFSET; the smelter also needs an energy port to draw power;
    // the solar generator is energy-only.
    {
        let mut layouts = app.world_mut().resource_mut::<MachinePortLayouts>();
        for id in ["storage_crate", "miner"] {
            layouts.by_machine.insert(
                id.to_string(),
                MachinePortLayout {
                    energy: vec![],
                    logistics: vec![PORT_OFFSET],
                },
            );
        }
        // analysis_station needs an energy port too: basic_analysis is free (0E) but the higher
        // analyses — analyze_circuit(30E)/analyze_field_sample(25E)/analyze_exotic_reaction(60E) —
        // draw power, so without this the energy-gated analyses would never dispatch.
        layouts.by_machine.insert(
            "analysis_station".to_string(),
            MachinePortLayout {
                energy: vec![ENERGY_OFFSET],
                logistics: vec![PORT_OFFSET],
            },
        );
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
        for id in ["assembler", "wire_drawer"] {
            layouts.by_machine.insert(
                id.to_string(),
                MachinePortLayout {
                    energy: vec![ENERGY_OFFSET],
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
    // and chain into the next tier. basic_smelting is basic_processing's only prerequisite and
    // is a zero-prereq PrerequisiteChain node, so it already auto-unlocked the first Playing
    // frame — no injection needed; the request below simply spends material on basic_processing.
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
    // basic_smelting (auto-unlocked) grants the whole `smelt_metal` template, so every
    // smelt_metal__* variant — iron/copper/aluminum/titanium — is already craftable, no injection.
    assert!(
        app.world()
            .resource::<TechTreeProgress>()
            .unlocked_recipes
            .contains("smelt_metal__iron"),
        "basic_smelting's smelt_metal template must have auto-unlocked smelt_metal__iron"
    );

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

    // Stage 4 — the engineering economy bootstraps for real through the NetworkCraftQueue.
    // This proves the multi-step craft path the research grind (Stage 6) leans on: mined
    // copper_ore/iron_ore → smelt → draw wire → assemble a circuit_board, every step a real
    // queued job dispatched by recipe_check onto an idle ManualCraftOnly machine under power.
    // Nothing is injected: the origin deposit (iron_copper) already feeds copper_ore + iron_ore
    // into storage, and every recipe here is craftable from earned nodes —
    //   * smelt_metal__copper / smelt_metal__iron  ← basic_smelting template (auto)
    //   * draw_metal__copper, make_circuit         ← basic_processing (earned in Stage 2)
    // make_circuit = 1 iron_ingot + 2 copper_wire → 1 circuit_board (assembler tier 1, 15s).
    let drawer_pos = deposit_pos + Vec3::new(32.0, 0.0, 0.0);
    let assembler_pos = deposit_pos + Vec3::new(20.0, 0.0, 0.0);
    place(&mut app, "wire_drawer", drawer_pos);
    place(&mut app, "assembler", assembler_pos);
    app.update();
    let assembler_e = machine_entity(&mut app, "assembler");
    let drawer_e = machine_entity(&mut app, "wire_drawer");

    // Wire both onto the shared logistics + power networks.
    for pos in [drawer_pos, assembler_pos] {
        connect(&mut app, storage_pos + PORT_OFFSET, pos + PORT_OFFSET);
        connect_power(&mut app, generator_pos + ENERGY_OFFSET, pos + ENERGY_OFFSET);
    }
    app.update();

    // Queue is the sole scheduler from here on: make the smelter (Stage 3), wire_drawer, and
    // assembler ManualCraftOnly so they run only queued jobs — deterministic, no auto-craft
    // draining raw ore. (recipe_check still dispatches queued jobs onto ManualCraftOnly machines.)
    for machine in [smelter_e, drawer_e, assembler_e] {
        app.world_mut().entity_mut(machine).insert(ManualCraftOnly);
    }

    // Enqueue the real leaf→circuit chain. Producers before consumers is only a hint —
    // recipe_check gates every job on real input feasibility, so smelt fires as ore arrives,
    // draw fires as copper_ingot appears, make_circuit fires once wire + iron_ingot exist.
    let craft_net = net_of(&mut app, storage_e);
    {
        let mut queue = app
            .world_mut()
            .get_mut::<NetworkCraftQueue>(craft_net)
            .expect("logistics network carries a craft queue");
        for recipe in [
            "smelt_metal__copper",
            "smelt_metal__copper",
            "smelt_metal__iron",
        ] {
            queue.jobs.push_back(QueuedJob {
                recipe_id: recipe.to_string(),
            });
        }
        for _ in 0..2 {
            queue.jobs.push_back(QueuedJob {
                recipe_id: "draw_metal__copper".to_string(),
            });
        }
        queue.jobs.push_back(QueuedJob {
            recipe_id: "make_circuit".to_string(),
        });
    }

    // Advance until the assembler crafts a circuit_board into network storage.
    let circuit_board = |app: &App| -> u32 { stored(app, storage_e, "circuit_board") };
    let mut assembler_ran = false;
    advance_until(&mut app, 0.25, 3_000.0, |app| {
        if app.world().get::<MachineState>(assembler_e).copied() == Some(MachineState::Running) {
            assembler_ran = true;
        }
        circuit_board(app) >= 1
    });
    assert!(
        assembler_ran,
        "assembler must actually run the queued make_circuit recipe under power"
    );
    assert!(
        circuit_board(&app) >= 1,
        "the mined-ore → smelt → draw → assemble chain must yield a real circuit_board"
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

    // Stage 6 — the full Standard victory, research EARNED end-to-end. Mine every raw material,
    // convert it into the four research currencies through real analysis recipes, spend those
    // through real `UnlockNodeRequest`s to walk the entire `launch_successor` tech closure, then
    // build + launch the successor. Completion fires `EscapeEvent` (via the auto-tagged
    // `EscapeObjective`) → `RunState::Completed`, and we read simulated `Time` as the automated-
    // optimal "virtual time to victory".
    //
    // NOTHING is injected into `TechTreeProgress`. Every node unlocks by its real mechanism:
    //   * PrerequisiteChain (auto):    science_basics, basic_smelting, power_basics
    //   * ProductionMilestone (auto):  ore_crusher(100 iron_ingot), ore_washer(50 iron_crushed),
    //                                  plate_roller(150 iron_ingot) — off the real ProductionTally
    //   * ExplorationDiscovery (recon): exotic_materials(xalite, prereq free → reconned now),
    //                                  fluxite_studies(fluxite) + cryophase_extraction(cryophase),
    //                                  reconned inside the loop once their researched prereqs land
    //   * ResearchSpend (earned+spent): the 25 target nodes below, funded by the analysis economy:
    //       basic_analysis    4 stone         → 10 material   (8s, 0E)
    //       analyze_circuit   1 circuit_board → 20 engineering(10s, 30E)  [circuit = 1 iron_ingot
    //                                                                       + 2 copper_wire]
    //       analyze_field_sample 1 field_sample → 12 discovery(10s, 25E)  [mined @ xalite 10%]
    //       analyze_exotic_reaction 1 resonite_shard → 20 synthesis(12s, 60E) [xalite 20%]
    //   Node effects then cascade-unlock every successor-tree recipe the build phase runs.
    //
    // Only headless provisioning remains a "cheat": simulated-time fast-forward, instant
    // placement via `WorldObjectEvent`, `StorageUnit`/`GeneratorUnit`, `MachinePortLayout` stubs,
    // GLTF stubs, and the drone spawn for recon. Nothing refined/researched is injected.

    // Exotic-site discovery via REAL drone recon: still in DronePilot from Stage 5, pilot the
    // drone to the xalite deposit so the real deposit_discovery_system fires its DiscoveryEvent.
    // xalite→exotic_materials unlocks now (prereq science_basics is a free chain); the fluxite +
    // cryophase sites gate on researched prereqs and are reconned inside the grind loop below.
    let xalite_site = recon_deposit(&mut app, "xalite");
    assert!(
        app.world().get::<Discovered>(xalite_site).is_some(),
        "drone recon must mark the xalite deposit Discovered (real DiscoveryEvent fired)"
    );
    assert!(
        node_unlocked(&app, "exotic_materials"),
        "recon of the xalite site must unlock exotic_materials (prereq science_basics is free)"
    );

    // Stub port layouts for the remaining machine types (wire_drawer stubbed in Stage 0).
    {
        let mut layouts = app.world_mut().resource_mut::<MachinePortLayouts>();
        for id in [
            "crusher",
            "washer",
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

    // Stand up the research/build factory at scale. Placement auto-assigns each machine its max
    // tier (assembler/advanced_assembler/launch_site = 2, the rest = 1). Several stations of each
    // type so the long grind parallelises. The Stage 0 analysis_station + Stage 3 smelter + Stage 4
    // wire_drawer/assembler already exist; these are the additions. All are `ManualCraftOnly`, so
    // the single shared `NetworkCraftQueue` is the only scheduler.
    place_factory(
        &mut app,
        "analysis_station",
        deposit_pos + Vec3::new(8.0, 0.0, 0.0),
        7,
        storage_pos,
        generator_pos,
    );
    place_factory(
        &mut app,
        "smelter",
        deposit_pos + Vec3::new(12.0, 0.0, 0.0),
        3,
        storage_pos,
        generator_pos,
    );
    place_factory(
        &mut app,
        "wire_drawer",
        deposit_pos + Vec3::new(32.0, 0.0, 0.0),
        2,
        storage_pos,
        generator_pos,
    );
    place_factory(
        &mut app,
        "assembler",
        deposit_pos + Vec3::new(20.0, 0.0, 0.0),
        2,
        storage_pos,
        generator_pos,
    );
    place_factory(
        &mut app,
        "crusher",
        deposit_pos + Vec3::new(24.0, 0.0, 0.0),
        2,
        storage_pos,
        generator_pos,
    );
    place_factory(
        &mut app,
        "washer",
        deposit_pos + Vec3::new(28.0, 0.0, 0.0),
        2,
        storage_pos,
        generator_pos,
    );
    place_factory(
        &mut app,
        "refinery",
        deposit_pos + Vec3::new(36.0, 0.0, 0.0),
        2,
        storage_pos,
        generator_pos,
    );
    place_factory(
        &mut app,
        "plate_roller",
        deposit_pos + Vec3::new(40.0, 0.0, 0.0),
        1,
        storage_pos,
        generator_pos,
    );
    place_factory(
        &mut app,
        "advanced_assembler",
        deposit_pos + Vec3::new(44.0, 0.0, 0.0),
        2,
        storage_pos,
        generator_pos,
    );
    let launch_e = place_factory(
        &mut app,
        "launch_site",
        deposit_pos + Vec3::new(48.0, 0.0, 0.0),
        1,
        storage_pos,
        generator_pos,
    )[0];
    // tag_escape_machines_system observes `Added<Machine>`; place_factory's updates let it tag.
    app.update();
    assert!(
        app.world().get::<EscapeObjective>(launch_e).is_some(),
        "a placed launch_site must be tagged EscapeObjective so its recipe fires the win"
    );

    // A real solar farm powers it — no hand-fed joules. Sixteen 100 W panels (1600 W) share the
    // Stage-3 generator's power network, above the whole factory's peak draw; panels charge via
    // generator_tick, and we let them build a reserve before the heavy phase (the ramp a player
    // waits through). Buffer is drawn every tick — if draw outpaces generation crafts just wait.
    let solar_farm_anchor = deposit_pos + Vec3::new(0.0, 0.0, 12.0);
    let farm: Vec<Entity> = (0..16)
        .map(|i| {
            place_solar_panel(
                &mut app,
                solar_farm_anchor + Vec3::new(0.0, 0.0, i as f32 * 3.0),
                generator_pos,
            )
        })
        .collect();
    advance_until(&mut app, 1.0, 400.0, |app| {
        farm.iter().all(|&e| {
            app.world()
                .get::<GeneratorUnit>(e)
                .is_some_and(|g| g.buffer_joules >= g.max_buffer_joules * 0.9)
        })
    });

    // Mine every raw material for real. Each `mine_deposit` latches miners onto a worldgen
    // deposit found by `nearest_vein`; they feed ore into shared storage every tick for the whole
    // run so smelt/crush/analyze jobs become feasible as ore arrives. The iron_copper vein feeds
    // the research economy heavily (stone→material, iron→circuits+milestones, copper→wire) so it
    // gets the most miners; xalite yields resonite_shard(20%)+field_sample(10%) for synthesis +
    // discovery; cryophase feeds all 20 exotic_fuel (60 shards). Stage 0's origin miner (also
    // iron_copper) keeps running alongside.
    let iron_copper_deposit = mine_deposit(&mut app, "copper_ore", storage_pos, 8, true);
    mine_deposit(&mut app, "resonite_shard", storage_pos, 10, false);
    mine_deposit(&mut app, "aluminum_ore", storage_pos, 3, false);
    mine_deposit(&mut app, "titanium_ore", storage_pos, 2, false);
    mine_deposit(&mut app, "coal", storage_pos, 1, false);
    let fluxite_site = mine_deposit(&mut app, "fluxite_shard", storage_pos, 2, false);
    let cryophase_deposit = mine_deposit(&mut app, "cryophase_shard", storage_pos, 4, false);

    // The ResearchSpend target nodes, by theme. The grind-driver spams `UnlockNodeRequest` for
    // every one each frame — each is a no-op until its prereqs are met AND its theme's pool can
    // pay, so they self-order by the real tech graph. (Total costs: material 760, engineering
    // 1800, discovery 1070, synthesis 2570.)
    let material_nodes = [
        "ore_extraction",
        "drone_recon",
        "basic_processing",
        "silicon_refining",
        "aluminum_extraction",
        "titanium_forming",
    ];
    let engineering_nodes = [
        "advanced_processing",
        "resonite_engineering",
        "advanced_assembler",
        "fluxite_coil",
        "provisioning_module",
    ];
    let discovery_nodes = [
        "exotic_processing",
        "precursor_survey",
        "synthesis_lab",
        "space_scanner",
        "cryophase_prospecting",
    ];
    let synthesis_nodes = [
        "coolant_reclaim",
        "vitreite_synthesis",
        "exotic_fuel_refining",
        "successor_core",
        "successor_chassis",
        "successor_drive",
        "successor_sensor",
        "launch_site_assembly",
        "launch_successor",
    ];
    let all_nodes: Vec<&str> = material_nodes
        .iter()
        .chain(&engineering_nodes)
        .chain(&discovery_nodes)
        .chain(&synthesis_nodes)
        .copied()
        .collect();

    // The build-phase job list (mass-balanced from the successor tree; unchanged from before —
    // only its unlock source is now earned). Enqueued ONCE, when the whole research closure is
    // unlocked. launch_successor has no output item, so these are explicit `QueuedJob`s.
    let build_jobs: Vec<(&str, usize)> = vec![
        ("crush_stone", 8),
        ("crush_aluminum", 6),
        ("smelt_metal__iron", 10),
        ("smelt_metal__copper", 18),
        ("smelt_metal__aluminum", 4),
        ("smelt_metal__titanium", 2),
        ("draw_metal__copper", 17),
        ("wash_aluminum", 2),
        ("make_circuit", 4),
        ("refine_silicon", 4),
        ("refine_fluxite", 4),
        ("synth_vitreite", 2),
        ("roll_iron_plate", 2),
        ("roll_aluminum_plate", 2),
        ("roll_titanium_plate", 1),
        ("form_silicon_chip", 4),
        ("make_resonite_circuit", 2),
        ("form_resonite_lattice", 1),
        ("make_fluxite_coil", 2),
        ("make_power_cell", 1),
        ("make_miner_kit", 1),
        ("make_generator_kit", 1),
        ("make_assembler_kit", 1),
        ("refine_exotic_fuel__raw", 20),
        ("make_successor_core", 1),
        ("make_successor_sensor", 1),
        ("make_successor_chassis", 1),
        ("make_successor_drive", 1),
        ("make_provisioning_module", 1),
        ("launch_successor", 1),
    ];

    // The grind-driver loop. Advances simulated time in fixed dt steps (below the 4s shortest
    // recipe) and, each frame: (a) requests every target node; (b) recons fluxite/cryophase once
    // their prereqs are researched; (c) adaptively tops up the analysis chain while any theme's
    // nodes are still locked; (d) once the full closure is unlocked, swaps the queue to the build
    // list; then advances. Breaks on RunState::Completed. `max_secs` is a generous runaway guard —
    // this is a long run (the whole ~6200-point research economy plus the build).
    let craft_net = net_of(&mut app, storage_e);
    let dt = 0.5f32;
    let max_secs = 40_000.0f32;
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        dt,
    )));
    let mut elapsed = 0.0f32;
    let mut build_enqueued = false;
    let mut launch_ran = false;
    let mut ever_analyzed_circuit = false;
    let mut ever_analyzed_exotic = false;

    while *app.world().resource::<RunState>() != RunState::Completed {
        assert!(
            elapsed < max_secs,
            "earned-research grind did not complete within {max_secs}s of simulated time \
             (research pools/points: check the analysis economy or a stalled milestone/recon)"
        );

        // (a) Spend: request every target node (no-op unless prereqs met + pool can pay).
        for node in &all_nodes {
            app.world_mut()
                .write_message(UnlockNodeRequest((*node).into()));
        }

        // (b) Conditional recon: each site fires its DiscoveryEvent exactly once, and the unlock
        // only lands if the node's prereq is already researched — so recon only after it is.
        if node_unlocked(&app, "precursor_survey")
            && app.world().get::<Discovered>(fluxite_site).is_none()
        {
            recon_deposit(&mut app, "fluxite_shard");
        }
        if node_unlocked(&app, "cryophase_prospecting")
            && app.world().get::<Discovered>(cryophase_deposit).is_none()
        {
            recon_deposit(&mut app, "cryophase_shard");
        }

        // (c) Adaptive top-up of the analysis chain (research phase only).
        if !build_enqueued {
            let mat_done = nodes_unlocked(&app, &material_nodes);
            let eng_done = nodes_unlocked(&app, &engineering_nodes);
            let disc_done = nodes_unlocked(&app, &discovery_nodes);
            let synth_done = nodes_unlocked(&app, &synthesis_nodes);
            let circuit_ready = recipe_unlocked(&app, "analyze_circuit");
            let field_ready = recipe_unlocked(&app, "analyze_field_sample");
            let exotic_ready = recipe_unlocked(&app, "analyze_exotic_reaction");
            let crush_ready = recipe_unlocked(&app, "crush_iron");
            let need_iron_crushed = produced(&app, "iron_crushed") < 55.0;
            let plate_locked = !node_unlocked(&app, "plate_roller");
            ever_analyzed_circuit |= circuit_ready;
            ever_analyzed_exotic |= exotic_ready;
            let mut queue = app
                .world_mut()
                .get_mut::<NetworkCraftQueue>(craft_net)
                .expect("logistics network carries a craft queue");
            // material: 4 stone → 10 material.
            if !mat_done {
                ensure_jobs(&mut queue, "basic_analysis", 16);
            }
            // Keep iron smelting for circuits + the ore_crusher(100)/plate_roller(150) tally.
            if !eng_done || plate_locked {
                ensure_jobs(&mut queue, "smelt_metal__iron", 12);
            }
            // engineering: circuit_board → 20 engineering (copper→wire→circuit chain).
            if !eng_done && circuit_ready {
                ensure_jobs(&mut queue, "smelt_metal__copper", 20);
                ensure_jobs(&mut queue, "draw_metal__copper", 20);
                ensure_jobs(&mut queue, "make_circuit", 12);
                ensure_jobs(&mut queue, "analyze_circuit", 12);
            }
            // Crush iron to clear the ore_washer(50 iron_crushed) milestone.
            if crush_ready && need_iron_crushed {
                ensure_jobs(&mut queue, "crush_iron", 12);
            }
            // discovery: field_sample → 12 discovery.
            if !disc_done && field_ready {
                ensure_jobs(&mut queue, "analyze_field_sample", 12);
            }
            // synthesis: resonite_shard → 20 synthesis.
            if !synth_done && exotic_ready {
                ensure_jobs(&mut queue, "analyze_exotic_reaction", 12);
            }
        }

        // (d) Whole research closure earned → swap to the mass-balanced successor build list.
        if !build_enqueued && node_unlocked(&app, "launch_successor") {
            let mut queue = app
                .world_mut()
                .get_mut::<NetworkCraftQueue>(craft_net)
                .expect("logistics network carries a craft queue");
            queue.jobs.clear();
            for (recipe, n) in &build_jobs {
                for _ in 0..*n {
                    queue.jobs.push_back(QueuedJob {
                        recipe_id: (*recipe).to_string(),
                    });
                }
            }
            build_enqueued = true;
        }

        if app.world().get::<MachineState>(launch_e).copied() == Some(MachineState::Running) {
            launch_ran = true;
        }

        app.update();
        elapsed += dt;
    }

    // Every research node must have been EARNED (spent/auto), never injected.
    for node in &all_nodes {
        assert!(
            node_unlocked(&app, node),
            "target node {node} must have been earned before the build phase"
        );
    }
    assert!(
        build_enqueued,
        "the successor build list must have been enqueued after the research closure unlocked"
    );
    assert!(
        ever_analyzed_circuit && ever_analyzed_exotic,
        "the engineering + synthesis analysis recipes must have been earned and run for real"
    );
    let launch_ran = std::cell::Cell::new(launch_ran);

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

//! End-to-end integration test for a full Standard run — landing all the way to launching the
//! successor vehicle — exercised through the real world-generation + placement + logistics +
//! recipe + research + power systems from a fixed seed, driven on simulated time (see
//! `advance_until` / `run_until`). Every machine the factory uses is OBTAINED FOR REAL: it
//! either arrives in the landing kit or is crafted from mined ore through a `make_*` recipe into
//! networked storage and then placed the same way the player places (inventory-gated).
//!
//!   Stage 0: fixed seed → generate terrain + surface deposits → the real landing (PodPlugin)
//!            sets down a networked `storage_crate` stocked with the starting kit (1 assembler,
//!            1 solar_generator, 1 miner, 1 analysis_station, cables). Place those kit machines
//!            through the real inventory-gated path (`place_real` decrements the owned copy),
//!            wire them, and prove the miner latched + the network formed.
//!   Stage 1: queue-driven mine→analyse loop earns the first ResearchSpend node (ore_extraction).
//!   Stage 1b: the first research spend reveals both atmospheric properties.
//!   Stage 2: sustained grind → basic_processing.
//!   Stage 3: CRAFT a smelter (make_smelter), place + power it, smelt mined iron_ore → iron_ingot.
//!   Stage 4: CRAFT a wire_drawer + a second assembler, run the copper→wire→circuit chain.
//!   Stage 5: drone scan reveals geological activity.
//!   Stage 6: the full Standard victory with EARNED research and a REAL machine economy. Craft +
//!            place the mining fleet, the extra analysis stations / smelter / solar farm, then
//!            grind the four research currencies to walk the whole launch_successor closure
//!            (incl. steel_alloying, earned+spent). As each gate opens, lazily CRAFT + place the
//!            processing machines (crusher, washer, plate_roller, refinery) and finally the
//!            advanced_assembler + launch_site — the last two built from real steel/plate/circuit
//!            bodies (steel needs the earned steel_alloying node). Then run the untouched
//!            successor build list on those real machines until launch_successor → victory.
//!
//! NOTHING gameplay-relevant is injected. No StorageUnit, GeneratorUnit, TechTreeProgress or
//! ResearchPool is hand-provisioned: storage comes from the real landing crate + crafted crates,
//! generator watts are the seed-scaled values `place_machine_system` assigns, and every tech
//! node is earned through the real `UnlockNodeRequest` grind (PrerequisiteChain + ProductionMile-
//! stone nodes auto-unlock off real production; ExplorationDiscovery nodes unlock from real drone
//! recon; ResearchSpend nodes are paid from analysis-produced currency). Every machine is owned
//! before it is placed (kit or crafted).
//!
//! The only headless stubs that remain — provisioning the render/physics context a real client
//! has, never the logic under test — are: simulated-time fast-forward (`ManualDuration`),
//! `MachinePortLayout` stubs (no GLTF port metadata headless), GLTF/scene asset stubs, and the
//! bare `Drone` spawn for recon (the full DronePlugin needs avian physics we can't run headless).

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
use exergon::inventory::Hotbar;
use exergon::logistics::{
    LogisticsNetworkMember, LogisticsSimPlugin, NetworkCraftQueue, QueuedJob, StorageUnit,
};
use exergon::machine::{
    Machine, MachinePlugin, MachinePortLayout, MachinePortLayouts, MachineState, ManualCraftOnly,
    MinerMachine,
};
use exergon::planet::{Planet, PlanetPlugin, PlanetPropertyVisibility, PropertyVisibility};
use exergon::pod::PodPlugin;
use exergon::power::{GeneratorUnit, PowerPlugin};
use exergon::recipe_graph::{RecipeGraph, RecipeGraphPlugin};
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
        // Hotbar is the only resource PodPlugin needs that the full InventoryPlugin would
        // otherwise provide; init just the resource to avoid pulling the UI/input plugin.
        .init_resource::<Hotbar>()
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
            // Real landing: spawns the escape pod + a networked storage_crate stocked with the
            // starting kit (spawn_escape_pod + stock_bootstrap_storage), the bootstrap the run
            // draws its first machines from.
            PodPlugin,
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

/// Inventory-gated placement — the real game only emits `Placed` if the item was taken from a
/// `StorageUnit` (see `object_interaction::take_from_any_storage`). This decrements one owned
/// copy of `item_id` from ANY networked storage (panics if the player doesn't own one), then
/// runs the real placement path. So every machine must be OWNED (kit or crafted) before placement.
fn place_real(app: &mut App, item_id: &str, pos: Vec3) {
    let took = {
        let world = app.world_mut();
        let mut q = world.query::<&mut StorageUnit>();
        let mut done = false;
        for mut unit in q.iter_mut(world) {
            if let Some(c) = unit.items.get_mut(item_id)
                && *c > 0
            {
                *c -= 1;
                if *c == 0 {
                    unit.items.remove(item_id);
                }
                done = true;
                break;
            }
        }
        done
    };
    assert!(
        took,
        "place_real: no owned `{item_id}` in any storage — it must be in the landing kit or crafted first"
    );
    place(app, item_id, pos);
    app.update();
}

fn wire_logi(app: &mut App, hub: Vec3, pos: Vec3) {
    connect(app, hub + PORT_OFFSET, pos + PORT_OFFSET);
}

fn wire_power(app: &mut App, gen_pos: Vec3, pos: Vec3) {
    connect_power(app, gen_pos + ENERGY_OFFSET, pos + ENERGY_OFFSET);
}

fn machine_at(app: &mut App, machine_type: &str, pos: Vec3) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Machine, &Transform)>();
    q.iter(app.world())
        .find(|(_, m, t)| m.machine_type == machine_type && t.translation.distance(pos) < 0.6)
        .map(|(e, _, _)| e)
        .unwrap_or_else(|| panic!("no {machine_type} placed at {pos:?}"))
}

/// The logistics network a machine's ports have joined.
fn net_of(app: &mut App, machine: Entity) -> Entity {
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
}

/// Hand-push `n` copies of `recipe` — the direct queue push the craft modal performs.
fn push_jobs(app: &mut App, net: Entity, recipe: &str, n: usize) {
    let mut queue = app
        .world_mut()
        .get_mut::<NetworkCraftQueue>(net)
        .expect("logistics network carries a craft queue");
    for _ in 0..n {
        queue.jobs.push_back(QueuedJob {
            recipe_id: recipe.to_string(),
        });
    }
}

/// Enqueue the full dependency tree to craft `count × item` (leaves first, with reservation) —
/// the real `NetworkCraftQueue::enqueue_item`. Used to craft machine bodies: for a tier-1
/// machine whose inputs are raw ore this is just its `make_*` job; for the advanced_assembler /
/// launch_site it expands into the steel / plate / circuit sub-chain automatically.
fn craft(app: &mut App, net: Entity, storage_e: Entity, item: &str, count: u32) {
    app.world_mut()
        .resource_scope(|world, rg: Mut<RecipeGraph>| {
            let snapshot = world
                .get::<StorageUnit>(storage_e)
                .map(|s| s.items.clone())
                .unwrap_or_default();
            let mut queue = world
                .get_mut::<NetworkCraftQueue>(net)
                .expect("logistics network carries a craft queue");
            queue.enqueue_item(item, count, &rg, &snapshot);
        });
}

/// Own → place → wire logistics (+power) → mark ManualCraftOnly so the queue is the sole
/// scheduler. Returns the placed machine entity.
fn deploy(
    app: &mut App,
    hub: Vec3,
    gen_pos: Vec3,
    machine_type: &str,
    pos: Vec3,
    powered: bool,
) -> Entity {
    place_real(app, machine_type, pos);
    wire_logi(app, hub, pos);
    if powered {
        wire_power(app, gen_pos, pos);
    }
    app.update();
    let e = machine_at(app, machine_type, pos);
    app.world_mut().entity_mut(e).insert(ManualCraftOnly);
    app.update();
    e
}

/// Own → place a solar panel → wire it onto the power grid. `place_machine_system` inserts its
/// `GeneratorUnit` with seed-scaled watts (`100·solar_modifier`); it charges via `generator_tick`.
fn deploy_panel(app: &mut App, gen_pos: Vec3, pos: Vec3) -> Entity {
    place_real(app, "solar_generator", pos);
    wire_power(app, gen_pos, pos);
    app.update();
    machine_at(app, "solar_generator", pos)
}

/// Craft `count` miners, wait for them in storage, then place them onto the nearest deposit
/// yielding `ore_id` (offset so their ports stay distinct within the 8.0 latch range) and wire
/// each onto the shared logistics network. Returns the deposit entity.
fn craft_and_mine(
    app: &mut App,
    net: Entity,
    storage_e: Entity,
    hub: Vec3,
    ore_id: &str,
    count: usize,
    skip_origin: bool,
) -> Entity {
    let (deposit_e, pos) = nearest_vein(app, ore_id, skip_origin);
    craft(app, net, storage_e, "miner", count as u32);
    advance_until(app, 0.5, 120_000.0, |app| {
        stored(app, storage_e, "miner") >= count as u32
    });
    for i in 0..count {
        let p = pos + Vec3::new(i as f32 * 2.0, 0.0, 0.0);
        place_real(app, "miner", p);
        wire_logi(app, hub, p);
    }
    app.update();
    deposit_e
}

fn research_points(app: &App, theme: &str) -> f32 {
    app.world().resource::<ResearchPool>().get(theme)
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

/// Like `advance_until` but runs `each` before every step — a hook for topping up the craft
/// queue while a grind progresses (the queue is the player's craft-modal stand-in).
fn run_until(
    app: &mut App,
    dt: f32,
    max_secs: f32,
    mut each: impl FnMut(&mut App),
    mut done: impl FnMut(&App) -> bool,
) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        dt,
    )));
    let mut elapsed = 0.0;
    while elapsed < max_secs {
        if done(app) {
            return;
        }
        each(app);
        app.update();
        elapsed += dt;
    }
    panic!("run_until: condition not met within {max_secs}s of simulated time");
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
    // Two updates: `deposit_discovery_system` writes its one-shot `DiscoveryEvent` and marks the
    // deposit `Discovered` in the same frame, so if `check_research_unlocks` runs before it that
    // frame the event would only be read on the next — a second update guarantees the read.
    app.update();
    app.update();
    deposit_e
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
fn ensure_jobs(app: &mut App, net: Entity, recipe: &str, target: usize) {
    let mut queue = app
        .world_mut()
        .get_mut::<NetworkCraftQueue>(net)
        .expect("logistics network carries a craft queue");
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

    app.world_mut()
        .spawn((exergon::save::Run, DomainSeeds::from_master(MASTER_SEED)));
    app.world_mut().spawn((Transform::default(), MainCamera));

    app.update();

    // Land: enter Loading so worldgen activates, then let chunks + deposits settle.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Loading);
    for _ in 0..4 {
        app.update();
    }

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

    // Stub every machine's port layout BEFORE entering Playing — the pod crate is placed on the
    // Loading→Playing transition and needs its logistics port. Logistics-only machines (crate,
    // miner) get one logistics port; the solar generator is energy-only; everything else has both.
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
        layouts.by_machine.insert(
            "solar_generator".to_string(),
            MachinePortLayout {
                energy: vec![ENERGY_OFFSET],
                logistics: vec![],
            },
        );
        for id in [
            "analysis_station",
            "smelter",
            "assembler",
            "wire_drawer",
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

    // Enter Playing: PodPlugin lands the pod + a stocked storage_crate. Let it settle.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    for _ in 0..8 {
        app.update();
    }

    // Locate the landing crate — the stocked storage hub — and confirm the kit arrived.
    let storage_e = machine_entity(&mut app, "storage_crate");
    let storage_pos = app
        .world()
        .get::<Transform>(storage_e)
        .expect("pod crate transform")
        .translation;
    for essential in ["miner", "assembler", "solar_generator", "analysis_station"] {
        assert!(
            stored(&app, storage_e, essential) >= 1,
            "landing kit must contain a {essential} (got {})",
            stored(&app, storage_e, essential)
        );
    }

    // Factory layout: distinct X lane per machine type, copies stacked along +Z, all at hub y.
    let y = storage_pos.y;
    let lane = |x: f32, i: usize| Vec3::new(x, y, 10.0 + 4.0 * i as f32);
    let generator_pos = Vec3::new(5.0, y, 0.0);

    // Stage 0 — deploy the four kit machines through the real inventory-gated path. The kit miner
    // latches the origin stone/iron/copper deposit; the rest sit on the hub network.
    place_real(&mut app, "solar_generator", generator_pos); // power anchor
    app.update();
    let generator_e = machine_at(&mut app, "solar_generator", generator_pos);

    place_real(&mut app, "miner", deposit_pos);
    wire_logi(&mut app, storage_pos, deposit_pos);
    app.update();
    let miner_e = machine_at(&mut app, "miner", deposit_pos);
    assert_eq!(
        app.world().get::<MinerMachine>(miner_e).map(|m| m.deposit),
        Some(deposit_e),
        "placed kit miner should latch onto the generated origin deposit"
    );

    let assembler_pos = lane(25.0, 0);
    let station_pos = lane(10.0, 0);
    let assembler_e = deploy(
        &mut app,
        storage_pos,
        generator_pos,
        "assembler",
        assembler_pos,
        true,
    );
    let station_e = deploy(
        &mut app,
        storage_pos,
        generator_pos,
        "analysis_station",
        station_pos,
        true,
    );

    // Wire the hub crate itself onto the network (shares the miner cable's endpoint).
    connect(
        &mut app,
        storage_pos + PORT_OFFSET,
        deposit_pos + PORT_OFFSET,
    );
    app.update();

    let net = net_of(&mut app, storage_e);
    assert_eq!(net, net_of(&mut app, miner_e), "miner shares hub network");
    assert_eq!(
        net,
        net_of(&mut app, assembler_e),
        "assembler shares hub network"
    );
    assert_eq!(
        net,
        net_of(&mut app, station_e),
        "station shares hub network"
    );

    // Stage 1 — earn ore_extraction (30 material). Queue basic_analysis (4 stone → 10 material,
    // 0E) and let the mine→analyse loop run under real time. The station is ManualCraftOnly, so
    // the queue is the sole scheduler; the miner feeds stone every tick.
    let mut station_ran = false;
    run_until(
        &mut app,
        0.5,
        4_000.0,
        |app| ensure_jobs(app, net, "basic_analysis", 12),
        |app| {
            if app.world().get::<MachineState>(station_e).copied() == Some(MachineState::Running) {
                station_ran = true;
            }
            research_points(app, "material") >= 30.0
        },
    );
    assert!(
        station_ran,
        "analysis station must actually run basic_analysis"
    );

    let points_before = research_points(&app, "material");
    app.world_mut()
        .write_message(UnlockNodeRequest("ore_extraction".into()));
    app.update();

    let progress = app.world().resource::<TechTreeProgress>();
    assert!(
        progress.unlocked_nodes.contains("ore_extraction"),
        "first research node should be unlocked after spending research points"
    );
    assert!(
        research_points(&app, "material") < points_before,
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

    // Stage 2 — sustained grind to basic_processing (150 material). basic_smelting (its only
    // prereq) is a zero-prereq PrerequisiteChain node auto-unlocked the first Playing frame.
    run_until(
        &mut app,
        0.5,
        12_000.0,
        |app| ensure_jobs(app, net, "basic_analysis", 12),
        |app| research_points(app, "material") >= 150.0,
    );
    let points_before = research_points(&app, "material");
    app.world_mut()
        .write_message(UnlockNodeRequest("basic_processing".into()));
    app.update();

    let progress = app.world().resource::<TechTreeProgress>();
    assert!(
        progress.unlocked_nodes.contains("basic_processing"),
        "second-tier node should unlock after a sustained grind to 150 research points"
    );
    assert_eq!(
        research_points(&app, "material"),
        points_before - 150.0,
        "unlocking basic_processing must deduct its 150-point cost"
    );

    // Stage 3 — CRAFT a smelter, place + power it, prove the energy-gated smelt. make_smelter
    // (basic_smelting auto) = 20 stone + 10 iron_ore → smelter; smelt_metal__iron draws power.
    assert!(
        recipe_unlocked(&app, "make_smelter"),
        "basic_smelting must have auto-unlocked make_smelter"
    );
    assert!(
        recipe_unlocked(&app, "smelt_metal__iron"),
        "basic_smelting's smelt_metal template must have auto-unlocked smelt_metal__iron"
    );
    craft(&mut app, net, storage_e, "smelter", 1);
    advance_until(&mut app, 0.5, 6_000.0, |app| {
        stored(app, storage_e, "smelter") >= 1
    });
    let smelter_pos = lane(15.0, 0);
    let smelter_e = deploy(
        &mut app,
        storage_pos,
        generator_pos,
        "smelter",
        smelter_pos,
        true,
    );

    push_jobs(&mut app, net, "smelt_metal__iron", 3);
    let mut smelter_ran = false;
    advance_until(&mut app, 0.25, 6_000.0, |app| {
        if app.world().get::<MachineState>(smelter_e).copied() == Some(MachineState::Running) {
            smelter_ran = true;
        }
        stored(app, storage_e, "iron_ingot") >= 1
    });
    assert!(
        smelter_ran,
        "smelter must run the energy-gated smelt recipe"
    );
    assert!(
        app.world()
            .get::<GeneratorUnit>(generator_e)
            .is_some_and(|g| g.buffer_joules > 0.0),
        "kit solar generator must have charged its buffer over simulated time"
    );

    // Stage 4 — CRAFT a wire_drawer + a second assembler, run copper→wire→circuit for real.
    // make_circuit = 1 iron_ingot + 2 copper_wire → 1 circuit_board (assembler, basic_processing).
    assert!(recipe_unlocked(&app, "make_wire_drawer"));
    assert!(recipe_unlocked(&app, "make_assembler"));
    craft(&mut app, net, storage_e, "wire_drawer", 1);
    craft(&mut app, net, storage_e, "assembler", 1);
    advance_until(&mut app, 0.5, 8_000.0, |app| {
        stored(app, storage_e, "wire_drawer") >= 1 && stored(app, storage_e, "assembler") >= 1
    });
    let drawer_pos = lane(20.0, 0);
    let assembler2_pos = lane(25.0, 1);
    let _drawer_e = deploy(
        &mut app,
        storage_pos,
        generator_pos,
        "wire_drawer",
        drawer_pos,
        true,
    );
    let assembler2_e = deploy(
        &mut app,
        storage_pos,
        generator_pos,
        "assembler",
        assembler2_pos,
        true,
    );

    push_jobs(&mut app, net, "smelt_metal__iron", 2);
    push_jobs(&mut app, net, "smelt_metal__copper", 2);
    push_jobs(&mut app, net, "draw_metal__copper", 2);
    push_jobs(&mut app, net, "make_circuit", 1);
    let mut assembler_ran = false;
    advance_until(&mut app, 0.25, 8_000.0, |app| {
        if app.world().get::<MachineState>(assembler_e).copied() == Some(MachineState::Running)
            || app.world().get::<MachineState>(assembler2_e).copied() == Some(MachineState::Running)
        {
            assembler_ran = true;
        }
        stored(app, storage_e, "circuit_board") >= 1
    });
    assert!(
        assembler_ran,
        "an assembler must run make_circuit under power"
    );
    assert!(stored(&app, storage_e, "circuit_board") >= 1);

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

    // Stage 6 — the full Standard victory with a REAL machine economy and EARNED research. Mine
    // every raw material, convert it into the four research currencies through real analysis
    // recipes, spend those through real `UnlockNodeRequest`s to walk the whole `launch_successor`
    // closure (incl. steel_alloying), and lazily CRAFT + place each processing machine as its gate
    // opens — including the advanced_assembler + launch_site, whose bodies are built from real
    // steel/plate/circuit. Then run the untouched successor build list on those machines until
    // launch_successor fires `EscapeEvent` (via the auto-tagged `EscapeObjective`) →
    // `RunState::Completed`, read as the "virtual time to victory".

    // Exotic-site discovery via REAL drone recon (still in DronePilot from Stage 5). xalite →
    // exotic_materials unlocks now (prereq science_basics is a free chain).
    let xalite_site = recon_deposit(&mut app, "xalite");
    assert!(
        app.world().get::<Discovered>(xalite_site).is_some(),
        "drone recon must mark the xalite deposit Discovered (real DiscoveryEvent fired)"
    );
    assert!(node_unlocked(&app, "exotic_materials"));

    // Scale up the early factory: two more analysis stations (research is the dominant grind — the
    // ~6.5k-point closure serialises badly on one station), a second smelter (the whole metal
    // economy — iron for circuits/steel/plates + the ore_crusher/plate_roller tally — funnels
    // through it), and a solar farm sized to the build-phase peak draw. Every machine is crafted.
    craft(&mut app, net, storage_e, "analysis_station", 2);
    craft(&mut app, net, storage_e, "smelter", 1);
    advance_until(&mut app, 0.5, 25_000.0, |app| {
        stored(app, storage_e, "analysis_station") >= 2 && stored(app, storage_e, "smelter") >= 1
    });
    let station2_e = deploy(
        &mut app,
        storage_pos,
        generator_pos,
        "analysis_station",
        lane(10.0, 1),
        true,
    );
    let station3_e = deploy(
        &mut app,
        storage_pos,
        generator_pos,
        "analysis_station",
        lane(10.0, 2),
        true,
    );
    let _smelter2_e = deploy(
        &mut app,
        storage_pos,
        generator_pos,
        "smelter",
        lane(15.0, 1),
        true,
    );
    let _ = (station2_e, station3_e);

    // Solar farm: peak concurrent build-phase draw is ~120 W (launch_successor 44 W + a couple of
    // refinery/advanced-assembler recipes). Craft 6 panels (+ the kit panel = 7); at the seed's
    // solar_modifier this covers the draw with margin, and pre-charged buffers absorb transients.
    let panels = 6usize;
    craft(&mut app, net, storage_e, "solar_generator", panels as u32);
    advance_until(&mut app, 0.5, 30_000.0, |app| {
        stored(app, storage_e, "solar_generator") >= panels as u32
    });
    let mut farm: Vec<Entity> = Vec::new();
    for i in 0..panels {
        farm.push(deploy_panel(&mut app, generator_pos, lane(60.0, i)));
    }
    advance_until(&mut app, 1.0, 6_000.0, |app| {
        farm.iter().all(|&e| {
            app.world()
                .get::<GeneratorUnit>(e)
                .is_some_and(|g| g.buffer_joules >= g.max_buffer_joules * 0.9)
        })
    });

    // Mine every raw material for real (miners are crafted first via `craft_and_mine`). The fresh
    // iron_copper vein (2 miners) is the heavy feed for steel/circuits/plates; cryophase (2 miners)
    // supplies all 60 shards for the 20 exotic_fuel; trace ores get 1 miner each. Stage 0's origin
    // miner keeps running alongside the fresh iron_copper vein.
    let iron_copper_deposit =
        craft_and_mine(&mut app, net, storage_e, storage_pos, "copper_ore", 2, true);
    craft_and_mine(
        &mut app,
        net,
        storage_e,
        storage_pos,
        "resonite_shard",
        1,
        false,
    );
    craft_and_mine(
        &mut app,
        net,
        storage_e,
        storage_pos,
        "aluminum_ore",
        1,
        false,
    );
    craft_and_mine(
        &mut app,
        net,
        storage_e,
        storage_pos,
        "titanium_ore",
        1,
        false,
    );
    craft_and_mine(&mut app, net, storage_e, storage_pos, "coal", 1, false);
    let fluxite_site = craft_and_mine(
        &mut app,
        net,
        storage_e,
        storage_pos,
        "fluxite_shard",
        1,
        false,
    );
    let cryophase_deposit = craft_and_mine(
        &mut app,
        net,
        storage_e,
        storage_pos,
        "cryophase_shard",
        2,
        false,
    );

    // The ResearchSpend target nodes, by theme (steel_alloying added to material so the run can
    // craft real steel for the advanced_assembler + launch_site bodies). The grind spams an
    // `UnlockNodeRequest` for each every frame — a no-op until prereqs are met AND the pool can
    // pay, so they self-order by the real tech graph.
    let material_nodes = [
        "ore_extraction",
        "drone_recon",
        "basic_processing",
        "silicon_refining",
        "aluminum_extraction",
        "titanium_forming",
        "steel_alloying",
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

    // Lazy machine buildout: each processing machine is CRAFTED + placed only once its `make_*`
    // recipe unlocks (crusher/washer/plate_roller off real ProductionMilestones; refinery off
    // silicon_refining; advanced_assembler/launch_site off their late nodes, their bodies built
    // from real steel/plate/circuit via `craft`'s dependency expansion).
    struct Slot {
        machine: &'static str,
        gate_recipe: &'static str,
        pos: Vec3,
        body: bool,
        enqueued: bool,
        placed: bool,
    }
    let mut slots = vec![
        Slot {
            machine: "crusher",
            gate_recipe: "make_crusher",
            pos: lane(30.0, 0),
            body: false,
            enqueued: false,
            placed: false,
        },
        Slot {
            machine: "washer",
            gate_recipe: "make_washer",
            pos: lane(35.0, 0),
            body: false,
            enqueued: false,
            placed: false,
        },
        Slot {
            machine: "plate_roller",
            gate_recipe: "make_plate_roller",
            pos: lane(45.0, 0),
            body: false,
            enqueued: false,
            placed: false,
        },
        Slot {
            machine: "refinery",
            gate_recipe: "make_refinery",
            pos: lane(40.0, 0),
            body: false,
            enqueued: false,
            placed: false,
        },
        // Terminal machines: their bodies are prepped in bulk (see below), not per-slot, so the
        // shared steel/plate/circuit accounting is robust — the slot only places them.
        Slot {
            machine: "advanced_assembler",
            gate_recipe: "make_advanced_assembler",
            pos: lane(50.0, 0),
            body: true,
            enqueued: false,
            placed: false,
        },
        Slot {
            machine: "launch_site",
            gate_recipe: "make_launch_site",
            pos: lane(55.0, 0),
            body: true,
            enqueued: false,
            placed: false,
        },
    ];

    // The grind-driver loop. Each frame: (a) request every target node; (b) recon fluxite/
    // cryophase once their prereqs are researched; (c) craft + place each machine as its gate
    // opens; (d) top up the analysis + milestone chain while any theme is locked; (e) once the
    // whole closure is earned AND the terminal machines are placed, swap to the untouched build
    // list. Breaks on RunState::Completed. `max_secs` is a generous runaway guard.
    let dt = 0.5f32;
    // Generous runaway guard. The measured victory is ~11.5k simulated seconds; this leaves
    // ample margin for content-balance tweaks (raise only if a slowdown is intended).
    let max_secs = 40_000.0f32;
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        dt,
    )));
    let mut elapsed = 0.0f32;
    let mut bodies_prepped = false;
    let mut build_enqueued = false;
    let mut launch_ran = false;
    let mut launch_e: Option<Entity> = None;
    let mut ever_analyzed_circuit = false;
    let mut ever_analyzed_exotic = false;

    while *app.world().resource::<RunState>() != RunState::Completed {
        assert!(
            elapsed < max_secs,
            "earned-research + real-build grind did not complete within {max_secs}s of simulated \
             time (check the analysis economy, a stalled milestone/recon, or machine buildout)"
        );

        // (a) Spend: request every target node (no-op unless prereqs met + pool can pay).
        for node in &all_nodes {
            app.world_mut()
                .write_message(UnlockNodeRequest((*node).into()));
        }

        // (b) Conditional recon: each site fires its DiscoveryEvent exactly once, honored only if
        // the node's prereq is already researched — so recon only after it is.
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

        // (c0) Terminal body prep: once make_launch_site unlocks (the last machine gate), push a
        // generous, over-provisioned set of body-intermediate jobs (steel/plates/circuits + their
        // sub-inputs) plus the two make_* machine jobs. Hand-pushed rather than enqueue_item so the
        // accounting is robust across BOTH terminals — enqueue_item's per-call storage snapshot
        // double-counts shared intermediates (make_launch_site sees steel the advanced_assembler
        // job will consume). Bodies: advanced_assembler = 4 steel + 2 circuit + 2 iron_plate;
        // launch_site = 12 steel + 4 titanium_plate + 4 circuit. Excess sits in storage (harmless).
        if !bodies_prepped && recipe_unlocked(&app, "make_launch_site") {
            for (recipe, n) in [
                ("smelt_metal__iron", 44),
                ("smelt_metal__copper", 20),
                ("smelt_metal__titanium", 8),
                ("draw_metal__copper", 20),
                ("alloy_steel", 10),        // → 20 steel_ingot (need 16)
                ("roll_iron_plate", 2),     // → 6 iron_plate (need 2)
                ("roll_titanium_plate", 3), // → 9 titanium_plate (need 4)
                ("make_circuit", 8),        // → 8 circuit_board (need 6)
                ("make_advanced_assembler", 1),
                ("make_launch_site", 1),
            ] {
                push_jobs(&mut app, net, recipe, n);
            }
            bodies_prepped = true;
        }

        // (c) Lazy machine buildout as gates open.
        for i in 0..slots.len() {
            if slots[i].placed || !recipe_unlocked(&app, slots[i].gate_recipe) {
                continue;
            }
            if !slots[i].enqueued {
                if !slots[i].body {
                    let machine = slots[i].machine;
                    craft(&mut app, net, storage_e, machine, 1);
                }
                slots[i].enqueued = true;
            }
            if stored(&app, storage_e, slots[i].machine) >= 1 {
                let (machine, pos) = (slots[i].machine, slots[i].pos);
                let e = deploy(&mut app, storage_pos, generator_pos, machine, pos, true);
                if machine == "launch_site" {
                    assert!(
                        app.world().get::<EscapeObjective>(e).is_some(),
                        "a placed launch_site must be tagged EscapeObjective so its recipe wins"
                    );
                    launch_e = Some(e);
                }
                slots[i].placed = true;
            }
        }

        // (d) Analysis + milestone job top-up (research phase only). CAPPED: each analysis chain
        // is topped up only while its pool is below `pool_cap` and its theme still has locked
        // nodes, and smelting/drawing only while stock is below a ceiling. Without caps the easy
        // chains (basic_analysis, smelt_iron) run away and consume every scrap of raw stone/
        // iron_ore, starving the machine-body crafts (crusher/refinery/…) that need raw ore —
        // which blocks the ore_crusher→ore_washer→plate_roller→titanium_forming milestone chain.
        if !build_enqueued {
            let mat_done = nodes_unlocked(&app, &material_nodes);
            let eng_done = nodes_unlocked(&app, &engineering_nodes);
            let disc_done = nodes_unlocked(&app, &discovery_nodes);
            let synth_done = nodes_unlocked(&app, &synthesis_nodes);
            let circuit_ready = recipe_unlocked(&app, "analyze_circuit");
            let field_ready = recipe_unlocked(&app, "analyze_field_sample");
            let exotic_ready = recipe_unlocked(&app, "analyze_exotic_reaction");
            let crush_ready = recipe_unlocked(&app, "crush_iron");
            ever_analyzed_circuit |= circuit_ready;
            ever_analyzed_exotic |= exotic_ready;
            let cap = 600.0; // pool ceiling — covers the costliest single node (200) with margin.

            // material: 4 stone → 10 material (also funds steel_alloying).
            if !mat_done && research_points(&app, "material") < cap {
                ensure_jobs(&mut app, net, "basic_analysis", 8);
            }
            // Keep iron flowing to reach the ore_crusher(100)/plate_roller(150) PRODUCTION
            // milestones, but cap the STOCK so raw iron_ore stays free for the machine bodies.
            if stored(&app, storage_e, "iron_ingot") < 200 {
                ensure_jobs(&mut app, net, "smelt_metal__iron", 8);
            }
            // engineering: circuit_board → 20 engineering (copper→wire→circuit chain).
            if !eng_done && circuit_ready && research_points(&app, "engineering") < cap {
                if stored(&app, storage_e, "copper_ingot") < 60 {
                    ensure_jobs(&mut app, net, "smelt_metal__copper", 8);
                }
                if stored(&app, storage_e, "copper_wire") < 60 {
                    ensure_jobs(&mut app, net, "draw_metal__copper", 8);
                }
                if stored(&app, storage_e, "circuit_board") < 25 {
                    ensure_jobs(&mut app, net, "make_circuit", 8);
                }
                ensure_jobs(&mut app, net, "analyze_circuit", 8);
            }
            // Crush iron to clear the ore_washer(50 iron_crushed) milestone (needs a placed crusher).
            if crush_ready && produced(&app, "iron_crushed") < 60.0 {
                ensure_jobs(&mut app, net, "crush_iron", 8);
            }
            // discovery: field_sample → 12 discovery.
            if !disc_done && field_ready && research_points(&app, "discovery") < cap {
                ensure_jobs(&mut app, net, "analyze_field_sample", 8);
            }
            // synthesis: resonite_shard → 20 synthesis.
            if !synth_done && exotic_ready && research_points(&app, "synthesis") < cap {
                ensure_jobs(&mut app, net, "analyze_exotic_reaction", 8);
            }
        }

        // (e) Whole closure earned AND terminal machines placed → swap to the successor build list.
        let closure_done = node_unlocked(&app, "launch_successor");
        let terminals_ready = slots
            .iter()
            .filter(|s| s.machine == "advanced_assembler" || s.machine == "launch_site")
            .all(|s| s.placed);
        if !build_enqueued && closure_done && terminals_ready {
            let mut queue = app
                .world_mut()
                .get_mut::<NetworkCraftQueue>(net)
                .expect("logistics network carries a craft queue");
            queue.jobs.clear();
            queue.reserved.clear();
            for (recipe, n) in &build_jobs {
                for _ in 0..*n {
                    queue.jobs.push_back(QueuedJob {
                        recipe_id: (*recipe).to_string(),
                    });
                }
            }
            build_enqueued = true;
        }

        if let Some(le) = launch_e
            && app.world().get::<MachineState>(le).copied() == Some(MachineState::Running)
        {
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
        node_unlocked(&app, "steel_alloying"),
        "steel_alloying must have been earned so the advanced_assembler/launch_site use real steel"
    );
    assert!(
        build_enqueued,
        "the successor build list must have been enqueued"
    );
    assert!(
        ever_analyzed_circuit && ever_analyzed_exotic,
        "the engineering + synthesis analysis recipes must have run for real"
    );

    let virtual_secs = app.world().resource::<Time>().elapsed_secs();
    let virtual_hours = virtual_secs / 3600.0;
    println!(
        "\n=== Standard run complete: virtual time to victory = {virtual_secs:.1}s ({virtual_hours:.2}h) ===\n"
    );

    assert!(
        launch_ran,
        "launch_site must actually run the launch_successor recipe"
    );

    // The victory must have been fed by REAL mining, not injected refined items.
    let extracted = |app: &App, deposit: Entity| -> f32 {
        app.world()
            .get::<OreDeposit>(deposit)
            .map(|d| d.total_extracted)
            .unwrap_or(0.0)
    };
    assert!(
        extracted(&app, cryophase_deposit) >= 60.0,
        "cryophase deposit must have been mined for ≥60 shards (the 20 exotic_fuel), got {}",
        extracted(&app, cryophase_deposit)
    );
    assert!(
        extracted(&app, iron_copper_deposit) >= 18.0,
        "fresh iron_copper vein must have been mined for real, got {}",
        extracted(&app, iron_copper_deposit)
    );
    assert_eq!(
        *app.world().resource::<RunState>(),
        RunState::Completed,
        "completing launch_successor on the launch_site must set RunState::Completed \
         (virtual time to victory = {virtual_secs:.1}s / {virtual_hours:.2}h)"
    );
    assert!(
        (100.0..86_400.0).contains(&virtual_secs),
        "virtual time to victory {virtual_secs:.1}s outside the sane [100s, 24h) bound"
    );
}

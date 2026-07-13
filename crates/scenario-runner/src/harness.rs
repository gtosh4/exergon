//! Reusable end-to-end **run harness** for authoring landing→victory scenarios on simulated
//! time. `Scenario` owns the headless `App` plus the landing anchors (the bootstrap storage
//! crate, its logistics network, the power-anchor position) and exposes the proven, real-systems
//! mechanics as methods: inventory-gated placement (`place_real`/`deploy`), crafting into
//! networked storage (`craft`/`ensure_jobs`), mining (`craft_and_mine`), drone recon
//! (`recon_deposit`), and simulated-time drivers (`advance_until`/`run_until`). On top of those,
//! `run()` executes a [`ScenarioSpec`]'s prescriptive `steps` list through the step interpreter
//! (`run_steps` → `exec_step`): each [`Step`] maps to one mechanic, positions auto-assigned.
//!
//! NOTHING gameplay-relevant is injected: storage comes from the real landing crate + crafted
//! crates, generator watts are the seed-scaled values `place_machine_system` assigns, every tech
//! node is earned through the real `UnlockNodeRequest` grind, and every machine is OWNED (kit or
//! crafted) before it is placed. The only headless stubs are the render/physics context a real
//! client would provide: simulated-time fast-forward (`ManualDuration`), `MachinePortLayout`
//! stubs (no GLTF port metadata headless), GLTF/scene asset stubs, and a bare `Drone` for recon.

use std::collections::HashMap;
use std::time::Duration;

use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::world_serialization::WorldAsset;

use exergon::content::ContentPlugin;
use exergon::drone::{Drone, FogCellRevealedEvent, deposit_discovery_system};
use exergon::escape::{EscapePlugin, RunState};
use exergon::logistics::{
    InstallConfigModule, LogisticsNetworkMember, LogisticsSimPlugin, NetworkCraftQueue, QueuedJob,
    StorageUnit,
};
use exergon::machine::{
    LogisticsPortOf, Machine, MachinePlugin, MachinePortLayout, MachinePortLayouts, MachineState,
    ManualCraftOnly,
};
use exergon::planet::PlanetPlugin;
use exergon::pod::PodPlugin;
use exergon::power::PowerPlugin;
use exergon::recipe_graph::{RecipeGraph, RecipeGraphPlugin};
use exergon::research::{
    ProductionTally, ResearchPlugin, ResearchPool, TechTreeProgress, TierCap, UnlockNodeRequest,
};
use exergon::seed::DomainSeeds;
use exergon::tech_tree::TechTreePlugin;
use exergon::world::{
    CableConnectionEvent, MainCamera, OreDeposit, WorldObjectEvent, WorldObjectKind, WorldgenPlugin,
};
use exergon::{GameState, PlayMode};

use crate::report::RunReport;
use crate::spec::{MineTarget, ScenarioSpec, Step};

/// Logistics ports sit one unit +X of a machine; energy ports one unit −X, so a power cable and a
/// logistics cable to the same machine snap to different port entities.
const PORT_OFFSET: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const ENERGY_OFFSET: Vec3 = Vec3::new(-1.0, 0.0, 0.0);

/// A landing→victory run in progress. `app` is public as an escape hatch for one-off world reads
/// the typed methods don't cover; the anchors (`storage_e`/`net`/`generator_pos`) are set by
/// `land()` + `bind_network()`. `report` accumulates milestones as the time-advancing
/// [`Scenario::run_steps`] interpreter runs.
pub struct Scenario {
    pub app: App,
    storage_e: Entity,
    storage_pos: Vec3,
    generator_pos: Vec3,
    net: Entity,
    origin_pos: Vec3,
    origin_ores: Vec<(String, f32)>,
    report: RunReport,
    /// When true, every simulated-time wait tops up the affordable analysis chains (the step
    /// interpreter's `Pump` verb — see [`Scenario::feed_economy_frame`]). Node requests come from
    /// the `Research` steps, not from a node list, so this only feeds the economy.
    feed_economy: bool,
}

impl Scenario {
    /// Builds the headless app for `seed`, spawns the run entity + camera, and **lands** — every
    /// scenario always starts landed, so this runs the real worldgen + PodPlugin landing before
    /// returning. Ready to place the kit; wire the hub then call `bind_network()`.
    pub fn new(seed: u64) -> Self {
        let mut app = build_app();
        app.world_mut()
            .spawn((exergon::save::Run, DomainSeeds::from_master(seed)));
        app.world_mut().spawn((Transform::default(), MainCamera));
        app.update();
        let mut s = Scenario {
            app,
            storage_e: Entity::PLACEHOLDER,
            storage_pos: Vec3::ZERO,
            generator_pos: Vec3::ZERO,
            net: Entity::PLACEHOLDER,
            origin_pos: Vec3::ZERO,
            origin_ores: Vec::new(),
            report: RunReport {
                seed,
                ..Default::default()
            },
            feed_economy: false,
        };
        s.land();
        s
    }

    /// Enters Loading (worldgen), records the origin-chunk starter deposit's position + ores, stubs
    /// every machine's port layout (no GLTF headless), then enters Playing so the real PodPlugin
    /// lands the pod + a stocked `storage_crate`. Locates that crate as the bootstrap hub and fixes
    /// the power anchor at its y. Called by `new()` — scenarios never invoke it directly.
    fn land(&mut self) {
        self.set_game_state(GameState::Loading);
        for _ in 0..4 {
            self.app.update();
        }

        let (deposit_tf, ores) = {
            let mut q = self.app.world_mut().query::<(&Transform, &OreDeposit)>();
            q.iter(self.app.world())
                .find(|(_, d)| d.chunk_pos == IVec2::ZERO)
                .map(|(t, d)| (*t, d.ores.clone()))
                .expect("world generation must place a deposit on the origin chunk")
        };
        self.origin_pos = deposit_tf.translation;
        self.origin_ores = ores;

        self.stub_port_layouts();

        self.set_game_state(GameState::Playing);
        for _ in 0..8 {
            self.app.update();
        }

        self.storage_e = self.machine_entity("storage_crate");
        self.storage_pos = self
            .app
            .world()
            .get::<Transform>(self.storage_e)
            .expect("pod crate transform")
            .translation;
        self.generator_pos = Vec3::new(5.0, self.storage_pos.y, 0.0);
    }

    /// Logistics-only machines (crate, miner) get one logistics port; the solar generator is
    /// energy-only; everything else has both. Must run BEFORE Playing — the pod crate is placed
    /// on the Loading→Playing transition and needs its logistics port immediately.
    fn stub_port_layouts(&mut self) {
        let mut layouts = self.app.world_mut().resource_mut::<MachinePortLayouts>();
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

    /// Binds `net` to the network the bootstrap hub crate's port has joined. Call after the hub is
    /// wired (a cable from the crate to any other machine forms the network).
    pub fn bind_network(&mut self) {
        self.net = self.net_of(self.storage_e);
    }

    // ── anchors / layout ────────────────────────────────────────────────────────────────────

    pub fn storage_pos(&self) -> Vec3 {
        self.storage_pos
    }
    pub fn generator_pos(&self) -> Vec3 {
        self.generator_pos
    }
    pub fn origin_pos(&self) -> Vec3 {
        self.origin_pos
    }
    pub fn origin_ores(&self) -> &[(String, f32)] {
        &self.origin_ores
    }

    /// Factory layout helper: a distinct X lane per machine type, copies stacked along +Z at hub y.
    pub fn lane(&self, x: f32, i: usize) -> Vec3 {
        Vec3::new(x, self.storage_pos.y, 10.0 + 4.0 * i as f32)
    }

    // ── placement / wiring ──────────────────────────────────────────────────────────────────

    fn place(&mut self, item_id: &str, pos: Vec3) {
        self.app.world_mut().write_message(WorldObjectEvent {
            transform: Transform::from_translation(pos),
            item_id: item_id.to_string(),
            kind: WorldObjectKind::Placed,
        });
    }

    /// Inventory-gated placement — the real game only emits `Placed` if the item was taken from a
    /// `StorageUnit` (see `object_interaction::take_from_any_storage`). Decrements one owned copy
    /// of `item_id` from ANY networked storage (panics if the player doesn't own one), then runs
    /// the real placement path. Every machine must be OWNED (kit or crafted) before placement.
    pub fn place_real(&mut self, item_id: &str, pos: Vec3) {
        let took = {
            let world = self.app.world_mut();
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
        self.place(item_id, pos);
        self.app.update();
    }

    fn cable(&mut self, item_id: &str, from: Vec3, to: Vec3) {
        self.app.world_mut().write_message(CableConnectionEvent {
            from,
            to,
            item_id: item_id.to_string(),
            kind: WorldObjectKind::Placed,
            from_port: None,
            to_port: None,
        });
    }

    /// Runs a logistics cable between the logistics ports of the machines at `hub` and `pos`.
    pub fn wire_logi(&mut self, hub: Vec3, pos: Vec3) {
        self.cable("logistics_cable", hub + PORT_OFFSET, pos + PORT_OFFSET);
    }

    /// Runs a power cable between the energy ports of the machines at `gen_pos` and `pos`.
    pub fn wire_power(&mut self, gen_pos: Vec3, pos: Vec3) {
        self.cable("power_cable", gen_pos + ENERGY_OFFSET, pos + ENERGY_OFFSET);
    }

    /// Own → place → wire logistics (+power) → mark `ManualCraftOnly` so the queue is the sole
    /// scheduler. Wires to the bootstrap hub + power anchor. Returns the placed machine entity.
    pub fn deploy(&mut self, machine_type: &str, pos: Vec3, powered: bool) -> Entity {
        let (hub, gen_pos) = (self.storage_pos, self.generator_pos);
        self.place_real(machine_type, pos);
        self.wire_logi(hub, pos);
        if powered {
            self.wire_power(gen_pos, pos);
        }
        self.app.update();
        let e = self.machine_at(machine_type, pos);
        self.app.world_mut().entity_mut(e).insert(ManualCraftOnly);
        self.app.update();
        e
    }

    /// Fit a config-module item into a machine (machine dedication): fires `InstallConfigModule`,
    /// which consumes one module from the network and writes its axis/value onto the machine's
    /// `MachineConfig`. The module must already be in a network storage (craft it first).
    pub fn install_config(&mut self, machine: Entity, item_id: &str) {
        self.app.world_mut().write_message(InstallConfigModule {
            machine,
            item_id: item_id.to_string(),
        });
        self.app.update();
    }

    /// Own → place a solar panel → wire it onto the power grid. `place_machine_system` inserts its
    /// `GeneratorUnit` with seed-scaled watts (`100·solar_modifier`); it charges via `generator_tick`.
    pub fn deploy_panel(&mut self, pos: Vec3) -> Entity {
        let gen_pos = self.generator_pos;
        self.place_real("solar_generator", pos);
        self.wire_power(gen_pos, pos);
        self.app.update();
        self.machine_at("solar_generator", pos)
    }

    // ── crafting ────────────────────────────────────────────────────────────────────────────

    /// Enqueue the full dependency tree to craft `count × item` (leaves first, with reservation) —
    /// the real `NetworkCraftQueue::enqueue_item`. For a tier-1 machine whose inputs are raw ore
    /// this is just its `make_*` job; for the advanced_assembler / launch_site it expands into the
    /// steel / plate / circuit sub-chain automatically.
    pub fn craft(&mut self, item: &str, count: u32) {
        let (net, storage_e) = (self.net, self.storage_e);
        self.app
            .world_mut()
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

    /// Tops the queue up so it holds at least `target` jobs of `recipe` (counts current, pushes
    /// the deficit). Over-queuing is harmless (infeasible jobs wait, surplus is reused).
    pub fn ensure_jobs(&mut self, recipe: &str, target: usize) {
        let mut queue = self
            .app
            .world_mut()
            .get_mut::<NetworkCraftQueue>(self.net)
            .expect("logistics network carries a craft queue");
        let have = queue.jobs.iter().filter(|j| j.recipe_id == recipe).count();
        for _ in have..target {
            queue.jobs.push_back(QueuedJob {
                recipe_id: recipe.to_string(),
            });
        }
    }

    /// Craft `count` miners, wait for them in storage, then place them onto the nearest deposit
    /// yielding `ore_id` (offset so their ports stay distinct within latch range) and wire each
    /// onto the shared logistics network. `skip_origin` excludes the (0,0) starter deposit so a
    /// caller can mine a fresh, un-depleted vein. Returns the deposit entity.
    pub fn craft_and_mine(&mut self, ore_id: &str, count: usize, skip_origin: bool) -> Entity {
        let (deposit_e, pos) = self.nearest_vein(ore_id, skip_origin);
        let hub = self.storage_pos;
        self.craft("miner", count as u32);
        self.advance_until(0.5, 120_000.0, |s| s.hub_stored("miner") >= count as u32);
        for i in 0..count {
            let p = pos + Vec3::new(i as f32 * 2.0, 0.0, 0.0);
            self.place_real("miner", p);
            self.wire_logi(hub, p);
        }
        self.app.update();
        deposit_e
    }

    // ── recon ───────────────────────────────────────────────────────────────────────────────

    /// Locates the nearest loaded surface deposit yielding `ore_id`, closest to origin. Reads only
    /// the real `OreDeposit` entities world generation spawned (nothing hand-placed).
    pub fn nearest_vein(&mut self, ore_id: &str, skip_origin: bool) -> (Entity, Vec3) {
        let mut q = self
            .app
            .world_mut()
            .query::<(Entity, &Transform, &OreDeposit)>();
        q.iter(self.app.world())
            .filter(|(_, _, d)| !(skip_origin && d.chunk_pos == IVec2::ZERO))
            .filter(|(_, _, d)| d.ores.iter().any(|(id, _)| id == ore_id))
            .map(|(e, t, _)| (e, t.translation))
            .min_by(|(_, a), (_, b)| {
                a.length_squared()
                    .partial_cmp(&b.length_squared())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|| {
                panic!("world generation produced no loaded deposit yielding {ore_id}")
            })
    }

    /// Pilots the drone to the deposit carrying `signature_ore` and runs so the real
    /// `deposit_discovery_system` fires its one-shot `DiscoveryEvent`. Requires `PlayMode::
    /// DronePilot`. A deposit fires exactly once (then is `Discovered` forever) and the research
    /// systems honor the event only in the frame it fires — recon a site only once its node's
    /// prerequisites are met. Returns the deposit entity.
    pub fn recon_deposit(&mut self, signature_ore: &str) -> Entity {
        let (deposit_e, pos) = self.nearest_vein(signature_ore, false);
        let existing = {
            let mut q = self.app.world_mut().query_filtered::<Entity, With<Drone>>();
            q.iter(self.app.world()).next()
        };
        match existing {
            Some(e) => {
                self.app
                    .world_mut()
                    .entity_mut(e)
                    .insert(Transform::from_translation(pos));
            }
            None => {
                self.app
                    .world_mut()
                    .spawn((Drone, Transform::from_translation(pos)));
            }
        }
        // Two updates: `deposit_discovery_system` writes its one-shot `DiscoveryEvent` and marks
        // the deposit `Discovered` in the same frame; a second update guarantees the read if
        // `check_research_unlocks` ran first that frame.
        self.app.update();
        self.app.update();
        deposit_e
    }

    // ── simulated-time drivers ──────────────────────────────────────────────────────────────

    /// Advances simulated time deterministically until `done` holds, in fixed `dt` steps.
    /// `ManualDuration` makes every `app.update()` advance the clock by exactly `dt` so the
    /// rate-integrating systems progress. Panics if `done` is not met within `max_secs`.
    pub fn advance_until(
        &mut self,
        dt: f32,
        max_secs: f32,
        mut done: impl FnMut(&Scenario) -> bool,
    ) {
        self.app
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                dt,
            )));
        let mut elapsed = 0.0;
        while elapsed < max_secs {
            if done(self) {
                return;
            }
            self.pump();
            self.app.update();
            self.capture();
            elapsed += dt;
        }
        panic!("advance_until: condition not met within {max_secs}s of simulated time");
    }

    /// Like `advance_until` but runs `each` before every step — a hook for topping up the craft
    /// queue while a grind progresses (the queue is the player's craft-modal stand-in).
    pub fn run_until(
        &mut self,
        dt: f32,
        max_secs: f32,
        mut each: impl FnMut(&mut Scenario),
        mut done: impl FnMut(&Scenario) -> bool,
    ) {
        self.app
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                dt,
            )));
        let mut elapsed = 0.0;
        while elapsed < max_secs {
            if done(self) {
                return;
            }
            each(self);
            self.pump();
            self.app.update();
            self.capture();
            elapsed += dt;
        }
        panic!("run_until: condition not met within {max_secs}s of simulated time");
    }

    /// One pump step, run before every simulated-time frame: feed the economy while the
    /// interpreter's `Pump` verb has it armed (see [`Scenario::feed_economy_frame`]).
    fn pump(&mut self) {
        if self.feed_economy {
            self.feed_economy_frame();
        }
    }

    /// One frame of the interpreter's auto-feed economy: top up every affordable analysis/milestone
    /// chain that is unlocked and below the pool ceiling, leaving raw-ore reserves for the machine
    /// bodies crafted from the same hub. Unlike [`Scenario::pump_frame`] it requests **no** nodes
    /// (the `Research` steps do that) and has no per-theme "done" gate (there is no node list) — it
    /// just keeps the four currencies flowing until the `Build` step disarms it.
    fn feed_economy_frame(&mut self) {
        let circuit_ready = self.recipe_unlocked("analyze_circuit");
        let field_ready = self.recipe_unlocked("analyze_field_sample");
        let exotic_ready = self.recipe_unlocked("analyze_exotic_reaction");
        let crush_ready = self.recipe_unlocked("crush_iron");
        let cap = 600.0; // pool ceiling — covers the costliest single node (500) with margin.
        let stone_ok = self.hub_stored("stone") > 80;
        let iron_ore_ok = self.hub_stored("iron_ore") > 60;

        // material: 4 stone → 10 material.
        if stone_ok && self.research_points("material") < cap {
            self.ensure_jobs("basic_analysis", 8);
        }
        // Keep iron flowing for the ore_crusher/plate_roller PRODUCTION milestones without draining
        // raw iron_ore below the machine-body reserve.
        if iron_ore_ok && self.hub_stored("iron_ingot") < 200 {
            self.ensure_jobs("smelt_metal__iron", 8);
        }
        // engineering: circuit_board → 20 engineering (copper→wire→circuit chain).
        if circuit_ready && self.research_points("engineering") < cap {
            if self.hub_stored("copper_ingot") < 60 {
                self.ensure_jobs("smelt_metal__copper", 8);
            }
            if self.hub_stored("copper_wire") < 60 {
                self.ensure_jobs("draw_metal__copper", 8);
            }
            if self.hub_stored("circuit_board") < 25 {
                self.ensure_jobs("make_circuit", 8);
            }
            self.ensure_jobs("analyze_circuit", 8);
        }
        // Crush iron to clear the ore_washer(50 iron_crushed) milestone.
        if crush_ready && iron_ore_ok && self.produced("iron_crushed") < 60.0 {
            self.ensure_jobs("crush_iron", 8);
        }
        // discovery: field_sample → 12 discovery.
        if field_ready && self.research_points("discovery") < cap {
            self.ensure_jobs("analyze_field_sample", 8);
        }
        // synthesis: resonite_shard → 20 synthesis.
        if exotic_ready && self.research_points("synthesis") < cap {
            self.ensure_jobs("analyze_exotic_reaction", 8);
        }
    }

    /// Fold this frame's world state into the accumulating [`RunReport`] (node unlocks, tier
    /// completions, research-curve samples). Split off `self.report` first so the report's
    /// mutable borrow doesn't collide with the immutable world read.
    fn capture(&mut self) {
        let secs = self.virtual_secs();
        let mut report = std::mem::take(&mut self.report);
        report.observe(self.app.world(), secs);
        report.observe_flags(self.app.world());
        self.report = report;
    }

    // ── queries / requests ──────────────────────────────────────────────────────────────────

    pub fn machine_entity(&mut self, machine_type: &str) -> Entity {
        let mut q = self.app.world_mut().query::<(Entity, &Machine)>();
        q.iter(self.app.world())
            .find(|(_, m)| m.machine_type == machine_type)
            .map(|(e, _)| e)
            .unwrap_or_else(|| panic!("no placed machine of type {machine_type}"))
    }

    pub fn machine_at(&mut self, machine_type: &str, pos: Vec3) -> Entity {
        let mut q = self
            .app
            .world_mut()
            .query::<(Entity, &Machine, &Transform)>();
        q.iter(self.app.world())
            .find(|(_, m, t)| m.machine_type == machine_type && t.translation.distance(pos) < 0.6)
            .map(|(e, _, _)| e)
            .unwrap_or_else(|| panic!("no {machine_type} placed at {pos:?}"))
    }

    /// The logistics network a machine's ports have joined.
    pub fn net_of(&mut self, machine: Entity) -> Entity {
        let ports: Vec<Entity> = {
            let mut q = self.app.world_mut().query::<(Entity, &LogisticsPortOf)>();
            q.iter(self.app.world())
                .filter(|(_, p)| p.0 == machine)
                .map(|(e, _)| e)
                .collect()
        };
        ports
            .iter()
            .find_map(|&p| {
                self.app
                    .world()
                    .get::<LogisticsNetworkMember>(p)
                    .map(|m| m.0)
            })
            .expect("machine port should have joined a network")
    }

    /// How many of `item` sit in `storage`'s `StorageUnit`.
    pub fn stored(&self, storage: Entity, item: &str) -> u32 {
        self.app
            .world()
            .get::<StorageUnit>(storage)
            .and_then(|s| s.items.get(item).copied())
            .unwrap_or(0)
    }

    /// How many of `item` sit in the bootstrap hub crate.
    pub fn hub_stored(&self, item: &str) -> u32 {
        self.stored(self.storage_e, item)
    }

    pub fn research_points(&self, theme: &str) -> f32 {
        self.app.world().resource::<ResearchPool>().get(theme)
    }

    pub fn produced(&self, item: &str) -> f32 {
        self.app.world().resource::<ProductionTally>().get(item)
    }

    pub fn node_unlocked(&self, node: &str) -> bool {
        self.app
            .world()
            .resource::<TechTreeProgress>()
            .unlocked_nodes
            .contains(node)
    }

    pub fn recipe_unlocked(&self, recipe: &str) -> bool {
        self.app
            .world()
            .resource::<TechTreeProgress>()
            .unlocked_recipes
            .contains(recipe)
    }

    pub fn is_completed(&self) -> bool {
        *self.app.world().resource::<RunState>() == RunState::Completed
    }

    pub fn virtual_secs(&self) -> f32 {
        self.app.world().resource::<Time>().elapsed_secs()
    }

    pub fn set_game_state(&mut self, state: GameState) {
        self.app
            .world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(state);
    }

    pub fn enter_drone_pilot(&mut self) {
        self.app
            .world_mut()
            .resource_mut::<NextState<PlayMode>>()
            .set(PlayMode::DronePilot);
    }

    pub fn reveal_fog(&mut self, cell: IVec2) {
        self.app
            .world_mut()
            .write_message(FogCellRevealedEvent { cell });
    }

    pub fn request_node(&mut self, node: &str) {
        self.app
            .world_mut()
            .write_message(UnlockNodeRequest(node.into()));
    }

    // ── the prescriptive step interpreter ───────────────────────────────────────────────────────

    /// Run `spec` to victory by executing its prescriptive `steps` list. Dispatches on nothing —
    /// every scenario is a step list now (the `difficulty` only sets the tier cap inside
    /// [`Scenario::run_steps`]).
    pub fn run(&mut self, spec: &ScenarioSpec) -> RunReport {
        assert!(
            !spec.steps.is_empty(),
            "scenario `{}` has no steps — a prescriptive scenario must list its run",
            spec.name
        );
        self.run_steps(spec)
    }

    /// Execute a prescriptive scenario: apply the tier cap, run every [`Step`] in order, then
    /// finalize the report (completion, virtual time, mined ore). Positions are auto-assigned — a
    /// distinct X lane per machine type, copies stacked in Z (see [`StepState::next_lane`]).
    pub fn run_steps(&mut self, spec: &ScenarioSpec) -> RunReport {
        self.report.name = spec.name.clone();
        self.app
            .insert_resource(TierCap(spec.difficulty.max_tier()));

        let mut st = StepState::default();
        for step in &spec.steps {
            self.exec_step(step, spec, &mut st);
        }

        self.capture();
        self.report.completed = self.is_completed();
        self.report.virtual_secs = self.virtual_secs();
        for (ore, deposit) in &st.mined {
            let amt = self
                .app
                .world()
                .get::<OreDeposit>(*deposit)
                .map(|o| o.total_extracted)
                .unwrap_or(0.0);
            self.report.ore_extracted.push((ore.clone(), amt));
        }
        self.report.clone()
    }

    /// Dispatch one [`Step`] to the matching real-systems mechanic. Time-advancing steps guard on
    /// `spec.max_secs`; `bind` names a placed machine in `st` for a later `Install`.
    fn exec_step(&mut self, step: &Step, spec: &ScenarioSpec, st: &mut StepState) {
        let max = spec.max_secs;
        match step {
            Step::Deploy {
                machine,
                powered,
                count,
                on,
                bind,
            } => self.step_place(st, machine, *powered, *count, on, bind, max, true),
            Step::Place {
                machine,
                powered,
                count,
                on,
                bind,
            } => self.step_place(st, machine, *powered, *count, on, bind, max, false),
            Step::Craft { item, count } => {
                self.craft(item, *count);
                self.advance_until(0.5, max, |s| s.hub_stored(item) >= *count);
            }
            Step::Ensure { recipe, count } => self.ensure_jobs(recipe, *count),
            Step::Research { node } => {
                self.run_until(
                    0.5,
                    max,
                    |s| s.request_node(node),
                    |s| s.node_unlocked(node),
                );
            }
            Step::Recon { ore } => {
                self.recon_deposit(ore);
            }
            Step::Scan => {
                self.enter_drone_pilot();
                self.app.update();
                self.reveal_fog(IVec2::ZERO);
                self.app.update();
            }
            Step::Install { machine, module } => {
                let e = *st.binds.get(machine).unwrap_or_else(|| {
                    panic!("Install: no machine bound as `{machine}` — Deploy it with `bind: \"{machine}\"` first")
                });
                self.own_or_craft(module, max);
                self.install_config(e, module);
            }
            Step::Pump(on) => self.feed_economy = *on,
            Step::Build { jobs } => self.step_build(jobs, max),
        }
        // Bind the hub network as soon as the first logistics cable has formed it (the legacy
        // drivers call `bind_network()` explicitly; the interpreter does it lazily). No-op once set.
        self.maybe_bind_network();
    }

    /// Bind the hub network the first time the bootstrap crate's port has joined one (a logistics
    /// cable from the hub to any machine forms it). Idempotent — returns immediately once bound.
    fn maybe_bind_network(&mut self) {
        if self.net != Entity::PLACEHOLDER {
            return;
        }
        let storage = self.storage_e;
        let joined = {
            let mut q = self
                .app
                .world_mut()
                .query::<(&LogisticsPortOf, &LogisticsNetworkMember)>();
            q.iter(self.app.world()).any(|(po, _)| po.0 == storage)
        };
        if joined {
            self.bind_network();
        }
    }

    /// Craft `item` into storage if none is owned, then wait for it (used before placing a machine
    /// or installing a config module).
    fn own_or_craft(&mut self, item: &str, max_secs: f32) {
        if self.hub_stored(item) < 1 {
            self.craft(item, 1);
            self.advance_until(0.5, max_secs, |s| s.hub_stored(item) >= 1);
        }
    }

    /// Make sure one `machine` is in storage before it's placed. `Deploy` (`do_craft`) enqueues its
    /// default recipe and waits; `Place` only waits — so a preceding `Ensure(specific_recipe)` (for
    /// a machine with several recipes, e.g. the minimal launch site) can finish crafting it.
    fn ensure_owned(&mut self, machine: &str, max_secs: f32, do_craft: bool) {
        if do_craft {
            self.own_or_craft(machine, max_secs);
        } else {
            self.advance_until(0.5, max_secs, |s| s.hub_stored(machine) >= 1);
        }
    }

    /// Place `count` copies of `machine`, wiring each onto the hub network (+ power if `powered`).
    /// `do_craft` first owns-or-crafts each copy (`Deploy`) vs. requiring it already owned (`Place`).
    /// `miner`s route through `on`; solar generators anchor the first copy then panel the rest.
    #[allow(clippy::too_many_arguments)]
    fn step_place(
        &mut self,
        st: &mut StepState,
        machine: &str,
        powered: bool,
        count: usize,
        on: &Option<MineTarget>,
        bind: &Option<String>,
        max_secs: f32,
        do_craft: bool,
    ) {
        if machine == "miner" {
            match on {
                Some(MineTarget::Origin) | None => {
                    // The owned kit miner onto the origin deposit.
                    let pos = self.origin_pos();
                    let hub = self.storage_pos();
                    self.place_real("miner", pos);
                    self.wire_logi(hub, pos);
                    self.app.update();
                }
                Some(MineTarget::Vein(ore)) => {
                    // Craft the miners and place them on a fresh vein (always crafted, so `Place`
                    // and `Deploy` behave the same here).
                    let deposit = self.craft_and_mine(ore, count, true);
                    st.mined.push((ore.clone(), deposit));
                }
            }
            return;
        }

        for _ in 0..count {
            if machine == "solar_generator" && !st.generator_anchored {
                // The first generator is the power anchor every other machine wires to.
                self.ensure_owned("solar_generator", max_secs, do_craft);
                let gp = self.generator_pos();
                self.place_real("solar_generator", gp);
                self.app.update();
                st.generator_anchored = true;
                continue;
            }
            self.ensure_owned(machine, max_secs, do_craft);
            let (x, i) = st.next_lane(machine);
            let pos = self.lane(x, i);
            let e = if machine == "solar_generator" {
                self.deploy_panel(pos)
            } else {
                self.deploy(machine, pos, powered)
            };
            if let Some(name) = bind {
                st.binds.insert(name.clone(), e);
            }
        }
    }

    /// The terminal step: disarm the auto-feed pump, clear the craft queue, enqueue the successor
    /// build list (hand-pushed so shared intermediates aren't double-reserved), then run to
    /// `RunState::Completed`. Records the `build_enqueued` / `launch_ran` report outcomes.
    fn step_build(&mut self, jobs: &[(String, usize)], max_secs: f32) {
        self.feed_economy = false;
        {
            let net = self.net;
            let mut queue = self
                .app
                .world_mut()
                .get_mut::<NetworkCraftQueue>(net)
                .expect("logistics network carries a craft queue");
            queue.jobs.clear();
            queue.reserved.clear();
            for (recipe, n) in jobs {
                for _ in 0..*n {
                    queue.jobs.push_back(QueuedJob {
                        recipe_id: recipe.clone(),
                    });
                }
            }
        }
        self.report.build_enqueued = true;

        let mut launch_ran = false;
        self.run_until(
            0.5,
            max_secs,
            |s| {
                if s.any_running("launch_site") {
                    launch_ran = true;
                }
            },
            |s| s.is_completed(),
        );
        self.report.launch_ran = launch_ran;
    }

    /// True if any placed machine of `machine_type` is currently `Running`.
    pub fn any_running(&self, machine_type: &str) -> bool {
        self.app.world().iter_entities().any(|e| {
            e.get::<Machine>()
                .is_some_and(|m| m.machine_type == machine_type)
                && matches!(e.get::<MachineState>(), Some(MachineState::Running))
        })
    }
}

/// Interpreter bookkeeping for one prescriptive run: per-machine-type placement lanes, the
/// power-anchor flag, name→entity binds for `Install`, and the veins mined (for the ore report).
#[derive(Default)]
struct StepState {
    /// machine type → (X lane, next copy's Z index).
    lanes: HashMap<String, (f32, usize)>,
    /// Whether the first solar generator (the power anchor) has been placed.
    generator_anchored: bool,
    /// `bind` name → placed machine entity, for a later `Install`.
    binds: HashMap<String, Entity>,
    /// `(ore_id, deposit)` for each vein a `Deploy(miner, on: Vein(..))` mined.
    mined: Vec<(String, Entity)>,
}

impl StepState {
    /// Allocate the next placement lane for `machine`: a fresh X the first time the type appears
    /// (spaced so types don't overlap), then successive Z indices for each copy.
    fn next_lane(&mut self, machine: &str) -> (f32, usize) {
        let n = self.lanes.len();
        let slot = self
            .lanes
            .entry(machine.to_string())
            .or_insert_with(|| (15.0 + 15.0 * n as f32, 0));
        let i = slot.1;
        slot.1 += 1;
        (slot.0, i)
    }
}

/// Builds the headless app: minimal plugins + the asset stores machine startup systems expect (no
/// renderer here) + the real game plugins under test + a bare drone-recon discovery system.
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
        // Fog reveal events come from DronePlugin in the real game; it is not in this headless
        // test (avian physics), so register the message the reveal system reads.
        .add_message::<FogCellRevealedEvent>()
        // EscapePlugin writes RunEndEvent (normally registered by SavePlugin, which we omit here
        // to avoid its persistence deps); register the message it needs directly.
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
            // Real landing: spawns the escape pod + a networked storage_crate stocked with the
            // starting kit (spawn_escape_pod + stock_bootstrap_storage), the bootstrap the run
            // draws its first machines from.
            PodPlugin,
        ))
        // Real exotic-site recon: the full DronePlugin pulls avian physics we can't run headless,
        // so register just the discovery system it owns (gated on DronePilot, same as the game).
        .add_systems(
            Update,
            deposit_discovery_system.run_if(in_state(PlayMode::DronePilot)),
        );
    app
}

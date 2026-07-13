//! Reusable end-to-end **run harness** for authoring landing→victory scenarios on simulated
//! time. `Scenario` owns the headless `App` plus the landing anchors (the bootstrap storage
//! crate, its logistics network, the power-anchor position) and exposes the proven, real-systems
//! mechanics as methods: inventory-gated placement (`place_real`/`deploy`), crafting into
//! networked storage (`craft`/`push_jobs`/`ensure_jobs`), mining (`craft_and_mine`), drone recon
//! (`recon_deposit`), and simulated-time drivers (`advance_until`/`run_until`). The composites do
//! the heavy lifting: `run()` dispatches on difficulty to `run_standard()` (the whole scripted
//! standard run) or `run_initiation()` (the tier-3 minimal-successor escape), both driven from a
//! [`ScenarioSpec`]; `drive_to_victory()` is the standard run's earned-research grind.
//!
//! NOTHING gameplay-relevant is injected: storage comes from the real landing crate + crafted
//! crates, generator watts are the seed-scaled values `place_machine_system` assigns, every tech
//! node is earned through the real `UnlockNodeRequest` grind, and every machine is OWNED (kit or
//! crafted) before it is placed. The only headless stubs are the render/physics context a real
//! client would provide: simulated-time fast-forward (`ManualDuration`), `MachinePortLayout`
//! stubs (no GLTF port metadata headless), GLTF/scene asset stubs, and a bare `Drone` for recon.

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
    InstallConfigModule, LogisticsNetworkMember, LogisticsSimPlugin, NetworkCraftQueue, QueuedJob,
    StorageUnit,
};
use exergon::machine::{
    LogisticsPortOf, Machine, MachinePlugin, MachinePortLayout, MachinePortLayouts, MachineState,
    ManualCraftOnly, MinerMachine,
};
use exergon::planet::{Planet, PlanetPlugin, PlanetPropertyVisibility, PropertyVisibility};
use exergon::pod::PodPlugin;
use exergon::power::{GeneratorUnit, PowerPlugin};
use exergon::recipe_graph::{RecipeGraph, RecipeGraphPlugin};
use exergon::research::{
    Discovered, ProductionTally, ResearchPlugin, ResearchPool, TechTreeProgress, TierCap,
    UnlockNodeRequest,
};
use exergon::seed::DomainSeeds;
use exergon::tech_tree::TechTreePlugin;
use exergon::world::{
    CableConnectionEvent, MainCamera, OreDeposit, WorldObjectEvent, WorldObjectKind, WorldgenPlugin,
};
use exergon::{GameState, PlayMode};

use crate::report::RunReport;
use crate::spec::ScenarioSpec;

/// Logistics ports sit one unit +X of a machine; energy ports one unit −X, so a power cable and a
/// logistics cable to the same machine snap to different port entities.
const PORT_OFFSET: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const ENERGY_OFFSET: Vec3 = Vec3::new(-1.0, 0.0, 0.0);

/// The physical half of a "standard escape victory": the mass-balanced successor build list + the
/// two exotic recon sites. The research half — the four themed target-node lists — drives the run
/// through the armed [`PumpPlan`] ([`Scenario::activate_pump`]), so it is no longer carried here.
pub struct GrindPlan<'a> {
    pub build_jobs: &'a [(&'a str, usize)],
    pub fluxite_site: Entity,
    pub cryophase_deposit: Entity,
    /// Runaway guard for the grind, in simulated seconds.
    pub max_secs: f32,
}

/// The four themed research target lists, owned so a run can keep pumping them across every
/// simulated-time wait (not just the final grind). Set once via [`Scenario::activate_pump`]; while
/// active, [`Scenario::advance_until`]/[`Scenario::run_until`] request every node and top up each
/// affordable analysis chain each frame, so a node unlocks the instant it is affordable instead of
/// waiting for the grind phase. Each list self-orders by the real tech graph (a request is a no-op
/// until prereqs are met and the pool can pay).
#[derive(Clone, Default)]
pub struct PumpPlan {
    pub material_nodes: Vec<String>,
    pub engineering_nodes: Vec<String>,
    pub discovery_nodes: Vec<String>,
    pub synthesis_nodes: Vec<String>,
}

/// What the victory grind observed, for the caller's post-run regression assertions.
pub struct DriveOutcome {
    pub launch_ran: bool,
    pub build_enqueued: bool,
    pub ever_analyzed_circuit: bool,
    pub ever_analyzed_exotic: bool,
}

/// A landing→victory run in progress. `app` is public as an escape hatch for one-off world reads
/// the typed methods don't cover; the anchors (`storage`/`net`/`generator_pos`) are set by
/// `land()` + `bind_network()` and read through getters. `report` accumulates milestones as the
/// time-advancing drivers run.
pub struct Scenario {
    pub app: App,
    storage_e: Entity,
    storage_pos: Vec3,
    generator_pos: Vec3,
    net: Entity,
    origin_deposit: Entity,
    origin_pos: Vec3,
    origin_ores: Vec<(String, f32)>,
    report: RunReport,
    /// When set, every simulated-time wait pumps the four-currency economy (see [`PumpPlan`]).
    active_pump: Option<PumpPlan>,
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
            origin_deposit: Entity::PLACEHOLDER,
            origin_pos: Vec3::ZERO,
            origin_ores: Vec::new(),
            report: RunReport {
                seed,
                ..Default::default()
            },
            active_pump: None,
        };
        s.land();
        s
    }

    /// Enters Loading (worldgen), records the origin-chunk starter deposit, stubs every machine's
    /// port layout (no GLTF headless), then enters Playing so the real PodPlugin lands the pod +
    /// a stocked `storage_crate`. Locates that crate as the bootstrap hub and fixes the power
    /// anchor at its y. Called by `new()` — scenarios never invoke it directly.
    fn land(&mut self) {
        self.set_game_state(GameState::Loading);
        for _ in 0..4 {
            self.app.update();
        }

        let (deposit_e, deposit_tf, ores) = {
            let mut q = self
                .app
                .world_mut()
                .query::<(Entity, &Transform, &OreDeposit)>();
            q.iter(self.app.world())
                .find(|(_, _, d)| d.chunk_pos == IVec2::ZERO)
                .map(|(e, t, d)| (e, *t, d.ores.clone()))
                .expect("world generation must place a deposit on the origin chunk")
        };
        self.origin_deposit = deposit_e;
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

    pub fn storage(&self) -> Entity {
        self.storage_e
    }
    pub fn storage_pos(&self) -> Vec3 {
        self.storage_pos
    }
    pub fn generator_pos(&self) -> Vec3 {
        self.generator_pos
    }
    pub fn net(&self) -> Entity {
        self.net
    }
    pub fn origin_deposit(&self) -> Entity {
        self.origin_deposit
    }
    pub fn origin_pos(&self) -> Vec3 {
        self.origin_pos
    }
    pub fn origin_ores(&self) -> &[(String, f32)] {
        &self.origin_ores
    }
    /// The milestone/statistics report accumulated so far.
    pub fn report(&self) -> &RunReport {
        &self.report
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

    /// Hand-push `n` copies of `recipe` onto the hub network's craft queue — the direct queue
    /// push the craft modal performs.
    pub fn push_jobs(&mut self, recipe: &str, n: usize) {
        let mut queue = self
            .app
            .world_mut()
            .get_mut::<NetworkCraftQueue>(self.net)
            .expect("logistics network carries a craft queue");
        for _ in 0..n {
            queue.jobs.push_back(QueuedJob {
                recipe_id: recipe.to_string(),
            });
        }
    }

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

    /// Arm the four-currency pump for the rest of the run (see [`PumpPlan`]). Idempotent per run —
    /// call once the material loop exists; every subsequent simulated-time wait then requests the
    /// target nodes and tops up the affordable analysis chains, so unlocks fire as soon as they are
    /// affordable rather than waiting for the grind phase.
    pub fn activate_pump(&mut self, spec: &ScenarioSpec) {
        self.active_pump = Some(PumpPlan {
            material_nodes: spec.material_nodes.clone(),
            engineering_nodes: spec.engineering_nodes.clone(),
            discovery_nodes: spec.discovery_nodes.clone(),
            synthesis_nodes: spec.synthesis_nodes.clone(),
        });
    }

    /// One pump step, if armed. Take/restore the plan so `pump_frame` can borrow `self` mutably.
    fn pump(&mut self) {
        if let Some(plan) = self.active_pump.take() {
            self.pump_frame(&plan);
            self.active_pump = Some(plan);
        }
    }

    /// One frame of the four-currency economy: request every target node (each self-orders by the
    /// real tech graph — a no-op until prereqs are met and the pool can pay) and top up each themed
    /// analysis chain that is unlocked, below the pool ceiling, and still has locked nodes. Shared
    /// by the setup stages (through the pumped time-drivers) and [`Scenario::drive_to_victory`].
    fn pump_frame(&mut self, plan: &PumpPlan) {
        for node in plan
            .material_nodes
            .iter()
            .chain(&plan.engineering_nodes)
            .chain(&plan.discovery_nodes)
            .chain(&plan.synthesis_nodes)
        {
            self.request_node(node);
        }

        // Per-theme "all target nodes unlocked?" — once true, stop feeding that theme's chain.
        let done = |s: &Scenario, nodes: &[String]| nodes.iter().all(|n| s.node_unlocked(n));
        let mat_done = done(self, &plan.material_nodes);
        let eng_done = done(self, &plan.engineering_nodes);
        let disc_done = done(self, &plan.discovery_nodes);
        let synth_done = done(self, &plan.synthesis_nodes);
        let circuit_ready = self.recipe_unlocked("analyze_circuit");
        let field_ready = self.recipe_unlocked("analyze_field_sample");
        let exotic_ready = self.recipe_unlocked("analyze_exotic_reaction");
        let crush_ready = self.recipe_unlocked("crush_iron");
        let cap = 600.0; // pool ceiling — covers the costliest single node (500) with margin.

        // Raw-ore reserves: because this pump now runs during the setup stages too, its raw-ore
        // consumers must leave enough stone/iron_ore for the machine-body crafts those stages queue
        // from the SAME hub (e.g. make_smelter = 20 stone + 10 iron_ore). Without the reserves the
        // easy analysis/smelt chains drain the hub and the smelter/crusher/… bodies never craft.
        let stone_ok = self.hub_stored("stone") > 80;
        let iron_ore_ok = self.hub_stored("iron_ore") > 60;

        // material: 4 stone → 10 material.
        if !mat_done && stone_ok && self.research_points("material") < cap {
            self.ensure_jobs("basic_analysis", 8);
        }
        // Keep iron flowing to reach the ore_crusher(100)/plate_roller(150) PRODUCTION milestones,
        // but cap the STOCK so raw iron_ore stays free for the machine bodies.
        if iron_ore_ok && self.hub_stored("iron_ingot") < 200 {
            self.ensure_jobs("smelt_metal__iron", 8);
        }
        // engineering: circuit_board → 20 engineering (copper→wire→circuit chain).
        if !eng_done && circuit_ready && self.research_points("engineering") < cap {
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
        // Crush iron to clear the ore_washer(50 iron_crushed) milestone (needs a crusher).
        if crush_ready && iron_ore_ok && self.produced("iron_crushed") < 60.0 {
            self.ensure_jobs("crush_iron", 8);
        }
        // discovery: field_sample → 12 discovery.
        if !disc_done && field_ready && self.research_points("discovery") < cap {
            self.ensure_jobs("analyze_field_sample", 8);
        }
        // synthesis: resonite_shard → 20 synthesis.
        if !synth_done && exotic_ready && self.research_points("synthesis") < cap {
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

    /// The generated planet's per-property visibility (the reveal system mutates this in place).
    pub fn planet_vis(&mut self) -> PlanetPropertyVisibility {
        let mut q = self
            .app
            .world_mut()
            .query_filtered::<&PlanetPropertyVisibility, With<Planet>>();
        q.single(self.app.world())
            .cloned()
            .expect("generate_planet_properties must have spawned the run's planet")
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

    /// True once every node id in `nodes` is unlocked — a theme's grind is "done".
    pub fn nodes_unlocked(&self, nodes: &[&str]) -> bool {
        let prog = self.app.world().resource::<TechTreeProgress>();
        nodes.iter().all(|n| prog.unlocked_nodes.contains(*n))
    }

    pub fn recipe_unlocked(&self, recipe: &str) -> bool {
        self.app
            .world()
            .resource::<TechTreeProgress>()
            .unlocked_recipes
            .contains(recipe)
    }

    pub fn machine_state(&self, e: Entity) -> Option<MachineState> {
        self.app.world().get::<MachineState>(e).copied()
    }

    pub fn discovered(&self, deposit: Entity) -> bool {
        self.app.world().get::<Discovered>(deposit).is_some()
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

    // ── the whole scripted standard run ─────────────────────────────────────────────────────

    /// Drive an entire standard run from landing to `launch_successor`, parameterized by `spec`.
    /// This is the scripted choreography the e2e smoke test proves and the `scenario` binary
    /// replays for balancing: deploy the kit, earn the early research tiers, craft + place the
    /// processing economy, then hand the four-currency grind to [`Scenario::drive_to_victory`].
    /// Stage observations land in [`Scenario::report`]; returns a clone of that report.
    pub fn run_standard(&mut self, spec: &ScenarioSpec) -> RunReport {
        self.report.name = spec.name.clone();
        // Apply the run's difficulty tier ceiling (the harness spawns no RunSaveHeader, so
        // `sync_tier_cap` never fires — set the cap directly).
        self.app
            .insert_resource(TierCap(spec.difficulty.max_tier()));

        let deposit_e = self.origin_deposit();
        let deposit_pos = self.origin_pos();
        let storage_pos = self.storage_pos();
        let generator_pos = self.generator_pos();

        // Stage 0 — deploy the four kit machines through the real inventory-gated path.
        self.place_real("solar_generator", generator_pos);
        self.app.update();
        let generator_e = self.machine_at("solar_generator", generator_pos);

        self.place_real("miner", deposit_pos);
        self.wire_logi(storage_pos, deposit_pos);
        self.app.update();
        let miner_e = self.machine_at("miner", deposit_pos);
        self.report.kit_miner_latched = self
            .app
            .world()
            .get::<MinerMachine>(miner_e)
            .map(|m| m.deposit)
            == Some(deposit_e);

        let assembler_pos = self.lane(25.0, 0);
        let station_pos = self.lane(10.0, 0);
        let assembler_e = self.deploy("assembler", assembler_pos, true);
        let station_e = self.deploy("analysis_station", station_pos, true);

        self.wire_logi(storage_pos, deposit_pos);
        self.app.update();
        self.bind_network();

        let net = self.net();
        self.report.networks_shared = net == self.net_of(miner_e)
            && net == self.net_of(assembler_e)
            && net == self.net_of(station_e);

        // Arm the four-currency pump for the rest of the run: from here every simulated-time wait
        // requests the target nodes + tops up affordable analysis chains, so a node unlocks the
        // instant it is affordable instead of waiting for the grind phase (spreads unlocks out).
        self.activate_pump(spec);

        // Stage 1 — earn ore_extraction. Queue basic_analysis and let the mine→analyse loop run.
        let mut station_ran = false;
        self.run_until(
            0.5,
            4_000.0,
            |s| s.ensure_jobs("basic_analysis", 12),
            |s| {
                if s.machine_state(station_e) == Some(MachineState::Running) {
                    station_ran = true;
                }
                s.research_points("material") >= 30.0
            },
        );
        self.report.station_ran = station_ran;
        self.request_node("ore_extraction");
        self.app.update();

        // Stage 1b — first research spend reveals both atmospheric properties.
        self.app.update();
        let vis = self.planet_vis();
        self.report.oxygen_revealed = vis.atmospheric_oxygen == PropertyVisibility::Revealed;
        self.report.pressure_revealed = vis.atmospheric_pressure == PropertyVisibility::Revealed;

        // Stage 2 — sustained grind to basic_processing.
        self.run_until(
            0.5,
            12_000.0,
            |s| s.ensure_jobs("basic_analysis", 12),
            |s| s.research_points("material") >= 150.0,
        );
        self.request_node("basic_processing");
        self.app.update();

        // Stage 3 — CRAFT a smelter, place + power it, prove the energy-gated smelt.
        self.craft("smelter", 1);
        self.advance_until(0.5, 6_000.0, |s| s.hub_stored("smelter") >= 1);
        let smelter_pos = self.lane(15.0, 0);
        let smelter_e = self.deploy("smelter", smelter_pos, true);
        self.push_jobs("smelt_metal__iron", 3);
        let mut smelter_ran = false;
        self.advance_until(0.25, 6_000.0, |s| {
            if s.machine_state(smelter_e) == Some(MachineState::Running) {
                smelter_ran = true;
            }
            s.hub_stored("iron_ingot") >= 1
        });
        self.report.smelter_ran = smelter_ran;
        self.report.generator_charged = self
            .app
            .world()
            .get::<GeneratorUnit>(generator_e)
            .is_some_and(|g| g.buffer_joules > 0.0);

        // Stage 4 — CRAFT a wire_drawer + a second assembler, run copper→wire→circuit for real.
        self.craft("wire_drawer", 1);
        self.craft("assembler", 1);
        self.advance_until(0.5, 8_000.0, |s| {
            s.hub_stored("wire_drawer") >= 1 && s.hub_stored("assembler") >= 1
        });
        let drawer_pos = self.lane(20.0, 0);
        let assembler2_pos = self.lane(25.0, 1);
        let _drawer_e = self.deploy("wire_drawer", drawer_pos, true);
        let assembler2_e = self.deploy("assembler", assembler2_pos, true);

        self.push_jobs("smelt_metal__iron", 2);
        self.push_jobs("smelt_metal__copper", 2);
        self.push_jobs("draw_metal__copper", 2);
        self.push_jobs("make_circuit", 1);
        let mut assembler_ran = false;
        self.advance_until(0.25, 8_000.0, |s| {
            if s.machine_state(assembler_e) == Some(MachineState::Running)
                || s.machine_state(assembler2_e) == Some(MachineState::Running)
            {
                assembler_ran = true;
            }
            s.hub_stored("circuit_board") >= 1
        });
        self.report.assembler_ran = assembler_ran;

        // Stage 5 — drone scan reveals geological activity.
        self.enter_drone_pilot();
        self.app.update();
        self.reveal_fog(IVec2::ZERO);
        self.app.update();
        self.report.geo_revealed_after_scan =
            self.planet_vis().geological_activity == PropertyVisibility::Qualitative;

        // Stage 6 — the full Standard victory with a REAL machine economy and EARNED research.
        let xalite_site = self.recon_deposit("xalite");
        self.report.xalite_discovered = self.discovered(xalite_site);

        // Scale up the early factory: two more analysis stations, a second smelter, a solar farm.
        self.craft("analysis_station", 2);
        self.craft("smelter", 1);
        self.advance_until(0.5, 25_000.0, |s| {
            s.hub_stored("analysis_station") >= 2 && s.hub_stored("smelter") >= 1
        });
        let _station2 = self.deploy("analysis_station", self.lane(10.0, 1), true);
        let _station3 = self.deploy("analysis_station", self.lane(10.0, 2), true);
        let _smelter2 = self.deploy("smelter", self.lane(15.0, 1), true);

        let panels = 6usize;
        self.craft("solar_generator", panels as u32);
        self.advance_until(0.5, 30_000.0, |s| {
            s.hub_stored("solar_generator") >= panels as u32
        });
        let mut farm: Vec<Entity> = Vec::new();
        for i in 0..panels {
            farm.push(self.deploy_panel(self.lane(60.0, i)));
        }
        self.advance_until(1.0, 6_000.0, |s| {
            farm.iter().all(|&e| {
                s.app
                    .world()
                    .get::<GeneratorUnit>(e)
                    .is_some_and(|g| g.buffer_joules >= g.max_buffer_joules * 0.9)
            })
        });

        // Mine every raw material for real (miners crafted first via `craft_and_mine`).
        let iron_copper_deposit = self.craft_and_mine("copper_ore", 2, true);
        self.craft_and_mine("resonite_shard", 1, false);
        self.craft_and_mine("aluminum_ore", 1, false);
        self.craft_and_mine("titanium_ore", 1, false);
        self.craft_and_mine("coal", 1, false);
        let fluxite_site = self.craft_and_mine("fluxite_shard", 1, false);
        let cryophase_deposit = self.craft_and_mine("cryophase_shard", 2, false);

        // Build the grind plan from the spec's build jobs + recon sites. The four target-node lists
        // drive the run through the armed pump (see `activate_pump`), not through the plan.
        let build_jobs: Vec<(&str, usize)> = spec
            .build_jobs
            .iter()
            .map(|(r, n)| (r.as_str(), *n))
            .collect();

        let plan = GrindPlan {
            build_jobs: &build_jobs,
            fluxite_site,
            cryophase_deposit,
            max_secs: spec.max_secs,
        };
        let outcome = self.drive_to_victory(&plan);

        // Finalize the report.
        self.capture();
        self.report.build_enqueued = outcome.build_enqueued;
        self.report.launch_ran = outcome.launch_ran;
        self.report.ever_analyzed_circuit = outcome.ever_analyzed_circuit;
        self.report.ever_analyzed_exotic = outcome.ever_analyzed_exotic;
        self.report.completed = self.is_completed();
        self.report.virtual_secs = self.virtual_secs();

        let extracted = |world: &World, d: Entity| -> f32 {
            world
                .get::<OreDeposit>(d)
                .map(|o| o.total_extracted)
                .unwrap_or(0.0)
        };
        self.report.ore_extracted = vec![
            (
                "iron_copper_vein".to_string(),
                extracted(self.app.world(), iron_copper_deposit),
            ),
            (
                "cryophase_shard".to_string(),
                extracted(self.app.world(), cryophase_deposit),
            ),
        ];

        self.report.clone()
    }

    /// Run `spec` to victory, dispatching on difficulty: `Initiation` → the tier-3
    /// minimal-successor escape ([`Scenario::run_initiation`]); any other difficulty → the full
    /// standard run ([`Scenario::run_standard`]).
    pub fn run(&mut self, spec: &ScenarioSpec) -> RunReport {
        match spec.difficulty {
            exergon::save::DifficultyTier::Initiation => self.run_initiation(spec),
            _ => self.run_standard(spec),
        }
    }

    // ── the Initiation (tier-3) run ─────────────────────────────────────────────────────────

    /// Drive a full **Initiation** run (tier cap 3) from landing to the minimal-successor launch.
    /// The tech tree is capped at tier 3, so this earns only the T1–T3 path and builds the
    /// `minimal_successor` escape: craft a launch site through `make_launch_site__minimal` (steel +
    /// circuit + silicon, no tier-4 titanium) and run `launch_minimal_successor` on it — the same
    /// escape engine that wins the standard run. Every machine + material is earned for real.
    pub fn run_initiation(&mut self, spec: &ScenarioSpec) -> RunReport {
        self.report.name = spec.name.clone();
        self.app
            .insert_resource(TierCap(spec.difficulty.max_tier()));

        // Stage 0 — kit deploy (same inventory-gated path as the standard run).
        let deposit_e = self.origin_deposit();
        let deposit_pos = self.origin_pos();
        let storage_pos = self.storage_pos();
        let generator_pos = self.generator_pos();

        self.place_real("solar_generator", generator_pos);
        self.app.update();
        let _generator_e = self.machine_at("solar_generator", generator_pos);

        self.place_real("miner", deposit_pos);
        self.wire_logi(storage_pos, deposit_pos);
        self.app.update();
        let miner_e = self.machine_at("miner", deposit_pos);
        self.report.kit_miner_latched = self
            .app
            .world()
            .get::<MinerMachine>(miner_e)
            .map(|m| m.deposit)
            == Some(deposit_e);

        let assembler_e = self.deploy("assembler", self.lane(25.0, 0), true);
        let station_e = self.deploy("analysis_station", self.lane(10.0, 0), true);
        self.wire_logi(storage_pos, deposit_pos);
        self.app.update();
        self.bind_network();
        let net = self.net();
        self.report.networks_shared = net == self.net_of(miner_e)
            && net == self.net_of(assembler_e)
            && net == self.net_of(station_e);

        // Stage 1 — earn ore_extraction (so miners can be crafted); the kit station analyses stone.
        let mut station_ran = false;
        self.run_until(
            0.5,
            8_000.0,
            |s| {
                s.ensure_jobs("basic_analysis", 12);
                s.request_node("ore_extraction");
            },
            |s| {
                if s.machine_state(station_e) == Some(MachineState::Running) {
                    station_ran = true;
                }
                s.node_unlocked("ore_extraction")
            },
        );
        self.report.station_ran = station_ran;

        // Stage 2 — real mining: a fresh iron/copper vein for throughput + a coal vein (steel needs
        // carbon). The kit miner keeps feeding the origin stone the research grind burns.
        let iron_copper_deposit = self.craft_and_mine("copper_ore", 2, true);
        let coal_deposit = self.craft_and_mine("coal", 1, false);

        // More analysis stations + a small solar farm — the 480-material + 400-engineering grind
        // plus the processing line would starve/brown-out on the kit alone.
        self.craft("analysis_station", 2);
        self.craft("solar_generator", 4);
        self.advance_until(0.5, 25_000.0, |s| {
            s.hub_stored("analysis_station") >= 2 && s.hub_stored("solar_generator") >= 4
        });
        let _s2 = self.deploy("analysis_station", self.lane(10.0, 1), true);
        let _s3 = self.deploy("analysis_station", self.lane(10.0, 2), true);
        for i in 0..4 {
            self.deploy_panel(self.lane(60.0, i));
        }

        // Lazy processing buildout — each machine crafted + placed the frame its make_* unlocks.
        struct Slot {
            machine: &'static str,
            gate: &'static str,
            pos: Vec3,
            enqueued: bool,
            placed: bool,
        }
        let mut slots = vec![
            Slot {
                machine: "smelter",
                gate: "make_smelter",
                pos: self.lane(15.0, 0),
                enqueued: false,
                placed: false,
            },
            Slot {
                machine: "smelter",
                gate: "make_smelter",
                pos: self.lane(15.0, 1),
                enqueued: false,
                placed: false,
            },
            Slot {
                machine: "wire_drawer",
                gate: "make_wire_drawer",
                pos: self.lane(20.0, 0),
                enqueued: false,
                placed: false,
            },
            Slot {
                machine: "assembler",
                gate: "make_assembler",
                pos: self.lane(25.0, 1),
                enqueued: false,
                placed: false,
            },
            Slot {
                machine: "crusher",
                gate: "make_crusher",
                pos: self.lane(30.0, 0),
                enqueued: false,
                placed: false,
            },
            Slot {
                machine: "refinery",
                gate: "make_refinery",
                pos: self.lane(40.0, 0),
                enqueued: false,
                placed: false,
            },
        ];

        let material_nodes = [
            "ore_extraction",
            "basic_processing",
            "silicon_refining",
            "steel_alloying",
        ];
        let dt = 0.5f32;
        let max_secs = spec.max_secs;
        self.app
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                dt,
            )));
        let mut elapsed = 0.0f32;
        let mut launch_e: Option<Entity> = None;
        let mut pad_enqueued = false;
        let mut launch_job = false;
        let mut launch_ran = false;

        while !self.is_completed() {
            assert!(
                elapsed < max_secs,
                "initiation run did not complete within {max_secs}s of simulated time \
                 (check the T1–T3 research economy or the minimal-launch build)"
            );

            // (a) Research: the T1–T3 path + the terminal minimal_successor node.
            for n in material_nodes {
                self.request_node(n);
            }
            self.request_node("minimal_successor");

            // (b) Lazy machine buildout as gates open.
            for i in 0..slots.len() {
                if slots[i].placed || !self.recipe_unlocked(slots[i].gate) {
                    continue;
                }
                if !slots[i].enqueued {
                    let m = slots[i].machine;
                    self.craft(m, 1);
                    slots[i].enqueued = true;
                }
                if self.hub_stored(slots[i].machine) >= 1 {
                    let (m, pos) = (slots[i].machine, slots[i].pos);
                    let _e = self.deploy(m, pos, true);
                    slots[i].placed = true;
                }
            }

            // (c) Capped job top-ups — keep every intermediate flowing without draining raw ore.
            let mat_done = self.nodes_unlocked(&material_nodes);
            let minimal_done = self.node_unlocked("minimal_successor");
            if !mat_done && self.research_points("material") < 500.0 {
                self.ensure_jobs("basic_analysis", 10);
            }
            if !minimal_done
                && self.recipe_unlocked("analyze_circuit")
                && self.research_points("engineering") < 450.0
            {
                self.ensure_jobs("analyze_circuit", 8);
            }
            // Smelting drives the ore_crusher milestone (100 iron) and feeds steel + circuits.
            if self.hub_stored("iron_ingot") < 60 {
                self.ensure_jobs("smelt_metal__iron", 10);
            }
            if self.hub_stored("copper_ingot") < 60 {
                self.ensure_jobs("smelt_metal__copper", 10);
            }
            if self.hub_stored("copper_wire") < 40 {
                self.ensure_jobs("draw_metal__copper", 10);
            }
            if self.hub_stored("circuit_board") < 25 {
                self.ensure_jobs("make_circuit", 8);
            }
            if self.recipe_unlocked("crush_stone") && self.hub_stored("crushed_stone") < 30 {
                self.ensure_jobs("crush_stone", 10);
            }
            if self.recipe_unlocked("refine_silicon") && self.hub_stored("silicon") < 15 {
                self.ensure_jobs("refine_silicon", 8);
            }
            if self.recipe_unlocked("form_silicon_chip") && self.hub_stored("silicon_chip") < 8 {
                self.ensure_jobs("form_silicon_chip", 6);
            }
            if self.recipe_unlocked("alloy_steel") && self.hub_stored("steel_ingot") < 16 {
                self.ensure_jobs("alloy_steel", 8);
            }

            // (d) Terminal: build + place the minimal launch site, then run the successor launch.
            if minimal_done {
                if launch_e.is_none() {
                    if !pad_enqueued
                        && self.hub_stored("steel_ingot") >= 8
                        && self.hub_stored("circuit_board") >= 4
                        && self.hub_stored("silicon_chip") >= 2
                    {
                        self.push_jobs("make_launch_site__minimal", 1);
                        pad_enqueued = true;
                    }
                    if self.hub_stored("launch_site") >= 1 {
                        let e = self.deploy("launch_site", self.lane(55.0, 0), true);
                        assert!(
                            self.app.world().get::<EscapeObjective>(e).is_some(),
                            "a placed launch_site must be tagged EscapeObjective so its recipe wins"
                        );
                        launch_e = Some(e);
                    }
                } else if !launch_job
                    && self.hub_stored("steel_ingot") >= 4
                    && self.hub_stored("circuit_board") >= 3
                    && self.hub_stored("silicon_chip") >= 2
                    && self.hub_stored("copper_wire") >= 4
                {
                    self.push_jobs("launch_minimal_successor", 1);
                    launch_job = true;
                }
            }

            if let Some(le) = launch_e
                && self.machine_state(le) == Some(MachineState::Running)
            {
                launch_ran = true;
            }

            self.app.update();
            self.capture();
            elapsed += dt;
        }

        self.capture();
        self.report.launch_ran = launch_ran;
        self.report.build_enqueued = launch_job;
        self.report.completed = self.is_completed();
        self.report.virtual_secs = self.virtual_secs();
        let extracted = |world: &World, d: Entity| -> f32 {
            world
                .get::<OreDeposit>(d)
                .map(|o| o.total_extracted)
                .unwrap_or(0.0)
        };
        self.report.ore_extracted = vec![
            (
                "iron_copper_vein".to_string(),
                extracted(self.app.world(), iron_copper_deposit),
            ),
            (
                "coal".to_string(),
                extracted(self.app.world(), coal_deposit),
            ),
        ];
        self.report.clone()
    }

    // ── the victory grind ───────────────────────────────────────────────────────────────────

    /// Drives the earned-research + real-machine-economy grind to `launch_successor`. Each frame:
    /// (a) request every target node (no-op unless prereqs met + pool can pay); (b) recon the
    /// exotic sites once their prereqs are researched; (c0) once the last machine gate opens, prep
    /// the terminal machine bodies (steel/plate/circuit) in bulk; (c) lazily craft + place each
    /// processing machine as its gate opens; (d) top up the analysis/milestone chain (capped so
    /// the easy chains don't starve the machine-body crafts of raw ore); (e) once the whole
    /// closure is earned AND the terminal machines are placed, swap the queue to the build list.
    /// Loops until `RunState::Completed`. Panics on a `plan.max_secs` runaway guard.
    pub fn drive_to_victory(&mut self, plan: &GrindPlan) -> DriveOutcome {
        // Lazy machine buildout slots: each processing machine is crafted + placed only once its
        // `make_*` recipe unlocks. Terminal machines (`body: true`) have their bodies prepped in
        // bulk (see c0), not per-slot, so the shared steel/plate/circuit accounting is robust.
        struct Slot {
            machine: &'static str,
            gate_recipe: &'static str,
            pos: Vec3,
            body: bool,
            enqueued: bool,
            placed: bool,
            /// Set once deployed — the target for config-module install.
            entity: Option<Entity>,
            /// `(config-module item, its make_* recipe)` for a machine that must be dedicated
            /// via an installed config (machine dedication). `None` = config-agnostic machine.
            bed: Option<(&'static str, &'static str)>,
            bed_enqueued: bool,
            installed: bool,
        }
        impl Slot {
            fn new(
                machine: &'static str,
                gate_recipe: &'static str,
                pos: Vec3,
                body: bool,
            ) -> Self {
                Slot {
                    machine,
                    gate_recipe,
                    pos,
                    body,
                    enqueued: false,
                    placed: false,
                    entity: None,
                    bed: None,
                    bed_enqueued: false,
                    installed: false,
                }
            }
            fn with_bed(mut self, item: &'static str, make_recipe: &'static str) -> Self {
                self.bed = Some((item, make_recipe));
                self
            }
        }
        // The two refineries are dedicated by config: the `carbothermal` bed runs
        // refine_xalite/refine_fluxite, the `cryogenic` bed runs reclaim_coolant/refine_exotic_fuel.
        // One time-shared refinery can no longer cover the run (design-decisions.md 2026-07-12).
        let mut slots = vec![
            Slot::new("crusher", "make_crusher", self.lane(30.0, 0), false),
            Slot::new("washer", "make_washer", self.lane(35.0, 0), false),
            Slot::new(
                "plate_roller",
                "make_plate_roller",
                self.lane(45.0, 0),
                false,
            ),
            Slot::new("refinery", "make_refinery", self.lane(40.0, 0), false)
                .with_bed("carbothermal_bed", "make_carbothermal_bed"),
            Slot::new("refinery", "make_refinery", self.lane(40.0, 1), false)
                .with_bed("cryogenic_bed", "make_cryogenic_bed"),
            Slot::new(
                "advanced_assembler",
                "make_advanced_assembler",
                self.lane(50.0, 0),
                true,
            ),
            Slot::new("launch_site", "make_launch_site", self.lane(55.0, 0), true),
        ];

        let dt = 0.5f32;
        let max_secs = plan.max_secs;
        self.app
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                dt,
            )));
        let mut elapsed = 0.0f32;
        let mut bodies_prepped = false;
        let mut build_enqueued = false;
        let mut launch_ran = false;
        let mut launch_e: Option<Entity> = None;
        let mut ever_analyzed_circuit = false;
        let mut ever_analyzed_exotic = false;

        while !self.is_completed() {
            assert!(
                elapsed < max_secs,
                "earned-research + real-build grind did not complete within {max_secs}s of \
                 simulated time (check the analysis economy, a stalled milestone/recon, or \
                 machine buildout)"
            );

            // (a)+(d) Four-currency pump: request every target node (no-op unless prereqs met +
            // pool can pay) and top up each affordable analysis/milestone chain (capped so the easy
            // chains don't starve the machine-body crafts). Shared with the setup stages through
            // `pump_frame`; runs only in the research phase — once the build list is enqueued the
            // craft queue is dedicated to the successor and analysis top-up must stop.
            if !build_enqueued {
                ever_analyzed_circuit |= self.recipe_unlocked("analyze_circuit");
                ever_analyzed_exotic |= self.recipe_unlocked("analyze_exotic_reaction");
                self.pump();
            }

            // (b) Conditional recon: each site fires its DiscoveryEvent exactly once, honored only
            // if the node's prereq is already researched — so recon only after it is.
            if self.node_unlocked("precursor_survey") && !self.discovered(plan.fluxite_site) {
                self.recon_deposit("fluxite_shard");
            }
            if self.node_unlocked("cryophase_prospecting")
                && !self.discovered(plan.cryophase_deposit)
            {
                self.recon_deposit("cryophase_shard");
            }

            // (c0) Terminal body prep: once make_launch_site unlocks (the last machine gate), push
            // a generous, over-provisioned set of body-intermediate jobs (steel/plates/circuits +
            // sub-inputs) plus the two make_* machine jobs. Hand-pushed rather than enqueue_item so
            // accounting is robust across BOTH terminals — enqueue_item's per-call storage snapshot
            // double-counts shared intermediates. advanced_assembler = 4 steel + 2 circuit + 2
            // iron_plate; launch_site = 12 steel + 4 titanium_plate + 4 circuit. Excess is harmless.
            if !bodies_prepped && self.recipe_unlocked("make_launch_site") {
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
                    self.push_jobs(recipe, n);
                }
                bodies_prepped = true;
            }

            // (c) Lazy machine buildout as gates open.
            for i in 0..slots.len() {
                // (c1) place the machine once its gate opens.
                if !slots[i].placed {
                    if !self.recipe_unlocked(slots[i].gate_recipe) {
                        continue;
                    }
                    if !slots[i].enqueued {
                        if !slots[i].body {
                            let machine = slots[i].machine;
                            self.craft(machine, 1);
                        }
                        slots[i].enqueued = true;
                    }
                    if self.hub_stored(slots[i].machine) >= 1 {
                        let (machine, pos) = (slots[i].machine, slots[i].pos);
                        let e = self.deploy(machine, pos, true);
                        if machine == "launch_site" {
                            assert!(
                                self.app.world().get::<EscapeObjective>(e).is_some(),
                                "a placed launch_site must be tagged EscapeObjective so its recipe wins"
                            );
                            launch_e = Some(e);
                        }
                        slots[i].entity = Some(e);
                        slots[i].placed = true;
                    }
                }

                // (c2) dedicate a placed machine: craft its config module once the make_* recipe
                // unlocks, then install it so the machine's config-gated recipes can run on it.
                if slots[i].placed
                    && !slots[i].installed
                    && let Some((bed_item, make_recipe)) = slots[i].bed
                {
                    if !slots[i].bed_enqueued && self.recipe_unlocked(make_recipe) {
                        self.craft(bed_item, 1);
                        slots[i].bed_enqueued = true;
                    }
                    if slots[i].bed_enqueued && self.hub_stored(bed_item) >= 1 {
                        let e = slots[i].entity.expect("placed slot records its entity");
                        self.install_config(e, bed_item);
                        slots[i].installed = true;
                    }
                }
            }

            // (e) Whole closure earned AND terminal machines placed → swap to the build list.
            let closure_done = self.node_unlocked("launch_successor");
            let terminals_ready = slots
                .iter()
                .filter(|s| s.machine == "advanced_assembler" || s.machine == "launch_site")
                .all(|s| s.placed);
            if !build_enqueued && closure_done && terminals_ready {
                let mut queue = self
                    .app
                    .world_mut()
                    .get_mut::<NetworkCraftQueue>(self.net)
                    .expect("logistics network carries a craft queue");
                queue.jobs.clear();
                queue.reserved.clear();
                for (recipe, n) in plan.build_jobs {
                    for _ in 0..*n {
                        queue.jobs.push_back(QueuedJob {
                            recipe_id: (*recipe).to_string(),
                        });
                    }
                }
                build_enqueued = true;
            }

            if let Some(le) = launch_e
                && self.machine_state(le) == Some(MachineState::Running)
            {
                launch_ran = true;
            }

            self.app.update();
            self.capture();
            elapsed += dt;
        }

        DriveOutcome {
            launch_ran,
            build_enqueued,
            ever_analyzed_circuit,
            ever_analyzed_exotic,
        }
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

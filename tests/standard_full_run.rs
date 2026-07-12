//! End-to-end integration test for a full Standard run — landing all the way to launching the
//! successor vehicle — exercised through the real world-generation + placement + logistics +
//! recipe + research + power systems from a fixed seed, driven on simulated time. Every machine
//! the factory uses is OBTAINED FOR REAL: it either arrives in the landing kit or is crafted from
//! mined ore through a `make_*` recipe into networked storage and then placed inventory-gated.
//!
//! This file is the *script*; the reusable mechanics live in the [`scenario`] harness
//! (`Scenario::land`, `deploy`, `craft`, `craft_and_mine`, `run_until`, `drive_to_victory`, …).
//! Here we keep the regression assertions that pin each stage's real-systems behaviour:
//!
//!   Stage 0: fixed seed → generate terrain + surface deposits → the real landing (PodPlugin)
//!            sets down a networked `storage_crate` stocked with the starting kit. Place those kit
//!            machines through the real inventory-gated path, wire them, prove the miner latched
//!            + the network formed.
//!   Stage 1: queue-driven mine→analyse loop earns the first ResearchSpend node (ore_extraction).
//!   Stage 1b: the first research spend reveals both atmospheric properties.
//!   Stage 2: sustained grind → basic_processing.
//!   Stage 3: CRAFT a smelter, place + power it, smelt mined iron_ore → iron_ingot.
//!   Stage 4: CRAFT a wire_drawer + a second assembler, run the copper→wire→circuit chain.
//!   Stage 5: drone scan reveals geological activity.
//!   Stage 6: the full Standard victory with EARNED research and a REAL machine economy — craft +
//!            place the mining fleet / analysis stations / smelter / solar farm, then hand the
//!            four-currency grind + lazy machine buildout + successor build to `drive_to_victory`.
//!
//! NOTHING gameplay-relevant is injected — see the [`scenario`] module docs for the full stub list.

mod scenario;

use bevy::prelude::*;

use exergon::escape::{EscapeObjective, RunState};
use exergon::machine::{MachineState, MinerMachine};
use exergon::planet::PropertyVisibility;
use exergon::power::GeneratorUnit;
use exergon::research::Discovered;
use exergon::world::OreDeposit;

use scenario::{GrindPlan, Scenario};

/// Fixed master seed for this run — makes terrain + deposit placement reproducible.
const MASTER_SEED: u64 = 0xE7E6_0007;

#[test]
fn standard_run_lands_mines_and_launches_successor() {
    let mut s = Scenario::new(MASTER_SEED);

    // Land: real worldgen + PodPlugin landing. Locates the origin deposit + bootstrap crate.
    s.land();

    let deposit_e = s.origin_deposit();
    let deposit_pos = s.origin_pos();
    assert_eq!(
        (deposit_pos.x, deposit_pos.z),
        (32.0, 32.0),
        "origin chunk deposit sits at the chunk centre — reproducible for a fixed seed"
    );
    assert!(
        s.origin_ores().iter().any(|(id, _)| id == "stone"),
        "origin deposit must yield stone to bootstrap research, got {:?}",
        s.origin_ores()
    );

    // Confirm the kit arrived in the landing crate.
    let storage_pos = s.storage_pos();
    for essential in ["miner", "assembler", "solar_generator", "analysis_station"] {
        assert!(
            s.hub_stored(essential) >= 1,
            "landing kit must contain a {essential} (got {})",
            s.hub_stored(essential)
        );
    }

    let generator_pos = s.generator_pos();

    // Stage 0 — deploy the four kit machines through the real inventory-gated path. The kit miner
    // latches the origin stone/iron/copper deposit; the rest sit on the hub network.
    s.place_real("solar_generator", generator_pos); // power anchor
    s.app.update();
    let generator_e = s.machine_at("solar_generator", generator_pos);

    s.place_real("miner", deposit_pos);
    s.wire_logi(storage_pos, deposit_pos);
    s.app.update();
    let miner_e = s.machine_at("miner", deposit_pos);
    assert_eq!(
        s.app
            .world()
            .get::<MinerMachine>(miner_e)
            .map(|m| m.deposit),
        Some(deposit_e),
        "placed kit miner should latch onto the generated origin deposit"
    );

    let assembler_pos = s.lane(25.0, 0);
    let station_pos = s.lane(10.0, 0);
    let assembler_e = s.deploy("assembler", assembler_pos, true);
    let station_e = s.deploy("analysis_station", station_pos, true);

    // Wire the hub crate itself onto the network (shares the miner cable's endpoint), then bind.
    s.wire_logi(storage_pos, deposit_pos);
    s.app.update();
    s.bind_network();

    let net = s.net();
    assert_eq!(net, s.net_of(miner_e), "miner shares hub network");
    assert_eq!(net, s.net_of(assembler_e), "assembler shares hub network");
    assert_eq!(net, s.net_of(station_e), "station shares hub network");

    // Stage 1 — earn ore_extraction (30 material). Queue basic_analysis (4 stone → 10 material,
    // 0E) and let the mine→analyse loop run under real time. The station is ManualCraftOnly, so
    // the queue is the sole scheduler; the miner feeds stone every tick.
    let mut station_ran = false;
    s.run_until(
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
    assert!(
        station_ran,
        "analysis station must actually run basic_analysis"
    );

    let points_before = s.research_points("material");
    s.request_node("ore_extraction");
    s.app.update();
    assert!(
        s.node_unlocked("ore_extraction"),
        "first research node should be unlocked after spending research points"
    );
    assert!(
        s.research_points("material") < points_before,
        "unlocking a ResearchSpend node must deduct research points"
    );

    // Stage 1b — planet property reveal (research-spend trigger). `property_reveal_system` watches
    // the `TechNodeUnlocked { via_research }` the unlock above emits and advances both atmospheric
    // properties Hidden→Revealed. Geological activity stays Hidden until a drone scan (Stage 5).
    s.app.update();
    let vis = s.planet_vis();
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
    s.run_until(
        0.5,
        12_000.0,
        |s| s.ensure_jobs("basic_analysis", 12),
        |s| s.research_points("material") >= 150.0,
    );
    let points_before = s.research_points("material");
    s.request_node("basic_processing");
    s.app.update();
    assert!(
        s.node_unlocked("basic_processing"),
        "second-tier node should unlock after a sustained grind to 150 research points"
    );
    assert_eq!(
        s.research_points("material"),
        points_before - 150.0,
        "unlocking basic_processing must deduct its 150-point cost"
    );

    // Stage 3 — CRAFT a smelter, place + power it, prove the energy-gated smelt. make_smelter
    // (basic_smelting auto) = 20 stone + 10 iron_ore → smelter; smelt_metal__iron draws power.
    assert!(
        s.recipe_unlocked("make_smelter"),
        "basic_smelting must have auto-unlocked make_smelter"
    );
    assert!(
        s.recipe_unlocked("smelt_metal__iron"),
        "basic_smelting's smelt_metal template must have auto-unlocked smelt_metal__iron"
    );
    s.craft("smelter", 1);
    s.advance_until(0.5, 6_000.0, |s| s.hub_stored("smelter") >= 1);
    let smelter_pos = s.lane(15.0, 0);
    let smelter_e = s.deploy("smelter", smelter_pos, true);

    s.push_jobs("smelt_metal__iron", 3);
    let mut smelter_ran = false;
    s.advance_until(0.25, 6_000.0, |s| {
        if s.machine_state(smelter_e) == Some(MachineState::Running) {
            smelter_ran = true;
        }
        s.hub_stored("iron_ingot") >= 1
    });
    assert!(
        smelter_ran,
        "smelter must run the energy-gated smelt recipe"
    );
    assert!(
        s.app
            .world()
            .get::<GeneratorUnit>(generator_e)
            .is_some_and(|g| g.buffer_joules > 0.0),
        "kit solar generator must have charged its buffer over simulated time"
    );

    // Stage 4 — CRAFT a wire_drawer + a second assembler, run copper→wire→circuit for real.
    // make_circuit = 1 iron_ingot + 2 copper_wire → 1 circuit_board (assembler, basic_processing).
    assert!(s.recipe_unlocked("make_wire_drawer"));
    assert!(s.recipe_unlocked("make_assembler"));
    s.craft("wire_drawer", 1);
    s.craft("assembler", 1);
    s.advance_until(0.5, 8_000.0, |s| {
        s.hub_stored("wire_drawer") >= 1 && s.hub_stored("assembler") >= 1
    });
    let drawer_pos = s.lane(20.0, 0);
    let assembler2_pos = s.lane(25.0, 1);
    let _drawer_e = s.deploy("wire_drawer", drawer_pos, true);
    let assembler2_e = s.deploy("assembler", assembler2_pos, true);

    s.push_jobs("smelt_metal__iron", 2);
    s.push_jobs("smelt_metal__copper", 2);
    s.push_jobs("draw_metal__copper", 2);
    s.push_jobs("make_circuit", 1);
    let mut assembler_ran = false;
    s.advance_until(0.25, 8_000.0, |s| {
        if s.machine_state(assembler_e) == Some(MachineState::Running)
            || s.machine_state(assembler2_e) == Some(MachineState::Running)
        {
            assembler_ran = true;
        }
        s.hub_stored("circuit_board") >= 1
    });
    assert!(
        assembler_ran,
        "an assembler must run make_circuit under power"
    );
    assert!(s.hub_stored("circuit_board") >= 1);

    // Stage 5 — drone scan reveals geological activity. Entering DronePilot and revealing a fog
    // cell drives `property_reveal_system` to advance geological_activity Hidden→Qualitative.
    s.enter_drone_pilot();
    s.app.update();
    s.reveal_fog(IVec2::ZERO);
    s.app.update();
    assert_eq!(
        s.planet_vis().geological_activity,
        PropertyVisibility::Qualitative,
        "a drone scan (fog reveal in DronePilot) must reveal geological activity"
    );

    // Stage 6 — the full Standard victory with a REAL machine economy and EARNED research.

    // Exotic-site discovery via REAL drone recon (still in DronePilot from Stage 5). xalite →
    // exotic_materials unlocks now (prereq science_basics is a free chain).
    let xalite_site = s.recon_deposit("xalite");
    assert!(
        s.discovered(xalite_site),
        "drone recon must mark the xalite deposit Discovered (real DiscoveryEvent fired)"
    );
    assert!(s.node_unlocked("exotic_materials"));

    // Scale up the early factory: two more analysis stations (research is the dominant grind — the
    // ~6.5k-point closure serialises badly on one station), a second smelter (the whole metal
    // economy funnels through it), and a solar farm sized to the build-phase peak draw. All crafted.
    s.craft("analysis_station", 2);
    s.craft("smelter", 1);
    s.advance_until(0.5, 25_000.0, |s| {
        s.hub_stored("analysis_station") >= 2 && s.hub_stored("smelter") >= 1
    });
    let station2_e = s.deploy("analysis_station", s.lane(10.0, 1), true);
    let station3_e = s.deploy("analysis_station", s.lane(10.0, 2), true);
    let _smelter2_e = s.deploy("smelter", s.lane(15.0, 1), true);
    let _ = (station2_e, station3_e);

    // Solar farm: peak concurrent build-phase draw is ~120 W. Craft 6 panels (+ the kit panel =
    // 7); at the seed's solar_modifier this covers the draw with margin, buffers absorb transients.
    let panels = 6usize;
    s.craft("solar_generator", panels as u32);
    s.advance_until(0.5, 30_000.0, |s| {
        s.hub_stored("solar_generator") >= panels as u32
    });
    let mut farm: Vec<Entity> = Vec::new();
    for i in 0..panels {
        farm.push(s.deploy_panel(s.lane(60.0, i)));
    }
    s.advance_until(1.0, 6_000.0, |s| {
        farm.iter().all(|&e| {
            s.app
                .world()
                .get::<GeneratorUnit>(e)
                .is_some_and(|g| g.buffer_joules >= g.max_buffer_joules * 0.9)
        })
    });

    // Mine every raw material for real (miners crafted first via `craft_and_mine`). The fresh
    // iron_copper vein (2 miners) is the heavy feed for steel/circuits/plates; cryophase (2 miners)
    // supplies all 60 shards for the 20 exotic_fuel; trace ores get 1 miner each.
    let iron_copper_deposit = s.craft_and_mine("copper_ore", 2, true);
    s.craft_and_mine("resonite_shard", 1, false);
    s.craft_and_mine("aluminum_ore", 1, false);
    s.craft_and_mine("titanium_ore", 1, false);
    s.craft_and_mine("coal", 1, false);
    let fluxite_site = s.craft_and_mine("fluxite_shard", 1, false);
    let cryophase_deposit = s.craft_and_mine("cryophase_shard", 2, false);

    // The ResearchSpend target nodes, by theme (steel_alloying in material so the run can craft
    // real steel for the advanced_assembler + launch_site bodies).
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

    // The build-phase job list (mass-balanced from the successor tree). launch_successor has no
    // output item, so these are explicit jobs the grind enqueues once the closure is earned.
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

    let plan = GrindPlan {
        material_nodes: &material_nodes,
        engineering_nodes: &engineering_nodes,
        discovery_nodes: &discovery_nodes,
        synthesis_nodes: &synthesis_nodes,
        build_jobs: &build_jobs,
        fluxite_site,
        cryophase_deposit,
    };
    let outcome = s.drive_to_victory(&plan);

    // Every research node must have been EARNED (spent/auto), never injected.
    let all_nodes: Vec<&str> = material_nodes
        .iter()
        .chain(&engineering_nodes)
        .chain(&discovery_nodes)
        .chain(&synthesis_nodes)
        .copied()
        .collect();
    for node in &all_nodes {
        assert!(
            s.node_unlocked(node),
            "target node {node} must have been earned before the build phase"
        );
    }
    assert!(
        s.node_unlocked("steel_alloying"),
        "steel_alloying must have been earned so the advanced_assembler/launch_site use real steel"
    );
    assert!(
        outcome.build_enqueued,
        "the successor build list must have been enqueued"
    );
    assert!(
        outcome.ever_analyzed_circuit && outcome.ever_analyzed_exotic,
        "the engineering + synthesis analysis recipes must have run for real"
    );

    let virtual_secs = s.virtual_secs();
    let virtual_hours = virtual_secs / 3600.0;
    println!(
        "\n=== Standard run complete: virtual time to victory = {virtual_secs:.1}s ({virtual_hours:.2}h) ===\n"
    );

    assert!(
        outcome.launch_ran,
        "launch_site must actually run the launch_successor recipe"
    );

    // The victory must have been fed by REAL mining, not injected refined items.
    let extracted = |s: &Scenario, deposit: Entity| -> f32 {
        s.app
            .world()
            .get::<OreDeposit>(deposit)
            .map(|d| d.total_extracted)
            .unwrap_or(0.0)
    };
    assert!(
        extracted(&s, cryophase_deposit) >= 60.0,
        "cryophase deposit must have been mined for ≥60 shards (the 20 exotic_fuel), got {}",
        extracted(&s, cryophase_deposit)
    );
    assert!(
        extracted(&s, iron_copper_deposit) >= 18.0,
        "fresh iron_copper vein must have been mined for real, got {}",
        extracted(&s, iron_copper_deposit)
    );
    assert!(
        s.is_completed(),
        "completing launch_successor on the launch_site must set RunState::Completed \
         (virtual time to victory = {virtual_secs:.1}s / {virtual_hours:.2}h)"
    );
    assert!(
        (100.0..86_400.0).contains(&virtual_secs),
        "virtual time to victory {virtual_secs:.1}s outside the sane [100s, 24h) bound"
    );

    // Silence unused-import lints for types kept for their assertions above.
    let _ = (RunState::Completed, EscapeObjective, Discovered);
}

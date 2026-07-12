//! End-to-end smoke test for a full Standard run — landing all the way to launching the successor
//! vehicle — exercised through the real world-generation + placement + logistics + recipe +
//! research + power systems from a fixed seed, driven on simulated time. Every machine the factory
//! uses is OBTAINED FOR REAL: it either arrives in the landing kit or is crafted from mined ore
//! through a `make_*` recipe into networked storage and then placed inventory-gated.
//!
//! The driving choreography and the milestone/statistics report live in the shared
//! [`scenario_runner`] crate — the same code path the `scenario` balancing binary runs. This test
//! loads the canonical `scenarios/standard.ron`, runs it, and pins the regression assertions each
//! stage's real-systems behaviour must satisfy (kit contents, network formation, the reveal +
//! recon milestones, the power-gated machine runs, an earned tech closure, and a real ore economy).
//!
//! NOTHING gameplay-relevant is injected — see the [`scenario_runner`] crate docs for the stub list.

use scenario_runner::{Scenario, load_spec};

#[test]
fn standard_run_lands_mines_and_launches_successor() {
    let spec = load_spec("scenarios/standard.ron").expect("canonical standard scenario must load");

    // `new` lands: real worldgen + PodPlugin landing. Assert the pre-run world before the kit is
    // consumed: the fixed-seed origin deposit and the stocked landing kit.
    let mut s = Scenario::new(spec.seed);

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
    for essential in ["miner", "assembler", "solar_generator", "analysis_station"] {
        assert!(
            s.hub_stored(essential) >= 1,
            "landing kit must contain a {essential} (got {})",
            s.hub_stored(essential)
        );
    }

    // Drive the entire scripted run: deploy kit → earn early tiers → craft the processing economy
    // → four-currency grind → successor build → launch. Milestones land in the report.
    let report = s.run_standard(&spec);

    // ── stage regression flags (real systems, not injected) ──
    assert!(
        report.kit_miner_latched,
        "placed kit miner should latch onto the generated origin deposit"
    );
    assert!(
        report.networks_shared,
        "the kit miner, assembler and station must share the bootstrap hub network"
    );
    assert!(
        report.station_ran,
        "analysis station must actually run basic_analysis"
    );
    assert!(
        report.oxygen_revealed && report.pressure_revealed,
        "the first research spend must reveal both atmospheric properties"
    );
    assert!(
        report.geo_revealed_after_scan,
        "a drone scan (fog reveal in DronePilot) must reveal geological activity"
    );
    assert!(
        report.xalite_discovered,
        "drone recon must mark the xalite deposit Discovered (real DiscoveryEvent fired)"
    );
    assert!(
        report.smelter_ran,
        "smelter must run the energy-gated smelt recipe"
    );
    assert!(
        report.generator_charged,
        "kit solar generator must have charged its buffer over simulated time"
    );
    assert!(
        report.assembler_ran,
        "an assembler must run make_circuit under power"
    );

    // ── every target research node EARNED (spent/auto), never injected ──
    for node in spec
        .material_nodes
        .iter()
        .chain(&spec.engineering_nodes)
        .chain(&spec.discovery_nodes)
        .chain(&spec.synthesis_nodes)
    {
        assert!(
            s.node_unlocked(node),
            "target node {node} must have been earned before the build phase"
        );
    }
    assert!(
        s.node_unlocked("steel_alloying"),
        "steel_alloying must have been earned so the advanced_assembler/launch_site use real steel"
    );

    // ── victory-grind outcome ──
    assert!(
        report.build_enqueued,
        "the successor build list must have been enqueued"
    );
    assert!(
        report.ever_analyzed_circuit && report.ever_analyzed_exotic,
        "the engineering + synthesis analysis recipes must have run for real"
    );
    assert!(
        report.launch_ran,
        "launch_site must actually run the launch_successor recipe"
    );

    // ── the victory must have been fed by REAL mining, not injected refined items ──
    let ore = |id: &str| -> f32 {
        report
            .ore_extracted
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    };
    assert!(
        ore("cryophase_shard") >= 60.0,
        "cryophase deposit must have been mined for ≥60 shards (the 20 exotic_fuel), got {}",
        ore("cryophase_shard")
    );
    assert!(
        ore("iron_copper_vein") >= 18.0,
        "fresh iron_copper vein must have been mined for real, got {}",
        ore("iron_copper_vein")
    );

    // ── completion + a sane virtual-time bound ──
    let virtual_secs = report.virtual_secs;
    let virtual_hours = virtual_secs / 3600.0;
    println!(
        "\n=== Standard run complete: virtual time to victory = {virtual_secs:.1}s ({virtual_hours:.2}h) ===\n"
    );
    assert!(
        report.completed,
        "completing launch_successor on the launch_site must set RunState::Completed \
         (virtual time to victory = {virtual_secs:.1}s / {virtual_hours:.2}h)"
    );
    assert!(
        (100.0..86_400.0).contains(&virtual_secs),
        "virtual time to victory {virtual_secs:.1}s outside the sane [100s, 24h) bound"
    );

    // The report must have timestamped real progression milestones (tier climb pace).
    assert!(
        !report.tier_progress.is_empty(),
        "the run must have climbed at least one tech tier"
    );
}

//! End-to-end smoke test for a full **Initiation** run — the easiest difficulty (tech tiers 1–3)
//! and the prerequisite that gates Standard. Driven on simulated time through the real
//! worldgen + placement + logistics + recipe + research + power systems, exactly like the standard
//! run, but under a tier-3 `TierCap`: the factory earns only the T1–T3 path and escapes via the
//! `minimal_successor` launch (steel + circuits + silicon, no tier-4 titanium). Every machine and
//! material is obtained for real; nothing gameplay-relevant is injected.
//!
//! The regression this pins: the tier cap actually bounds the run (no tier-4+ node ever unlocks),
//! and the T3 minimal-successor escape completes for real.

use scenario_runner::{Scenario, load_spec};

#[test]
fn initiation_run_lands_and_launches_minimal_successor() {
    let spec = load_spec("scenarios/initiation.ron").expect("initiation scenario must load");

    let mut s = Scenario::new(spec.seed);
    for essential in ["miner", "assembler", "solar_generator", "analysis_station"] {
        assert!(
            s.hub_stored(essential) >= 1,
            "landing kit must contain a {essential} (got {})",
            s.hub_stored(essential)
        );
    }

    let report = s.run_initiation(&spec);

    // Real-systems stage checks (shared with the standard run's early game).
    assert!(
        report.kit_miner_latched,
        "placed kit miner should latch the origin deposit"
    );
    assert!(
        report.networks_shared,
        "kit miner, assembler and station must share the bootstrap hub network"
    );
    assert!(
        report.station_ran,
        "analysis station must run basic_analysis"
    );

    // The Initiation escape completed for real.
    assert!(
        s.node_unlocked("minimal_successor"),
        "the T3 minimal_successor node must be earned"
    );
    assert!(
        report.launch_ran,
        "launch_site must actually run launch_minimal_successor"
    );
    assert!(report.completed, "the Initiation run must reach victory");

    // The tier cap held: nothing above tier 3 unlocked, and the Standard T4/T5 gates stayed shut.
    assert!(
        report.tier_progress.iter().all(|t| t.tier <= 3),
        "an Initiation run must not climb past tier 3, got tiers {:?}",
        report
            .tier_progress
            .iter()
            .map(|t| t.tier)
            .collect::<Vec<_>>()
    );
    assert!(
        !s.node_unlocked("titanium_forming"),
        "a tier-4 node must stay locked under the Initiation tier cap"
    );
    assert!(
        !s.node_unlocked("launch_successor"),
        "the Standard tier-5 escape must stay locked under the Initiation tier cap"
    );

    let virtual_secs = report.virtual_secs;
    println!(
        "\n=== Initiation run complete: virtual time to victory = {virtual_secs:.1}s ({:.2}h) ===\n",
        virtual_secs / 3600.0
    );
    assert!(
        (100.0..86_400.0).contains(&virtual_secs),
        "virtual time to victory {virtual_secs:.1}s outside the sane [100s, 24h) bound"
    );
}

//! What a standard run *produced* — the milestones and statistics a balancing pass reads.
//!
//! [`RunReport`] is filled incrementally as a [`crate::Scenario`] drives a run: the time-advancing
//! loops call [`RunReport::observe`] each simulated frame to timestamp node unlocks, tier
//! completions, and periodic research-currency samples; the stage choreography sets the regression
//! flags directly. The e2e smoke test asserts on the flags; the `scenario` binary prints the whole
//! thing for tuning.

use std::collections::{HashMap, HashSet};

use exergon::research::TechTreeProgress;
use exergon::tech_tree::TechTree;

/// Sample the four research currencies at most this often (simulated seconds) — keeps the curve
/// readable for a multi-thousand-second run without recording every frame.
const SNAPSHOT_INTERVAL: f32 = 300.0;

/// A research-currency reading at one point in simulated time.
#[derive(Debug, Clone)]
pub struct ResearchSnapshot {
    pub secs: f32,
    pub material: f32,
    pub engineering: f32,
    pub discovery: f32,
    pub synthesis: f32,
}

/// How far a run climbed one tech tier. A standard run cherry-picks the escape path rather than
/// clearing the whole tree, so `unlocked < total` is normal — the pace (`first_secs`/`last_secs`)
/// is the balancing signal, not full completion.
#[derive(Debug, Clone)]
pub struct TierProgress {
    pub tier: u8,
    /// nodes of this tier the run unlocked / nodes of this tier in the tree
    pub unlocked: usize,
    pub total: usize,
    /// simulated seconds at the first and last unlock in this tier
    pub first_secs: f32,
    pub last_secs: f32,
}

/// The full record of one standard run.
#[derive(Debug, Clone, Default)]
pub struct RunReport {
    pub name: String,
    pub seed: u64,
    pub completed: bool,
    /// Simulated seconds elapsed at victory (or at abort).
    pub virtual_secs: f32,

    /// `(node_id, secs)` in the order nodes unlocked.
    pub node_unlocks: Vec<(String, f32)>,
    /// Per-tier climb pace, ascending by tier. Recomputed each `observe`.
    pub tier_progress: Vec<TierProgress>,
    /// Periodic research-currency samples.
    pub research_curve: Vec<ResearchSnapshot>,
    /// `(ore_id, total_extracted)` for the veins the run mined for real.
    pub ore_extracted: Vec<(String, f32)>,

    // ── stage regression flags (the smoke test asserts these) ──
    pub kit_miner_latched: bool,
    pub networks_shared: bool,
    pub station_ran: bool,
    pub smelter_ran: bool,
    pub assembler_ran: bool,
    pub generator_charged: bool,
    pub oxygen_revealed: bool,
    pub pressure_revealed: bool,
    pub geo_revealed_after_scan: bool,
    pub xalite_discovered: bool,

    // ── victory-grind outcome ──
    pub build_enqueued: bool,
    pub launch_ran: bool,
    pub ever_analyzed_circuit: bool,
    pub ever_analyzed_exotic: bool,

    // capture bookkeeping (not part of the report proper)
    #[doc(hidden)]
    pub recorded_nodes: HashSet<String>,
    #[doc(hidden)]
    pub last_snapshot: f32,
}

impl RunReport {
    /// Timestamp any node unlocked, tier completed, or research sample due at `secs`. Reads the
    /// live tech tree + research state out of `world`. Cheap enough to call every frame (set diffs).
    pub fn observe(&mut self, world: &bevy::prelude::World, secs: f32) {
        let Some(tree) = world.get_resource::<TechTree>() else {
            return; // tree is inserted at Startup; nothing to observe before then
        };
        let unlocked = &world.resource::<TechTreeProgress>().unlocked_nodes;

        // New node unlocks, in discovery order.
        for id in unlocked {
            if self.recorded_nodes.insert(id.clone()) {
                self.node_unlocks.push((id.clone(), secs));
            }
        }

        // Per-tier climb pace: total nodes per tier from the tree; unlocked count + first/last
        // unlock time from the timestamped unlocks. Recomputed wholesale (few tiers, cheap).
        let mut total: HashMap<u8, usize> = HashMap::new();
        for node in tree.nodes.values() {
            *total.entry(node.tier).or_default() += 1;
        }
        let mut climbed: HashMap<u8, (usize, f32, f32)> = HashMap::new(); // tier → (count, first, last)
        for (id, at) in &self.node_unlocks {
            let Some(node) = tree.nodes.get(id) else {
                continue;
            };
            let e = climbed.entry(node.tier).or_insert((0, *at, *at));
            e.0 += 1;
            e.1 = e.1.min(*at);
            e.2 = e.2.max(*at);
        }
        self.tier_progress = climbed
            .into_iter()
            .map(|(tier, (unlocked, first, last))| TierProgress {
                tier,
                unlocked,
                total: total.get(&tier).copied().unwrap_or(unlocked),
                first_secs: first,
                last_secs: last,
            })
            .collect();
        self.tier_progress.sort_by_key(|t| t.tier);

        // Periodic research-currency snapshot.
        if self.research_curve.is_empty() || secs - self.last_snapshot >= SNAPSHOT_INTERVAL {
            let pool = world.resource::<exergon::research::ResearchPool>();
            self.research_curve.push(ResearchSnapshot {
                secs,
                material: pool.get("material"),
                engineering: pool.get("engineering"),
                discovery: pool.get("discovery"),
                synthesis: pool.get("synthesis"),
            });
            self.last_snapshot = secs;
        }
    }

    /// Human-readable milestone + statistics dump for the `scenario` binary.
    pub fn print(&self) {
        let h = |s: f32| s / 3600.0;
        println!("\n══════════════════════════════════════════════════════════════");
        println!("  scenario: {}   seed: {:#x}", self.name, self.seed);
        println!("══════════════════════════════════════════════════════════════");
        println!(
            "  outcome: {}   virtual time: {:.0}s ({:.2}h)",
            if self.completed {
                "VICTORY"
            } else {
                "DID NOT COMPLETE"
            },
            self.virtual_secs,
            h(self.virtual_secs),
        );

        println!("\n  ── tier progression ──");
        if self.tier_progress.is_empty() {
            println!("    (none)");
        }
        for t in &self.tier_progress {
            println!(
                "    tier {}  {}/{} nodes   {:8.0}s → {:8.0}s  ({:.2}h → {:.2}h)",
                t.tier,
                t.unlocked,
                t.total,
                t.first_secs,
                t.last_secs,
                h(t.first_secs),
                h(t.last_secs),
            );
        }

        println!("\n  ── research currency curve ──");
        println!("      secs    material  engineer  discover  synthesis");
        for s in &self.research_curve {
            println!(
                "    {:7.0}  {:9.0} {:9.0} {:9.0} {:9.0}",
                s.secs, s.material, s.engineering, s.discovery, s.synthesis
            );
        }

        println!("\n  ── raw ore extracted ──");
        for (ore, amt) in &self.ore_extracted {
            println!("    {ore:20} {amt:8.0}");
        }

        println!("\n  ── node unlocks ({}) ──", self.node_unlocks.len());
        for (id, secs) in &self.node_unlocks {
            println!("    {secs:8.0}s  {id}");
        }

        println!("\n  ── stage checks ──");
        for (label, ok) in [
            ("kit miner latched deposit", self.kit_miner_latched),
            ("kit machines share network", self.networks_shared),
            ("analysis station ran", self.station_ran),
            ("smelter ran (power-gated)", self.smelter_ran),
            ("assembler ran make_circuit", self.assembler_ran),
            ("solar generator charged", self.generator_charged),
            ("atmospheric oxygen revealed", self.oxygen_revealed),
            ("atmospheric pressure revealed", self.pressure_revealed),
            ("geological activity revealed", self.geo_revealed_after_scan),
            ("xalite site discovered", self.xalite_discovered),
            ("build list enqueued", self.build_enqueued),
            ("launch_site ran launch", self.launch_ran),
        ] {
            println!("    [{}] {label}", if ok { "x" } else { " " });
        }
        println!("══════════════════════════════════════════════════════════════\n");
    }
}

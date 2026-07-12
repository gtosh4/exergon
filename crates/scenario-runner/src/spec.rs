//! The tunable parameters of a standard run, loaded from a `.ron` file.
//!
//! Only the *knobs a balancing pass turns* live here: the world `seed`, the four themed research
//! target lists the victory grind drives toward, the mass-balanced successor `build_jobs`, and the
//! runaway-guard `max_secs`. The land→bootstrap→mine choreography ("how you play a standard run")
//! is fixed Rust in [`crate::Scenario::run_standard`] — it is the same content graph for every seed.

use std::path::Path;

use exergon::save::DifficultyTier;
use serde::Deserialize;

/// A parameterized standard run. See the module docs for what is a knob vs. fixed choreography.
#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioSpec {
    pub name: String,
    pub seed: u64,
    /// Difficulty of the run — sets the tech-tier ceiling (`TierCap`) and selects the driver
    /// (`Initiation` → the T3 minimal-successor escape; others → the full standard run). Defaults
    /// to `Standard` so an omitted field keeps the canonical standard scenario unchanged.
    #[serde(default = "default_difficulty")]
    pub difficulty: DifficultyTier,
    /// ResearchSpend target nodes by theme. Each list self-orders by the real tech graph (a request
    /// is a no-op until prereqs are met and the pool can pay), so order within a list is free.
    /// These drive the standard run only; the Initiation driver has a fixed T1–T3 path, so an
    /// Initiation spec omits them (they default empty).
    #[serde(default)]
    pub material_nodes: Vec<String>,
    #[serde(default)]
    pub engineering_nodes: Vec<String>,
    #[serde(default)]
    pub discovery_nodes: Vec<String>,
    #[serde(default)]
    pub synthesis_nodes: Vec<String>,
    /// The successor build list `(recipe_id, count)`, mass-balanced from the successor tree.
    #[serde(default)]
    pub build_jobs: Vec<(String, usize)>,
    /// Runaway guard for the victory grind, in simulated seconds.
    #[serde(default = "default_max_secs")]
    pub max_secs: f32,
}

fn default_max_secs() -> f32 {
    40_000.0
}

fn default_difficulty() -> DifficultyTier {
    DifficultyTier::Standard
}

/// Read + parse a scenario spec from a `.ron` file. Paths are resolved against the current dir, so
/// run the binary from the repo root (same as the game — `assets/` must be reachable too).
pub fn load_spec(path: impl AsRef<Path>) -> Result<ScenarioSpec, String> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("reading scenario {}: {e}", path.display()))?;
    ron::from_str(&text).map_err(|e| format!("parsing scenario {}: {e}", path.display()))
}

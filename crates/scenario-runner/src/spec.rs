//! A prescriptive scenario, loaded from a `.ron` file: the world `seed`, the `difficulty` (tier
//! ceiling), the runaway-guard `max_secs`, and the ordered `steps` list that *is* the run. The
//! step interpreter ([`crate::Scenario::run_steps`]) executes the list verbatim — re-sequence the
//! tech tree, machine buildout, and recipes by editing the data, no Rust changes.

use std::path::Path;

use exergon::save::DifficultyTier;
use serde::Deserialize;

/// A prescriptive scenario. `steps` is the run; the rest are top-level knobs.
#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioSpec {
    pub name: String,
    pub seed: u64,
    /// Difficulty of the run — sets the tech-tier ceiling (`TierCap`). Defaults to `Standard`.
    #[serde(default = "default_difficulty")]
    pub difficulty: DifficultyTier,
    /// The run script: an ordered list of [`Step`]s the interpreter executes verbatim (deploy a
    /// machine, research a node, install a config module, build the successor, …).
    pub steps: Vec<Step>,
    /// Runaway guard, in simulated seconds, applied per time-advancing step (a stall in any one
    /// step trips it).
    #[serde(default = "default_max_secs")]
    pub max_secs: f32,
}

/// Where a `Deploy`/`Place` of a `miner` goes.
#[derive(Debug, Clone, Deserialize)]
pub enum MineTarget {
    /// The origin-chunk starter deposit (the kit miner's home). No crafting — the miner is owned.
    Origin,
    /// The nearest fresh (non-origin) surface vein yielding this ore id. `Deploy` crafts the miners
    /// first; `count` sets how many to place on it.
    Vein(String),
}

/// One prescriptive instruction. Convenience verbs (`Deploy` owns-or-crafts; `Pump` arms an
/// auto-feed economy) sit alongside primitives (`Place`, `Craft`, `Ensure`) so a scenario can be
/// as hands-off or as explicit as a tuning pass wants. Positions are auto-assigned (a lane per
/// machine type) — the runner is about *what/when*, not *where*.
#[derive(Debug, Clone, Deserialize)]
pub enum Step {
    /// Own-or-craft the machine, then place + wire it (logistics, + power if `powered`). For a
    /// `miner`, `on` picks the deposit. `bind` names the placed machine for a later `Install`.
    Deploy {
        machine: String,
        #[serde(default = "default_true")]
        powered: bool,
        #[serde(default = "default_one")]
        count: usize,
        #[serde(default)]
        on: Option<MineTarget>,
        #[serde(default)]
        bind: Option<String>,
    },
    /// Place + wire an already-owned machine (a kit item), without crafting. Same fields as
    /// `Deploy`; panics if the machine isn't in storage.
    Place {
        machine: String,
        #[serde(default = "default_true")]
        powered: bool,
        #[serde(default = "default_one")]
        count: usize,
        #[serde(default)]
        on: Option<MineTarget>,
        #[serde(default)]
        bind: Option<String>,
    },
    /// Enqueue the full dependency tree to craft `count × item`, then wait until they're in storage.
    Craft { item: String, count: u32 },
    /// Top the craft queue up to `count` jobs of `recipe` (a feed / one-shot bulk prep — no wait).
    Ensure { recipe: String, count: usize },
    /// Request the tech node every frame and advance time until it unlocks (paid from the pool the
    /// economy feeds). Order `Research` after whatever `Recon`/`Deploy` its prereqs need.
    Research { node: String },
    /// Pilot the drone to the nearest deposit yielding `ore` so its one-shot discovery fires
    /// (honored only once the gated node's prereq is researched).
    Recon { ore: String },
    /// Enter drone-pilot and reveal the origin fog cell — the geological-activity scan.
    Scan,
    /// Own-or-craft the config module, then install it into the machine bound as `machine`
    /// (dedicates it — see machine dedication).
    Install { machine: String, module: String },
    /// Arm (`true`) or disarm (`false`) the auto-feed economy: while armed, every time-advancing
    /// step tops up the affordable analysis chains so `Research` steps get paid automatically.
    Pump(bool),
    /// Terminal: disarm the pump, clear the craft queue, enqueue the successor build list, and run
    /// to `RunState::Completed`.
    Build { jobs: Vec<(String, usize)> },
}

fn default_true() -> bool {
    true
}

fn default_one() -> usize {
    1
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

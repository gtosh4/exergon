//! Shared landing→victory **run harness** for Exergon, plus the prescriptive scenario spec (an
//! ordered [`Step`] list) and a milestone/statistics report. Both the `scenario` binary
//! (balancing/tuning) and the e2e smoke tests (`exergon`'s `tests/standard_full_run.rs` +
//! `tests/initiation_run.rs`) drive runs through this one `Scenario::run` code path, so what the
//! smoke tests prove is exactly what the balancing tool exercises.

mod harness;
mod report;
mod smoke;
mod spec;

pub use harness::{Scenario, load_registries};
pub use report::{ResearchSnapshot, RunReport};
pub use smoke::{SmokePlan, SmokeReport, Target, baseline_path, build_spec, plan_smoke, run_smoke};
pub use spec::{MineTarget, ScenarioSpec, Select, Step, load_spec};

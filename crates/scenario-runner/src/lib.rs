//! Shared landing→victory **run harness** for Exergon, plus the prescriptive scenario spec (an
//! ordered [`Step`] list) and a milestone/statistics report. Both the `scenario` binary
//! (balancing/tuning) and the e2e smoke tests (`exergon`'s `tests/standard_full_run.rs` +
//! `tests/initiation_run.rs`) drive runs through this one `Scenario::run` code path, so what the
//! smoke tests prove is exactly what the balancing tool exercises.

mod harness;
mod report;
mod spec;

pub use harness::Scenario;
pub use report::{ResearchSnapshot, RunReport};
pub use spec::{MineTarget, ScenarioSpec, Step, load_spec};

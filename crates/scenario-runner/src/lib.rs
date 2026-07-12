//! Shared landing→victory **run harness** for Exergon standard runs, plus a data-driven scenario
//! spec and a milestone/statistics report. Both the `scenario` binary (balancing/tuning) and the
//! e2e smoke test (`exergon`'s `tests/standard_full_run.rs`) drive runs through this one code path,
//! so what the smoke test proves is exactly what the balancing tool exercises.

mod harness;
mod report;
mod spec;

pub use harness::{DriveOutcome, GrindPlan, Scenario};
pub use report::{ResearchSnapshot, RunReport};
pub use spec::{ScenarioSpec, load_spec};

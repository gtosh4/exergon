//! Run an Exergon standard-run scenario headless and print its milestones + statistics.
//!
//! Usage (run from the repo root so `assets/` is reachable):
//!   cargo run -p scenario-runner --bin scenario -- scenarios/standard.ron
//!
//! The scenario file is a `.ron` [`ScenarioSpec`]: a world seed, difficulty, and the prescriptive
//! `steps` list that is the run. Edit the steps (or the seed) and re-run to compare balance.

use scenario_runner::{Scenario, load_spec};

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: scenario <scenario.ron>   (run from repo root)");
            std::process::exit(2);
        }
    };

    let spec = match load_spec(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    println!("running scenario `{}` (seed {:#x})…", spec.name, spec.seed);
    let mut scenario = Scenario::new(spec.seed);
    let report = scenario.run(&spec);
    report.print();

    if !report.completed {
        std::process::exit(1);
    }
}

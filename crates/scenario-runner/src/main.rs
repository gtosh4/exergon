//! Run an Exergon scenario headless and print its milestones + statistics.
//!
//! Two modes (run from the repo root so `assets/` is reachable):
//!
//!   # Run a prescriptive scenario file.
//!   cargo run -p scenario-runner --bin scenario -- run scenarios/standard.ron
//!
//!   # Auto-generate + run a SMOKE scenario proving one piece of content is reachable — no scenario
//!   # file needed. Picks the lowest difficulty that covers the target, deriving from the matching
//!   # e2e baseline. `<kind>` is item | node | recipe; the optional difficulty forces one.
//!   cargo run -p scenario-runner --bin scenario -- smoke item circuit_board
//!   cargo run -p scenario-runner --bin scenario -- smoke node ore_crusher standard
//!
//! A bare `scenario <path.ron>` (no subcommand) is still accepted as shorthand for `run <path.ron>`.

use exergon::save::DifficultyTier;
use scenario_runner::{Scenario, Target, load_spec, run_smoke};

mod balance;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("smoke") => smoke(args.get(1..).unwrap_or_default()),
        Some("run") => run_file(args.get(1).map(String::as_str)),
        Some("balance") => balance::balance(args.get(1..).unwrap_or_default()),
        // Back-compat: `scenario <path.ron>` with no subcommand runs the file.
        Some(path) => run_file(Some(path)),
        None => usage_exit(),
    }
}

/// `run <path>` — load a prescriptive scenario file and drive it to completion.
fn run_file(path: Option<&str>) {
    let Some(path) = path else { usage_exit() };

    let spec = match load_spec(path) {
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

/// `smoke <kind> <id> [difficulty]` — plan, derive, and run a smoke for one content id.
fn smoke(args: &[String]) {
    let (Some(kind), Some(id)) = (args.first(), args.get(1)) else {
        eprintln!("usage: scenario smoke <item|node|recipe> <id> [difficulty]");
        std::process::exit(2);
    };

    let target = match kind.as_str() {
        "item" => Target::Item(id.clone()),
        "node" => Target::Node(id.clone()),
        "recipe" => Target::Recipe(id.clone()),
        other => {
            eprintln!("error: unknown target kind `{other}` (expected item | node | recipe)");
            std::process::exit(2);
        }
    };

    let difficulty = match args.get(2).map(String::as_str) {
        None => None,
        Some(d) => match parse_difficulty(d) {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(2);
            }
        },
    };

    println!("smoke: proving `{kind} {id}` is reachable…");
    match run_smoke(&target, difficulty) {
        Ok(report) => {
            println!(
                "  chose difficulty {:?} (lowest that covers the target)",
                report.difficulty
            );
            report.report.print();
            println!(
                "  ── smoke result ──\n    [{}] `{kind} {id}` reached",
                if report.reached { "x" } else { " " }
            );
            if !report.reached {
                std::process::exit(1);
            }
        }
        // A preflight failure (unknown id, no producer, broken prereq chain) surfaces here as a
        // legible line instead of a mid-simulation panic — the whole point for content authors.
        Err(e) => {
            eprintln!("  smoke could not run: {e}");
            std::process::exit(1);
        }
    }
}

fn parse_difficulty(s: &str) -> Result<DifficultyTier, String> {
    match s.to_ascii_lowercase().as_str() {
        "initiation" => Ok(DifficultyTier::Initiation),
        "standard" => Ok(DifficultyTier::Standard),
        "advanced" => Ok(DifficultyTier::Advanced),
        "pinnacle" => Ok(DifficultyTier::Pinnacle),
        other => Err(format!(
            "unknown difficulty `{other}` (expected initiation | standard | advanced | pinnacle)"
        )),
    }
}

fn usage_exit() -> ! {
    eprintln!(
        "usage:\n  scenario run <scenario.ron>\n  scenario smoke <item|node|recipe> <id> [difficulty]\n  scenario balance <scenario.ron> [--seeds N]\n  scenario balance --emit <path> [--seeds N]\n(run from repo root)"
    );
    std::process::exit(2);
}

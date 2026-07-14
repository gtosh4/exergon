//! `balance <scenario.ron> [--seeds N]` — run one baseline scenario across N seeds and print a
//! pacing table. Worldgen is the only thing the seed varies (the step list is identical), so the
//! spread across seeds shows how seed-driven deposit layout swings run length and tier pace. Reuses
//! the exact single-run path (`Scenario::new(seed).run(&spec)`) the e2e tests and `run` use.

use scenario_runner::{RunReport, Scenario, load_spec};

const DEFAULT_SEEDS: u64 = 8;

/// A run slower than this (simulated seconds to victory) is dragging — hard fail. A generous
/// ceiling above a healthy standard run (~2.5h): this catches a run that is grossly stuck, not
/// ordinary seed variance (which the outlier flag surfaces instead).
const MAX_VICTORY_SECS: f32 = 14400.0; // 4h
/// A single tier that takes longer than this to climb is a progression wall — hard fail. Above the
/// ~1h a healthy standard run spends in its slowest tier.
const MAX_TIER_DURATION_SECS: f32 = 5400.0; // 90m
/// A seed whose victory time deviates from the mean by more than this fraction is flagged as an
/// outlier (warning only — variance is expected, this just surfaces the worst seeds).
const OUTLIER_FRAC: f32 = 0.35;

/// One row's derived metrics.
struct Row {
    seed: u64,
    completed: bool,
    victory_secs: f32,
    slowest_tier: Option<(u8, f32)>, // (tier, duration_secs)
    idle_themes: Vec<&'static str>,
}

pub fn balance(args: &[String]) {
    let (path, seeds) = parse_args(args);

    let spec = match load_spec(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "balance sweep: `{}` across {} seeds (base {:#x})…",
        spec.name, seeds, spec.seed
    );

    let mut rows: Vec<Row> = Vec::with_capacity(seeds as usize);
    for i in 0..seeds {
        // Derived deterministically from the base seed so any flagged run is reproducible via
        // `run` after temporarily setting the scenario's seed to the printed value.
        let seed = spec.seed.wrapping_add(i);
        eprint!("  seed {seed:#x} ({}/{seeds})… ", i + 1);
        let report = Scenario::new(seed).run(&spec);
        eprintln!("{}", if report.completed { "ok" } else { "DNF" });
        rows.push(derive_row(seed, &report));
    }

    print_table(&rows);

    // Hard failure (non-zero exit, CI-usable): any run that didn't finish, dragged past the victory
    // ceiling, or hit a tier wall. Outliers are warnings only.
    let breached = rows.iter().any(|r| {
        !r.completed
            || r.victory_secs > MAX_VICTORY_SECS
            || r.slowest_tier
                .is_some_and(|(_, d)| d > MAX_TIER_DURATION_SECS)
    });
    if breached {
        std::process::exit(1);
    }
}

fn parse_args(args: &[String]) -> (String, u64) {
    let mut path: Option<String> = None;
    let mut seeds = DEFAULT_SEEDS;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--seeds" => {
                seeds = match it.next().and_then(|s| s.parse::<u64>().ok()) {
                    Some(n) if n >= 1 => n,
                    _ => {
                        eprintln!("error: --seeds needs a positive integer");
                        std::process::exit(2);
                    }
                };
            }
            other if path.is_none() => path = Some(other.to_string()),
            other => {
                eprintln!("error: unexpected argument `{other}`");
                std::process::exit(2);
            }
        }
    }
    let Some(path) = path else {
        eprintln!("usage: scenario balance <scenario.ron> [--seeds N]");
        std::process::exit(2);
    };
    (path, seeds)
}

fn derive_row(seed: u64, report: &RunReport) -> Row {
    let slowest_tier = report
        .tier_progress
        .iter()
        .map(|t| (t.tier, t.last_secs - t.first_secs))
        .max_by(|a, b| a.1.total_cmp(&b.1));

    // A theme that never rose above zero across the whole curve was never produced — either
    // genuinely starved, or simply not needed by this scenario. Surfaced, not failed.
    let mut idle = Vec::new();
    idle_themes_into(report, &mut idle);

    Row {
        seed,
        completed: report.completed,
        victory_secs: report.virtual_secs,
        slowest_tier,
        idle_themes: idle,
    }
}

fn idle_themes_into(report: &RunReport, out: &mut Vec<&'static str>) {
    let max = |sel: fn(&scenario_runner::ResearchSnapshot) -> f32| {
        report
            .research_curve
            .iter()
            .map(sel)
            .fold(0.0_f32, f32::max)
    };
    if max(|s| s.material) == 0.0 {
        out.push("material");
    }
    if max(|s| s.engineering) == 0.0 {
        out.push("engineering");
    }
    if max(|s| s.discovery) == 0.0 {
        out.push("discovery");
    }
    if max(|s| s.synthesis) == 0.0 {
        out.push("synthesis");
    }
}

fn print_table(rows: &[Row]) {
    // Mean/stddev of victory time over the runs that actually finished (outlier baseline).
    let done: Vec<f32> = rows
        .iter()
        .filter(|r| r.completed)
        .map(|r| r.victory_secs)
        .collect();
    let mean = if done.is_empty() {
        0.0
    } else {
        done.iter().sum::<f32>() / done.len() as f32
    };

    println!("\n══════════════════════════════════════════════════════════════════════");
    println!("  seed              victory     slowest tier    idle themes / flags");
    println!("──────────────────────────────────────────────────────────────────────");
    for r in rows {
        let victory = if r.completed {
            format!("{:.2}h", r.victory_secs / 3600.0)
        } else {
            "DNF".to_string()
        };
        let tier = match r.slowest_tier {
            Some((t, d)) => format!("t{t} {:.0}m", d / 60.0),
            None => "—".to_string(),
        };

        let mut flags: Vec<String> = Vec::new();
        if !r.completed || r.victory_secs > MAX_VICTORY_SECS {
            flags.push("⚠SLOW".to_string());
        }
        if r.slowest_tier
            .is_some_and(|(_, d)| d > MAX_TIER_DURATION_SECS)
        {
            flags.push("⚠TIER".to_string());
        }
        if r.completed && mean > 0.0 && (r.victory_secs - mean).abs() > OUTLIER_FRAC * mean {
            flags.push("⚠OUTLIER".to_string());
        }
        for t in &r.idle_themes {
            flags.push(format!("idle:{t}"));
        }

        println!(
            "  {:#018x}  {:>8}    {:>12}    {}",
            r.seed,
            victory,
            tier,
            flags.join(" ")
        );
    }
    println!("──────────────────────────────────────────────────────────────────────");
    if mean > 0.0 {
        println!("  mean victory (finished runs): {:.2}h", mean / 3600.0);
    }
    println!("══════════════════════════════════════════════════════════════════════\n");
}

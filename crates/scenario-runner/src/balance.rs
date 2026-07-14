//! `balance <scenario.ron> [--seeds N]` — run one baseline scenario across N seeds and print a
//! pacing table. Worldgen is the only thing the seed varies (the step list is identical), so the
//! spread across seeds shows how seed-driven deposit layout swings run length and tier pace. Reuses
//! the exact single-run path (`Scenario::new(seed).run(&spec)`) the e2e tests and `run` use.
//!
//! `balance --emit <path> [--seeds N]` instead sweeps every canonical difficulty scenario and
//! writes the results to `<path>` as markdown — the standing "current balance state" doc. It is a
//! generated artifact: rerun to refresh, never hand-edit.

use scenario_runner::{RunReport, Scenario, ScenarioSpec, load_spec};

const DEFAULT_SEEDS: u64 = 8;

/// The canonical scenario per difficulty, swept by `--emit`. Only the difficulties with an authored
/// scenario appear (Advanced/Pinnacle have none yet).
const DIFFICULTIES: &[(&str, &str)] = &[
    ("Initiation", "scenarios/initiation.ron"),
    ("Standard", "scenarios/standard.ron"),
];

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
    let Args { path, seeds, emit } = parse_args(args);

    if let Some(emit_path) = emit {
        emit_state(&emit_path, seeds);
        return;
    }

    let Some(path) = path else {
        eprintln!(
            "usage: scenario balance <scenario.ron> [--seeds N]   |   scenario balance --emit <path> [--seeds N]"
        );
        std::process::exit(2);
    };

    let spec = load_or_exit(&path);

    println!(
        "balance sweep: `{}` across {} seeds (base {:#x})…",
        spec.name, seeds, spec.seed
    );

    let rows = sweep(&spec, seeds);
    print_table(&rows);

    // Hard failure (non-zero exit, CI-usable): any run that didn't finish, dragged past the victory
    // ceiling, or hit a tier wall. Outliers are warnings only.
    if rows.iter().any(row_breached) {
        std::process::exit(1);
    }
}

/// Run `spec` across `seeds` derived seeds, returning one [`Row`] each. Progress goes to stderr.
fn sweep(spec: &ScenarioSpec, seeds: u64) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::with_capacity(seeds as usize);
    for i in 0..seeds {
        // Derived deterministically from the base seed so any flagged run is reproducible via
        // `run` after temporarily setting the scenario's seed to the printed value.
        let seed = spec.seed.wrapping_add(i);
        eprint!("  seed {seed:#x} ({}/{seeds})… ", i + 1);
        let report = Scenario::new(seed).run(spec);
        eprintln!("{}", if report.completed { "ok" } else { "DNF" });
        rows.push(derive_row(seed, &report));
    }
    rows
}

/// Sweep every canonical difficulty scenario and write a markdown "current balance state" doc.
fn emit_state(path: &str, seeds: u64) {
    let mut md = String::new();
    md.push_str("# Balance State\n\n");
    md.push_str(
        "> **Generated** by `scenario balance --emit`. Do not hand-edit — rerun to refresh.\n>\n",
    );
    md.push_str(&format!(
        "> Each difficulty's canonical scenario, swept across {seeds} seeds (worldgen varies per \
         seed; the step list is fixed). Deterministic: same seeds → same numbers.\n\n",
    ));

    for (label, spath) in DIFFICULTIES {
        let spec = load_or_exit(spath);
        eprintln!("sweeping {label} (`{spath}`)…");
        let rows = sweep(&spec, seeds);
        md.push_str(&render_difficulty(label, spath, spec.seed, &rows));
    }

    if let Err(e) = std::fs::write(path, md) {
        eprintln!("error: could not write {path}: {e}");
        std::process::exit(1);
    }
    println!("wrote {path}");
}

/// Render one difficulty's sweep as a markdown section: a summary line plus the per-seed table.
fn render_difficulty(label: &str, path: &str, base_seed: u64, rows: &[Row]) -> String {
    let done: Vec<f32> = rows
        .iter()
        .filter(|r| r.completed)
        .map(|r| r.victory_secs)
        .collect();
    let mean = mean_of(&done);

    let mut s = String::new();
    s.push_str(&format!("## {label}  (`{path}`)\n\n"));
    s.push_str(&format!(
        "- base seed: `{base_seed:#x}`, {} runs\n",
        rows.len()
    ));

    if done.is_empty() {
        s.push_str("- outcome: **NO RUN FINISHED**\n\n");
    } else {
        let min = done.iter().copied().fold(f32::MAX, f32::min);
        let max = done.iter().copied().fold(f32::MIN, f32::max);
        s.push_str(&format!(
            "- outcome: **{}/{} finished**\n",
            done.len(),
            rows.len()
        ));
        s.push_str(&format!(
            "- victory: mean **{:.2}h** (min {:.2}h, max {:.2}h)\n",
            mean / 3600.0,
            min / 3600.0,
            max / 3600.0,
        ));
    }

    s.push_str("\n| seed | victory | slowest tier | flags |\n");
    s.push_str("|---|---|---|---|\n");
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
        let flags = row_flags(r, mean);
        s.push_str(&format!(
            "| `{:#018x}` | {victory} | {tier} | {} |\n",
            r.seed,
            if flags.is_empty() {
                "—".to_string()
            } else {
                flags.join(" ")
            },
        ));
    }
    s.push('\n');
    s
}

struct Args {
    path: Option<String>,
    seeds: u64,
    emit: Option<String>,
}

fn parse_args(args: &[String]) -> Args {
    let mut path: Option<String> = None;
    let mut seeds = DEFAULT_SEEDS;
    let mut emit: Option<String> = None;
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
            "--emit" => {
                emit = match it.next() {
                    Some(p) => Some(p.clone()),
                    None => {
                        eprintln!("error: --emit needs an output path");
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
    Args { path, seeds, emit }
}

fn load_or_exit(path: &str) -> ScenarioSpec {
    match load_spec(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
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

/// Whether a row trips a CI hard-fail (DNF, dragged past the victory ceiling, or a tier wall).
fn row_breached(r: &Row) -> bool {
    !r.completed
        || r.victory_secs > MAX_VICTORY_SECS
        || r.slowest_tier
            .is_some_and(|(_, d)| d > MAX_TIER_DURATION_SECS)
}

/// The warning/status flags for one row, given the finished-run mean victory time (for the outlier
/// check; pass `0.0` to skip it).
fn row_flags(r: &Row, mean: f32) -> Vec<String> {
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
    flags
}

/// Mean of the values, or `0.0` if empty.
fn mean_of(v: &[f32]) -> f32 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f32>() / v.len() as f32
    }
}

fn print_table(rows: &[Row]) {
    // Mean of victory time over the runs that actually finished (outlier baseline).
    let done: Vec<f32> = rows
        .iter()
        .filter(|r| r.completed)
        .map(|r| r.victory_secs)
        .collect();
    let mean = mean_of(&done);

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

        println!(
            "  {:#018x}  {:>8}    {:>12}    {}",
            r.seed,
            victory,
            tier,
            row_flags(r, mean).join(" ")
        );
    }
    println!("──────────────────────────────────────────────────────────────────────");
    if mean > 0.0 {
        println!("  mean victory (finished runs): {:.2}h", mean / 3600.0);
    }
    println!("══════════════════════════════════════════════════════════════════════\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(seed: u64, victory_secs: f32, tier: Option<(u8, f32)>) -> Row {
        Row {
            seed,
            completed: victory_secs.is_finite() && victory_secs > 0.0,
            victory_secs,
            slowest_tier: tier,
            idle_themes: Vec::new(),
        }
    }

    #[test]
    fn render_difficulty_reports_summary_and_per_seed_table() {
        let rows = vec![
            row(0x10, 9000.0, Some((3, 2400.0))),  // 2.50h, t3 40m
            row(0x11, 10800.0, Some((3, 2700.0))), // 3.00h, t3 45m
        ];
        let md = render_difficulty("Standard", "scenarios/standard.ron", 0x10, &rows);

        assert!(md.contains("## Standard  (`scenarios/standard.ron`)"));
        assert!(md.contains("2/2 finished"));
        // mean of 2.50h and 3.00h = 2.75h; min 2.50h, max 3.00h.
        assert!(md.contains("mean **2.75h** (min 2.50h, max 3.00h)"));
        // one table row per seed.
        assert!(md.contains("`0x0000000000000010`"));
        assert!(md.contains("`0x0000000000000011`"));
        assert!(md.contains("t3 40m"));
    }

    #[test]
    fn render_difficulty_marks_a_dnf_and_flags_it() {
        let rows = vec![row(0x20, -1.0, None)]; // DNF sentinel: not completed
        let md = render_difficulty("Initiation", "scenarios/initiation.ron", 0x20, &rows);

        assert!(md.contains("NO RUN FINISHED"));
        assert!(md.contains("DNF"));
        assert!(md.contains("⚠SLOW"));
    }
}

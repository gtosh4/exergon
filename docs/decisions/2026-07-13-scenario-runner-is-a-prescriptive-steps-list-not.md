---
type: decision
title: "Scenario runner is a prescriptive `steps` list, not fixed Rust choreography"
date: 2026-07-13
summary: "Scenario runner is a prescriptive steps list, not fixed Rust choreography"
tags: [scenarios]
---
# Scenario runner is a prescriptive `steps` list, not fixed Rust choreography


**Decision:** A scenario (`scenarios/*.ron`) is now an ordered **`steps` list** that the step interpreter (`Scenario::run` → `run_steps` → `exec_step`, `crates/scenario-runner`) executes verbatim — one real mechanic per `Step` (`Deploy`/`Place`/`Craft`/`Ensure`/`Research`/`Recon`/`Scan`/`Install`/`Pump`/`Build`). The previous per-difficulty Rust choreographies (`run_standard`, `run_initiation`, `drive_to_victory`) and the data "knobs" they read (`material_nodes`/…/`synthesis_nodes`, `build_jobs`) are deleted. `ScenarioSpec` is now `seed` + `difficulty` (tier ceiling only) + `steps` + `max_secs`. Machine positions are auto-assigned (a lane per machine type) — the runner is about *what/when*, not *where*.

**Rationale:** The runner exists to **tune progression, tech-tree gating, and recipe balance** by editing data, but the old format only exposed research-target and build lists — the entire machine buildout, config-module installs, recon timing, and terminal sequence were hardcoded, so re-sequencing any of that meant editing Rust. A prescriptive step list puts the whole run in the `.ron`: a balancing pass reorders `Research`/`Deploy`/`Build` steps directly. The smoke-test milestone flags were decoupled at the same time — `RunReport::observe_flags` sticky-observes them from world state each frame, so the data-driven run needn't assign them. Both e2e paths (`tests/standard_full_run.rs` Standard, `tests/initiation_run.rs` Initiation) now drive `Scenario::run` over their `.ron`, so the tests prove exactly what the balancing binary exercises.

**Notes / tradeoff:** A prescriptive run is **slower in virtual time** than the old parallel pump — each `Research(node)` blocks until that one node's currency accrues, sequentially, whereas the deleted pump requested all target nodes at once and unlocked opportunistically (Standard ≈ 2.68h vs the old ≈ 0.83h; Initiation ≈ 1.56h). Well within the test's `[100s, 24h)` bound, and the readability/tunability of an explicit ordered list is worth it. Three interpreter idioms follow from the tech-graph unlock mechanisms: `ExplorationDiscovery` nodes auto-unlock the frame their `Recon` fires (no `Research` step), `ProductionMilestone` nodes are a `Research`-as-wait, and a multi-recipe item (e.g. the minimal launch site) uses `Ensure(recipe)` + `Place`. Author guide: `testing.md §3`.


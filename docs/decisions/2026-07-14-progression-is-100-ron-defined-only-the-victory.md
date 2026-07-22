---
type: decision
title: "Progression is 100% RON-defined; only the victory trigger is hardcoded"
date: 2026-07-14
summary: "Progression is 100% RON-defined; only the victory trigger is hardcoded"
tags: [tech-tree, progression, escape]
---
# Progression is 100% RON-defined; only the victory trigger is hardcoded


**Decision:** All tech-tree progression — which recipes/machines a node unlocks, and how a node is earned — lives entirely in `assets/tech_nodes/*.ron` and is applied by the generic `apply_effects`/`check_research_unlocks` engine (`src/research/mod.rs`). The **only** progression left in Rust is the *victory kernel*: the `EscapeObjective` marker → `escape_objective_system` → `EscapeEvent` chain (`src/escape/mod.rs`), which fires the win when a recipe completes on an escape-objective machine. The stray `unlock_gateway_on_discovery` system (hardcoded: on `gateway_ruins` discovery, insert the `activate_gateway` recipe) was **deleted** and folded into the `gateway_theory` node, which already gates on that same `ExplorationDiscovery("gateway_ruins")` and now also carries `UnlockRecipes([activate_gateway])`.

**Rationale:** Content authors work only in RON via the `exergon-assets` tooling; any unlock hidden in Rust is invisible to them and to the content lint, so it silently drifts. `gateway_theory` firing on the same discovery event made the hardcode pure duplication. Keeping the victory *trigger* hardcoded is deliberate — it's a single engine primitive (a recipe completing on a marked machine), not progression data, and every escape structure routes through it.

**Research currency = item + naming convention (kept, not refactored to a field):** A recipe output `research.<theme>` (or legacy `research_points` → `material`) is credited to the `ResearchPool` under `<theme>` instead of stored, decoded by `research_theme_of`. New point types need only a new `research.<theme>` output + item def — no Rust. We considered promoting this to an explicit `ItemDef` field but chose to keep the convention (less churn; already moddable). The four in-use currencies (`research.{material,engineering,discovery,synthesis}`) now have real item defs for discoverability + lint validity.

**Tooling added:** (1) a whole-tree **content lint** (`tests/content_lint.rs`, in `cargo test`) that rejects dangling/inverted prerequisites, unlocks of undefined recipes/templates/items, and recipes no node can reach — it caught 9 real gaps on introduction (a nonexistent `drone_scan` unlock, the unwired `wash→dust→ingot` smelt recipes, plate-scrap recycling, the whole-metal `draw_metal` template, and the unwired coal-combustion power path). (2) a **balance sweep** subcommand, `scenario balance <file> [--seeds N]`, that runs one baseline across N seeds and flags victory-time / tier-pacing outliers and DNFs (`crates/scenario-runner/src/balance.rs`).

**Notes:** The coal path was completed rather than deleted — a new `combustion_power` (Power, T2) node unlocks the `combustion_generator` machine + `make_combustion_generator` + `combust_coal`. Known still-latent (not lint-caught, machine-side): `wind_power` unlocks a `wind_generator` machine that has no make- or run-recipe. The dead `Observation` unlock vector and unused `exclusive_group` field are kept for later, like before.


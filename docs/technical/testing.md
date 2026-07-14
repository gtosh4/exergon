# Testing & Dev Tooling

Two things live here: how the end-to-end integration test is built and extended, and the
`assets` MCP server for inspecting and editing the RON content the game loads. They belong
together — you use the `assets` tools to look up the recipes / tech nodes a new test stage must
exercise.

Read `gdd.md` / the relevant `technical/*.md` for design intent; this doc is about *verifying*
that intent in code with minimal manual play.

---

## 1. Test layers

| Layer | Where | Speed | Purpose |
| --- | --- | --- | --- |
| System tests | `#[cfg(test)] mod tests` next to each system (e.g. `src/logistics/miner.rs`) | fastest | Test one system directly against a bare `World`/`App` — no full plugin graph. See `.claude/skills/bevy/ecs.md`. |
| Recipe/content tests | `tests/assembler_recipe.rs`, `tests/smelter_recipe.rs` | fast | One machine + one recipe through the real logistics plugin. |
| End-to-end run (Standard) | `tests/standard_full_run.rs` (+ `crates/scenario-runner`) | ~seconds | The whole vertical slice from a fixed seed: worldgen → placement → wiring → mining every raw material → analysis → research → power → crafting the successor → launch/escape. |
| End-to-end run (Initiation) | `tests/initiation_run.rs` | ~seconds | The tier-3 difficulty: earn the T1–T3 path under a `TierCap`, build the `minimal_successor` escape (no tier-4 titanium), launch. Asserts the cap holds (no tier-4+ node unlocks). |
| Content lint | `tests/content_lint.rs` | fast | Whole-tree integrity over all RON: no dangling or higher-tier prerequisites, no node unlocking an undefined recipe/template/item, and **no recipe left unreachable** by any node. Fails with the offending ids on any structural content gap. |

The e2e test is the regression net for "a real run still completes." It is the one place the
systems are proven to compose. **Every new gameplay stage on the landing→victory path gets a
stage added here** (see §3), so the dev loop is `cargo test` rather than launching the game.

The driving mechanics and the run itself live in the **`scenario-runner` workspace crate**, not in
the test file:

- `crates/scenario-runner/src/harness.rs` — the `Scenario` harness (placement, wiring, crafting,
  mining, recon, `advance_until`/`run_until`) plus the **step interpreter**: `Scenario::run` →
  `run_steps` → `exec_step` executes a scenario's ordered `steps` list, one real mechanic per
  [`Step`]. There is **no per-difficulty Rust choreography** — the run is data; `difficulty` only
  sets the `TierCap`.
- `crates/scenario-runner/src/spec.rs` — `ScenarioSpec` + the `Step`/`MineTarget` enums. A scenario
  is a world `seed`, a `difficulty` (tier ceiling), a `max_secs` runaway guard, and the **`steps`
  list that is the run** (`Deploy`/`Place`/`Craft`/`Ensure`/`Research`/`Recon`/`Scan`/`Install`/
  `Pump`/`Build`), loaded from a `.ron` file. Positions are auto-assigned — a lane per machine type.
- `crates/scenario-runner/src/report.rs` — `RunReport`, the milestones + statistics a run produces
  (tier climb pace, node-unlock timeline, research-currency curve, ore extracted, stage checks).
  The stage-check flags are **sticky-observed from world state** in `RunReport::observe_flags`, so
  the data-driven run needn't set them.

Each e2e test loads its `scenarios/*.ron`, calls `Scenario::run`, and asserts on the returned
`RunReport`. The **`scenario` binary** replays the same code path for balancing —
`cargo run -p scenario-runner --bin scenario -- scenarios/standard.ron` (or `initiation.ron`) from
the repo root prints the report. Copy a scenario, change the seed or re-sequence the `steps`, and
compare the printed milestones across runs.

For **seed variance**, `scenario balance <file> [--seeds N]` runs one baseline across N seeds
(worldgen varies, steps identical) and prints a pacing table — victory time, slowest tier, and
per-seed flags (`⚠SLOW`/`⚠TIER`/`⚠OUTLIER`, DNF). It exits non-zero if any seed drags past the
sanity ceilings, so it's CI-usable for catching a content change that wrecks a subset of seeds.

`scenario balance --emit <path> [--seeds N]` instead sweeps every canonical difficulty scenario
(`scenarios/*.ron`) and writes the results to `<path>` as markdown — this generates the standing
[`docs/balance-state.md`](../balance-state.md) "current balance state" doc. It's a generated
artifact: rerun to refresh, never hand-edit. See [`balance.md`](../balance.md) for how the sweep
fits the tuning loop.

---

## 2. How the e2e test drives time

Headless tests have no render loop, so wall-clock time barely advances between `app.update()`
calls and rate-based systems (mining, recipe progress, power) never make progress on their own.

The test fixes this with **`TimeUpdateStrategy::ManualDuration(dt)`**: every `app.update()`
advances the clock by exactly `dt`, independent of wall-clock. All the systems that integrate
`time.delta_secs()` then progress deterministically. The reusable primitive is:

```rust
advance_until(&mut app, dt, max_secs, |app| /* predicate */);
```

It steps the sim in fixed `dt` increments until the predicate holds, and panics if it is not
met within `max_secs` of *simulated* time (a runaway guard). The predicate is polled before
each step, so it can also observe transient state (e.g. "was the station ever mid-recipe").

Guidelines:
- `dt = 0.5` is fine for production grinds. Keep `dt` **below the shortest recipe/transition
  time** so no state edge is skipped.
- Keep `dt` small (≈`1.0/60.0`) while worldgen / physics settle — those run on `FixedUpdate`
  and a large `dt` forces many catch-up substeps.
- Prefer `advance_until(... predicate)` over a fixed frame count: it self-scales as content
  values (recipe times, costs, yields) change, so tuning a recipe won't silently break timing.

Do **not** reintroduce the old approach of hand-poking internal state (`accumulator = 1.0`,
`progress = 1000.0`). Let real time drive the systems — that is what makes the test meaningful.

---

## 3. Adding a stage to the e2e test

> For the full `Step` vocabulary and how to author a scenario from scratch, see
> [`scenarios.md`](scenarios.md). This section is the e2e-specific slice: slotting a stage into an
> existing scenario and asserting it.

A new gameplay stage on the landing→victory path (research tier, crafting step, exploration unlock,
escape) is added as a **`Step` in the `scenarios/*.ron` list**, not as Rust — nothing gameplay-wide
is injected, every unlock is earned. The pattern:

1. **Look up the content** the stage needs with the `assets` MCP tools (§4): recipe inputs/outputs,
   machine type, tech-node cost, prerequisites, and the unlock *mechanism* (`ResearchSpend` vs
   `ProductionMilestone` vs `ExplorationDiscovery`).
2. **Place the step in dependency order.** A `Research(node)` blocks until the node unlocks, so it
   must come after whatever earns its prereqs. `ExplorationDiscovery` nodes auto-unlock the frame
   their `Recon` fires (no `Research` step); `ProductionMilestone` nodes are a `Research`-as-wait;
   an item with several recipes uses `Ensure(recipe)` + `Place`. Arm `Pump(true)` early so the
   `Research` steps get paid; the terminal `Build` disarms it.
3. **Feed the raw inputs.** Add a `Deploy(miner, on: Vein("<ore>"))` for any new raw material the
   stage (or the auto-feed economy) consumes — the run mines everything for real.
4. **Assert the milestone** in the test (`tests/standard_full_run.rs` / `initiation_run.rs`): the
   node in `unlocked_nodes`, an ore mined, completion. If the stage is a *world-observable*
   milestone the smoke test should pin (a machine type reaching `Running`, a property revealed),
   add a sticky observation to `RunReport::observe_flags` and assert the flag — that keeps the
   assertion decoupled from the step ordering.

Keep the step list **self-documenting**: a comment on each section saying what it proves and which
content values (from the `assets` MCP tools) it depends on, so a later content tweak points the
reader straight at the step / assertion to update.

The target arc, in stage order (from `tech_path escape_synthesis`):

```
science_basics → ore_extraction(30) → basic_smelting → basic_processing(150)
  → advanced_processing(300) → resonite_engineering(500)
  → exotic_materials / gateway_theory (ExplorationDiscovery, needs drone recon)
  → escape_synthesis(1000) → forge_gateway_key → activate_gateway  == victory
```

The eventual goal is landing→victory for **each difficulty**, parameterised on `MASTER_SEED`
plus a difficulty config; keep the stage bodies difficulty-agnostic so that loop is cheap.

### Cost / runtime

Simulated seconds ≈ `Σ(stage milestone times)`; wall-clock ≈ that divided by `dt`, so a full
victory run stays in the seconds range. If a grind stage's `max_secs` guard trips, the deposit
has likely decayed toward its yield floor — raise `max_secs`, don't lower the milestone.

---

## 4. `assets` — RON content MCP server

`crates/assets/src/main.rs` (the `exergon-assets` workspace crate, binary `assets`) is a **Model
Context Protocol (MCP) stdio server** that loads *and edits* the game's content through the **real
(de)serializers** (`load_ron_dir`, `build_recipe_graph()`), so what a tool returns/writes is
exactly what the game loads — schema drift or malformed RON shows up here, not just at runtime.
Prefer it over reading/editing `assets/**.ron` by hand.

It is registered for this repo in `.mcp.json` as `exergon-assets`, so Claude Code auto-discovers
it. It launches via `cargo run -q -p exergon-assets --bin assets` from the repo root (so `assets/`
is reachable); logs go to stderr, stdout is the JSON-RPC channel.

### Tools

The write surface is **generic over a `kind` argument** — five CRUD tools plus two discovery
tools, so the whole surface is ~13 tools rather than one set per type:

| tool | purpose |
|---|---|
| `list_kinds` | the kinds this server manages + each kind's identity field |
| `describe_kind` | JSON schema for a kind's entity (call before `create_asset`/`update_asset` to see required fields) |
| `list_assets` | `{kind}` → all ids of that kind |
| `get_asset` | `{kind, id}` → one entity |
| `create_asset` | `{kind, value}` → create from a JSON object, validated against the kind's schema (errors if the id exists) |
| `update_asset` | `{kind, id, patch}` → JSON merge-patch (`{ "energy_cost": 50 }`) — nested objects merge, arrays/scalars replace wholesale |
| `delete_asset` | `{kind, id}` → remove the file |
| `query_assets` | `{kind, jq}` → run a jq program (pure-Rust `jaq`) over the JSON array of every entity of `kind`; a single output is returned bare, a multi-value stream as an array |

`query_assets` is the general-purpose search/query tool over a kind — a jq program runs over the
JSON array of every entity of that kind, e.g. `query_assets kind="recipe" jq="[.[] | {id,
energy_cost}] | sort_by(-.energy_cost) | .[:5]"` (5 costliest recipes),
`query_assets kind="tech" jq="[.[] | select(.id | test(\"circuit|silicon\")) | .id]"`
(ids matching a pattern), or `query_assets kind="item" jq="length"` (a count).

`kind` is one of: `recipe`, `tech` (tech node), `item`, `material`, `form_group`,
`recipe_template`, `vein`, `layer`, `biome`, `deposit`, `machine`, `placeable`,
`planet_archetype`, `seed`. The `id` is the entity's `id` field, except `placeable` (its
`item.id`) and `planet_archetype` / `seed` (their `name`); curated seeds live in the single
`curated.ron` list. The block texture atlas has its own pair (it isn't an id-keyed entity):
`get_texture_manifest` / `update_texture_manifest`.

Resolved-graph **read** queries — the full graph, including template-expanded recipes and derived
items that have no backing file:

| tool | purpose |
|---|---|
| `resolve_recipe` | one recipe from the resolved graph (incl. template-expanded) |
| `list_all_recipes` | every resolved recipe id |
| `tech_path` | prerequisite chain to reach a node, in dependency order |
| `item_uses` | recipes that produce / consume an item |

`tech_path` is the tool for sequencing e2e stages — it returns prerequisites before the node that
needs them, i.e. the order the test must unlock them in (e.g. `tech_path basic_processing` →
`["basic_smelting", "basic_processing"]`).

**Writes are canonical:** the server re-emits every field, so `#[serde(default)]` fields (e.g.
`energy_output`, `template_id`, `max_reach`) become explicit on any file it rewrites. The file
still loads identically.

Manual poke without an MCP client — newline-delimited JSON-RPC over stdio:

```
$ printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"c","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_asset","arguments":{"kind":"recipe","id":"make_miner"}}}' \
  | cargo run -q -p exergon-assets --bin assets
```

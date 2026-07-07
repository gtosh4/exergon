# Testing & Dev Tooling

Two things live here: how the end-to-end integration test is built and extended, and the
`assets` CLI for inspecting the RON content the game loads. They belong together — you use
the `assets` CLI to look up the recipes / tech nodes a new test stage must exercise.

Read `gdd.md` / the relevant `technical/*.md` for design intent; this doc is about *verifying*
that intent in code with minimal manual play.

---

## 1. Test layers

| Layer | Where | Speed | Purpose |
| --- | --- | --- | --- |
| System tests | `#[cfg(test)] mod tests` next to each system (e.g. `src/logistics/miner.rs`) | fastest | Test one system directly against a bare `World`/`App` — no full plugin graph. See `.claude/skills/bevy/ecs.md`. |
| Recipe/content tests | `tests/assembler_recipe.rs`, `tests/smelter_recipe.rs` | fast | One machine + one recipe through the real logistics plugin. |
| End-to-end run | `tests/landing_to_first_research.rs` | ~seconds | The whole vertical slice from a fixed seed: worldgen → placement → wiring → mining → analysis → research → (future) crafting → escape. |

The e2e test is the regression net for "a real run still completes." It is the one place the
systems are proven to compose. **Every new gameplay stage on the landing→victory path gets a
stage added here** (see §3), so the dev loop is `cargo test` rather than launching the game.

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

Each stage of the game (research tier, crafting step, exploration unlock, escape) becomes one
labelled block in `land_generate_place_wire_mine_and_complete_first_research`, appended after
the previous stage. The pattern is always the same:

1. **Look up the content** the stage needs with the `assets` CLI (§4): recipe inputs/outputs,
   machine type, tech-node cost and prerequisites.
2. **Set up** whatever the stage consumes — place & wire the machines via `place()` / `connect()`
   (the real `WorldObjectEvent` / `CableConnectionEvent` contracts), provision storage, and stub
   the machine's port layout in `MachinePortLayouts`.
3. **Inject gating you are not testing.** The mechanic under test is the sim loop, not the
   tech-tree gate — insert prerequisite recipes/nodes directly into `TechTreeProgress`
   (`.unlocked_recipes` / `.unlocked_nodes`), the same way `basic_analysis` / `basic_smelting`
   are injected today. Only exercise a gate directly when the gate *is* the thing under test.
4. **Advance time** with `advance_until(...)` until the stage's milestone holds (research
   threshold reached, item crafted, node unlocked).
5. **Assert** the milestone: node in `unlocked_nodes`, item count in a `StorageUnit`, points
   deducted, etc. Assert the mechanism actually ran (e.g. a machine reached `Running`), not
   just that an end resource appeared — otherwise a stage can pass without doing any work.

Keep stages **surgical and self-documenting**: a comment saying what the stage proves and which
content values (from the `assets` CLI) it depends on, so a later content tweak points the reader
straight at the assertion to update.

The target arc, in stage order (from `assets path escape_synthesis`):

```
science_basics → ore_extraction(30) → basic_smelting → basic_processing(150)
  → advanced_processing(300) → resonite_engineering(500)
  → alien_materials / gateway_theory (ExplorationDiscovery, needs drone recon)
  → escape_synthesis(1000) → forge_gateway_key → activate_gateway  == victory
```

The eventual goal is landing→victory for **each difficulty**, parameterised on `MASTER_SEED`
plus a difficulty config; keep the stage bodies difficulty-agnostic so that loop is cheap.

### Cost / runtime

Simulated seconds ≈ `Σ(stage milestone times)`; wall-clock ≈ that divided by `dt`, so a full
victory run stays in the seconds range. If a grind stage's `max_secs` guard trips, the deposit
has likely decayed toward its yield floor — raise `max_secs`, don't lower the milestone.

---

## 4. `assets` — RON content query CLI

`src/bin/assets.rs` loads the game's content through the **real loaders**
(`build_recipe_graph()`, `load_ron_dir::<NodeDef>`), so what it prints is exactly what the game
deserializes — schema drift or a malformed RON shows up here, not just at runtime. Prefer it
over reading `assets/**.ron` by hand.

Run from the repo root (so `assets/` is reachable):

```
cargo run -q --bin assets recipe <id>    # one recipe: inputs, outputs, machine, time, energy
cargo run -q --bin assets recipes        # list every recipe id
cargo run -q --bin assets tech <id>      # one tech node: tier, unlock cost, prereqs, effects
cargo run -q --bin assets techs          # list every tech node id
cargo run -q --bin assets path <node>    # full prerequisite chain to reach a node
cargo run -q --bin assets uses <item>    # recipes that produce / consume an item
```

Examples:

```
$ cargo run -q --bin assets recipe make_miner
make_miner
  machine : assembler (tier 1)
  time    : 25s   energy_cost: 0
  inputs  : 25xstone, 20xiron_ore
  outputs : 1xminer

$ cargo run -q --bin assets path basic_processing
prerequisite chain for basic_processing:
  basic_smelting (PrerequisiteChain)
  basic_processing (ResearchSpend(150))
```

`path` is the tool for sequencing e2e stages — it prints prerequisites before the node that
needs them, i.e. the order the test must unlock them in.

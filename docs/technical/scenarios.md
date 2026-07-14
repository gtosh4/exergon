# Authoring Scenarios

A **scenario** is a prescriptive, data-driven run: a `.ron` file whose ordered `steps` list *is* the
run, executed verbatim by the step interpreter (`crates/scenario-runner`, `Scenario::run_steps`).
The same files drive three things:

- the **e2e regression tests** (`tests/standard_full_run.rs`, `tests/initiation_run.rs`),
- the **balancing tool** (`cargo run -p scenario-runner --bin scenario -- run <file>`),
- the **smoke generator** (`smoke_test` derives a minimal scenario from these baselines).

So a scenario is the one place the landing→victory choreography lives — re-sequence the tech tree,
machine buildout, and recipes by editing data, no Rust. This doc is the reference (every `Step`) and
the approach (how to write one). It is the source the Rust doc-comments in `spec.rs` mirror; when
they disagree, `spec.rs` wins (it's what runs) — fix this doc.

Related: [`testing.md` §3](testing.md#3-adding-a-stage-to-the-e2e-test) (adding a stage to an
existing e2e scenario + asserting it), [`contributing-content.md`](../contributing-content.md)
(validating one item without authoring a whole scenario).

## File shape

```ron
#![enable(implicit_some)]
(
    name: "standard",         // label in the printed report
    seed: 0xE7E60007,         // world seed — fixes worldgen, deposits, deposit positions
    difficulty: Standard,     // tier ceiling: Initiation=3, Standard=5, Advanced=7, Pinnacle=10
    max_secs: 40000.0,        // runaway guard, per time-advancing step (simulated seconds)
    steps: [ /* ... */ ],
)
```

`seed` + `difficulty` are the only knobs; everything else is `steps`. `max_secs` trips if any single
time-advancing step stalls (a research that never gets paid, a craft whose inputs never arrive).
Positions are **auto-assigned** (a lane per machine type) — a scenario says *what* and *when*, never
*where*. (`require` / `select` also exist but are a reserved seam for future dynamic node selection;
leave them off — the full tree is always available today.)

## Step reference

Convenience verbs (`Deploy`, `Pump`) own-or-craft and automate; primitives (`Place`, `Craft`,
`Ensure`) are explicit. Mix them to taste — hands-off early, explicit in a terminal tuning pass.

| Step | Fields | What it does |
|---|---|---|
| `Deploy` | `machine`, `powered=true`, `count=1`, `on?`, `bind?` | **Own-or-craft** the machine (crafts it from mined ore if not in storage), then place + wire it — logistics, plus power if `powered`. For a `miner`, `on` picks the deposit. `bind` names the placed machine for a later `Install`. |
| `Place` | same as `Deploy` | Place + wire an **already-owned** machine (a landing-kit item) — no crafting. **Panics if it isn't in storage.** Use for the kit; `Deploy` for everything earned. |
| `Craft` | `item`, `count` | Enqueue the **full dependency tree** to craft `count × item`, then **wait** until they're in storage. The "make me one of these, mining/smelting/assembling whatever it takes" verb. |
| `Ensure` | `recipe`, `count` | Top the craft queue up to `count` jobs of one `recipe` — a feed / bulk prep, **no wait**. Use to pre-stage bulk intermediates before a terminal `Build`. |
| `Research` | `node` | Request the tech node every frame and advance time **until it unlocks**, paid from the pool the economy feeds. Blocks — so it must come *after* whatever earns its prereqs. |
| `Recon` | `ore` | Pilot the drone to the nearest deposit yielding `ore` so its one-shot discovery fires. Honored only once the gated node's prereq is researched. Unlocks `ExplorationDiscovery` nodes. |
| `Scan` | — | Enter drone-pilot and reveal the origin fog cell (the geological-activity scan). |
| `Install` | `machine`, `module` | Own-or-craft the config module, install it into the machine `bind`-named earlier — **dedicates** that machine (machine dedication; needed for some refinery recipes). |
| `Pump` | `bool` | Arm (`true`) / disarm (`false`) the **auto-feed economy**: while armed, every time-advancing step tops up the affordable analysis chains so `Research` steps get paid automatically. |
| `Build` | `jobs: [(recipe, count)]` | **Terminal.** Disarm the pump, clear the queue, enqueue the successor build list, and run to completion (`RunState::Completed`). The last step. |

`on` (for miners) is a **`MineTarget`**:
- `Origin` — the origin-chunk starter deposit (the kit miner's home). Owned, no crafting.
- `Vein("<ore_id>")` — the nearest fresh (non-origin) surface vein yielding that ore. `Deploy` crafts
  the miners first; `count` sets how many go on it.

## Unlock mechanisms — how a node gets earned

A `Research(node)` step behaves differently by the node's `primary_unlock` (look it up with the
`assets` MCP: `get_asset {kind:"tech", id}`):

- **`ResearchSpend`** — costs currency. Needs `Pump(true)` armed (or explicit `Ensure` of analysis
  chains) so the pool can pay. The common case.
- **`ProductionMilestone`** — unlocks when a tally trips (e.g. 100 iron_ingot produced). A `Research`
  step here just **advances time until it trips** — make sure the economy is actually producing that
  material first.
- **`ExplorationDiscovery`** — unlocks the frame its `Recon(ore)` fires. **No `Research` step needed**;
  place the `Recon` after its prereq node is researched.
- **`PrerequisiteChain`** — unlocks as soon as prereqs are met (no cost); a `Research` step confirms it.

## The approach — phases of a run

Scenarios follow a rough arc. Write it in labelled sections (see `scenarios/standard.ron`):

1. **Bootstrap.** `Pump(true)`, then `Place` the four landing-kit machines (`solar_generator`,
   `miner` on `Origin`, `assembler`, `analysis_station`), then `Research(ore_extraction)` so miners
   can be crafted.
2. **Real ore + scale.** `Deploy(miner, on: Vein("copper_ore"), count: N)` and any early ore, then
   scale research throughput (`Deploy(analysis_station, ...)`) and power (`Deploy(solar_generator, ...)`).
   Deploy machines **while veins are still rich** — mining degrades yield.
3. **Research economy + buildout.** Interleave `Research(node)` with the `Deploy` / `Recon` / `Scan`
   its gates need, **in dependency order**. Each new processing tier deploys its machine
   (`crusher`, `refinery`, `washer`, `plate_roller`, `advanced_assembler`) as it unlocks;
   `Install` config modules where a recipe needs a dedicated machine.
4. **Terminal prep.** `Pump(false)` to disarm, then `Ensure(recipe, count)` the bulk intermediates the
   successor needs (explicit so shared intermediates aren't double-reserved), `Place` the final
   machines (`advanced_assembler`, `launch_site`).
5. **Build + launch.** One `Build(jobs: [...])` listing every successor recipe and count. Runs to
   victory.

### Ordering rules (the ones that bite)

- **`Research` blocks.** Nodes must appear in dependency order; a node whose prereqs aren't earned yet
  will stall the step until `max_secs` trips.
- **Feed before you consume.** Any raw material a stage (or the auto-feed economy) needs wants a
  `Deploy(miner, on: Vein("<ore>"))` earlier. The run mines everything for real — nothing is injected.
- **Recon needs its prereq first.** An `ExplorationDiscovery` node's `Recon` is only honored once the
  gating node is researched.
- **Deploy early, while ore is rich.** Sequential `Research` runs long; place ore-hungry machines
  before veins thin.
- **Keep it self-documenting.** A comment per section stating what it proves and which content values
  (from the assets MCP) it depends on — so a later content tweak points straight at the step to update.

## Iterating

```sh
# Run a scenario and print milestones + the research-currency curve + stage checks.
cargo run -p scenario-runner --bin scenario -- run scenarios/standard.ron
```

Read the printed report: tier pace (`first_secs → last_secs` per tier), the currency curve (is a
`Research` starved?), ore extracted, and the stage checks. A stall → the report shows how far it got
before `max_secs`; usually a missing feed or an out-of-order `Research`. Tune values or re-sequence
and re-run — no rebuild of game logic needed, it's all data.

To prove a *single* new item/node/recipe is reachable without hand-authoring, use `smoke_test`
instead — see [`contributing-content.md`](../contributing-content.md).

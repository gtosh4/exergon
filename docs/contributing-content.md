# Contributing Content

**For anyone who wants to add game content — tech-tree nodes, items, recipes, machines — without touching Rust or editing files by hand.**

Content lives in RON files under `assets/`, but you never edit them directly. You describe what you want to Claude; the **content-designer** agent authors it through the `exergon-assets` tool (which writes through the game's real serializers, so malformed content is caught immediately) and then *proves it works* with a smoke test before you're done.

## The loop

1. **Describe the content** to Claude, in plain English (see the shapes below).
2. Claude (content-designer) **authors** it — creating/updating the RON via the `exergon-assets` tool.
3. Claude **smoke-tests** it — `smoke_test` auto-builds a minimal run from the closest tested baseline, plays it headless, and checks your content is actually reachable and craftable.
4. **Green** → done. **Red** → Claude reads the plain-language reason, fixes the content, and retries.

You don't write scenarios, run any commands, or read RON. The smoke test is the "does it work?" button.

## What to say

Give Claude enough to place the content in the existing web. Useful shapes:

- **New recipe / item:**
  > Add a recipe `make_plasma_coil` that makes a `plasma_coil` from 1 `resonite_lattice` + 1 `circuit_board`, in the advanced assembler. Unlock it from a new `plasma_forming` node.

- **New tech node:**
  > Add a tier-4 tech node `plasma_forming`, prerequisite `advanced_assembler`, that unlocks `make_plasma_coil`. It should cost engineering currency.

  (A node's prerequisites must be the same tier or lower — a higher-tier prereq is a content bug the [lint](#validation) rejects.)

- **Tuning:**
  > `make_circuit` feels too cheap — bump its energy cost and time so it paces with tier 2.

Always name **what it connects to** — the node that unlocks a recipe, the items an input comes from. Content that connects to nothing is the most common thing a smoke test rejects.

## Reading the result

The smoke test returns one of:

- **`ok: true, reached: true`** — your content is reachable and works in a real run at the chosen difficulty. 
- **`ok: false, failure_reason: "..."`** — a content gap, in plain language. Common reasons:
  - *"item `X` has no producing recipe"* — an input nothing makes. Add a recipe for it, or use an existing item.
  - *"prerequisite `X` is missing from the tech tree"* — a node points at a prereq that doesn't exist. Fix the id.
  - *"unknown recipe/node/item `X`"* — a typo, or the content wasn't created yet.

Claude acts on these for you; you'll see it fix and re-test.

## What the smoke test does *not* prove

- **Balance/pacing** — it proves *reachable*, not *fun* or *well-tuned*. Currency curves and pacing are a separate judgement (the content-designer flags concerns).
- **World-law interactions** — it validates content in a forced context, so it won't tell you whether a world's physics naturally surface your node. That's a playtest question.
- **Deep new chains** — a brand-new node whose prerequisites aren't already on a tested baseline path may need its prereq chain smoke-tested first. Claude will sequence this.

## Validation

Two layers catch content problems:

- **`smoke_test` (per item)** — proves *your one thing* is reachable and craftable in a real run. Use it while authoring, as above.
- **The content lint (whole tree)** — `cargo test --test content_lint` sweeps *all* content at once and fails on structural gaps `smoke_test` can't see across the graph: a prerequisite that doesn't exist, a prerequisite from a higher tier, a node unlocking a recipe/template/item that isn't defined, or a recipe **no node ever unlocks** (unreachable dead content). It runs in the normal `cargo test` suite, so a bad edit trips it before commit. Any failure prints the exact offending ids — fix the RON, not the test.

### Research currency is a naming convention

A recipe that outputs an item id `research.<theme>` (e.g. `research.engineering`) — or the legacy `research_points` (→ theme `material`) — has that output **credited to the research pool** under `<theme>` instead of being stored as a physical item (`research_theme_of`, `src/research/mod.rs`). To add a new research point type, just output `research.<newtheme>` from an analysis recipe and add a matching `assets/items/research.<newtheme>.ron` item def — no Rust change. Tech nodes spend it via `primary_unlock: ResearchSpend(type_id: "<theme>", amount)`.

## Under the hood (for the curious)

`smoke_test` is one tool on the `exergon-assets` server; it calls the scenario runner's `run_smoke`, which picks the lowest difficulty covering your content's tier, splices a target-exercising step onto the matching end-to-end baseline scenario (`scenarios/initiation.ron` / `standard.ron`), and runs it. The same thing is available at the terminal as `cargo run -p scenario-runner --bin scenario -- smoke <item|node|recipe> <id>`. See [`technical/scenarios.md`](technical/scenarios.md) for the scenario format the generator builds on.

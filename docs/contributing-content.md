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
  > Add a recipe `make_resonite_circuit` that makes a `resonite_circuit` from 1 `resonite_lattice` + 1 `circuit_board`, in the advanced assembler. Unlock it from the `resonite_engineering` node.

- **New tech node:**
  > Add a tier-4 tech node `resonite_engineering`, prerequisite `advanced_assembler`, that unlocks `make_resonite_circuit`. It should cost engineering currency.

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

## Under the hood (for the curious)

`smoke_test` is one tool on the `exergon-assets` server; it calls the scenario runner's `run_smoke`, which picks the lowest difficulty covering your content's tier, splices a target-exercising step onto the matching end-to-end baseline scenario (`scenarios/initiation.ron` / `standard.ron`), and runs it. The same thing is available at the terminal as `cargo run -p scenario-runner --bin scenario -- smoke <item|node|recipe> <id>`. See [`technical/scenarios.md`](technical/scenarios.md) for the scenario format the generator builds on.

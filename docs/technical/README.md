# Technical design docs
These docs describe the deep technical (Bevy, ECS, events, etc) implementation designs. They should be detailed enough to write integration tests without guessing so the tests can validate the implementation.

These should always be up to date or ahead of the code. When implementing new features or changes, always update the designs first then the code to match.

## Designs

### [`technical-design.md`](technical-design.md) — Technical Design Document
Implementation architecture for all core systems. Covers: seed system, recipe graph, tech tree, world & chunk system, multiblock machine system, logistics network, power system, drone system, science & research system, world reactivity, codex & meta-progression. Includes data structures and invariants. **Read the relevant section before implementing a system. Update when architecture decisions are made.**

### [`networks.md`](networks.md) — Network System Design
Generic cable network system and both concrete kinds (logistics and power). Covers ECS structure, topology systems (place/remove/split/merge), routing, and how power gating interacts with recipe start. **Read before touching `src/network/`, `src/logistics/`, or `src/power/`.**

### [`research.md`](research.md) — Science & Research System
ECS components, system step-by-step logic, events/messages, and edge cases for research stations, research pool, knowledge visibility, and the player-initiated unlock flow. VS and MVP differences noted inline. **Read before touching `src/research/` or adding research station recipes.**

### [`escape-condition.md`](escape-condition.md) — Escape Condition Design
ECS components, system logic (gateway charge, interact, status UI), events, edge cases, and integration test descriptions for the Initiation escape (alien gateway activation). Includes recipe system extension for catalyst inputs. MVP escape types outlined. **Read before implementing gateway activation, EscapeEvent, or RunState.**

### [`mining.md`](mining.md) — Mining & Deposit System
Ore extraction from surface deposits. Covers ECS components (`OreDeposit`, `MinerMachine`), miner placement and deposit linking, depletion curve, weighted ore sampling, manual mining, and deposit discovery. **Read before touching `src/logistics/miner.rs`, `src/drone/`, or deposit-related code in `src/world/generation.rs`.**

### [`drone.md`](drone.md) — Drone System Design
ECS components, system logic (Local↔Remote mode transition, fog-of-war reveal, sample collection, range scanning, multiple drone switching), events, edge cases, and execution order. VS and MVP scope noted inline. **Read before touching `src/drone/` or anything involving `PlayMode::DronePilot`.**

### [`aegis.md`](aegis.md) — Aegis System Design
ECS components, system logic (boundary check, Local mode constraint, atmospheric exposure, outpost beacon power, body switching), events, edge cases, and execution order. VS and MVP scope noted inline. **Read before implementing aegis fields, body switching, or outpost beacons.**

### [`crafting.md`](crafting.md) — Crafting System Design
ECS components, system logic (recipe execution, job dispatch, catalyst reservation, module effects, plan resolution), events, edge cases, and execution order. Resolves: Recipe Graph Runtime Integration, Catalyst Inputs, and Auto-crafting Job Dispatch todos. VS and MVP scope noted inline. **Read before implementing recipe execution, crafting jobs, or auto-crafting dispatch.**

### [`inventory.md`](inventory.md) — Inventory System Design
ECS components, system logic (hotbar, drone inventory, storage units, Terminal screen runtime data, goal tracker), events, edge cases, and execution order. Covers the no-personal-inventory model, hotbar-as-network-view, drone deposit flow, NetworkFlowLedger (Δ/min), and pin-based goal tracker. VS and MVP scope noted inline. **Read before implementing the hotbar, Terminal screen, drone deposit, or storage capacity.**

### [`machine-ui.md`](machine-ui.md) — Machine UI Technical Design
ECS components, system logic (open/close, identity, progress, power status, module slots, port binding editor, recipe table C/P flag editing), events, edge cases. Also defines the **revised `MachineJobPolicy`** (supersedes `crafting.md §4`) with per-recipe `RecipePolicy` carrying independent C/P flags and machine-level `CraftingJobMode`/`passive` defaults. VS and MVP scope noted inline. **Read before implementing the machine panel, `MachineJobPolicy`, `PortPolicy` editing, or `SlotBlockReason`.**

---

## Save Architecture

Two save scopes exist in every run:

**Run save** — one file per run. Contains world state, factory state, tech tree progress, research pools, drone positions, tunnel graph, run seed, and completion status. Runs are **never automatically deleted** — players can revisit completed runs. Each run save carries a header: seed string, difficulty tier, status (`InProgress` / `Completed` / `Abandoned`), start time, completion time, total run time.

**Meta save** — single file. Contains codex, unlocked content, blueprints, starting boons pool. Persists across all runs. Updated at run completion and on mid-run milestone triggers.

**Format and library:** Both scopes use **RON format** via `moonshine-save` (v0.6.1, Bevy 0.18 compatible). Saveable entities are tagged with `moonshine_save::Save`; rendering/aesthetic entities (particle effects, camera rigs, UI) are excluded via `moonshine_save::Unload`. `SQLite` is not used.

---

## TODO
These are systems/designs known to be needed **for the vertical slice** but without a doc yet. Write the spec before writing the code.

### Planet Identity & Seed System
Detailed spec for: seed parameters, planet property generation (atmospheric, geological, thermal axes), property-to-gameplay effect bindings, visual/UI surface for landing panel and property display, and property-to-decision connection validation. Required for vertical slice signal §3.1 (Seeded Planet Identity) and §3.2 (First-Hour Insight Beat).

**Partial coverage in `technical-design.md §1`:** seed string format, per-domain sub-seed derivation (keyed hash; stable domain keys), lazy chunk generation, backwards-from-terminal validity guarantee, seed versioning strategy, RNG algorithm (`rand` crate, `SmallRng`/`Pcg64`). **Missing:** planet property generation algorithm (atmospheric/geological/thermal axes and parameter ranges), property-to-gameplay effect bindings, visual/UI surface for the landing panel and in-run property display, and property-to-decision connection validation required by VS §3.2.

### Planning UI
Spec for the recipe browser, escape-item dependency graph view, machine count estimator, bottleneck/blocked-production alerts panel, and 3D network topology overlay. `machine-ui.md` covers the per-machine panel; this doc covers the cross-factory planning surface. Required for vertical slice signal §3.5 (Recipe Graph and Planner UX) and §3.6 (3D factory readability).

### Telemetry
Event schema (run start, property viewed, tech revealed, drone deployed, etc.), derived metric derivation (time-to-first-insight, blocked-state duration, etc.), and dev-build logging interface. Required for vertical slice §6 (Instrumentation) and playtest protocol §7.

### World & Chunk Generation
Procedural surface generation, deposit placement, discovery site generation, and how seed parameters shape geography. Required before world generation code can be written with testable invariants.

**Partial coverage in `technical-design.md §4`:** scale (1 unit = 1 m), world extent and bounded-radius model, layer extents (surface/underground/sky/orbital), terrain chunk system (64×64 m heightmap chunks), core zone deposit guarantee, deposit system (surface markers, two-stage discovery, depletion), tunnel graph structure, world generation sequence. **Missing:** discovery site generation (trigger types, placement rules, seeded positions at integration-test depth), biome assignment algorithm, chunk stream boundary conditions, and how seed parameters shape geography — all needed to write testable invariants.

### Tech Tree UI
Spec for the tech tree page, node reveal panel, and node state display: locked (shadow visible — category, tier, rarity), revealable, revealed, unlocked, and disabled/locked-out. Covers the node reveal interaction flow, blocked-reason display (why a node is not yet revealable), and the proactive choice-surfacing treatment for exclusive-group nodes — the UX form (modal, sidebar, tree highlight) is unresolved in `tech-tree-design.md` issue #9 and must be decided here. `tech-tree-design.md` covers content and data design; this doc covers the UI implementation surface. Required for vertical slice signal §3.3 (Minimal Tech Tree).

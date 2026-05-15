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

### [`planning-ui.md`](planning-ui.md) — Planning UI Technical Design
ECS components, system logic (Sankey production graph, per-node Inspector rail, Recipe Picker overlay, 3D network topology overlay with per-network filter), events, edge cases, and execution order. Plans future factory additions using ratio math — does not read live machine state. Multiple named plans per run; each is a saved `PlanState` component on a plan entity. Wireframe: `ui_mock/planner-wireframes.html`. VS and MVP scope noted inline. **Read before implementing the planner panel, Sankey graph, Inspector, Recipe Picker, or topology overlay.**

### [`telemetry.md`](telemetry.md) — Telemetry System
ECS resource structure, event schema (run lifecycle, first-occurrence, repeated), derived metrics, system logic, JSONL log format, and edge cases for development-build telemetry. Covers all VS §6 required events and derived metrics. `#[cfg(debug_assertions)]` gated — no analytics pipeline, no network. **Read before implementing the telemetry plugin, `TelemetryLog` resource, or adding new observable events.**

### [`generation.md`](generation.md) — World & Chunk Generation
Coordinate system, chunk streaming (spawn/despawn distances, hysteresis), heightmap generation (`HybridMulti<Perlin>`, ±50 m range), underground resource-domain query system (cell grid, biome bands, ellipsoidal veins), surface deposit placement (one per 64×64 m cell, 33% probability), discovery site placement (seeded XZ, drone proximity trigger), chunk boundary conditions (seam-free by construction), seed→geography mapping (all generation domains keyed independently from `DomainSeeds.world`), and 25 integration test invariants. VS vs. MVP scope (biomes, world bounds, core zone guarantee) noted inline. **Read before touching `src/world/generation.rs`, `src/world/ruins.rs`, `src/content/mod.rs` (resource domains/biomes/veins), or anything that places world objects.**

### [`tech-tree-ui.md`](tech-tree-ui.md) — Tech Tree UI Technical Design
ECS components, node visual states (Shadow/Partial/Revealed/Unlockable/Locked-Out), tier-paged questbook layout (BFS topological X, category Y, gate bridge cards, cross-tier port stubs), inspector rail, reveal overlay (tier ladder, before/after diff, prereq chain), exclusive-group choice modal (resolves `tech-tree-design.md` issue #9 — modal approach), top bar, events, edge cases, and 14 integration test invariants. VS scope: T1–T3, full reveal mechanic, tier gate display. **Read before implementing the tech tree panel, `TechTreePanelState`, reveal overlay, or exclusive-group surfacing.**

### [`planet-identity.md`](planet-identity.md) — Planet Identity & Seed System
ECS components, archetype-based property generation algorithm (3 VS archetypes; 6 float axes + hazard type), property-to-gameplay effect bindings (solar/combustion/geothermal/wind/thermodynamic/pressure modifiers with exact formulas), property visibility model (Hidden → Qualitative → Revealed with reveal triggers), landing panel UI (`PlayMode::Landing`), in-run Terminal Planet page, and insight beat feedback system (`PropertyDecisionValidated`). Also requires adding `planet` domain to `DomainSeeds`. **Read before implementing planet property generation, the landing panel, or the VS §3.1/§3.2 insight beat.**

---

## TODO
These are systems/designs known to be needed **for the vertical slice** but without a doc yet. Write the spec before writing the code.

### Save Architecture

Two save scopes exist in every run:

**Run save** — one file per run. Contains world state, factory state, tech tree progress, research pools, drone positions, tunnel graph, run seed, and completion status. Runs are **never automatically deleted** — players can revisit completed runs. Each run save carries a header: seed string, difficulty tier, status (`InProgress` / `Completed` / `Abandoned`), start time, completion time, total run time.

**Meta save** — single file. Contains codex, unlocked content, blueprints, starting boons pool. Persists across all runs. Updated at run completion and on mid-run milestone triggers.

**Format and library:** Both scopes use **RON format** via `moonshine-save` (v0.6.1, Bevy 0.18 compatible). Saveable entities are tagged with `moonshine_save::Save`; rendering/aesthetic entities (particle effects, camera rigs, UI) are excluded via `moonshine_save::Unload`. `SQLite` is not used.

Support for cloud saves (eg, Steam)

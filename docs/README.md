# Exergon — Docs Index

All design and technical documentation lives here. Read this file first to find where information is.

---

## Documents

### [`gdd.md`](gdd.md) — Game Design Document
The canonical design reference. Covers vision, design pillars, all game systems at the design level (seed system, tech tree, recipe graph, factory layer, world/exploration, power, science loop, meta-progression, escape condition, failure, moddability). Open questions are tracked inline and in §18. **Read the relevant GDD section before implementing any system.**

### [`design-decisions.md`](design-decisions.md) — Design Decisions Log
Rationale and context behind key decisions — the *why* behind what's in the GDD. Records alternatives considered and reasons they were rejected. Also captures decisions that were tentative or may be revisited. **Update this file whenever a significant design decision is made.**

### [`tech-tree-design.md`](tech-tree-design.md) — Tech Tree Content Design
Content design layer for the tech tree: node definitions, pacing targets, unlock structure. Sits between `gdd.md §7` (design intent) and implementation. Status: first draft, pacing unvalidated. **Update when tech tree nodes or unlock order changes.**

### [`vertical_slice.md`](vertical_slice.md) — Vertical Slice Signal Spec
Defines what the vertical slice must prove: the five core signals (first-hour insight, repeat-run discovery, Remote mode feel, 3D factory readability, Standard-length pacing), required features and interfaces per system, playtest protocol, instrumentation, and success criteria. Also lists deeper designs required before implementation plans are written. **Read before scoping or implementing any vertical-slice-targeted feature.**

### [`milestones.md`](milestones.md) — Milestones
Milestone ladder: Vertical Slice → Alpha → Demo (MVP) → Release → Post-release. Each milestone states its purpose, gate conditions, and what it explicitly does not require. **Check before asking whether a feature is in scope.**

---

## Technical Specs (`technical/`)

Deep implementation specs: ECS components, systems, events, edge cases — enough to write integration tests without guessing. Read before implementing or modifying the relevant system.

### [`technical/technical-design.md`](technical/technical-design.md) — Technical Design Document
Implementation architecture for all core systems. Covers: seed system, recipe graph, tech tree, world & chunk system, multiblock machine system, logistics network, power system, drone system, science & research system, world reactivity, codex & meta-progression. Includes data structures and invariants. **Read the relevant section before implementing a system. Update when architecture decisions are made.**

### [`technical/networks.md`](technical/networks.md) — Network System Design
Implementation design for the generic cable network system and both concrete kinds (logistics and power). Covers ECS structure, topology systems (place/remove/split/merge), routing, and how power gating interacts with recipe start. **Read before touching `src/network/`, `src/logistics/`, or `src/power/`.**

### [`technical/research.md`](technical/research.md) — Science & Research System
ECS components, system step-by-step logic, events/messages, and edge cases for research stations, research pool, knowledge visibility, and the player-initiated unlock flow. VS and MVP differences noted inline. **Read before touching `src/research/` or adding research station recipes.**

### [`technical/escape-condition.md`](technical/escape-condition.md) — Escape Condition Design
ECS components, system logic (gateway charge, interact, status UI), events, edge cases, and integration test descriptions for the Initiation escape (alien gateway activation). Includes recipe system extension for catalyst inputs. MVP escape types outlined. **Read before implementing gateway activation, EscapeEvent, or RunState.**

### [`technical/mining.md`](technical/mining.md) — Mining & Deposit System
Ore extraction from surface deposits. Covers ECS components (`OreDeposit`, `MinerMachine`), miner placement and deposit linking, depletion curve, weighted ore sampling, manual mining, and deposit discovery. **Read before touching `src/logistics/miner.rs`, `src/drone/`, or deposit-related code in `src/world/generation.rs`.**

### [`technical/drone.md`](technical/drone.md) — Drone System Design
ECS components, system logic (Local↔Remote mode transition, fog-of-war reveal, sample collection, range scanning, multiple drone switching), events, edge cases, and execution order. VS and MVP scope noted inline. **Read before touching `src/drone/` or anything involving `PlayMode::DronePilot`.**

### [`technical/aegis.md`](technical/aegis.md) — Aegis System Design
ECS components, system logic (boundary check, Local mode constraint, atmospheric exposure, outpost beacon power, body switching), events, edge cases, and execution order. VS and MVP scope noted inline. **Read before implementing aegis fields, body switching, or outpost beacons.**

### [`technical/crafting.md`](technical/crafting.md) — Crafting System Design
ECS components, system logic (recipe execution, job dispatch, catalyst reservation, module effects, plan resolution), events, edge cases, and execution order. Resolves: Recipe Graph Runtime Integration, Catalyst Inputs, and Auto-crafting Job Dispatch todos. VS and MVP scope noted inline. **Read before implementing recipe execution, crafting jobs, or auto-crafting dispatch.**

### [`technical/inventory.md`](technical/inventory.md) — Inventory System Design
ECS components, system logic (hotbar, drone inventory, storage units, Terminal screen runtime data, goal tracker), events, edge cases, and execution order. Covers the no-personal-inventory model, hotbar-as-network-view, drone deposit flow, NetworkFlowLedger (Δ/min), and pin-based goal tracker. VS and MVP scope noted inline. **Read before implementing the hotbar, Terminal screen, drone deposit, or storage capacity.**

### [`technical/machine-ui.md`](technical/machine-ui.md) — Machine UI Technical Design
ECS components, system logic (open/close, identity, progress, power status, module slots, port binding editor, recipe table C/P flag editing), events, edge cases. Also defines the **revised `MachineJobPolicy`** (supersedes `crafting.md §4`) with per-recipe `RecipePolicy` carrying independent C/P flags and machine-level `CraftingJobMode`/`passive` defaults. VS and MVP scope noted inline. **Read before implementing the machine panel, `MachineJobPolicy`, `PortPolicy` editing, or `SlotBlockReason`.**

### [`ui.md`](ui.md) — User Interface
UI layout and mockups for inventory, machine panel, planner, and tech tree screens.

---

## How to keep docs current

- **Design decision made** → update `gdd.md` (the what) + `design-decisions.md` (the why)
- **Architecture decision made** → update `technical/technical-design.md`
- **Open question resolved** → mark resolved in `gdd.md` §18 open questions register
- **New open question** → add to `gdd.md` §18 and the relevant section

Docs are the source of truth. Code is the implementation of docs. When they diverge, update the docs to reflect the current decision, or update the code to match the docs — but never leave them silently out of sync.

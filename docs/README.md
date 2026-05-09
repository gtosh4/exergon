# Exergon — Docs Index

All design and technical documentation lives here. Read this file first to find where information is.

---

## Documents

### [`gdd.md`](gdd.md) — Game Design Document
The canonical design reference. Covers vision, design pillars, all game systems at the design level (seed system, tech tree, recipe graph, factory layer, world/exploration, power, science loop, meta-progression, escape condition, failure, moddability). Open questions are tracked inline and in §18. **Read the relevant GDD section before implementing any system.**

### [`design-decisions.md`](design-decisions.md) — Design Decisions Log
Rationale and context behind key decisions — the *why* behind what's in the GDD. Records alternatives considered and reasons they were rejected. Also captures decisions that were tentative or may be revisited. **Update this file whenever a significant design decision is made.**

### [`technical-design.md`](technical-design.md) — Technical Design Document
Implementation architecture for all core systems. Covers: seed system, recipe graph, tech tree, world & chunk system, multiblock machine system, logistics network, power system, drone system, science & research system, world reactivity, codex & meta-progression. Includes data structures and invariants. **Read the relevant section before implementing a system. Update when architecture decisions are made.**

### [`milestones.md`](milestones.md) — Milestones
Defines two pre-release milestones: **Vertical Slice** (core loop playable end-to-end, systems simplified) and **MVP** (all systems at production depth). Includes explicit post-MVP backlog. **Reference when scoping work. Update as scope changes.**

## [`implementation-plan.md`](implementation-plan.md) - Implementation Plan
The current milestone's implementation plan steps/tasks. **Update this file when implementing features.**

### [`tech-tree-design.md`](tech-tree-design.md) — Tech Tree Content Design
Content design layer for the tech tree: node definitions, pacing targets, unlock structure. Sits between `gdd.md §7` (design intent) and implementation. Status: first draft, pacing unvalidated. **Update when tech tree nodes or unlock order changes.**

### [`networks.md`](networks.md) — Network System Design
Implementation design for the generic cable network system and both concrete kinds (logistics and power). Covers ECS structure, topology systems (place/remove/split/merge), routing, and how power gating interacts with recipe start. **Read before touching `src/network/`, `src/logistics/`, or `src/power/`.**

### [`mining.md`](mining.md) — Mining & Deposit System
Ore extraction from surface deposits. Covers ECS components (`OreDeposit`, `MinerMachine`), miner placement and deposit linking, depletion curve, weighted ore sampling, manual mining, and deposit discovery. **Read before touching `src/logistics/miner.rs`, `src/drone/`, or deposit-related code in `src/world/generation.rs`.**

### [`design-todos.md`](design-todos.md) — Design Todos
Systems that need a `networks.md`-depth spec (ECS components, system logic, events, edge cases) before implementation. Organized by priority: Vertical Slice and MVP. **Check here before starting a new system. Remove entries as specs are written.**

### [`ui.md`](ui.md) — User Interface
UI layout and mockups for inventory, machine panel, planner, and tech tree screens.

---

## How to keep docs current

- **Design decision made** → update `gdd.md` (the what) + `design-decisions.md` (the why)
- **Architecture decision made** → update `technical-design.md`
- **Scope changes** → update `milestones.md`
- **Open question resolved** → mark resolved in `gdd.md` §18 open questions register
- **New open question** → add to `gdd.md` §18 and the relevant section

Docs are the source of truth. Code is the implementation of docs. When they diverge, update the docs to reflect the current decision, or update the code to match the docs — but never leave them silently out of sync.

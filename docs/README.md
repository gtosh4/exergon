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

### [`balance.md`](balance.md) — Balance Methodology
How balancing is done: the `.ron` assets are the source of truth for every number; the scenario runner (`scenario balance … --seeds N`) is the evidence engine; the e2e tests prove reachability; the current measured baseline lives in the generated `balance-state.md`. Covers content-shaping methodology (staggered-complexity ladders, material-spine sizing, interlock) and balance open questions. **Read before tuning numbers — but numbers live in the assets, not here.**

### [`vertical_slice.md`](vertical_slice.md) — Vertical Slice Signal Spec
Defines what the vertical slice must prove: the five core signals (first-hour insight, repeat-run discovery, Remote mode feel, 3D factory readability, Standard-length pacing), required features and interfaces per system, playtest protocol, instrumentation, and success criteria. Also lists deeper designs required before implementation plans are written. **Read before scoping or implementing any vertical-slice-targeted feature.**

### [`milestones.md`](milestones.md) — Milestones
Milestone ladder: Vertical Slice → Alpha → Demo (MVP) → Release → Post-release. Each milestone states its purpose, gate conditions, and what it explicitly does not require. **Check before asking whether a feature is in scope.**

### [`contributing-content.md`](contributing-content.md) — Contributing Content
How anyone — no Rust, no file editing — adds tech nodes, items, and recipes by describing them to Claude, which authors the RON and proves it works with a `smoke_test`. **Read this to contribute content, or to understand the author → validate → smoke loop.**

### [`ui.md`](ui.md) — UI Design
Palette, screens and modes (In-World HUD, Terminal, Index, Planner, Tech Tree, Machine panel, Landing), hotbar/research-pool/alerts widgets, mockup image references, and links to `ui_mock/` prototypes. **Read before implementing or restyling any UI surface.**

---

## Technical Specs (`technical/`)

Deep implementation specs: ECS components, systems, events, edge cases — enough to write integration tests without guessing. Read before implementing or modifying the relevant system. See [technical/README.md](./technical/README.md) for a summary of the included documents.

---

## Market Research (`market/`)

Reference notes on competing/adjacent titles (Factorio, Satisfactory, DSP, GTNH, Outworld Station, Outer Wilds, Duskers, etc.), distilled market position, product lessons, and demo guidance. See [market/README.md](./market/README.md).

---

## How to keep docs current

- **Design decision made** → update `gdd.md` (the what) + `design-decisions.md` (the why)
- **Architecture decision made** → update `technical/*.md`
- **Open question resolved** → mark resolved in `gdd.md` §18 open questions register
- **New open question** → add to `gdd.md` §18 and the relevant section

Docs are the source of truth. Code is the implementation of docs. When they diverge, update the docs to reflect the current decision, or update the code to match the docs — but never leave them silently out of sync.

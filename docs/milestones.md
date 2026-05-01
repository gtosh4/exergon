# Milestones

Two pre-release milestones. Vertical Slice proves the core loop is fun. MVP is the first complete game.

---

## Milestone 1 — Vertical Slice

**Goal:** One complete run playable end-to-end. Systems are simplified or stubbed where needed. The point is to validate that the seed → discover → build → escape loop is fun, not to have production-ready systems.

### In scope
See [./implementation-plan.md](./implementation-plan.md)

### Out of scope for vertical slice
Everything not listed above, including: underground layer, digger drone, reactivity, meta-progression, codex, modules, auto-crafting, partial reveal, multiple research types, procedural recipe generation.

---

## Milestone 2 — MVP

**Goal:** All core systems implemented at production depth. Full procedural generation. Complete difficulty ladder. First complete, shippable game.

### Systems completed or upgraded from vertical slice

**World**
- All vertical layers: underground, surface, sky/atmosphere (orbital post-MVP)
- Full procedural biome placement (all biome types in base content pack)
- Seeded resource deposit placement with layer + biome affinity
- Seeded persistent sites with interaction system
- Core zone guarantee (critical resources within bounded radius)

**Seed system**
- Full validity validation (recipe graph reachable, tech tree nodes reachable)
- Constrained generation ordering (backwards-from-terminal)
- Seed versioning

**Recipe graph**
- Full procedural generation from seed
- All tier counts (3–6 by difficulty)
- Hybrid base + alien materials
- Bounded parameter variance
- Byproduct routing

**Tech tree**
- Full procedural generation
- All 5 unlock vectors active
- Alternative prerequisites per node
- Partial reveal earned through gameplay
- Node pool seeding from content pack

**Machines**
- Full content pack of machine types
- Module system (module slots per tier, functional tradeoffs)
- Tier upgrade (additive in-place)
- All 8 orientations + mirror

**Logistics**
- Auto-crafting job dispatch system
- Machine capability auto-registration (no manual patterns)
- Job priority + filter configuration
- Sub-network interfaces (pass-through)

**Power**
- Amps & Voltage tier implementation
- Separate power cable tiers
- Planet modifier efficiency multipliers
- Multiple generator types (seeded availability)
- Full tier applicability / demand growth pressure

**Drones**
- Land drone + digger drone
- Amphibious drone
- Tier-gated construction from factory components
- Multiple deployed drones, sequential switching

**Science & research**
- All research types (Material Science, Field Research, Engineering, Discovery)
- All analysis station types (Geological, Biological, Atmospheric)
- Full experiment recipe tiers
- Partial reveal via milestone/discovery triggers
- Direct known-to-exist → fully-revealed for experienced players

**World reactivity**
- Per-region tracking with spread
- Continuous efficiency modifiers
- Discrete threshold events
- Recovery faster than buildup
- Reactivity seeded rate per region

**Habitats**
- Expand habitat
- Outposts

**Escape conditions**
- All escape types by difficulty
  - Easy = find artifact (gateway)
  - Mid = build an intra-system ship ("mass effect relay")
  - Hard = build an inter-system ship

**Save system**
- Run save (full resume, run history, completed runs persist)
- Meta save (codex, unlocks, blueprints)
- Run status tracking

**Codex**
- Created on first encounter
- Biome resource pools, node tier ranges, machine functions, modifier effects

**Meta-progression**
- Difficulty tier unlocks (run completion)
- Biome type unlocks
- Blueprint slots (basic)
- Starting boons pool (basic)
- Some milestone-triggered unlocks

**Difficulty ladder**
- All 4 tiers (Initiation, Standard, Advanced, Pinnacle)
- All difficulty axes tuned

**UI**
- Main menu
  - Settings
  - New Run
    - Run parameters (seed, difficulty, challenges)
  - Load Save
- In-game save/load
  - Quick save & quick load
- In-game settings
- In-game recipe graph viewer with ratio calculator
  - see [expert](./planner_expert_mock_v0.1.png) + [sanky](./planner_sanky_mock_v0.1.png)
  - Bottleneck visualization
- Better inventory screen
  - see [mock](./inventory_ui_mock_v0.1.png)
  - Drag-drop with hotbar
  - Codex browser
- Hotbar cycling
- Tech tree with shadow view
- Reactivity region display
- World map
- Minimap

---

## Post-MVP Backlog

Features explicitly deferred. Not prioritised within this list.

| Feature | Notes |
|---|---|
| Flying drone | Sky/atmosphere layer access |
| Space drone | Orbital layer, inter-system launch |
| Permadeath modes | Multiple variants; design TBD |
| Blueprints (full) | Beyond basic slot unlock |
| Photo mode | Screenshot/sharing tooling |
| Ghost overlay for multiblock | Orientation/chirality problem to solve |
| Reactivity opportunities | Two-sided reactivity (GDD Q#4) — considered core, deferred |
| Building blind mode | Optional challenge mechanic |
| Power transition drama | Dramatic in-world events on power switch |
| Full narrative content | Prior civilisation story, run completion screens |
| Mod tooling | Content editor, run validator, balance checker |
| Full mod support | Load order, conflict resolution |
| Interface throughput constraints | Beyond pass-through |
| Multiple viable recipe paths | Alternative routes to escape artifact |
| Cause breakdown for reactivity | Per-source contribution display |
| Higher-level drone commands | "Mine this deposit" automation |
| Post-MVP difficulty modifiers | Extreme scenarios, custom run builders |
| Steam integration | Achievements, cloud saves |
| Discord integration | Rich presence |
| Multiplayer? | via steam? |
| Demo build | First run maybe two? |
| Mod workshop | Steam workshop? Or more integrated like Factorio |

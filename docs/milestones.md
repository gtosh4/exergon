# Milestones

Two pre-release milestones. Vertical Slice proves the core loop is fun. MVP is the first complete game.

---

## Milestone 1 — Vertical Slice

**Goal:** One complete run playable end-to-end. Systems are simplified or stubbed where needed. The point is to validate that the seed → discover → build → escape loop is fun, not to have production-ready systems.

### In scope

**World**
- Surface layer only (no underground, no sky/orbital)
- Infinite horizontal world, chunk-based generation
- 2–3 biome types, hand-tuned (not fully procedural)
- Basic resource deposits on surface
- 1–2 persistent sites (hand-placed, not seeded)

**Seed system**
- Text → hash → master seed
- Per-domain sub-seeds
- Deterministic world generation from seed
- No full validity validation yet (hand-tuned to be solvable)

**Recipe graph**
- Hand-authored graph for the slice (not procedurally generated)
- 2 tiers, ~6–8 recipes
- Mix of base and alien materials
- Single escape condition: alien gateway activation

**Tech tree**
- Hand-authored tree matching the recipe graph
- 2 tiers, ~8–10 nodes
- Research spend + prerequisite chain unlock vectors only
- Known-to-exist → fully-revealed (no partial reveal step)

**Machines**
- 3–4 machine types as multiblock structures
- Passive scan validation, 8 orientations
- Tier 1 only (no tier upgrades)
- No modules

**Logistics**
- Cable adjacency network
- Unified storage
- Manual crafting (no auto-crafting job system yet)
- Basic channel limits

**Power**
- Single generator type
- Flow-based, recipe-based machine demand
- Brownout throttling
- No planet modifier efficiency yet

**Drones**
- Land drone only
- Full camera transfer, persistent
- Point-based interaction, sample collection
- Basic fog of war reveal

**Science & research**
- Single research type
- Single analysis station type
- Crafting-style experiments
- Research spend to unlock nodes

**World reactivity**
- Not included in vertical slice

**Escape condition**
- Alien gateway: produce required components at sufficient throughput

**Save system**
- Run save (basic, no resume from mid-session required)
- No meta save

**UI**
- In-game recipe graph viewer
- Basic tech tree view
- Power/network status indicators

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
- Planet modifier efficiency multipliers
- Multiple generator types (seeded availability)
- Full tier applicability / demand growth pressure
- Separate power cable tiers

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

**Escape conditions**
- All escape types by difficulty (gateway, intra-system ship, inter-system ship)
- Sustained throughput requirement

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
- In-game recipe graph viewer with ratio calculator
- Critical path analyzer
- Bottleneck visualization
- Codex browser
- Tech tree with shadow view
- Reactivity region display

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

# Implementation Plan — Vertical Slice

Phased build order. Dependencies flow top→bottom. Check off items as done.

---

## Phase 1 — Foundation (World + Run Start)

- [x] Main menu: seed text entry → hash → init `RunSeed` + `DomainSeeds` → `GameState::Loading → Playing`
- [x] `bevy_voxel_world` integration: chunk generation, noise-based surface terrain
- [x] 2–3 biomes (height + noise thresholds) — surface (mat 1) + stone (mat 0) layers
- [x] Player controller: WASD + mouse-look (fly camera, cursor locked)
- [ ] Resource deposit placement (surface ore nodes, seeded per chunk)

**Deliverable:** Walk around a generated alien world.

---

## Phase 2 — Content Data *(parallel with Phase 1)*

- [ ] Content loader: RON files → typed resources at startup
- [ ] `RecipeGraph` resource: hand-authored 2-tier graph (~6–8 recipes), materials, escape artifact
- [ ] `TechTree` resource: hand-authored 2-tier tree (~8–10 nodes)

**Deliverable:** Game data loaded and queryable.

---

## Phase 3 — Machines

- [ ] Voxel block placement/removal (hotbar + raycast)
- [ ] Multiblock scanner: adjacency pattern matching, 8 orientations
- [ ] 3–4 machine types (Smelter, Processor, Analyzer, Gateway)
- [ ] Machine validity state (scanning → valid/invalid → running)

**Deliverable:** Place and validate multiblock machines.

---

## Phase 4 — Logistics + Power

- [ ] Cable adjacency graph traversal → `Network` resource
- [ ] Unified storage: item stacks pooled per network
- [ ] Machine I/O: pull inputs from storage, push outputs to storage
- [ ] Generator block + power output resource
- [ ] Power flow: sum capacity vs. demand → brownout throttle

**Deliverable:** Self-running factory loops.

---

## Phase 5 — Research

- [ ] Analysis station interaction: spend items → research points
- [ ] Tech tree unlock: `ResearchSpend` vector triggers node reveal
- [ ] Unlocked nodes gate machine/recipe availability

**Deliverable:** Science unlocks progression.

---

## Phase 6 — Drones

- [ ] Land drone entity + physics movement
- [ ] Camera handoff (`DronePilot` play mode)
- [ ] Fog-of-war reveal (bitmask per chunk)
- [ ] Sample collection interaction → add items to inventory

**Deliverable:** Drone exploration + resource sampling.

---

## Phase 7 — UI

- [ ] Recipe graph viewer (egui panel, nodes + edges)
- [ ] Tech tree panel (node grid, locked/unlocked state)
- [ ] Power/network status HUD (watts, throughput)
- [ ] Gateway escape condition display + win screen

**Deliverable:** Full readable game state.

---

## Phase 8 — Save

- [ ] SQLite schema: world state, inventory, tech tree, fog of war
- [ ] Save on pause/quit, load on continue

**Deliverable:** Persistent runs.

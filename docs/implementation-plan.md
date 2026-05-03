# Implementation Plan — Vertical Slice (Heightmap World)

## Context

World is **heightmap mesh chunks** — terrain is a continuous procedural mesh, world objects are discrete prefab entities placed at free positions on terrain surfaces. This plan tracks the complete vertical slice.

**Core loop**: seed → walk world → pilot drone to collect ore samples → build factory on platforms → run recipes → accumulate research → unlock machines → produce escape artifact.

## World Model (non-voxel)

| Concept | Implementation |
|---------|----------------|
| Terrain | Seeded HybridMulti noise → heightmap mesh, spawned as 64×64 chunks around camera |
| Platforms | Flat prefab entities placed on terrain; machines sit on platforms |
| Machines | Single-entity prefabs placed at free Vec3 positions (on platforms or terrain) |
| IO ports | Vec3 world positions = machine center + orientation-transformed offset |
| Cables | Two-click: player clicks IO port A → IO port B → segment entity connecting them |
| Networks | BFS over cable endpoints; power + logistics share generic network system |
| Ore deposits | Persistent surface marker entities at positions derived from DepositRegistry |
| Underground | Out of scope for vertical slice |

---

## Phase 1 — Foundation ✅

- [x] Main menu: seed text entry → `RunSeed` + `DomainSeeds` → `Loading → Playing`
- [x] Heightmap terrain: seeded HybridMulti noise, 64×64 chunk mesh gen, trimesh colliders
- [x] Fly camera: WASD + mouse-look, Space/Ctrl for Y-axis, 50 u/s
- [x] Cursor lock/unlock, pause (Escape), inventory toggle (Tab)
- [x] Content loader: `load_ron_dir()` → typed resources at startup

---

## Phase 2 — Content Data ✅

- [x] `DepositRegistry`: deterministic ore placement (ellipsoidal deposits, per-cell seeding, biome layers, `ore_at(seed, wx, wy, wz)`)
- [x] `RecipeGraph`: 2-tier hand-authored graph (materials, producer/consumer index, `is_terminal` flag)
- [x] `TechTree`: 2-tier hand-authored tree (prerequisites, unlock vectors, `NodeEffect`)
- [x] Assets: items, machines, recipes, tech_nodes, biomes, layers RON files

---

## Phase 2b — Material System Redesign ☐

Migrate from flat material→recipe model to the material/form/item hierarchy described in tech-design §2 and GDD §8. This is a content-layer change; machine/network code is unaffected.

**2b-1. Core types** (`src/recipe_graph/mod.rs`)
- [ ] Add `FormGroupId = String`, `FormId = String`
- [ ] Add `FormGroup { id: FormGroupId, forms: Vec<FormId> }` — content-defined
- [ ] Update `MaterialDef`: add `form_groups: Vec<FormGroupId>`; remove `is_terminal` (terminal is a `RecipeGraph`-level flag, not per-material)
- [ ] Add `ItemId = String` (replaces bare `MaterialId` as recipe node identifier)
- [ ] Add `ItemKind` enum: `Derived { material: MaterialId, form: FormId }`, `Composite { template: Option<TemplateId> }`, `Unique`
- [ ] Add `ItemDef { id: ItemId, name: String, kind: ItemKind }`
- [ ] Add `RecipeTemplate { id, input_forms: Vec<(FormId, f32)>, output_form: FormId, group: FormGroupId, machine_type: MachineTypeId, base_time: f32, base_energy: f32 }`
- [ ] Rename `RecipeDef` → `ConcreteRecipe`; change `ItemStack.material: MaterialId` → `item: ItemId`
- [ ] Update `RecipeGraph`: add `form_groups`, `templates`, `items` maps; `producers`/`consumers` key on `ItemId` not `MaterialId`; `terminal` becomes `ItemId`
- [ ] Tests: `RecipeGraph::from_vecs` still satisfies validity invariants with new types

**2b-2. Template expansion** (`src/recipe_graph/mod.rs`)
- [ ] `fn expand_templates(materials, form_groups, templates) -> Vec<ConcreteRecipe>` — for each template, find all materials whose groups include both input and output forms, instantiate one `ConcreteRecipe` per material
- [ ] `fn derive_items(materials, form_groups) -> Vec<ItemDef>` — generate all `DerivedItem` entries
- [ ] Both called inside `RecipeGraph::from_vecs`
- [ ] Tests: `expand_templates` produces correct concrete recipes; no recipe for material missing a required form

**2b-3. Asset files**
- [ ] Add `assets/form_groups/` dir with RON files: `metal.ron`, `combustible.ron`, `exotic.ron` (etc.) listing their forms
- [ ] Update `assets/materials/*.ron`: add `form_groups` field; remove `is_terminal` where present
- [ ] Rename `assets/recipes/*.ron` to recipe templates where applicable; add `input_form`/`output_form`/`group` fields
- [ ] Keep `assets/items/` for composite and unique items only; remove derived-item RON files (copper_ore, copper_wire, etc.)
- [ ] Update `DepositDef` asset format: replace single `ore_material` with `ores: Vec<(MaterialId, f32)>` weighted list

**2b-4. Deposit weighted ores** (`src/content/mod.rs`, `src/world/generation.rs`)
- [ ] Update `DepositDef`: `ores: Vec<(MaterialId, f32)>` (weights, not required to sum to 1 — normalised at load)
- [ ] Update `DepositRegistry::ore_at` return type: `Option<Vec<(MaterialId, f32)>>` — caller samples weighted distribution
- [ ] Update deposit spawn (Phase 6 placeholder) to use weighted pick
- [ ] Tests: `ore_at` returns weighted list; weights normalise correctly

**2b-5. Unify item registry** (`src/inventory/mod.rs`, `src/content/mod.rs`)
- [ ] `ItemRegistry` populated from `RecipeGraph::items` (derived + composite + unique) instead of separate `assets/items/` load
- [ ] Remove `ItemDef` from `inventory/mod.rs`; use `recipe_graph::ItemDef`
- [ ] Update `content/mod.rs` loader: load form_groups + materials + templates + unique/composite items → build `RecipeGraph` → register all items into `ItemRegistry`
- [ ] Tests: registry contains derived items after graph construction

---

## Phase 3 — Machines + Networks ⚠ (in-progress migration)

**Done:**
- [x] `Inventory`: HashMap-based, hotbar 9-slot, 1–9/scroll to select, Tab to open grid
- [x] `Machine` component: type, tier, `Orientation` (8-way), footprint, IO offsets
- [x] Ghost preview: translucent cube at look target
- [x] Generic cable network: `NetworkPlugin<N>` with `CableSegment + HasEndpoints` (segments, not nodes)
- [x] Two-click cable placement: `PendingCablePort` resource, `CableConnectionEvent{from, to}`
- [x] Power network: `GeneratorUnit` capacity sum, per-machine demand, brownout `speed_factor`
- [x] Logistics network: unified `StorageUnit` pool, recipe input-pull → progress → output-push
- [x] Research output: recipes producing `"research_points"` → `ResearchPool`
- [x] `--test` flag: fills hotbar with machines + cables for quick testing

**In-progress (IVec3 → Vec3 migration):**
- [ ] `WorldObjectEvent.pos` → `Vec3`; `CableConnectionEvent.from/to` → `Vec3`; `HasEndpoints` → `[Vec3; 2]`
- [ ] `src/power/mod.rs`: migrate `NetworkKind` impl to `CableSegment`/`new_cable_segment`; fix type mismatches
- [ ] `src/logistics/mod.rs`: same migration
- [ ] `src/machine/mod.rs`: `origin_pos: IVec3` → `Vec3`; port sets `HashSet<IVec3>` → `HashSet<Vec3>`
- [ ] Remove `snap_to_grid` from `src/world/interaction.rs`; ghost preview tracks raw surface hit

**Needed (free placement + collision):**
- [ ] `placement_collision_check`: before placing, `SpatialQuery::intersections_with_shape(AABB)` — reject if overlap
- [ ] IO port markers: spawn small sphere mesh at each port position when machine is placed (cable two-click target)
- [ ] Platforms: new item `"platform"` (flat 2×0.25×2 box) — placed at surface hit; `RigidBody::Static + Collider::cuboid`
  - Asset: `assets/items/platform.ron`
  - System: `place_platform_system` in `src/machine/mod.rs`
- [ ] `--test` flag: also give starting ore (`20 iron_ore`, `20 copper_ore`) + a few platforms

---

## Phase 4 — Research ✅

- [x] `ResearchPool` resource (accumulated `f32`)
- [x] `TechTreeProgress`: `unlocked_nodes`, `unlocked_recipes`, `unlocked_machines`
- [x] `check_research_unlocks`: loops until no more unlocks possible per frame
- [x] `ResearchSpend` unlock vector: deduct points, apply `NodeEffect`

---

## Phase 5 — Drone Movement ✅

- [x] Land drone: capsule collider, Avian3D + TNUA character controller
- [x] F-key toggle: `Exploring ↔ DronePilot` substates
- [x] WASD + mouse-look drone piloting (horizontal plane, camera follows at eye height)

---

## Phase 6 — Ore Deposits + Manual Mining ☐

**6a. Surface deposit markers** (`src/world/generation.rs`)
- [ ] Extract `pub(crate) fn terrain_height(seed: u64, wx: f32, wz: f32) -> f32` from mesh gen
- [ ] System `spawn_deposit_markers`: after `add_chunk_colliders`, sample grid (every 4 units) in chunk → `DepositRegistry::ore_at(seed, wx, surface_y, wz)` → spawn `OreDeposit { ores: Vec<(MaterialId, f32)>, total_extracted: f32, depletion_seed: u64 }` sphere mesh at surface height
- [ ] System `despawn_deposit_markers`: remove `OreDeposit` entities when parent chunk despawns
- [ ] Deposits must not spawn inside habitat boundaries; remove existing deposit entity when habitat expands to cover it
- [ ] Tests: deposit spawns when ore present, no spawn on empty cell, habitat boundary exclusion

**6b. Manual mining** (`src/drone/mod.rs`)
- [ ] `drone_mine_system`: `DronePilot` mode, right-click → `SpatialQuery::cast_ray` (reach 4.0) → hit `OreDeposit` → sample one ore from weighted distribution → `Inventory::add(sampled_ore, 1)` → increment `total_extracted`
- [ ] Deposit entity persists (not despawned); yield degrades per depletion curve (seeded per deposit, asymptotic, never zero)
- [ ] Tests: right-click adds ore sampled from weights; deposit persists; repeated mining degrades yield

**6c. Automatic miner** (`src/machine/mod.rs`)
- [ ] `MinerMachine` component: placed on a deposit (one per deposit); each tick samples weighted ore distribution, applies current yield factor, outputs to logistics network
- [ ] Yield factor computed from `total_extracted` + `depletion_seed` — monotonically decreasing, asymptotic to a floor > 0
- [ ] Tests: miner outputs ore at expected rate; yield factor decreases with extraction; floor not breached

---

## Phase 7 — Factory UI ☐

**7a. Machine status panel** (`src/ui/mod.rs`)
- [ ] `MachineStatusPanel` resource: `Option<Entity>`
- [ ] Right-click machine in `Exploring` mode → `SpatialQuery::cast_ray` → hit machine → set `MachineStatusPanel`
- [ ] `machine_status_ui`: egui side panel showing: machine type, `MachineState`, recipe in progress, inputs needed vs available, progress %, `speed_factor`

**7b. Tech tree + research panel** (`src/ui/mod.rs`)
- [ ] T key toggles `TechTreePanelOpen`
- [ ] `tech_tree_ui`: egui window with `ResearchPool` points, node grid colored by unlock status, hover shows cost + prereqs + effects

**7c. Power HUD** (`src/ui/mod.rs`)
- [ ] Persistent line: `⚡ {produced}W / {demanded}W ({pct}%)` aggregated across all power networks

---

## Phase 8 — Polish (optional) ☐

- [x] Cable tube rendering: `Cylinder` + `Sphere` joint meshes (yellow power, green logistics); `Cylinder::new(r, 1.0)` for correct 1-unit step height
- [x] Collision-aware cable routing: A* with turn penalty (`route_avoiding`) avoids placed machine positions; falls back to Manhattan on no-path
- [x] Cable endpoint snapping: connector tube from machine center → port voxel via `from_rotation_arc` + Y-scale
- [ ] Ghost preview shows actual machine mesh (colored, 50% alpha) instead of grey cube
- [ ] F10 cycles network debug overlay: none → power → logistics

---

## Post-VS / MVP — Win Condition ☐

*(Out of scope for vertical slice)*

- [ ] Add `GameState::Won` to `src/main.rs`
- [ ] `escape_condition_system`: scan `StorageUnit` maps → terminal material present → `NextState::set(Won)`
- [ ] Win screen: egui panel with seed string + restart button
- [ ] Tests: terminal material triggers Won; non-terminal no-op

---

## Verification

```
cargo test
```
Baseline: **106 tests, 0 failures**.

New tests required per phase:
- Phase 2b: ≥5 new tests (template expansion, item derivation, deposit weights, registry population, graph validity)
- Phase 3 (migration): existing network/power/logistics tests must still pass after Vec3 migration
- Phase 6: ≥6 new tests (deposit spawn, habitat exclusion, manual mining sampling, deposit persistence, yield degradation, miner output)
- Phase 7: no unit tests (egui)
- Post-VS: ≥2 new tests (escape condition)

### Manual vertical slice run
1. `cargo run` → enter seed → terrain visible
2. Place platforms on terrain
3. Place machines on platforms: generator + cables + smelter + storage
4. F → drone mode → find ore deposit (surface marker) → right-click (mining tool) → ore sampled into inventory → deposit persists
5. F → back → Tab → drag ore to storage
6. Watch factory run: smelter processes ore, research points accumulate
7. T → tech tree panel → nodes unlock as points arrive
8. Build chain to terminal material

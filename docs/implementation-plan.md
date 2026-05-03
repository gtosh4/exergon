# Implementation Plan — Vertical Slice (Heightmap World)
Always check off items as they are completed.

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

## Phase 2b — Material System Redesign ✅

Migrate from flat material→recipe model to the material/form/item hierarchy described in tech-design §2 and GDD §8. This is a content-layer change; machine/network code is unaffected.

**2b-1. Core types** (`src/recipe_graph/mod.rs`)
- [x] Add `FormGroupId = String`, `FormId = String`
- [x] Add `FormGroup { id: FormGroupId, forms: Vec<FormId> }` — content-defined
- [x] Update `MaterialDef`: add `form_groups: Vec<FormGroupId>`; remove `is_terminal` (terminal is a `RecipeGraph`-level flag, not per-material)
- [x] Add `ItemId = String` (replaces bare `MaterialId` as recipe node identifier)
- [x] Add `ItemKind` enum: `Derived { material: MaterialId, form: FormId }`, `Composite { template: Option<TemplateId> }`, `Unique`
- [x] Add `ItemDef { id: ItemId, name: String, kind: ItemKind }`
- [x] Add `RecipeTemplate { id, input_forms: Vec<(FormId, f32)>, output_form: FormId, group: FormGroupId, machine_type: MachineTypeId, base_time: f32, base_energy: f32 }`
- [x] Rename `RecipeDef` → `ConcreteRecipe`; change `ItemStack.material: MaterialId` → `item: ItemId`
- [x] Update `RecipeGraph`: add `form_groups`, `templates`, `items` maps; `producers`/`consumers` key on `ItemId` not `MaterialId`; `terminal` becomes `ItemId`
- [x] Tests: `RecipeGraph::from_vecs` still satisfies validity invariants with new types

**2b-2. Template expansion** (`src/recipe_graph/mod.rs`)
- [x] `fn expand_templates(materials, form_groups, templates) -> Vec<ConcreteRecipe>` — for each template, find all materials whose groups include both input and output forms, instantiate one `ConcreteRecipe` per material
- [x] `fn derive_items(materials, form_groups) -> Vec<ItemDef>` — generate all `DerivedItem` entries
- [x] Both called inside `RecipeGraph::from_vecs`
- [x] Tests: `expand_templates` produces correct concrete recipes; no recipe for material missing a required form

**2b-3. Asset files**
- [x] Add `assets/form_groups/` dir with RON files: `metal.ron`, `exotic.ron` listing their forms
- [x] Update `assets/materials/*.ron`: add `form_groups` field; remove `is_terminal` where present
- [x] Add `assets/recipe_templates/` with `smelt_metal.ron`, `draw_metal.ron`; concrete recipes updated to use `item:` field
- [x] Keep `assets/items/` for composite and unique items only; remove derived-item RON files (copper_ore, copper_wire, etc.)
- [x] Update `DepositDef` asset format: replace single `ore_material` with `ores: Vec<(MaterialId, f32)>` weighted list

**2b-4. Deposit weighted ores** (`src/content/mod.rs`)
- [x] Add `DepositDef`: `ores: Vec<(MaterialId, f32)>` (weights normalised at load)
- [x] Add `DepositRegistry::ore_at` return type: `Option<Vec<(MaterialId, f32)>>` — caller samples weighted distribution
- [x] Tests: `ore_at` returns weighted list; weights normalise correctly

**2b-5. Unify item registry** (`src/inventory/mod.rs`, `src/recipe_graph/mod.rs`)
- [x] `ItemRegistry` populated from `RecipeGraph::items` (derived + composite + unique) instead of separate `assets/items/` load
- [x] Remove `ItemDef` from `inventory/mod.rs`; use `recipe_graph::ItemDef`
- [x] `load_recipe_graph` loads form_groups + materials + templates + unique/composite items → builds `RecipeGraph` → registers all items into `ItemRegistry`
- [x] Tests: registry contains derived items after graph construction

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

**In-progress (IVec3 → Vec3 migration):** ✅
- [x] `WorldObjectEvent.pos` → `Vec3`; `CableConnectionEvent.from/to` → `Vec3`; `HasEndpoints` → `[Vec3; 2]`
- [x] `src/power/mod.rs`: migrate `NetworkKind` impl to `CableSegment`/`new_cable_segment`; fix type mismatches
- [x] `src/logistics/mod.rs`: same migration
- [x] `src/machine/mod.rs`: `origin_pos: IVec3` → `Vec3`; port sets `HashSet<IVec3>` → `Vec<Vec3>`
- [x] Remove `snap_to_grid` from `src/world/interaction.rs`; ghost preview tracks raw surface hit

**Needed (free placement + collision):**
- [x] `placement_collision_check`: before placing, `SpatialQuery::intersections_with_shape(AABB)` — reject if overlap
- [x] IO port markers: spawn small sphere mesh at each port position when machine is placed (cable two-click target)
- [x] Platforms: new item `"platform"` (flat 2×0.25×2 box) — placed at surface hit; `RigidBody::Static + Collider::cuboid`
  - Asset: `assets/items/platform.ron`
  - System: `place_platform_system` in `src/machine/mod.rs`
- [x] `--test` flag: also give starting ore (`20 iron_ore`, `20 copper_ore`) + a few platforms
- [x] shift-click to remove: machines, platforms, & cables
  - [x] hold shift: highlight what would be removed if clicked

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

## Phase 6 — Ore Deposits + Manual Mining ✅

**6a. Surface deposit markers** (`src/world/generation.rs`)
- [x] `TerrainSampler::height_at` already pub(crate); `chunk_deposit(seed, chunk_pos, registry)` helper added
- [x] System `spawn_deposit_markers`: after `add_chunk_colliders`, checks deposit cell per chunk → spawn `OreDeposit { chunk_pos, ores, total_extracted, depletion_seed }` sphere mesh at surface height
- [x] System `despawn_deposit_markers`: removes `OreDeposit` entities whose chunk_pos left `SpawnedChunks`
- [x] Deposits must not spawn inside habitat boundaries (out of scope — no habitats yet)
- [x] Tests: `chunk_deposit_empty_registry_returns_none`, `chunk_deposit_is_deterministic`, `chunk_deposit_different_chunks_can_differ`

**6b. Manual mining** (`src/drone/mod.rs`)
- [x] `drone_mine_system`: `DronePilot` mode, right-click → `SpatialQuery::cast_ray` (reach 4.0) → hit `OreDeposit` → sample one ore from weighted distribution → `Inventory::add(sampled_ore, 1)` → increment `total_extracted`
- [x] Deposit entity persists (not despawned); yield degrades per depletion curve (seeded per deposit, asymptotic, never zero)
- [x] Tests: right-click adds ore sampled from weights; deposit persists; repeated mining degrades yield

**6c. Automatic miner** (`src/logistics/mod.rs`)
- [x] `MinerMachine` component: placed on a deposit (one per deposit); each tick samples weighted ore distribution, applies current yield factor, outputs to logistics network
- [x] Yield factor computed from `total_extracted` + `depletion_seed` — monotonically decreasing, asymptotic to a floor > 0
- [x] Tests: miner outputs ore at expected rate; yield factor decreases with extraction; floor not breached

---

## Phase 7 — Factory UI ✅

**7a. Machine status panel** (`src/ui/mod.rs`)
- [x] `MachineStatusPanel` resource: `Option<Entity>`
- [x] Right-click machine in `Exploring` mode → `SpatialQuery::cast_ray` → hit machine → set `MachineStatusPanel`
- [x] `machine_status_ui`: egui side panel showing: machine type, `MachineState`, recipe in progress, progress %, `speed_factor`

**7b. Tech tree + research panel** (`src/ui/mod.rs`)
- [x] T key toggles `TechTreePanelOpen`
- [x] `tech_tree_ui`: egui window with `ResearchPool` points, node grid colored by unlock status, hover shows cost + prereqs + effects

**7c. Power HUD** (`src/ui/mod.rs`)
- [x] Persistent line: `⚡ {produced}W / {demanded}W ({pct}%)` aggregated across all power networks

---

## Phase 8 — Polish (optional) ☐

- [x] Cable tube rendering: `Cylinder` + `Sphere` joint meshes (yellow power, green logistics); `Cylinder::new(r, 1.0)` for correct 1-unit step height
- [x] Collision-aware cable routing: A* with turn penalty (`route_avoiding`) avoids placed machine positions; falls back to Manhattan on no-path
- [x] Cable endpoint snapping: connector tube from machine center → port voxel via `from_rotation_arc` + Y-scale
- [ ] Ghost preview shows actual machine mesh (colored, 50% alpha) instead of grey cube
- [ ] F10 cycles network debug overlay: none → power → logistics

---

## Bugs
 - [ ] Power (and logistics?) cables don't attach properly to machine ports
 - [ ] Cables can clip through terrain

---

## Post-VS / MVP — Win Condition ☐

*(Out of scope for vertical slice)*

- [ ] Add `GameState::Won` to `src/main.rs`
- [ ] `escape_condition_system`: scan `StorageUnit` maps → terminal material present → `NextState::set(Won)`
- [ ] Win screen: egui panel with seed string + restart button
- [ ] Tests: terminal material triggers Won; non-terminal no-op

---

### Manual vertical slice run
1. `cargo run` → enter seed → terrain visible
2. Place platforms on terrain
3. Place machines on platforms: generator + cables + smelter + storage
4. F → drone mode → find ore deposit (surface marker) → right-click (mining tool) → ore sampled into inventory → deposit persists
5. F → back → Tab → drag ore to storage
6. Watch factory run: smelter processes ore, research points accumulate
7. T → tech tree panel → nodes unlock as points arrive
8. Build chain to terminal material

# World & Chunk Generation

Procedural surface world generation plus scoped underground resource-domain queries. Covers: coordinate system, chunk streaming (spawn/despawn), heightmap generation, underground vein lookup, surface deposit placement, discovery site placement, world extent, seed-to-geography mapping, and chunk boundary conditions. Sufficient detail to write integration tests without guessing.

**Read before:** touching `src/world/generation.rs`, `src/world/ruins.rs`, `src/content/mod.rs` (resource domains/biomes/veins/deposits), or any system that queries terrain height or places world objects.

---

## Table of Contents

1. [Coordinate System & Scale](#1-coordinate-system--scale)
2. [ECS Components & Resources](#2-ecs-components--resources)
3. [Surface Terrain — Chunk Streaming](#3-surface-terrain--chunk-streaming)
4. [Heightmap Generation](#4-heightmap-generation)
5. [Underground Resource Domain](#5-underground-resource-domain)
6. [Surface Deposit Placement](#6-surface-deposit-placement)
7. [Discovery Site Placement](#7-discovery-site-placement)
8. [World Extent & Bounds](#8-world-extent--bounds)
9. [Seed → Geography Mapping](#9-seed--geography-mapping)
10. [Generation Sequence](#10-generation-sequence)
11. [Chunk Boundary Conditions](#11-chunk-boundary-conditions)
12. [Events](#12-events)
13. [Edge Cases](#13-edge-cases)
14. [Integration Test Invariants](#14-integration-test-invariants)
15. [VS / MVP Scope](#15-vs--mvp-scope)

---

## 1. Coordinate System & Scale

**1 unit = 1 meter.** World-space axes: X and Z are horizontal (the surface plane); Y is vertical (up). World origin (0, 0, 0) is the spawn point. All positions in Transform components are in world-space meters.

Surface chunks are indexed by `IVec2(cx, cz)` where chunk `(cx, cz)` covers world-space rectangle `[cx*64, (cx+1)*64) × [cz*64, (cz+1)*64)`.

Underground cells use a separate coordinate system — see §5.

> **VS scope:** This coordinate system covers surface terrain. Orbital and beyond (Advanced/Pinnacle runs) will likely require additional coordinate spaces; exact scales are deferred to those milestones.

---

## 2. ECS Components & Resources

### Resources

| Resource | Fields | Notes |
|---|---|---|
| `SpawnedChunks` | `0: HashSet<IVec2>` | Runtime; currently-spawned chunk coords |
| `GatewayRuinsPosition` | `0: Vec3` | Deterministic from world seed; inserted on `OnEnter(Playing)` |
| `VeinRegistry` | veins, domain bands, biomes | Built from RON assets at startup; underground resource queries |
| `DepositRegistry` | deposits | Built from RON assets at startup; surface deposit queries |

### Planet entity additions

World seed is stored on the Planet entity (see `planet-identity.md §2`) as a component inserted by `setup_world_config` on `OnEnter(GameState::Loading)`:

| Component | Fields | Notes |
|---|---|---|
| `WorldSeed` | `seed: u64` | Copied from `DomainSeeds.world`; save-game state |

### Surface chunk entity

| Component | Fields | Notes |
|---|---|---|
| `TerrainChunk` | `chunk_pos: IVec2` | Chunk grid coordinate |
| `Mesh3d` | | Heightmap mesh (65×65 verts, 64×64 quads) |
| `MeshMaterial3d` | | Single flat-green material (VS); biome shaders post-VS |
| `Transform` | | `Vec3::new(cx*64, 0, cz*64)` — mesh verts carry local Y offset |
| `RigidBody::Static` | | Avian physics body |
| `Collider` | | Trimesh built from mesh geometry; inserted one frame after mesh |

### Surface deposit entity

| Component | Fields | Notes |
|---|---|---|
| `OreDeposit` | `chunk_pos: IVec2` | Owning chunk — used for co-despawn with chunk |
| | `ores: Vec<(String, f32)>` | Weighted ore blend; weights normalised to sum 1.0 |
| | `total_extracted: f32` | Cumulative extraction; drives depletion curve in mining.md |
| | `depletion_seed: u64` | Per-deposit seed; determines floor and decay rate |
| | `miner: Option<Entity>` | Miner currently occupying this deposit; `None` = open |
| `Transform` | | `Vec3::new(wx, surface_y + 0.75, wz)` — floats 0.75m above terrain |
| `Discovered` | (marker) | Absent until proximity reveal; gates ore-blend visibility |

### Discovery site entity (gateway ruins)

| Component | Fields | Notes |
|---|---|---|
| `GatewayRuins` | (marker) | Tag identifying this entity as the VS escape objective site |
| `Transform` | | World position set from `GatewayRuinsPosition` |
| `Discovered` | (marker) | Inserted by `ruins_discovery_system` on drone proximity |

---

## 3. Surface Terrain — Chunk Streaming

Surface terrain is **streamed by camera proximity**. No chunk is pre-generated — each is built on demand when it enters spawn range and discarded when it leaves despawn range.

**Constants:**
- `CHUNK_SIZE: i32 = 64` — chunk footprint in world units (meters)
- `SPAWN_DIST: i32 = 4` — chunk-grid radius to maintain loaded (Chebyshev distance ≤ 4)
- `DESPAWN_DIST: i32 = 6` — chunk-grid radius beyond which chunks are unloaded (> 6)

**`spawn_chunks` system** (runs every frame while `WorldConfig` exists):
1. Read camera `Transform`; compute `cam_chunk = floor(cam.translation.xz / 64)`
2. For each `(cx, cz)` with `max(|cx - cam_chunk.x|, |cz - cam_chunk.y|) ≤ SPAWN_DIST`:
   - Skip if `SpawnedChunks` already contains this coord
   - Generate heightmap mesh from `world_seed` + `(cx, cz)`
   - Spawn `TerrainChunk` entity at world pos `(cx*64, 0, cz*64)` with `RigidBody::Static`
   - Insert into `SpawnedChunks`

**`despawn_chunks` system** (runs every frame):
1. For each `TerrainChunk` entity:
   - Compute Chebyshev distance from `cam_chunk`
   - If distance > `DESPAWN_DIST`: despawn entity, remove from `SpawnedChunks`

**`add_chunk_colliders` system** (runs on `Changed<Mesh3d>` with `RigidBody`):
- Avian cannot read `Mesh3d` during spawn (render asset not yet assigned)
- One frame after mesh assignment, builds trimesh `Collider` from mesh geometry

**Deposit lifecycle** is tied to chunk lifecycle — `spawn_deposit_markers` runs after `add_chunk_colliders`; `despawn_deposit_system` reacts to `ChunkUnloaded` events. Pristine unoccupied deposits are despawned; mined or occupied deposits survive (see §6).

---

## 4. Heightmap Generation

Each chunk's heightmap is generated by `generate_chunk_mesh(seed, cx, cz)`.

**Mesh structure:**
- Vertex grid: `(CHUNK_SIZE + 1) × (CHUNK_SIZE + 1)` = 65×65 vertices
- Triangle indices: `CHUNK_SIZE² × 2` triangles = 8,192 triangles per chunk
- Local vertex positions: `(lx, height(lx, lz), lz)` for `lx, lz ∈ [0, 64]`
- World vertex: local + chunk origin `(cx*64, 0, cz*64)`

**`TerrainSampler`** — reusable height query object:
```
noise = HybridMulti<Perlin>(seed=(world_seed ^ (world_seed >> 32)) as u32)
  .octaves(5)
  .frequency(1.1)
  .lacunarity(2.8)
  .persistence(0.4)

height_at(wx, wz) = noise.get([wx / 1000.0, wz / 1000.0]) * 50.0
```

Height range is approximately ±50 m relative to Y=0. The sample coordinates are divided by 1000.0 — one noise period spans ~1000m, so terrain features are 100–1000m in scale.

**Normal computation:** Each vertex normal is computed from the finite-difference gradient of neighboring heights:
```
dx = height(lx+1, lz) - height(lx-1, lz)   (right edge: h_px - h used)
dz = height(lx, lz+1) - height(lx, lz-1)   (bottom edge: h_pz - h used)
normal = normalize(Vec3(-dx, 2.0, -dz))
```

**UV:** `(lx / 64, lz / 64)` — tiles once across the chunk.

**`TerrainSampler` is stateless given seed** — creating two samplers from the same seed produces identical output. Constructing one per query site (rather than once globally) is safe and correct.

---

## 5. Underground Resource Domain

> **Terminology:** A *vein* (this section) and a *deposit* (§6) are different systems. A vein is an underground ore probability model — queried programmatically from world coordinates, no entity spawned, no machine placed on it. A deposit is a surface ECS entity at a specific world position where a miner machine is installed. These terms are not interchangeable.

Underground content is modeled as a **resource domain** backed by a 3D cell grid independent of surface chunks. This supports buried resources, geothermal sites, and future underground POIs without committing the game to a complete parallel underground world.

**Cell geometry** (defined in `content/mod.rs`):
- `CHUNK_SIZE: i32 = 32` — underground chunk unit (distinct from surface 64m)
- `CELL_CHUNKS_XZ: i32 = 5` → one XZ cell = 160m × 160m
- `CELL_CHUNKS_Y: i32 = 2` → one Y cell = 64m tall

Cell coordinate from world coordinate:
```
cell_x = wx.div_euclid(160)
cell_y = wy.div_euclid(64)
cell_z = wz.div_euclid(160)
```

### Domain bands

`LayerDef` currently defines vertical domain bands in cell-Y units. The type name is historical; design-facing docs should treat these as resource-domain bands rather than a universal world-layer system:

```ron
LayerDef(
  id: "underground",
  name: "Underground",
  y_cell_range: (-5, -1),  // cell Y -5 through -1 inclusive
)
```

Domain band ranges must not overlap (validated at startup; overlaps emit `error!` and the higher band silently wins). Surface is approximately cell_y = 0 (Y world ≈ 0).

### Biomes

`BiomeDef` binds a domain band to a weighted vein pool. The serialized field remains `layer` until the content schema is renamed:

```ron
BiomeDef(
  id: "deep_geothermal",
  layer: "underground",
  vein_pool: [("iron_vein", 3), ("copper_vein", 2)],
)
```

`VeinRegistry.biome_at_cell_y(cell_y)` — returns the biome whose domain band's `y_cell_range` contains `cell_y`, or `None` if none match. Domain bands must not overlap (validated at startup; see §13 edge cases), so at most one band covers any given `cell_y` — "first match" is the only possible match when content is valid.

### Vein placement — `cell_vein`

Per-cell decision using a **deterministic seeded RNG**:

```
cell_seed = xxh64(world_seed ++ "vein" ++ cell_x ++ cell_y ++ cell_z)
rng = Pcg64::seed_from_u64(cell_seed)
if rng.gen_bool(0.33):  // 33% of cells contain a vein
    roll = rng.gen_range(0..pool_total)
    pick vein from weighted pool
```

### Per-position ore query — `ore_at(world_seed, wx, wy, wz)`

Given a vein exists in the cell:

1. Compute normalised position within cell: `nx, ny, nz ∈ [-1, 1]`
2. Ellipsoidal distance from cell center: `dist = sqrt(nx² + ny² + nz²)`
3. Per-position jitter (organic boundary):
   ```
   jitter_seed = xxh64(world_seed ++ "jit" ++ wx ++ wy ++ wz)
   jitter = jitter_seed / u64::MAX * 0.5 - 0.25   // ∈ [-0.25, 0.25]
   density_scale = clamp(1.0 - (dist + jitter), 0.0, 1.0)
   effective_density = min(vein.density * density_scale * 1.5, 1.0)
   ```
4. Per-position RNG:
   ```
   pos_seed = xxh64(world_seed ++ "ore" ++ wx ++ wy ++ wz)
   pos_rng = Pcg64::seed_from_u64(pos_seed)
   if pos_rng.gen_bool(effective_density):
       return vein.pick_ore(&mut pos_rng).material
   ```
5. Returns `Option<u8>` material index; `None` means no ore at this position.

**Vein shape:** Approximately ellipsoidal with jittered boundary. Cell center (dist=0) has `density_scale=1.0`; cell surface (dist=1) has `density_scale ≈ 0`. The 1.5× factor compensates so average density across the ellipsoid approximates `vein.density`.

### Content format

```ron
// assets/veins/iron.ron
VeinDef(
  id: "iron_vein",
  density: 0.6,
  primary:   OreSpec(name: "Iron Ore",   material: 10, weight: 70),
  secondary: OreSpec(name: "Copper Ore", material: 11, weight: 25),
  sporadic:  Some(OreSpec(name: "Tin Ore", material: 12, weight: 5)),
)

// assets/resource_domains/underground.ron
LayerDef(id: "underground", name: "Underground", y_cell_range: (-5, -1))

// assets/biomes/cave.ron
BiomeDef(id: "cave", layer: "underground", vein_pool: [("iron_vein", 1)])
```

---

## 6. Surface Deposit Placement

Surface deposits are **one per 64×64m cell**, placed lazily when a chunk loads.

**`DepositRegistry.ore_at(seed, wx, wz)`:**
1. `cell_x = wx.div_euclid(64.0) as i64`, `cell_z = wz.div_euclid(64.0) as i64`
2. `cell_seed = xxh64(seed ++ "dep" ++ cell_x ++ cell_z)`
3. `rng = Pcg64::seed_from_u64(cell_seed)`
4. `rng.gen_bool(0.33)` — 33% of cells have a deposit; `None` if false
5. `idx = rng.gen_range(0..deposits.len())` — pick deposit type
6. Return `deposits[idx].ores.clone()`

Since surface CHUNK_SIZE = 64 and deposit cell = 64, **one chunk maps to exactly one deposit cell.** `chunk_deposit` queries the cell at the chunk's center point `(cx*64 + 32, cz*64 + 32)`.

**Deposit spawn** (`spawn_deposit_markers`, fires on `Added<TerrainChunk>`):
1. Query `chunk_deposit` — if `None`, skip
2. If an `OreDeposit` already exists with matching `chunk_pos` (player re-entered chunk whose deposit survived unload): skip — no double-spawn
3. Sample terrain height at cell center with `TerrainSampler`
4. Derive `depletion_seed = xxh64(world_seed ++ "depl" ++ cx ++ cz)`
5. Spawn `OreDeposit { miner: None, total_extracted: 0.0, ... }` at `(wx, surface_y + 0.75, wz)`

**Deposit despawn** (`despawn_deposit_system`, reacts to `ChunkUnloaded { chunk_pos }` event):
- If `deposit.total_extracted == 0.0 && deposit.miner.is_none()` → despawn (pristine, unoccupied)
- Otherwise → **keep alive** — a deposit that has been mined or has a miner attached persists indefinitely regardless of chunk state

This prevents depletion reset via chunk reload. When the player returns, step 2 of spawn skips the existing entity.

> **Code gap:** current `despawn_deposit_markers` polls `SpawnedChunks` and despawns unconditionally — it does not implement the persist-if-mined rule. The event-driven `despawn_deposit_system` with survival logic must be implemented to match this spec.

**Content format:** see `mining.md §10` (`DepositDef`).

---

## 7. Discovery Site Placement

Discovery sites are **persistent world objects** spawned on `OnEnter(Playing)` (not lazily with chunks). VS has one site type: `GatewayRuins`.

### Gateway ruins placement

```
x = (derive(world_seed, "ruins_x") % 200) as f32 - 100.0   // ∈ [-100, 100)
z = (derive(world_seed, "ruins_z") % 200) as f32 - 100.0   // ∈ [-100, 100)
y = TerrainSampler::new(world_seed).height_at(x, z)
```

`derive(seed, key)` = `xxh64(seed.to_le_bytes() ++ key.as_bytes(), 0)`.

Position range: XZ within 141m of origin (diagonal max). The site is guaranteed within the core exploration zone from spawn.

**Discovery trigger** (`ruins_discovery_system`, runs in `PlayMode::DronePilot`):
- Queries active drone `Transform`
- For each `GatewayRuins` without `Discovered`:
  - If `drone.distance(ruins) ≤ 8.0m`: write `DiscoveryEvent("gateway_ruins")`, insert `Discovered`
- `Discovered` marker prevents re-firing. Query uses `Without<Discovered>` filter.

**Post-VS:** Generic `DiscoverySite` component replacing `GatewayRuins`; multiple site types with data-driven placement rules (biome affinity, min-distance constraints, tier-gated trigger types). See §15.

### Placement invariants (VS)

- Ruins position is deterministic for a given `world_seed`
- Ruins position is independent of player exploration order
- Discovery fires exactly once per run (idempotent via `Discovered` marker)
- Discovery radius: 8m drone proximity

---

## 8. World Extent & Bounds

**VS:** No hard world boundary enforced. Terrain generates in all directions without limit. The de facto play area is constrained by:
- Ruins within 141m of origin (§7)
- Surface deposits within visited chunks only (~256m active radius)
- Drone range limits (see `drone.md`)

**MVP:** A bounded world radius derived from `world_seed` and difficulty tier. Terrain beyond the radius uses a flat placeholder and blocks further generation. World radius is a seeded parameter within difficulty-tier bounds (exact ranges TBD in balance pass). A `WorldBounds` resource holds the run's computed radius.

The core zone — region around spawn guaranteed to contain all recipe-graph-required resource types — is a seeded sub-radius of the world radius. VS approximation: ruins within 141m implicitly acts as a single-site core zone guarantee.

---

## 9. Seed → Geography Mapping

All world generation derives from `DomainSeeds.world` (the world sub-seed). Sub-seeds are computed once at run start; no shared RNG stream is consumed.

| Geographic feature | Derivation |
|---|---|
| Surface heightmap | `TerrainSampler::new(world_seed)` — noise seeded via `(world_seed ^ (world_seed >> 32)) as u32` |
| Underground cell vein | `xxh64(world_seed ++ "vein" ++ cell_x ++ cell_y ++ cell_z)` |
| Underground ore at position | `xxh64(world_seed ++ "ore" ++ wx ++ wy ++ wz)` (presence) and `xxh64(world_seed ++ "jit" ++ wx ++ wy ++ wz)` (boundary jitter) |
| Surface deposit cell | `xxh64(world_seed ++ "dep" ++ cell_x ++ cell_z)` |
| Deposit depletion rate | `xxh64(world_seed ++ "depl" ++ cx ++ cz)` |
| Gateway ruins XZ | `xxh64(world_seed ++ "ruins_x")`, `xxh64(world_seed ++ "ruins_z")` |
| Gateway ruins Y | `TerrainSampler::new(world_seed).height_at(ruins_x, ruins_z)` |

Changing any one generation domain (e.g. deposit placement) does not affect other domains — each has independent key strings. Adding a new domain does not invalidate existing seeds.

**RNG algorithm:** `Pcg64` (`rand_pcg`) seeded per-site. Provides reproducible output across platforms and Rust versions. Surface heightmap uses `HybridMulti<Perlin>` from the `noise` crate, seeded via the u32 cast.

---

## 10. Generation Sequence

```
OnStartup:
  load_content()
    → DepositRegistry, VeinRegistry inserted as resources

OnEnter(GameState::Loading):
  setup_world_config()
    → WorldSeed { seed: DomainSeeds.world } inserted onto Planet entity

OnEnter(GameState::Playing):
  spawn_gateway_ruins_system()
    → GatewayRuinsPosition resource inserted
    → GatewayRuins entity spawned at seeded position

Every frame (while Playing):
  spawn_chunks()          — lazy surface mesh generation
  despawn_chunks()        — unload distant chunks
  add_chunk_colliders()   — deferred collider build (Changed<Mesh3d>)
  spawn_deposit_markers() — runs after add_chunk_colliders (Added<TerrainChunk>)
  despawn_deposit_system() — event-driven; reacts to ChunkUnloaded
```

Underground vein data is **not streamed** — `VeinRegistry.ore_at` is a pure function called on demand by any system that needs underground ore at a position. No underground chunk entities exist in VS.

---

## 11. Chunk Boundary Conditions

Surface terrain is **seam-free across chunk boundaries** by construction. `TerrainSampler.height_at(wx, wz)` is a pure function of world coordinates — the same `(wx, wz)` always returns the same height regardless of which chunk is querying it. Vertices at chunk edges share the same world coordinates with adjacent chunks, so heights are identical and no T-junction or height discontinuity exists.

**Normal computation** at chunk edges uses forward/backward finite differences that extend one vertex beyond the chunk boundary (`lx + 1` when `lx == CHUNK_SIZE`, `lz + 1` when `lz == CHUNK_SIZE`). These extra lookups sample the actual continuous noise — no clamping or edge-special-casing is applied. Normals at shared edges may differ between adjacent chunk meshes by one sample step (each chunk computes its own normals), which produces a negligible visual discontinuity that biome-aware shading will mask post-VS.

**UV** is per-chunk local (`[0, 1]` across each chunk). Biome shaders must use world-space coordinates for seamless triplanar texturing — per-chunk UV tiling is expected at this resolution.

**Chunk streaming** creates a brief single-frame gap between chunk despawn and adjacent chunk respawn when the camera moves exactly at DESPAWN_DIST. This is acceptable — SPAWN_DIST (4) < DESPAWN_DIST (6) provides a two-chunk hysteresis band. A chunk at distance 5 is already loaded (within SPAWN_DIST 4? no — 5 > 4) — wait, SPAWN_DIST=4 means chunks within Chebyshev distance 4 are spawned; DESPAWN_DIST=6 means chunks beyond 6 are despawned. Chunks at distance 5–6 are loaded but not being re-spawned — they persist from earlier spawn. This hysteresis prevents thrashing.

---

## 12. Events

| Event | Type | Emitted by | Payload |
|---|---|---|---|
| `DiscoveryEvent` | `Message` | `ruins_discovery_system`, `deposit_discovery_system` | `String` site/type ID |
| `ChunkUnloaded` | `Message` | `despawn_chunks` | `chunk_pos: IVec2` |

`despawn_chunks` emits `ChunkUnloaded` immediately before despawning each chunk entity. `despawn_deposit_system` reads these events to apply the persist-if-mined survival rule. `spawn_deposit_markers` uses `Added<TerrainChunk>` (no event needed for spawn path).

---

## 13. Edge Cases

**Empty content registries:** `DepositRegistry` with no defs returns `None` from all queries (no deposits spawn). `VeinRegistry` with no biomes returns `None` from all queries. Both cases are handled without panic.

**World seed = 0:** Valid seed; produces a deterministic world. All derivations use 0 as the base and produce non-trivially distributed outputs via xxh64.

**Camera at world origin:** `cam_chunk = IVec2::new(0, 0)`. All chunks within 4 tiles in each direction spawn normally. No special spawn logic at origin.

**Ruins inside a chunk that hasn't loaded yet:** Ruins entity is spawned unconditionally on `OnEnter(Playing)` regardless of chunk state. It floats at the seeded Y height derived from `TerrainSampler` — this is the same height the chunk mesh will render when loaded. No dependency on chunk state.

**Mined deposit outlives chunk unload:** `despawn_deposit_system` keeps any deposit with `total_extracted > 0.0` or `miner.is_some()` alive after the owning chunk unloads. On re-enter, `spawn_deposit_markers` finds the existing entity and skips — no double-spawn, no depletion reset. Pristine unoccupied deposits are still cleaned up to avoid accumulating unseen entities.

**Overlapping domain band definitions:** `validate_layers` at `VeinRegistry::new` emits `error!` for any overlap. The query `biome_at_cell_y` returns the first matching biome (iterator order = load order = filesystem ordering). Overlapping bands produce ambiguous results — treat as a content bug.

---

## 14. Integration Test Invariants

These are the testable guarantees the implementation must satisfy. Each can be verified in a `World`-based test without rendering.

**Heightmap:**
1. `TerrainSampler::new(seed).height_at(x, z)` returns the same value on repeated calls with the same inputs.
2. Two samplers from different seeds return different heights for the same coordinates (statistical; passes for all seed pairs tested).
3. `generate_chunk_mesh(seed, cx, cz)` produces `(CHUNK_SIZE+1)² = 4225` vertices and `CHUNK_SIZE² * 6 = 24576` indices.
4. `generate_chunk_mesh(seed, cx, cz)` called twice returns identical vertex positions.
5. Height at chunk boundary vertex `(cx*64, cz*64)` matches `TerrainSampler::height_at(cx*64, cz*64)`.

**Chunk streaming:**
6. After one `spawn_chunks` frame with camera at origin, all chunks within Chebyshev distance 4 are in `SpawnedChunks`.
7. A `TerrainChunk` entity beyond Chebyshev distance `DESPAWN_DIST` from camera is despawned by `despawn_chunks`.
8. A `TerrainChunk` entity within distance `DESPAWN_DIST` from camera is kept.

**Surface deposits:**
9. `DepositRegistry::new([])` → `ore_at` returns `None` for all inputs.
10. `ore_at(seed, wx, wz)` with same inputs returns same result on repeated calls.
11. Weights in returned ore list sum to 1.0 (within 1e-5).
12. Across 20 adjacent chunk positions with a non-empty registry, not all return `Some` and not all return `None` (statistical variance).
13. `despawn_deposit_system` on `ChunkUnloaded`: despawns a pristine (`total_extracted == 0.0`) unoccupied (`miner.is_none()`) deposit.
14. `despawn_deposit_system` on `ChunkUnloaded`: keeps a deposit that has `total_extracted > 0.0` (mined).
15. `despawn_deposit_system` on `ChunkUnloaded`: keeps a deposit that has `miner.is_some()` (occupied by a miner machine), even if `total_extracted == 0.0`.
16. `spawn_deposit_markers` skips spawning a new deposit for a chunk that already has a surviving `OreDeposit` with matching `chunk_pos`.

**Underground veins:**
17. `VeinRegistry::new([], [], [])` → all queries return `None`.
18. `cell_vein(seed, cx, cy, cz)` returns the same result on repeated calls with the same inputs.
19. `ore_at(seed, wx, wy, wz)` returns the same result on repeated calls.
20. `biome_at_cell_y(y)` returns `None` for any `y` outside all defined `y_cell_range` values.
21. `biome_at_cell_y(y)` returns `Some` for any `y` within a defined range's inclusive bounds.
22. Center of an active vein cell (dist=0) returns `Some` ore for most seeds (> 10/50 when density=1.0).

**Discovery sites:**
23. `spawn_gateway_ruins_system` inserts `GatewayRuinsPosition` with a deterministic position for a given `world_seed`.
24. Two apps with identical setup produce identical ruins positions.
25. `ruins_discovery_system` inserts `Discovered` on the ruins entity when drone is within 8m.
26. `ruins_discovery_system` does not insert `Discovered` when drone is beyond 8m.
27. `ruins_discovery_system` does not fire again once `Discovered` is already present (idempotent).

---

## 15. VS / MVP Scope

### Vertical Slice

- Surface terrain: heightmap chunks, streaming, colliders ✓
- Underground: cell/domain-band/biome/vein system, `ore_at` query ✓
- Surface deposits: `DepositRegistry`, per-chunk placement, depletion seed ✓
- Discovery sites: single `GatewayRuins` site, seeded placement within ±100m of origin ✓
- No world boundary enforcement (unbounded terrain)
- No surface biome assignment (all surface uses same material/shader)
- No core zone guarantee (ruins placement is close enough for VS single-site escape)

### MVP additions

- **World boundary:** `WorldBounds` resource with seeded radius; `spawn_chunks` skips chunks outside radius; boundary mesh rendered at edge
- **Core zone guarantee:** system at run start places all recipe-graph-required surface deposit types within `core_zone_radius` of origin; verifies satisfaction of `DomainSeeds`-derived resource requirements
- **Surface biomes:** `BiomeRegion` entities covering XZ areas with distinct material definitions; `TerrainChunk` queries its biome on spawn; chunk material set from biome def; triplanar shaders use world-space UV
- **Biome assignment algorithm:** Voronoi regions from seeded biome seed points; each surface point assigned to nearest point; boundary blending radius 32m; biome types filtered by planet archetype (e.g. high-geothermal planet biases toward volcanic biomes)
- **Generic discovery sites:** `DiscoverySite { site_type: String, trigger_radius: f32 }` component replaces `GatewayRuins`; data-driven placement rules per site type (biome affinity, min-distance-from-spawn, min-distance-between-sites); multiple site types per run
- **Chunk LOD:** reduced vertex density at distance > `SPAWN_DIST - 1` chunks; full density only in innermost ring

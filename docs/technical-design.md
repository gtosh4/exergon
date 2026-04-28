# Technical Design Document

Core systems technical specification. Read `gdd.md` for design intent; this document covers implementation architecture. Updated as systems are designed.

---

## Table of Contents

1. [Seed System](#1-seed-system)
2. [Recipe Graph](#2-recipe-graph)
3. [Tech Tree](#3-tech-tree)
4. [Voxel World & Chunk System](#4-voxel-world--chunk-system)
5. [Multiblock Machine System](#5-multiblock-machine-system)
6. [Logistics Network](#6-logistics-network)
7. [Power System](#7-power-system)
8. [Drone System](#8-drone-system)
9. [Science & Research System](#9-science--research-system)
10. [World Reactivity](#10-world-reactivity)
11. [Codex & Meta-Progression](#11-codex--meta-progression)

---

## 1. Seed System

The seed is the single source of truth for all run variance. Same seed = identical run. Seeds are shareable and replayable — this is an intentional community feature.

### Seed format

Seeds are **free-form text strings**, player-visible and player-enterable. Internally, the text is hashed to a `u64` master seed for RNG initialization. The original text string is displayed to the player and used for sharing.

```
"my cool seed" → hash → 0xA3F2_91C0_4D7E_B851 (master seed)
```

Any UTF-8 string is a valid seed. Empty string is valid. Players can enter descriptive seeds ("geothermal run 3"), numeric seeds ("12345"), or share seeds verbatim from friends.

### Per-domain sub-seed derivation

Each generation domain gets an **independent sub-seed** derived from the master seed via keyed hash. Sub-seeds are never consumed from a shared RNG stream — they are computed directly from the master seed and a stable domain key string.

```
world_seed       = hash(master_seed, "world")
tech_tree_seed   = hash(master_seed, "tech_tree")
recipe_seed      = hash(master_seed, "recipes")
power_seed       = hash(master_seed, "power")
reactivity_seed  = hash(master_seed, "reactivity")
biome_seed       = hash(master_seed, "biomes")
```

This guarantees:
- Changing recipe generation logic does not affect world layout for the same seed
- Each domain can be generated independently, in any order
- Adding new domains in future versions does not invalidate existing seeds

### Lazy world generation

The world is generated **lazily and deterministically** — chunks are generated on demand as the player explores, not all upfront. Each chunk's content is derived from a chunk-specific seed:

```
chunk_seed = hash(world_seed, chunk_x, chunk_y, chunk_z)
```

The chunk seed is fully determined by the master seed and coordinates. Exploration order has no effect on chunk content. A chunk visited for the first time on run hour 1 or run hour 20 produces identical content.

Non-spatial domains (tech tree, recipe graph, power sources) are generated upfront at run start since they must be fully known for solvability validation.

### Constrained generation and validity guarantee

The generation algorithm is **constrained to always produce valid output** — no post-hoc rejection and regeneration. A valid run satisfies:

1. The recipe graph is a DAG with the escape artifact as terminal node
2. Every present tech tree node has at least one reachable unlock path
3. All required resources for the critical path exist somewhere in the world
4. Power sources sufficient to run the critical path exist in the tech tree

To guarantee validity, generation follows a **backwards-from-terminal** ordering:

1. Generate escape artifact (terminal node)
2. Generate recipe graph backwards from terminal, ensuring all prerequisites are satisfiable within the run's parameter bounds
3. Generate tech tree ensuring all recipe-required nodes are present and reachable
4. Generate world ensuring all resource types required by the recipe graph have valid spawn locations given the world's biome+layer layout
5. Generate power sources ensuring sufficient capacity exists for the critical path
6. Apply planet modifiers (these tune existing content, do not affect graph validity)

### Seed versioning

Seed derivation and generation algorithms must be **stable across game versions** to preserve shareability. A seed that produces a given run in v1.0 must produce the same run in v1.1.

Strategy:
- The seed format is versioned: seeds are prefixed with a version tag internally (transparent to the player)
- Generation algorithm changes that would alter output for existing seeds require a version bump
- Old-version seeds remain valid and produce the correct run for their version
- Breaking changes are documented in the changelog

### RNG algorithm

Use a **fast, reproducible, non-cryptographic PRNG** seeded from the derived sub-seed. Requirements: identical output across platforms (x86, ARM), reproducible across Rust versions, fast enough for chunk generation.

Recommended: `rand` crate with `SmallRng` (currently xoshiro128++ or similar) seeded per-domain and per-chunk. Sub-seed derivation uses a stable hash function (e.g. `xxhash` or `ahash` with fixed seeds).

> Note: Verify `SmallRng` stability guarantees across `rand` crate versions before committing. If stability is not guaranteed, use an explicitly versioned algorithm (e.g. `rand_pcg::Pcg64`).

---

## 2. Recipe Graph

The recipe graph is a directed acyclic graph (DAG) with the escape artifact as its terminal node. It is the intellectual core of each run — its structure is what players are discovering and optimizing against.

### Material types

Materials are divided into two categories:

**Base materials** — real-world-inspired (iron, copper, carbon, silicon, water, etc.). Present in every run with consistent identity. Pre-populated in the codex. Form the backbone of early-tier recipes. Players develop genuine cross-run expertise about base materials.

**Alien materials** — seeded per run. Unique name, appearance, and in-world properties generated from `recipe_seed`. Mid-to-high tiers introduce progressively more alien materials. The escape artifact is primarily alien materials. Alien materials are what make each run's science feel genuinely different.

The ratio shifts across tiers: Tier 1 is mostly base materials; by the final tier, alien materials dominate.

### Graph structure

The graph is organized by **tech tiers** (3–6 depending on difficulty). Each tier has:
- A **tier product** — the primary material or component that represents mastery of that tier
- **Multiple recipe steps** to produce the tier product from lower-tier materials
- **Cross-tier reuse** — steps within a tier routinely consume base materials and intermediate products from earlier tiers (machines and recipes unlocked in tier 1 remain relevant in tier 3)

This mirrors GTNH's tier structure: each age introduces new materials and machines, but earlier infrastructure remains load-bearing rather than obsolete.

The escape artifact is the terminal node. Its recipe requires tier-product inputs from all (or most) tiers, ensuring the full graph must be solved.

### Single critical path

MVP uses a **single critical path** — one correct route to the escape artifact. The player's task is to discover, analyse, and optimise that path. Recipe parameter variance (input quantities, yields, processing time, energy cost) provides run-to-run challenge within the fixed structure.

Multiple viable paths (player chooses which branch to pursue) is a post-MVP consideration.

### Byproducts

Most byproducts are **inputs to other recipes**, not waste. A byproduct from a tier 2 processing step may be a required input for a tier 3 recipe. This creates interesting graph topology — efficient factories route byproducts rather than voiding them.

Waste and waste management exist but are limited to specific cases (e.g. a reaction that produces a genuinely inert exhaust). Waste is not a primary design mechanic.

### Generation algorithm

Graph generation follows a **backwards-from-terminal** approach to guarantee solvability:

1. **Place escape artifact** as terminal node. Assign its alien material inputs (seeded).
2. **Generate final-tier recipes** — for each escape artifact input, generate the recipe(s) that produce it. Ensure all required machines exist in the tech tree (generated in parallel with the graph).
3. **Recurse up the tier stack** — for each recipe input, either assign it to an existing base material, an existing alien material already in the graph, or generate a new recipe producing it.
4. **Cross-tier stitching** — insert cross-tier dependencies where the recipe generator selects base materials or lower-tier products as inputs to higher-tier recipes.
5. **Byproduct routing** — after the critical path is complete, assign byproducts to existing recipe inputs where possible to create graph interconnections.
6. **Apply parameter variance** — seed-derive parameter multipliers for each recipe within GDD bounds (inputs 50–200%, yield 60–150%, time 50–300%, energy 50–250%).

The graph is fully generated at run start (not lazily) since it must be partially visible to the player from the beginning.

### Data structures

```rust
struct Material {
    id: MaterialId,
    kind: MaterialKind, // Base | Alien
    // display name, appearance, etc.
}

struct Recipe {
    id: RecipeId,
    inputs: Vec<(MaterialId, f32)>,   // material + quantity
    outputs: Vec<(MaterialId, f32)>,  // primary outputs
    byproducts: Vec<(MaterialId, f32)>,
    machine_tier: u8,
    conditions: Vec<ProcessingCondition>, // temperature, pressure, catalyst
    processing_time: f32,  // seconds, post-variance
    energy_cost: f32,      // per operation, post-variance
}

struct RecipeGraph {
    materials: HashMap<MaterialId, Material>,
    recipes: HashMap<RecipeId, Recipe>,
    // DAG edges: material → recipes that consume it
    //            recipe → materials it produces
    terminal: MaterialId, // escape artifact
}
```

### Validity invariants

A generated graph must satisfy:
- Terminal node (escape artifact) is reachable from starting conditions
- No cycles (enforced by backwards generation)
- Every alien material has exactly one producing recipe
- Every recipe's required machine tier exists in the run's tech tree
- All base materials required are available as world resources

---

## 3. Tech Tree

The tech tree is a tiered DAG of nodes drawn from a content pool. It gates access to recipes, machines, and capabilities. Its shape is always visible to the player; its contents are revealed through play.

### Node pool

The content pack defines a pool of available nodes. At run generation, the seed selects a subset to populate the run's tech tree. The pool grows as the game matures and content is added.

**Pool size and run node count are design parameters determined through playtesting** — they are directly tied to run length targets (GDD Q#10). The architecture must support arbitrary pool and run sizes; no counts are hardcoded.

Each node in the pool defines:
- `category` — Power, Processing, Logistics, Science, Exploration, etc.
- `tier_range` — the tiers this node can appear in (e.g. tiers 2–3)
- `rarity` — probability weight for selection (Common, Uncommon, Rare, Unique)
- `primary_prerequisite` — the node that must be unlocked before this one (if any)
- `alternative_prerequisites` — a pool of additional nodes that can also satisfy the prerequisite requirement; the run selects 0–N active alternatives from this pool
- `primary_unlock_vector` — the node's identity-defining unlock method
- `alternative_unlock_vectors` — additional unlock methods the run may activate
- `effects` — what unlocking this node grants (recipes, machines, capabilities)

### Prerequisite edges

Prerequisite relationships are **mostly fixed** — if node A requires node B and both are present, A always requires B. This preserves cross-run expertise (players learn the tree's stable structure).

**Alternative prerequisites** add per-run variation without chaos. A node defines a set of possible prerequisites beyond the primary; the run seed selects which (if any) are active. This creates alternative unlock paths — a node might be reachable via B in one run and via C or B+C in another.

```rust
struct NodeDef {
    id: NodeId,
    // ...
    primary_prerequisite: Option<NodeId>,
    alternative_prerequisites: Vec<NodeId>, // run selects subset as active
}
```

At run generation, for each node with alternatives, `tech_tree_seed` determines which alternatives are active. The result is a run-specific DAG where some nodes have multiple valid unlock paths.

### Unlock vectors

Five unlock vector types (all MVP):

| Vector | Trigger |
|---|---|
| `ResearchSpend` | Spend N research currency |
| `PrerequisiteChain` | All required prerequisite nodes unlocked |
| `ProductionMilestone` | Produce N units of material M |
| `ExplorationDiscovery` | Find specific in-world site/artifact |
| `Observation` | Witness specific in-world event or process |

Each node has a **primary vector** (part of its identity — e.g. an exploration node is characteristically exploration-gated) and an **alternative vector pool**. The run seed activates 0–N alternatives. Any active vector suffices to unlock the node.

This means a node that is primarily exploration-gated may also have research spend as an active alternative this run — at higher cost, but accessible without the exploration trigger.

### Tier gates

Each tier has an **unlock condition** that must be met before its shadow becomes visible and its nodes become researchable. Tier gate conditions are **data-driven** — each tier in the content pack specifies its gate type and parameters.

Gate condition types (extensible):

```rust
enum TierGateCondition {
    ProductionMilestone { material: MaterialId, quantity: f32 },
    ResearchThreshold { amount: u32 },
    ExplorationMilestone { site_tag: String },
    NodeUnlocked { node: NodeId },
    // extensible
}
```

The specific gate conditions per tier are a **design parameter to be determined** — they should be thematically appropriate to each tier's content. This is noted as a design TODO; the architecture supports any condition type.

### Player-visible shadow

Locked nodes display their shadow: `category`, `tier`, and `rarity`. Contents (effects, exact prerequisites, exact unlock vector) are hidden until unlocked. This gives players enough information to plan without removing discovery.

```rust
enum NodeVisibility {
    Shadow { category: NodeCategory, tier: u8, rarity: NodeRarity },
    PartiallyRevealed { /* broad parameters visible */ },
    FullyRevealed { /* complete node data */ },
}
```

### Validity invariants

A generated tech tree must satisfy:
- Every node present in the run has at least one reachable unlock path given the run's other generated content
- No prerequisite cycles (enforced by generation ordering — nodes generated tier by tier, low to high)
- Every recipe required by the recipe graph has a corresponding tech tree node present in the run
- The tier count matches the run's difficulty tier

---

## 4. Voxel World & Chunk System

### Scale and units

**1 block = 1 meter.** Machines are large multi-block structures at human scale. A basic machine might occupy 3×3×3 blocks; a high-tier machine 10×10×10 or larger. Players and drones move through a world that feels physically proportioned.

### World extent

The world is **infinite horizontally**, generated lazily on demand as the player explores (see §1 Seed System for chunk-seed derivation). Infinite world avoids any possibility of resource exhaustion forcing an unwinnable state.

Vertically, the world has **fixed extents seeded at run start** — the layer boundaries vary per run within defined ranges. Approximate layer Y ranges (exact values are seeded parameters):

| Layer | Y range (approx) | Access |
|---|---|---|
| Orbital | 1024+ | Space drone |
| Sky / atmosphere | 256–1024 | Flying drone |
| Surface | 0–256 | Starting layer |
| Underground | -512–0 | Digger drone |

Layer boundaries are legible in-world (atmospheric density changes, visual sky transitions, geological layer transitions underground).

### Core zone

Despite the infinite world, **critical resources are guaranteed within a core zone** around the spawn point. The world generator places all recipe-graph-required resource types within a bounded radius, ensuring no run requires extreme travel to be solvable. Additional deposits of the same resources may exist further out.

Core zone radius is a seeded parameter — some runs have compact resource geography, others are more spread out.

### Chunk system

Chunk size: **16×16×16 blocks**, matching standard voxel plugin conventions.

**Voxel plugin:** Use an existing Bevy voxel plugin rather than implementing chunk meshing from scratch. Plugin selection is deferred to implementation start — evaluate available options at that time (greedy meshing support, LOD, Bevy version compatibility). The chunk system architecture below is plugin-agnostic at the logical level.

Each chunk is identified by `ChunkCoord(i32, i32, i32)` in chunk space. World position converts to chunk coord by integer division by 16.

```rust
struct ChunkCoord(i32, i32, i32); // chunk-space coordinates

struct Chunk {
    coord: ChunkCoord,
    blocks: Box<[BlockId; 16 * 16 * 16]>,
    state: ChunkState,
}

enum ChunkState {
    Unloaded,
    Generating,
    Generated,
    Meshed,
}
```

Chunks are generated on first access using `chunk_seed = hash(world_seed, x, y, z)`. Generated chunks are cached; unloaded when outside a configurable load radius.

### Block types

```rust
enum BlockKind {
    Air,
    Terrain(TerrainBlockId),   // stone, soil, ice, alien rock, etc.
    Resource(ResourceBlockId), // ore deposits, fluid pockets
    Machine(MachineBlockId),   // part of a multi-block machine structure
    Infrastructure(InfraId),   // cables, conduits, pipes
    Structural(StructuralId),  // player-placed construction blocks
}
```

Resource blocks carry a `deposit_id` linking them to their parent deposit entity (for depletion tracking and reactivity).

### Terrain generation

Terrain uses **layered noise** derived from `chunk_seed`:
- Base heightmap (surface elevation)
- Cave/cavern networks (underground)
- Biome boundaries (lateral regions with distinct block palettes)
- Resource deposit placement (constrained by biome + layer affinity rules)
- Point of interest placement (persistent sites, within core zone or seeded positions)

Noise parameters (frequency, amplitude, octaves) are seeded per-run to produce varied terrain within consistent structural rules.

---

## 5. Multiblock Machine System

### Overview

Machines are multi-block structures: a fixed core footprint defined by a machine data file, plus optional module blocks attached at declared positions. A complete, valid structure forms a single logical machine entity.

### Machine data format

Each machine type is defined in a data file (content pack). The definition declares the 3D block pattern for each tier and the valid module attachment positions:

```toml
[machine.electric_furnace]
key_block = "electric_furnace_core"

[[machine.electric_furnace.tier]]
tier = 1
# 3D pattern in canonical (north-facing) orientation
# Each cell: block type ID or "_" for air, "?" for any player block
pattern = [
  # layer y=0
  [["casing", "casing", "casing"],
   ["casing", "electric_furnace_core", "casing"],
   ["casing", "casing", "casing"]],
]
[[machine.electric_furnace.tier.module_slot]]
position = [1, 0, 2]   # relative to core block, canonical orientation
allowed_types = ["speed", "efficiency"]

[[machine.electric_furnace.tier]]
tier = 2
# larger pattern; tier-1 structure is a valid sub-structure
pattern = [ ... ]
# more module slots than tier 1
```

Higher-tier patterns are strict supersets of lower-tier patterns (the smaller structure is a valid sub-structure), enabling in-place upgrades by adding blocks outward.

### Structure validation

Validation is **passive and key-block-anchored**:

1. When a key block (e.g. `electric_furnace_core`) is placed, a validation scan fires from that block
2. When any block adjacent to a formed machine is modified or removed, the machine is invalidated and a re-scan fires from the key block
3. The scan checks the surrounding region against the machine's tier patterns in all 4 valid orientations (Y-axis rotation, 90° increments)
4. The highest matching tier is used (a tier-2 pattern match takes precedence over tier-1)
5. On successful match: machine entity is formed, orientation is recorded, component blocks are marked

Once formed, component blocks hold a `MachineId` reference. No per-tick revalidation occurs — the machine is only re-scanned on structural change.

### Orientation

Machines support **8 orientations** — 4 Y-axis rotations (north, east, south, west) × 2 mirror states (normal, mirrored). Templates are stored in canonical (north-facing, non-mirrored) form. Validation tests all 8 combinations.

Orientation and mirror state are determined by the first successful match and stored on the machine entity. Module attachment positions, input/output faces, and UI indicators are all orientation-relative and mirror-aware.

### ECS structure

A machine is a **single ECS entity**. Component blocks carry a back-reference:

```rust
// On the machine entity:
struct Machine {
    machine_type: MachineTypeId,
    tier: u8,
    orientation: Orientation,
}
struct MachineInventory { ... }
struct RecipeProcessor { current_recipe: Option<RecipeId>, progress: f32 }
struct PowerConsumer { demand: f32 }
struct ModuleSlots { slots: Vec<Option<ModuleId>> }

// On each component block:
struct PartOfMachine(Entity); // back-reference to machine entity
```

Module blocks also carry `PartOfMachine` and a `ModuleSlot(usize)` index.

### Post-MVP: build assistance

A ghost overlay showing missing or misplaced blocks for an in-progress machine structure is planned post-MVP. The primary challenge is orientation inference — determining whether a partially-placed structure is the left or right side of a machine requires resolving orientation from incomplete information. Design deferred; architecture should not preclude it.

---

## 6. Logistics Network

### Network topology

Networks are formed by **cable adjacency**. Any cable or network device (machine port, storage node, interface) connected to a cable is part of that cable's network. Network membership is determined by graph traversal from any member node — all connected components form one network.

Network graphs are recomputed on structural change (cable placed/removed, machine formed/invalidated). Between changes, network state is stable and does not require per-tick recomputation of topology.

```rust
struct NetworkId(u32);

struct LogisticsNetwork {
    id: NetworkId,
    cables: HashSet<BlockPos>,
    devices: HashSet<Entity>,    // machines, storage, interfaces
    channel_usage: u32,
    channel_capacity: u32,
}

// On each cable block:
struct CableBlock {
    network: NetworkId,
    tier: CableTier,             // determines max devices
}

// On each network device (machine, storage, etc.):
struct NetworkDevice {
    network: NetworkId,
    channels_consumed: u32,
}
```

### Channel limits

Cables have a **devices-per-cable** channel limit determined by cable tier. Higher-tier cables support more connected devices before the limit is hit. Exceeding the limit requires sub-network segmentation or cable upgrades.

Interface blocks bridge two sub-networks with a defined channel allowance — pass-through for MVP, with potential for richer constraints post-MVP.

Different interface tiers may offer higher throughput as a post-MVP extension.

### Unified storage

All storage nodes on a network present a **unified item inventory** — the network indexes items across all connected storage. Items are not physically moved to a central location; the network tracks which storage node holds what and routes retrieval as needed.

Storage is a necessary system but not a primary design constraint. Players expand storage by adding storage nodes; no inventory management puzzle is intended.

### Auto-crafting and job dispatch

Crafting requests generate **job entities** that the network dispatches to capable machines. This is an explicit improvement over AE2's pattern mechanic:

**AE2 approach (avoided):** Players must physically encode crafting recipes into each machine as "patterns." Tedious, fiddly, poor UX.

**Exergon approach:** Machines **auto-register their capable recipes** from the tech tree on formation — no physical patterns required. When a craft is requested, the network creates a job for the required recipe, and any machine capable of running that recipe can accept it.

Players configure job **priorities and filters** rather than patterns:
- Priority: prefer machine A over B for recipe X
- Filter: this machine only accepts jobs of category Y (e.g. smelting only)
- Exclusion: this machine never accepts auto-crafting jobs (manual-only)

```rust
struct CraftingJob {
    id: JobId,
    recipe: RecipeId,
    quantity: u32,
    priority: i32,
    status: JobStatus,
}

enum JobStatus {
    Queued,
    Dispatched(Entity),   // assigned to machine entity
    InProgress { progress: f32 },
    Complete,
}

// On machine entity:
struct JobAcceptance {
    accepts_auto_jobs: bool,
    category_filter: Option<Vec<RecipeCategory>>,
    priority_bias: i32,
}
```

The network's job dispatcher runs when machines become idle — it scans queued jobs, finds the highest-priority job the machine can run, and assigns it. No per-tick polling; event-driven on machine idle and job creation.

### Network events

The network system is event-driven, not tick-based:
- **Topology change** (cable placed/removed) → recompute network graph
- **Machine idle** → dispatcher assigns next job
- **Storage change** → update network item index
- **Channel limit exceeded** → emit warning event (surfaced to player as bottleneck)

---

## 7. Power System

### Model

Power is **flow-based**: generators produce power units per tick, cables carry that flow to consumers, machines draw power when running recipes. Modelled after GTNH's EU system — cable tier determines voltage and amperage capacity; mismatched tiers have consequences.

Power cables are **physically separate from logistics cables**. Players lay power infrastructure independently of the logistics network. A single power network typically spans the whole base (unlike logistics, which encourages sub-networks).

### Power units

Power is measured in **watts (W)** — production rate and consumption rate, both per second. Generators output a fixed wattage. Machines draw a recipe-defined wattage while processing; **idle machines draw zero or minimal standby power**. Total demand is the sum of all actively processing machines at any moment.

### Upgrade pressure

Generators have **fixed output capacity**. As the factory scales up and higher-tier machines run more energy-intensive recipes, aggregate demand grows. Early-tier generators are not penalized — they simply get outmatched by demand. Players are soft-forced to upgrade power infrastructure to maintain throughput.

This is the primary progression pressure on power: not degradation, but natural demand growth outpacing fixed supply.

### Planet modifier efficiency

Planet modifiers apply **efficiency multipliers** to specific generator types (GDD §9):
- Solar efficiency: `0.4×`–`1.6×` based on star distance
- Combustion efficiency: scaled by atmospheric oxygen content
- Geothermal availability: scaled by geological activity

A solar array on a dim world produces significantly less power than baseline. Experienced players read planet modifiers at run start to identify which power strategy is favoured this run.

Multipliers apply to generator output, not demand. A `0.4×` solar modifier means solar arrays produce 40% of their rated wattage.

### Cable tiers

Power cables have tiers determining maximum wattage capacity. Exceeding cable capacity causes inefficiency or outage (exact consequence TBD — at minimum, a clear player-visible warning). Higher-tier cables carry more power.

```rust
struct PowerCable {
    network: PowerNetworkId,
    tier: CableTier,
    max_watts: f32,
}

struct PowerNetwork {
    id: PowerNetworkId,
    total_production: f32,   // sum of all generator output this tick
    total_demand: f32,       // sum of all active machine demand
    cables: HashSet<BlockPos>,
    generators: HashSet<Entity>,
    consumers: HashSet<Entity>,
}
```

### Power network topology

Power network membership follows the same cable-adjacency model as logistics. Recomputed on structural change; stable between changes.

### Machine power draw

Machines declare power demand per recipe in the recipe data. Demand is applied when a recipe starts and released when it completes or the machine is interrupted.

```rust
struct PowerConsumer {
    network: PowerNetworkId,
    active_demand: f32,   // 0.0 when idle, recipe wattage when processing
}

struct PowerProducer {
    network: PowerNetworkId,
    base_output: f32,
    efficiency_modifier: f32,  // from planet modifiers
    actual_output: f32,        // base_output * efficiency_modifier
}
```

### Brownout behaviour

When demand exceeds supply, machines are **throttled proportionally** rather than randomly cut off. A factory running at 150% demand runs all machines at ~67% speed. This is legible (everything slows, nothing mysteriously stops) and gives players a clear signal to expand power capacity.

---

## 8. Drone System

### Overview

Drones are player-piloted exploration and interaction tools. The player's attention — not their character — travels via drone. The character is left idle at its last position while a drone is active.

### Control model

When the player activates a drone, **camera and control fully transfer** to the drone's perspective. The character remains stationary in the world. Returning to character control requires switching back explicitly (recall drone or deactivate).

Drones have no autonomous behaviour — they are inert when not actively piloted. A drone parked at a remote site does nothing until the player switches to it. This rewards planning (knowing where to send a drone and what to do there) over multitasking.

### Multiple drones

Players can deploy multiple drones simultaneously and **switch between them**. Only one drone is active (controlled) at a time. Inactive drones remain at their last position, inert.

Switching is not inherently advantageous — the game does not reward rapid context-switching. A player with three deployed drones at three sites works sequentially, not in parallel. The value of multiple drones is positional: park one at a distant site, switch to it when needed, without having to travel again.

### Drone tiers and layer access

| Drone tier | Layer access | Tech tree requirement |
|---|---|---|
| Land drone | Surface terrain | Starting equipment |
| Amphibious drone | Surface water + underwater | Mid-tier tech |
| Digger drone | Underground layer | Mid-tier tech |
| Flying drone | Sky / atmosphere layer | High-tier tech |
| Space drone | Orbital layer | Late-tier tech |

Drone construction requires factory-produced components. Factory progression unlocks new drone tiers naturally.

### Persistence

Deployed drones **persist in the world** across sessions. A drone left at a remote site is still there on return. World events are unlikely to destroy unattended drones (world reactivity is legible and non-sudden), but drones could become temporarily inaccessible if terrain changes block their position.

```rust
struct Drone {
    drone_type: DroneType,
    position: Vec3,
    orientation: Quat,
    inventory: DroneInventory,   // samples and items collected
    state: DroneState,
}

enum DroneState {
    Idle,
    ActivelyControlled,
}
```

### Interaction model

While actively piloted, drones interact with the world **point-based** — the drone must be adjacent to a block to interact with it (mine it, collect a sample, trigger a site). No area-of-effect collection.

Mining is block-by-block while actively controlled. Higher-level commands (e.g. "mine this deposit vein") are a natural post-MVP extension that reduce tedium without removing the cost of the player's attention.

### Fog of war and scanning

Drones reveal fog of war at their current position. Range scanning (biome type + broad resource category) is available from the drone without requiring physical adjacency — a scan action from the drone provides imprecise data about the surrounding area out to a defined radius. Precise data (exact deposit location, quantity) requires physical proximity.

---

## 9. Science & Research System

### Research currency types

Research is not a single currency. Multiple **research types** are earned through different activities and spent on different things. Research type gates ensure players cannot bypass the discovery loop by grinding one activity.

| Research type | Primary sources | Gates |
|---|---|---|
| Material Science | Mineral/ore/fluid sample analysis | Recipe reveals, machine tier unlocks |
| Field Research | Ecosystem/biological sample analysis | Exploration-gated tech nodes, biome knowledge |
| Engineering | Production milestones, machine operation | Machine module unlocks, logistics upgrades |
| Discovery | Exploration finds, site interactions, observations | Exploration-only tech nodes, tier unlocks |

Specific type names and exact gating are content/balance decisions to be tuned. Architecture supports arbitrary research types defined in the content pack.

```rust
struct ResearchPool {
    amounts: HashMap<ResearchTypeId, f32>,
}
```

### Analysis stations

Analysis stations are **specialized multiblock machines** — one per sample domain. Each station processes samples of its type and produces research currency + knowledge outputs.

| Station | Sample types processed |
|---|---|
| Geological Analysis Station | Rock, mineral, ore, fluid samples |
| Biological Analysis Station | Flora, fauna, ecosystem samples |
| Atmospheric Analysis Station | Gas, particulate, energy-field samples |

Stations are tech-tree gated. Upgrading a station (higher machine tier) unlocks higher-tier sample processing and more efficient research output — same structure as other machines.

### Experiment model

Experiments are **crafting-style interactions** at an analysis station:

- Player places sample(s) + optional reagents/catalysts into the station
- Station runs an "experiment recipe" — a timed process consuming inputs
- Output: research currency (of appropriate type) + possible knowledge reveal

Experiment recipes increase in complexity and output at higher tiers, mirroring Factorio's science pack progression. Early experiments: basic samples → small research gain. Late experiments: complex multi-material inputs → large research gain + high-value reveals.

```rust
struct ExperimentRecipe {
    id: ExperimentRecipeId,
    station_type: StationType,
    station_tier: u8,
    inputs: Vec<(ItemId, u32)>,          // samples + reagents
    research_output: Vec<(ResearchTypeId, f32)>,
    knowledge_trigger: Option<KnowledgeTrigger>,
    processing_time: f32,
}
```

### Knowledge visibility model

Three tiers per recipe/node:

```
Known-to-exist → Partially-revealed → Fully-revealed
```

**Known-to-exist** is the default state for any node present in the run. The shadow (category, tier, rarity) is always visible.

**Partially-revealed** is **earned through gameplay**, not purchased:
- Hitting a production milestone that relates to the node
- An exploration discovery or observation that reveals context
- Completing an experiment that produces a relevant knowledge trigger

Partial reveal surfaces broad parameters (approximate input types, rough output range). It is a reward for engagement, not a purchasable step.

**Fully-revealed** is **purchased with research currency** of the appropriate type. Players can go directly from known-to-exist → fully-revealed, skipping partial reveal — at higher cost. Experienced players who recognise a node from prior runs (via the Codex) can skip the intermediate step entirely and spend research directly to full reveal.

```rust
enum NodeKnowledge {
    KnownToExist,
    PartiallyRevealed { broad_params: BroadParams },
    FullyRevealed { complete_data: NodeData },
}
```

### Research scarcity

Research is intentionally scarce enough to force tradeoffs, especially early. Players cannot reveal everything before committing to a path. The tension between "reveal more before building" and "build now and generate more research through production" is a core strategic rhythm of each run.

---

## 10. World Reactivity

### Per-region tracking

Reactivity is tracked **per region** — each biome area has its own reactivity score (`0.0`–`1.0`). Regions are not isolated; reactivity spreads slowly to adjacent regions, but the spread rate is low enough that players can maintain a high-reactivity industrial zone while keeping the broader world relatively clean.

```rust
struct RegionReactivity {
    region_id: RegionId,
    level: f32,           // 0.0 = pristine, 1.0 = maximum
    rate_seed: f32,       // seeded per-run: how fast this region reacts
}
```

The world's reactivity profile is seeded — some worlds react quickly and dramatically, others are resilient. `rate_seed` is per-region, so different biomes within the same run may react at different rates.

### Reactivity sources

Each action that increases reactivity contributes a specific rate per second to the affected region:

| Source | Contribution |
|---|---|
| Machine operation (pollution) | Continuous, scales with machine tier and count |
| Resource extraction | Continuous while mining, scales with extraction rate |
| Experimentation | Pulse on experiment completion |
| Energy output / heat signature | Continuous, scales with power generation |

Sources are tracked individually per region. **Full cause breakdown** (showing each source's contribution to the player) is **post-MVP** — MVP exposes the current level and its effects. Architecture records per-source contributions from day one to enable this later.

### Effect model — hybrid continuous + threshold

Reactivity effects are applied via two mechanisms simultaneously:

**Continuous effects** — smooth modifiers that scale with reactivity level. Examples:
- Local machine efficiency: `efficiency = 1.0 - (reactivity * 0.3)` — high reactivity zones run machines at up to 30% reduced efficiency
- Deposit yield degradation: ore patches in high-reactivity regions yield less per block
- Sample quality reduction: lower-quality samples from disturbed ecosystems

**Threshold events** — discrete events triggered at specific reactivity levels. Each threshold fires once per region per run:

| Threshold | Example event |
|---|---|
| 0.25 | Ecosystem disturbance — local fauna behaviour shifts, new sample type available |
| 0.50 | Ecosystem shift — significant biome character change, some sample types lost |
| 0.75 | Atmospheric change — affects machine efficiency parameters in region |
| 0.90 | Terrain degradation — deposit contamination, possible new phenomena |

Threshold events are **legible and visible before they trigger** — the player can see the current reactivity level and know what's coming. No surprises.

**Post-MVP:** Some threshold events also create opportunities (new resources, access to phenomena) rather than only costs (GDD Q#4).

### Recovery

Reactivity **decreases when sources are removed or reduced**. Recovery rate is intentionally faster than buildup rate — clean play is rewarded. A player who decommissions a polluting machine or switches to a cleaner process sees measurable improvement.

```rust
// Per-region per-tick update:
fn update_reactivity(region: &mut RegionReactivity, sources: f32, dt: f32) {
    let buildup = sources * region.rate_seed * dt;
    let recovery = if sources < region.level {
        (region.level - sources) * RECOVERY_RATE * dt  // faster than buildup
    } else {
        0.0
    };
    region.level = (region.level + buildup - recovery).clamp(0.0, 1.0);
}
```

`RECOVERY_RATE` is tuned to be meaningfully faster than typical buildup — exact ratio is a balance parameter. Threshold events do not reverse when reactivity drops below their threshold (the ecosystem shift happened; reducing reactivity prevents further damage but doesn't undo past changes).

---

## 11. Codex & Meta-Progression

### Save file architecture

Two distinct save scopes:

**Run save** — one file per run. Contains world state, factory state, tech tree progress, research pools, drone positions, run seed, completion status. Runs are **never automatically deleted** — players can revisit completed runs to explore builds or take screenshots. Runs carry a status tag: `InProgress`, `Completed`, `Abandoned`.

**Meta save** — single file. Contains codex, unlocked content, blueprints, starting boons pool. Persists across all runs. Updated at run completion and on milestone triggers mid-run.

```rust
enum RunStatus { InProgress, Completed, Abandoned }

struct RunSaveHeader {
    seed: String,
    difficulty: DifficultyTier,
    status: RunStatus,
    started_at: DateTime,
    completed_at: Option<DateTime>,
    run_time_seconds: u64,
}
```

### Codex

The codex is a **persistent encyclopedia** filled in through play. Entries are created on **first encounter** — when the player (via drone or direct presence) first observes a thing, its entry is created. Entries fill in as more is learned.

Codex entry types and what triggers creation:

| Entry type | Created on | Records |
|---|---|---|
| Biome | Drone scans / enters region | Possible resource pool, machine efficiency modifiers |
| Node type | Node appears in any run's tech tree shadow | Observed tier range, category, behavior patterns |
| Planet modifier | Modifier active in any run | Affected systems, effect direction |
| Machine type | Machine first built | Function, module types, tier sizes |
| Alien material | Material appears in any run's recipe graph | Observed production chains, tier it appears in |

Codex entries record **type-level knowledge**, not run-specific values. A biome entry shows the possible resource pool (what *can* spawn) — not what spawned in the current or any specific run. This preserves per-run variance while rewarding thorough play.

```rust
struct CodexEntry {
    entry_type: CodexEntryType,
    first_seen: DateTime,
    observations: Vec<CodexObservation>,  // accumulated across runs
}

struct MetaSave {
    codex: HashMap<CodexEntryId, CodexEntry>,
    unlocked_content: UnlockedContent,
    blueprints: Vec<Blueprint>,
    starting_boons_pool: Vec<BoonId>,
}
```

### Meta-progression unlocks

Unlocks expand what runs can be — not how easy they are.

**Unlock triggers:**
- **Run completion** — primary trigger. Completing a run at a given difficulty tier unlocks new content at that tier and above.
- **In-run milestones** — some unlocks trigger mid-run on specific achievements (e.g. first flying drone built, first Pinnacle-tier machine constructed). These fire immediately and persist to meta save.

```rust
enum UnlockTrigger {
    RunCompleted { difficulty: DifficultyTier },
    MilestoneReached { milestone_id: MilestoneId },
}

struct UnlockDef {
    id: UnlockId,
    trigger: UnlockTrigger,
    grants: UnlockGrant,
}

enum UnlockGrant {
    BiomeType(BiomeId),
    RunModifier(ModifierId),
    Narrative(NarrativeId),
    BlueprintSlot,
    StartingBoon(BoonId),
}
```

### Blueprints

Blueprints are saved to the **meta save** and persist across runs. A blueprint captures a sub-factory layout (block positions, machine types, cable routing) as a placeable template.

Blueprints are **templates, not solutions** — they encode layout only, not recipe parameters or machine configuration. A blueprint for a smelting array still requires the player to configure it for this run's specific ratios and machine parameters. They save placement time, not thinking.

Blueprint slots are finite and expand through meta-progression unlocks.

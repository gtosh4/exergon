# Technical Design Document

Core systems technical specification. Read `gdd.md` for design intent; this document covers implementation architecture. Updated as systems are designed.

---

## Table of Contents

1. [Seed System](#1-seed-system)
2. [Recipe Graph](#2-recipe-graph)
3. [Tech Tree](#3-tech-tree)
4. [World & Terrain System](#4-world--terrain-system)
5. [Machine System](#5-machine-system)
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

Seeds are **free-form text strings**, player-visible and player-enterable. Internally, the text is hashed to a 64-bit master seed for RNG initialization. The original text string is displayed to the player and used for sharing. Any UTF-8 string is valid, including empty strings, descriptive seeds ("geothermal run 3"), or numeric seeds ("12345").

*Implementation: [`RunSeed`](../src/seed/mod.rs#L15)*

### Per-domain sub-seed derivation

Each generation domain gets an **independent sub-seed** derived from the master seed via keyed hash. Sub-seeds are never consumed from a shared RNG stream — they are computed directly from the master seed and a stable domain key string. Domains include: world, tech tree, recipes, power, reactivity, and biomes.

*Implementation: [`DomainSeeds`](../src/seed/mod.rs#L23)*

This guarantees:
- Changing recipe generation logic does not affect world layout for the same seed
- Each domain can be generated independently, in any order
- Adding new domains in future versions does not invalidate existing seeds

### Lazy world generation

The world is generated **lazily and deterministically** — chunks are generated on demand as the player explores, not all upfront. Each chunk's seed is derived from the world sub-seed combined with its chunk coordinates. Exploration order has no effect on chunk content. A chunk visited for the first time on run hour 1 or run hour 20 produces identical content.

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
- The seed format is versioned: seeds carry a version tag internally (transparent to the player)
- Generation algorithm changes that would alter output for existing seeds require a version bump
- Old-version seeds remain valid and produce the correct run for their version
- Breaking changes are documented in the changelog

### RNG algorithm

Use a **fast, reproducible, non-cryptographic PRNG** seeded from the derived sub-seed. Requirements: identical output across platforms (x86, ARM), reproducible across Rust versions, fast enough for chunk generation.

Recommended: `rand` crate with `SmallRng` (currently xoshiro128++ or similar) seeded per-domain and per-chunk. Sub-seed derivation uses a stable hash function (e.g. `xxhash` or `ahash` with fixed seeds).

> Note: Verify `SmallRng` stability guarantees across `rand` crate versions before committing. If stability is not guaranteed, use an explicitly versioned algorithm (e.g. `rand_pcg::Pcg64`).

---

## 2. Recipe Graph

The recipe graph is a directed acyclic graph (DAG) of items with the escape artifact as its terminal node. It is the intellectual core of each run — its structure is what players are discovering and optimizing against.

### Materials, forms, and items

**Materials** are abstract substance identities. Each records: a unique ID, kind (base or alien), display name, and a set of form group memberships. Base materials are content-defined and present in every run. Alien materials are generated from the recipe sub-seed — unique name and properties per run.

**Form groups** are content-defined sets of forms (ore, ingot, wire, plate, rotor, …). A material declares which groups it belongs to; this determines which derived items exist for it. Groups are defined once and shared across all materials — adding a new material with the `metal` group automatically grants it all metal forms and all metal recipe templates.

**Items** are the recipe graph's actual nodes — three kinds:
- **Derived** — (material, form) pairs, generated from material × group membership. No asset file; exist whenever the material is present in the run.
- **Composite template** — a pattern defined in assets that instantiates per material (e.g. `[X]_cable = [X]_wire + rubber`), generating one composite item per qualifying material.
- **Unique** — explicit asset-defined items with no material-form derivation.

The ratio of base to alien shifts across tiers: Tier 1 is mostly base material processing; the final tier and escape artifact are primarily alien.

### Graph structure

The graph is organized by **tech tiers** (3–6 depending on difficulty). Each tier has:
- A **tier product** — the primary item (usually a composite or unique alien item) that represents mastery of that tier
- **Multiple recipe steps** to produce the tier product from lower-tier items
- **Cross-tier reuse** — steps within a tier routinely consume base-material derived items and intermediates from earlier tiers (machines and recipes unlocked in tier 1 remain relevant in tier 3)

This mirrors GTNH's tier structure: each age introduces new materials, machine types, and item forms, but earlier infrastructure remains load-bearing rather than obsolete.

The escape artifact is the terminal node. Its recipe requires tier-product inputs from all (or most) tiers, ensuring the full graph must be solved.

### Single critical path

MVP uses a **single critical path** — one guaranteed route to the escape artifact. The player's task is to discover, analyse, and optimise that path. Recipe parameter variance (input quantities, yields, processing time, energy cost) provides run-to-run challenge within the fixed structure.

Multiple viable paths (player chooses which branch to pursue) is a post-MVP consideration. The data model supports multiple recipes per material from the start — the generator simply guarantees at least one complete path exists in MVP; post-MVP it can generate competing alternatives.

### Byproducts

Most byproducts are **inputs to other recipes**, not waste. A byproduct from a tier 2 processing step may be a required input for a tier 3 recipe. This creates interesting graph topology — efficient factories route byproducts rather than voiding them.

Waste and waste management exist but are limited to specific cases (e.g. a reaction that produces a genuinely inert exhaust). Waste is not a primary design mechanic.

### Generation algorithm

Graph generation follows a **backwards-from-terminal** approach to guarantee solvability:

1. **Place escape artifact** as terminal node. Assign its alien item inputs (seeded — typically unique or composite alien items).
2. **Generate final-tier recipes** — for each escape artifact input, generate the recipe(s) that produce it. Ensure all required machines exist in the tech tree (generated in parallel with the graph).
3. **Recurse up the tier stack** — for each recipe input, assign it to an existing derived item (base or alien material in the appropriate form), an existing composite/unique item, or generate a new item and recipe.
4. **Template expansion** — once critical-path materials and machines are determined, expand all applicable recipe templates across all materials present in the run. This populates the full derived-item graph around the critical path.
5. **Cross-tier stitching** — insert cross-tier dependencies where the recipe generator selects base-material items or lower-tier products as inputs to higher-tier recipes.
6. **Byproduct routing** — after the critical path is complete, assign byproducts to existing recipe inputs where possible to create graph interconnections.
7. **Apply parameter variance** — seed-derive parameter multipliers for each concrete recipe within GDD bounds (inputs 50–200%, yield 60–150%, time 50–300%, energy 50–250%).

The graph is fully generated at run start (not lazily) since it must be partially visible to the player from the beginning.

### Data model

Each **material** records: a unique ID, kind (base or alien), display name, appearance, and a list of form group IDs it belongs to.

Each **form group** records: a unique ID and an ordered list of form IDs (e.g. `metal` → `[ore, crushed_ore, dust, ingot, plate, wire, rotor]`). Form groups are content-defined.

Each **item** is one of:
- `DerivedItem { material: MaterialId, form: FormId }` — generated; no asset file
- `CompositeItem { id, template: Option<TemplateId>, inputs: Vec<ItemStack> }` — asset-defined; `template` is set when this item is one instantiation of a composite template
- `UniqueItem { id, inputs: Vec<ItemStack>, ... }` — asset-defined, one-off

Each **recipe template** records: a unique ID, input form(s) and base quantities, output form and base quantity, the form group both sides must belong to, required machine type, base processing time, and base energy cost.

Each **concrete recipe** (generated at run start) records: a unique ID, source template ID (if template-derived), input items with quantities, output items with quantities, byproduct items with quantities, required machine type and tier, processing conditions (temperature, pressure, catalyst), processing time (post-variance), energy cost in joules (post-variance), and `min_voltage_tier` (derived from required machine tier — higher machine tiers require higher voltage delivery).

A machine is capable of running a concrete recipe when machine type and tier match. A machine can run any number of matching recipes. An item can have multiple producing recipes — alternative production routes are valid.

The graph indexes concrete recipes by producing item and consuming item for efficient lookup in both directions. The terminal item (escape artifact) is stored as a root reference.

### Validity invariants

A generated graph must satisfy:
- Terminal node (escape artifact) is reachable from starting conditions
- No cycles (enforced by backwards generation)
- Every item on the critical path has at least one producing concrete recipe
- Every concrete recipe's required machine tier exists in the run's tech tree
- All base-material derived items required on the critical path have their material available as a world resource

---

## 3. Tech Tree

The tech tree is a tiered DAG of nodes drawn from a content pool. It gates access to recipes, machines, and capabilities. Its shape is always visible to the player; its contents are revealed through play.

> Content design (tier themes, unlock rates, node pool allocation, category breakdown, Tier 1 node set): see [`tech-tree-design.md`](../tech-tree-design.md). This section covers the data model and implementation architecture only.

### Node pool

The content pack defines a pool of available nodes. At run generation, the seed selects a subset to populate the run's tech tree. The pool grows as the game matures and content is added.

**Pool size and run node count are design parameters determined through playtesting** — they are directly tied to run length targets (GDD Q#10). The architecture must support arbitrary pool and run sizes; no counts are hardcoded.

Node fields (category, tier range, rarity, effects, unlock vectors, prerequisites, optional flag, exclusive group) are specified in [`tech-tree-design.md §5`](tech-tree-design.md#5-node-pool-design).

*Implementation: [`NodeDef`](../src/tech_tree/mod.rs#L52), [`NodeCategory`](../src/tech_tree/mod.rs#L19), [`NodeRarity`](../src/tech_tree/mod.rs#L29), [`NodeEffect`](../src/tech_tree/mod.rs#L46), [`TechTree`](../src/tech_tree/mod.rs#L64)*

### Prerequisite edges

Prerequisite relationships are **mostly fixed** — if node A requires node B and both are present, A always requires B. This preserves cross-run expertise (players learn the tree's stable structure).

**Alternative prerequisites** add per-run variation without chaos. A node defines a set of possible prerequisites beyond the primary; the run seed selects which (if any) are active. This creates alternative unlock paths — a node might be reachable via B in one run and via C or B+C in another. At run generation, the tech tree sub-seed determines which alternatives are active for each node.

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

*Implementation: [`UnlockVector`](../src/tech_tree/mod.rs#L37)*

### Tier gates

Each tier has an **unlock condition** that must be met before its shadow becomes visible and its nodes become researchable. Tier gate conditions are **data-driven** — each tier in the content pack specifies its gate type and parameters.

Gate condition types (extensible):
- Production milestone: produce N units of a given material
- Research threshold: accumulate N total research currency
- Exploration milestone: reach a tagged site type
- Node unlocked: a specific prerequisite node must be unlocked

Per-tier gate conditions are defined in [`tech-tree-design.md §3`](tech-tree-design.md#3-tier-structure).

### Player-visible shadow

Locked nodes display their shadow: `category`, `tier`, and `rarity`. Contents (effects, exact prerequisites, exact unlock vector) are hidden until unlocked. This gives players enough information to plan without removing discovery.

Nodes progress through three visibility states: **Shadow** (category, tier, rarity only) → **Partially Revealed** (broad parameters visible) → **Fully Revealed** (complete node data).

### Validity invariants

A generated tech tree must satisfy:
- Every node present in the run has at least one reachable unlock path given the run's other generated content
- No prerequisite cycles (enforced by generation ordering — nodes generated tier by tier, low to high)
- Every recipe required by the recipe graph has a corresponding tech tree node present in the run
- The tier count matches the run's difficulty tier

---

## 4. World & Terrain System

### Scale and units

**1 unit = 1 meter.** Machines are building-scale prefab structures. A basic machine occupies roughly a 5×5×5 meter footprint; high-tier machines are larger. Players and drones are small relative to machines and move through a world that feels physically proportioned.

### World extent

The world is **bounded** — a fixed-radius region generated at run start from the world seed. Bounded worlds avoid LOD complexity while providing ample space for factory expansion and exploration within a run.

World radius is a seeded parameter varying by difficulty. A standard run's world is large enough that players will not exhaust explorable terrain.

Vertically, the world has fixed layer extents seeded at run start. Layer boundaries vary per run within defined ranges:

| Layer | Elevation | Access |
|---|---|---|
| Orbital | Very high | Space drone |
| Sky / atmosphere | Above surface | Flying drone |
| Surface | Ground level | Starting layer |
| Underground | Below surface | Digger drone + tunnel graph |

Layer boundaries are legible in-world (atmospheric density changes, visual sky transitions, geological transitions underground).

### Core zone

Critical resources are guaranteed within a **core zone** around the spawn point. The world generator places all recipe-graph-required resource types within a bounded radius. Additional deposits may exist further out.

Core zone radius is a seeded parameter — some runs have compact resource geography, others are more spread out.

### Terrain system

The surface is a **heightmap-based mesh** rendered in chunks. No per-block terrain types — the surface is a continuous mesh with biome-driven appearance handled by terrain shaders, not a block texture atlas.

Terrain chunk size: **64×64 meters**. Chunks are loaded/unloaded based on player proximity. Each chunk is a mesh generated from heightmap data for that region, associated with a biome.

*Implementation: [`WorldConfig`](../src/world/generation.rs#L13), [`BiomeDef`](../src/content/mod.rs#L71), [`LayerDef`](../src/content/mod.rs#L63)*

Terrain generation uses **layered noise** derived from the world sub-seed:
- Base heightmap (elevation)
- Biome boundaries (lateral regions with distinct visual appearance)
- Resource deposit placement (constrained by biome + layer affinity rules)
- Point of interest placement (ruins, anomalies — seeded positions)

Biome appearance is handled by **terrain shaders**, not block type texture atlases. Different biomes use different surface materials and color variation.

### Deposits

Ore deposits are **persistent surface markers** — visible indicators on the terrain surface (similar to Satisfactory's resource nodes). Each deposit has a position, a seeded weighted ore blend, and a depletion state. Deposits do not spawn inside aegis field boundaries and are removed when aegis expansion covers their location.

**Discovery** is two-stage: drone scan or prospecting tool gives coarse data (resource category, approximate area); physical drone proximity reveals exact location, dominant ore type, and current yield.

**Manual mining** is the early-game extraction method. The player pilots a drone to a deposit and uses its mining tool to extract ore directly into inventory. This is quickly replaced by automatic miners.

**Automatic miners** are the primary extraction method. One miner per deposit. Once placed, the miner extracts continuously and feeds ore into the logistics network. Early miners output one ore per cycle, sampled probabilistically from the deposit's weighted distribution. Advanced miners output multiple items per cycle.

**Depletion** is tracked per deposit as total ore extracted. Yield degrades as extraction accumulates — the degradation curve shape is seeded per deposit for variety. Yield asymptotically approaches zero but never reaches it. Late-game **void miners** bypass depletion entirely, maintaining full yield regardless of extraction history. Miners can also augment base extraction rate.

A deposit produces a **weighted blend of ore items**: a copper-dominant deposit might yield 70% copper_ore, 20% tin_ore, 10% zinc_ore. This reflects mineral co-occurrence and creates incidental supply of secondary materials. Dominant ore type and secondary weights are seeded per deposit.

*Implementation: [`OreSpec`](../src/content/mod.rs), [`DepositDef`](../src/content/mod.rs), [`DepositRegistry`](../src/content/mod.rs)*

### Underground tunnel system

Underground access is provided via a **tunnel graph** — a logical graph of nodes and edges representing excavated passages.

When a player pilots a digger drone through subsurface terrain, passage creates tunnel nodes (positions) and edges (passages with a defined radius) along the path. The resulting tunnel is rendered as a mesh passage. The graph persists across sessions.

Underground deposits are placed at depth during world generation. Discovering them requires drone exploration; extracting them requires a logistics connection routed through the tunnel network.

### World generation sequence

Follows the backwards-from-terminal order established in §1:
1. Place critical resource deposits within core zone (surface and underground)
2. Generate surface heightmap and biome layout
3. Place points of interest (ruins, anomalies — seeded positions)
4. Apply planet modifier visual effects (atmospheric color, terrain character)

---

## 5. Machine System

### Overview

Machines are **building-scale prefab objects** placed in the world as whole units. Each machine type has a data file defining its tier variants, module attachment points, and IO port positions. No block-by-block assembly; no structure validation scan.

### Machine data format

Each machine type is defined in a data file (content pack). For each tier variant, the data file specifies: the 3D asset path, module slot positions and facing directions (in local machine space), and IO port positions, facing directions, and port kind (item in, item out, power in, fluid in, fluid out).

*Implementation: [`MachineDef`](../src/machine/mod.rs), [`MachineTierDef`](../src/machine/mod.rs), [`MachineRegistry`](../src/machine/mod.rs)*

### Placement

The player selects machine type and tier, then places it at a world position and orientation. Placement:
1. Spawn the machine prefab asset at chosen position + orientation
2. Create machine ECS entity with components
3. Register module attachment points and IO ports in world space

### Tier upgrades

Upgrading a machine:
1. Despawn current tier prefab
2. Spawn tier+1 prefab at same position and orientation
3. Transfer machine state (current recipe, inventory, module assignments where slot count allows, IO configuration)
4. Recompute IO port world positions from new model's data

Higher-tier models are visually more complex and impressive. Higher tiers provide more module slots.

### Module attachment

Modules are prefab objects that snap to attachment points defined in the machine data. When the player places a module near a machine's attachment zone, it snaps to the nearest available slot.

Modules carry functional tradeoffs (speed vs. efficiency, parallel slots, buffer capacity). Which modules exist in a run is a seed variance axis. Each slot records its index, world position, and the entity currently occupying it (if any).

### IO port configuration

Each machine has a set of IO ports (item in, item out, power in, fluid in/out). Port positions come from the machine data file but their **routing assignment is configurable** — the player assigns which logistics network channel carries which material via the machine's UI panel, not by physically placing hatch blocks.

Each port records: its ID, kind, world position, and the network channel assigned to it (if any).

### ECS structure

Each machine entity carries: machine type and tier, orientation, IO port positions (as grid coordinates for cable connectivity), inventory, recipe processor state (current recipe + progress), power consumer state (current demand), and module slots. World position and rotation are stored in Bevy's standard `Transform` component — machines are placed freely at any world position.

*Implementation: [`Machine`](../src/machine/mod.rs), [`MachineState`](../src/machine/mod.rs), [`Orientation`](../src/machine/mod.rs), [`Rotation`](../src/machine/mod.rs), [`Mirror`](../src/machine/mod.rs)*

---

## 6. Logistics Network

ECS structure, topology systems (place/remove/split/merge), routing, simulation systems, and messages: [`networks.md §1–2`](networks.md#1-generic-network-system). Job dispatch and crafting plan resolution: [`crafting.md §8`](crafting.md#8-craft-job-dispatch).

Key design choices: ME-style unified storage (network as index across all `StorageUnit`s; no personal inventory); machine auto-registration of capable recipes (no AE2-style physical pattern encoding); dependency-chain resolution at craft request time. See `gdd.md §10` for intent.

---

## 7. Power System

ECS components, topology systems, generator simulation, V×A checks, and execution order: [`networks.md §3–4`](networks.md#3-power-network).

Key design choices: buffer-based (generators fill per-generator internal buffer at rated wattage; recipes withdraw joules per tick via `take_energy`); V×A as two independent gates (voltage = qualitative tier gate, cannot overcome with more low-voltage; amperage = cable throughput cap per cable segment); all failure modes non-destructive (machines pause/block, never explode); planet modifier efficiency multipliers apply to generator output only (`0.4×`–`1.6×` depending on generator type and planet properties); inter-tier power routing requires a transformer machine. See `gdd.md` §Power for intent.

---

## 8. Drone System

ECS structure, Local↔Remote mode transition, fog-of-war reveal, sample collection, range scanning, multiple drone switching, events, edge cases, and execution order: [`drone.md`](drone.md).

VS scope: single land drone, F-key toggle, WASD piloting, ore sample collection, deposit discovery, fog reveal. See `gdd.md` §Drone for intent.

---

## 9. Science & Research System

ECS components, research pool, analysis station systems, experiment recipe model, knowledge visibility states (Known-to-exist → Partially-revealed → Fully-revealed), unlock flow, and edge cases: [`research.md`](research.md).

VS scope: Geological Analysis Station, research pool per type (Material Science, Field Research, Engineering, Discovery), experiment recipes, tech tree unlock integration. See `gdd.md` §Research for intent.

---

## 10. World Reactivity

### Per-region tracking

Reactivity is tracked **per region** — each biome area has its own reactivity score (0.0–1.0). Regions are not isolated; reactivity spreads slowly to adjacent regions, but the spread rate is low enough that players can maintain a high-reactivity industrial zone while keeping the broader world relatively clean.

Each region has a reactivity level and a seeded rate multiplier (how fast that region reacts). Some worlds react quickly and dramatically, others are resilient. Rate multipliers are per-region, so different biomes within the same run may react at different rates.

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

Reactivity **decreases when sources are removed or reduced**. Each tick, buildup equals the sum of active sources scaled by the region's rate seed and elapsed time. Recovery applies when sources are below the current level, at a rate intentionally faster than typical buildup — clean play is rewarded. The level is clamped to [0, 1].

`RECOVERY_RATE` is tuned to be meaningfully faster than typical buildup — exact ratio is a balance parameter. Threshold events do not reverse when reactivity drops below their threshold (the ecosystem shift happened; reducing reactivity prevents further damage but doesn't undo past changes).

---

## 11. Codex & Meta-Progression

### Save file architecture

Save scopes, RON format, `moonshine-save` details, and entity tagging rules: [`technical/README.md §Save Architecture`](README.md#save-architecture).

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

Each entry records its type, first-seen timestamp, and accumulated observations across runs. The meta save stores the full codex alongside unlocked content, blueprints, and the starting boons pool.

### Meta-progression unlocks

Unlocks expand what runs can be — not how easy they are.

**Unlock triggers:**
- **Run completion** — primary trigger. Completing a run at a given difficulty tier unlocks new content at that tier and above.
- **In-run milestones** — some unlocks trigger mid-run on specific achievements (e.g. first flying drone built, first Pinnacle-tier machine constructed). These fire immediately and persist to meta save.

Each unlock definition records: its ID, trigger condition (run completion at difficulty, or milestone reached), and what it grants. Grant types: a new biome type, a run modifier, narrative content, a blueprint slot, or a starting boon.

### Blueprints

Blueprints are saved to the **meta save** and persist across runs. A blueprint captures a sub-factory layout (machine types, tiers, relative positions, orientations, and logistics connections) as a placeable template.

Blueprints are **templates, not solutions** — they encode layout only, not recipe parameters or machine configuration. A blueprint for a smelting array still requires the player to configure it for this run's specific ratios and machine parameters. They save placement time, not thinking.

Blueprint slots are finite and expand through meta-progression unlocks.

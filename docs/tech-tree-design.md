# Tech Tree Design

> **Status:** First draft. Pacing targets unvalidated — requires playtesting. Node definitions are representative, not final. See `gdd.md §7` for design intent; this document is the content design layer.

---

## Table of Contents

1. [Design Goals](#1-design-goals)
2. [Unlock Rate Targets](#2-unlock-rate-targets)
3. [Tier Structure](#3-tier-structure)
4. [Categories](#4-categories)
5. [Node Pool Design](#5-node-pool-design)
6. [Tier 1 — Foundation](#6-tier-1--foundation)
7. [Open Questions](#7-open-questions)

---

## 1. Design Goals

- **Choice, not wait.** Research is plentiful enough to always unlock *something*. The constraint is deciding which of several visible options to take, not waiting for currency to accumulate. Scarcity comes from unlock conditions (milestones, exploration, exotic science gates) tightening across tiers — not from rate slowdown alone.
- **Constant momentum.** Players should feel forward motion throughout. Early tiers unlock fast; later tiers slow down but each unlock is a milestone. The gradient from fast → slow should feel like deepening, not grinding.
- **Lots of options.** At any moment, 3–5 nodes should be unlockable. Players build their own ordering within a tier, not a fixed sequence. Different orderings produce meaningfully different early factories.
- **Shadow legibility.** Locked nodes show category, tier, and rarity. Players can plan toward things they haven't revealed yet.

---

## 2. Unlock Rate Targets

### All tiers

Rate grows near-linearly (+2–3 min per tier). Node count grows with tier; terminal tiers dip ~25% from the prior non-terminal tier because escape construction fills the remaining time. **Bold** = terminal tier for that difficulty.

| Tier | Rate | Nodes | Duration | Notes |
|---|---|---|---|---|
| T1 Landfall | 1 / 6 min | 8 | ~0.8h | |
| T2 Foothold | 1 / 8 min | 12 | ~1.6h | |
| T3 Inheritance | 1 / 10 min | 16 | ~2.7h | **Initiation terminal** |
| T4 Ascent | 1 / 12 min | 20 | ~4.0h | |
| T5 Scion | 1 / 14 min | 15 | ~3.5h | **Standard terminal** (−25% vs T4) |
| T6 Traverse | 1 / 17 min | 22 | ~6.2h | |
| T7 Propagation | 1 / 20 min | 16 | ~5.3h | **Advanced terminal** (−27% vs T6) |
| T8 Breakthrough | 1 / 23 min | 24 | ~9.2h | |
| T9 Forge | 1 / 26 min | 26 | ~11.3h | |
| T10 Transcendence | 1 / 30 min | 20 | ~10.0h | **Pinnacle terminal** (−23% vs T9) |

### By difficulty

| Difficulty | Tiers | Nodes | Duration |
|---|---|---|---|
| Initiation | 1–3 | ~36 | ~5h |
| Standard | 1–5 | ~71 | ~13h |
| Advanced | 1–7 | ~109 | ~24h |
| Pinnacle | 1–10 | ~179 | ~55h |

### Scarcity gradient

Scarcity comes from **resource cost and unlock conditions**, not from rate. Even at T9 (26 min/unlock), there are always 3–5 affordable nodes visible — the constraint is deciding which to fund, not waiting for the next unlock.

- **T1–T2:** Research plentiful. Rate fast; challenge is choosing among 4–5 visible nodes.
- **T3–T5:** Research tightens. Unlocks increasingly require production milestones or exploration — research-spend alone insufficient for most nodes. More nodes visible than affordable.
- **T6+:** Each unlock requires deliberate investment. Production milestones, exploration triggers, and exotic science gates dominate. Research-spend alone very expensive. 3–4 nodes visible, 1–2 affordable at a time.

Rate growth alone does not create scarcity — the unlock condition does. A faster rate with harder conditions feels more active and less like waiting, which serves the "Choice, not wait" design pillar.

---

## 3. Tier Structure

The tech tree follows a canonical 10-tier sequence. Each difficulty uses a prefix. Tiers 3, 5, and 7 have terminal and intermediary variants — see GDD §7 and §12 for full design intent.

| Difficulty | Tiers | Unlocked by |
|---|---|---|
| Initiation | 1–3 | Available from start |
| Standard | 1–5 | Complete an Initiation run |
| Advanced | 1–7 | Complete a Standard run |
| Pinnacle | 1–10 | Complete an Advanced run |

### Tier summary

| # | Tier name | Terminal for | Exit gate (completes tier → opens next) |
|---|---|---|---|
| 1 | Landfall | — | **TBD** *(theme anchor: produce 100 refined base units)* |
| 2 | Foothold | — | **TBD** |
| 3 | Inheritance | **Initiation** | *Terminal (Initiation):* **escape** — launch 1 minimal successor (§12). *Non-terminal:* **TBD** |
| 4 | Ascent | — | **TBD** *(theme anchor: first orbital flight)* |
| 5 | Scion | **Standard** | *Terminal (Standard):* **escape** — fuller successor + provisioning (§12). *Non-terminal:* **TBD** |
| 6 | Traverse | — | **TBD** *(theme anchor: reach outer-system zone; terraforming provisioning comes online)* |
| 7 | Propagation | **Advanced** | *Terminal (Advanced):* **escape** — commission a replication line (§12). *Non-terminal:* **TBD** |
| 8 | Breakthrough | — | **TBD** *(theme anchor: synthesize first transcendent matter)* |
| 9 | Forge | — | **TBD** *(theme anchor: stand up the replication forge + forge-grade power)* |
| 10 | Transcendence | **Pinnacle** | *Terminal (Pinnacle):* **escape** — self-expanding forge / swarm seed (§12) |

*Gate = tier **exit**: the requirement that completes a tier and opens the next. A difficulty's **terminal** tier exits via the **escape** (§12), not into a next tier. The run begins at **landing** (pod + starting kit) — a pre-tier start state, not a gated tier; the player starts inside Tier 1 (Landfall), which has its own nodes (§6). **All non-terminal exit gates are TBD pending tech-tree progression design** (§7 Q#2); the parenthetical theme anchors are provisional intent, not settled gates. Only the terminal escape exits are fixed.*

**Tiers 3, 5, and 7** each have two precursor-remnant variants. *Terminal variant:* the tier is a difficulty's terminal, and the seeded precursor (if present) discounts that run's successor launch (§12; a frontier run scratch-builds instead — the escape *type* is not locked to difficulty). *Intermediary variant:* a non-launch remnant is present (relic, cache, archive) — value is what it teaches or produces, not escape. Each run draws a different precursor remnant, so the terminal act reads differently run to run.

### Tier themes

Each tier has a thematic identity that drives which categories dominate and what the factory looks like at that stage.

> The **Gate** lines below are *provisional exit-gate intent*, not settled — every non-terminal exit gate is TBD pending progression design (§7 Q#2). Only the terminal **escape** gates (§12) are fixed. See the §3 summary-table note for the exit-gate semantics.

#### Tier 1 — Landfall
*Survive. First power. Basic material loop.*

- 1–2 base materials (surface deposits only)
- Basic extraction, smelting, forming
- First power source (planet-dependent — combustion or solar)
- Land drone: surface scouting begins
- Basic logistics: first network nodes, manual storage
- Basic science: field analyzer unlocked, research loop established
- **Base/exotic ratio:** 100% base
- **Gate:** Analyze first exotic sample + deploy surface drone

#### Tier 2 — Foothold
*Push outward. First exotic science. Power begins to strain.*

- 1–2 more base materials + first exotic material (shallow, seeded)
- First multi-step processing chains (ore → crushed → dust → ingot)
- Optional second-domain access via amphibious or digger-capable drone, if the run's resource graph needs a water or underground site
- Network design pressure begins: channel limits first felt
- Power efficiency issues emerge; second source type needed
- **Base/exotic ratio:** ~70% base, 30% exotic
- **Gate:** Produce 100 units of any refined base material (ingot or plate form)

#### Tier 3 — Inheritance *(Initiation terminal)*
*First precursor structure. Gateway or ruin.*

- 2–3 exotic materials; base materials begin feeding exotic chains
- Processing conditions appear: temperature, pressure, catalysts
- Flight-capable drone *(post-MVP; T3 Initiation escape does not require atmospheric-domain content)*
- Sub-network segmentation becomes economical
- **Base/exotic ratio:** ~40% base, 60% exotic
- **Terraforming:** optional beneficial-vent opportunity appears (reward: cheaper exotic inputs); not required at this scale
- **Gate (terminal):** Fabricate + launch 1 minimal successor — a seeded precursor gateway/ruin discounts the launch step; a frontier run scratch-builds it
- **Gate (intermediary):** Activate precursor ruin/cache → unlocks exotic material or machine type

#### Tier 4 — Ascent
*Spacecraft. Orbital access.*

- Deeper exotic chains; byproduct routing creates graph interconnections
- Space-capable drone or spacecraft prerequisites unlock
- Orbital/space sites become accessible when the run's objective uses them
- Power Tier 2+ required; first major renegotiation
- **Base/exotic ratio:** ~30% base, 70% exotic
- **Gate:** Achieve first orbital flight

#### Tier 5 — Scion *(Standard terminal)*
*Terminal: fuller successor + provisioning — fabricate and launch. Intermediary: precursor fabrication relic — extract and learn.*

- Final base-material chains complete; exotic science dominates
- Orbital fabrication prerequisites
- Network requires sub-network architecture
- **Base/exotic ratio:** ~20% base, 80% exotic
- **Terraforming:** optional; beneficial streams now cheapen provisioning inputs
- **Gate (terminal):** Fabricate successor + provisioning module + exotic fuel; launch (a seeded precursor derelict, if present, discounts the hull/body step)
- **Gate (intermediary):** Locate precursor fabrication relic → extract fabrication data → unlocks exotic drive tech or fabrication machines

#### Tier 6 — Traverse
*Outer-system reach.*

- Deep exotic science chains; transcendent-matter precursors emerge
- Outer-system capable spacecraft prerequisites
- Power Tier 3+ required
- **Base/exotic ratio:** ~10% base, 90% exotic
- **Terraforming:** provisioning throughput now soft-required — Advanced-scale successors need terraform-products as sustained launch inputs (§12)
- **Gate:** Reach outer-system zone

#### Tier 7 — Propagation *(Advanced terminal)*
*Precursor megastructure. Operate or study.*

- Full exotic science graph visible; replication-line chains begin
- **Base/exotic ratio:** ~5% base, 95% exotic
- **Terraforming:** required for line provisioning (sustained terraform-product feed)
- **Gate (terminal):** Commission a replication line — sustained successor output fed by terraformed throughput (a seeded precursor relay, if present, discounts the range/boost step)
- **Gate (intermediary):** Access precursor repository, extract FTL theory → unlocks transcendent-matter synthesis routes

#### Tier 8 — Breakthrough
*FTL theory applied. Transcendent-matter chains.*

- Transcendent-matter synthesis unlocked via T7 knowledge
- New machine types for transcendent-matter processing
- **Gate:** Synthesize first transcendent matter

#### Tier 9 — Forge
*Stand up the replication forge. Final power renegotiation.*

- All successor-system chains available; the forge integrates them
- Forge-grade power infrastructure required
- **Terraforming:** high sustained terraform-product throughput required to feed the forge
- **Gate:** Stand up the replication forge + sustain forge-grade power

#### Tier 10 — Transcendence *(Pinnacle terminal)*
*Build the self-expanding forge. Seed the swarm.*

- Four major successor-system tracks: engines, FTL drive, reactor, shielding
- Each track requires a deep independent production chain, fully self-fabricated (always-frontier — no precursor discount)
- **Gate (terminal):** Construct all four successor systems + provision the forge + fire the swarm-seed cascade

*(Tiers 8–10 not yet fully designed — nail Standard first.)*

---

## 4. Categories

Eight categories. Each node belongs to exactly one.

| Category | Role | Primary tiers |
|---|---|---|
| **Extraction** | Getting raw resources from deposits, sites, and resource domains | 1–3 |
| **Smelting & Forming** | Base material transformation: ore → ingot → plate → wire | 1–3 |
| **Processing** | Chemical, thermal, pressure reactions; deep chains | 3–7 |
| **Power** | Generation, storage, distribution — renegotiated each tier | 1–10 |
| **Logistics** | Network cables, controllers, auto-crafting, storage | 1–5 |
| **Science** | Analyzers, labs, sample processors | 1–5 |
| **Exploration** | Drone types, scanners, aegis expanders | 1–7 |
| **Fabrication** | Assembly machines for composite and exotic items | 3–10 |

**Power** is the only category that spans all tiers with equal importance. Every other category has a peak window.

### Node pool allocation

Total pool target: ~215 nodes (Standard run selects ~71; pool provides ~3× for seeded variance).

*Category breakdown below is proportionally scaled from the original split — not yet redesigned per tier. Treat as first-pass targets pending per-tier node design.*

| Category | Pool size | Per-run (Standard) |
|---|---|---|
| Extraction | 18 | 6 |
| Smelting & Forming | 24 | 8 |
| Processing | 46 | 14 |
| Power | 32 | 10 |
| Logistics | 24 | 8 |
| Science | 21 | 6 |
| Exploration | 21 | 8 |
| Fabrication | 28 | 10 |
| **Total** | **~214** | **~70** |

### Rarity distribution

| Rarity | Pool share | Present in runs |
|---|---|---|
| Common | 50% | Most runs |
| Uncommon | 30% | ~half of runs |
| Rare | 15% | ~1 in 4 runs |
| Unique | 5% | 0–1 per run |

Power skews Common at the low end (every run needs generators) and has Rare/Unique generators that define run strategies when present. Exploration skews Common at Tier 1, Rare at Tier 3–4.

---

## 5. Node Pool Design

### What a node contains

Every node in the pool defines:

- **Category** — one of the eight above
- **Tier range** — tiers this node can appear in (e.g. `[2, 3]`)
- **Rarity** — Common / Uncommon / Rare / Unique
- **Effect** — what unlocking grants: a material, a machine type, a special recipe, or a capability
- **Primary unlock vector** — the node's characteristic unlock method
- **Alternative unlock pool** — additional methods the run seed may activate
- **Primary prerequisite** — the node *or exclusive group* that must be satisfied before this one (if any); for a group, any member unlocked satisfies it
- **Alternative prerequisites** — pool the seed draws additional prerequisite edges from
- **Optional** — boolean; if true, this node comes from the optional pool, does not count toward tier completion, and can only serve as a prerequisite for other optional nodes
- **Exclusive group** — optional identifier; nodes sharing a group are mutually exclusive — unlocking one permanently disables the rest; group spawn behavior (always together vs. conditional) is configured per group

### `TechTreeNode` ECS component

```rust
pub struct TechTreeNode {
    pub id: NodeId,
    pub category: NodeCategory,
    pub tier: u8,
    pub rarity: NodeRarity,
    pub effects: Vec<NodeEffect>,
    pub primary_vector: UnlockVector,
    pub alt_vectors: Vec<UnlockVector>,
    pub primary_prereq: Option<Prereq>,
    pub alt_prereqs: Vec<Prereq>,
    pub optional: bool,
    pub exclusive_group: Option<ExclusiveGroupId>,
}

pub enum Prereq {
    Node(NodeId),
    Group(ExclusiveGroupId),  // any member unlocked satisfies
}
```

### Node types (from GDD §7)

**Material nodes** — unlock a material and all its derived items + concrete recipes for already-known machines.

**Machine/Process nodes** — unlock a machine type or tier; all recipe templates using that machine (and tier) become available for all known materials.

**Special recipe nodes** — unlock one specific recipe not arising from template expansion.

### Optional nodes

Optional nodes draw from a separate pool and a separate seeded selection pass — they are not counted in tier node totals or per-run required counts. A player who ignores every optional node still completes every tier on schedule.

Rules:
- Can have prerequisites (regular or optional); can only serve as prerequisites for other optional nodes
- Same visibility rules as regular nodes: appear in shadow UI, same reveal mechanics
- Can have any rarity; Rare/Unique optionals are per-run finds worth going out of the way for

Design uses: efficiency upgrades, niche capabilities for specific run seeds, quality-of-life unlocks.

### Choice nodes

A choice node belongs to an **exclusive group**. When the player unlocks any member, all other members in the group are permanently disabled for the run. Disabled members remain visible in the tree with a distinct locked-out state so the tradeoff is legible.

Rules:
- **Always proactive.** The choice is surfaced explicitly when any group member first becomes unlockable. The player must acknowledge the group and select. No node in a group can be unlocked without the player seeing the full set of options first — lockouts are never a surprise.
- **No trap choices.** Every member must be a valid, complete path. Groups should not pair a Common node against a Rare — power asymmetry turns a choice into a non-choice.
- **Group as prerequisite.** A required node can list an exclusive group as its prerequisite; any member's unlock satisfies it. Individual choice nodes can only be prerequisites for optional nodes.
- **Spawn behavior is per-group config.** A group may always spawn all members together, or spawn conditionally based on run seed. Either way, all present members appear as a set.
- **Size:** arbitrary; in practice 2–3 members.

Design uses: architectural forks (two legitimate approaches to the same problem), precursor-contact path splits, risk/reward divergences at the same tier slot.

---

## 6. Tier 1 — Foundation

### Design intent

Tier 1 should feel like a fast-moving setup phase. The player is landing, building their first machines, and establishing the research loop. Unlocks come quickly — roughly one every 6 minutes — and there are always 3–5 unlockable nodes visible at once.

The player's choices in Tier 1 determine their early factory shape: power-first vs. production-first vs. science-first. All orderings lead to the same Tier 2 gate, but they produce meaningfully different infrastructure going in.

### Starting state

The player lands with a **starting kit** dropped by the escape pod — a miner, a solar generator, an assembler, an analysis station, and logistics/power cables — but no raw materials (see `pod::starting_kit`). This kit *is* the bootstrap: the player sets up the first base immediately rather than earning it.

The **first source of research** is the analysis station: place the solar generator and the miner on the spawn deposit, feed the mined stone into the analysis station's `basic_analysis` recipe, and it produces Material Science. That research unlocks 1–2 "Research spend (small)" nodes — typically Stone Furnace or a power source — and the standard loop begins: mine at scale → analyze → spend research on remaining nodes.

The origin deposit is guaranteed (always within the Aegis radius and always stone-bearing), and the starting machines are non-consumable, so the opening can never brick for lack of the right parts or a mineable deposit.

### Tier 1 node set (7 nodes, representative)

These are the 7 nodes that appear in *every* Standard run (all Common, Tier 1 only). They form the fixed skeleton; seeded nodes extend this in higher tiers.

> **Ore Crusher moved to Tier 2** (`standard-run-design.md §3.1` staggered forming ladder, §5 T2 note). T1 is now **direct-smelt-only** — the simplest `ore → ingot` chain. The crusher (and the `ore → crushed → ingot` +yield chain with its **gravel** byproduct) is the first *optional* forming deepening, unlocked at T2 by a production milestone (100 refined units). Authored in RON as the `ore_crusher` T2 node (`assets/tech_nodes/ore_crusher.ron`).

| Node | Category | Effect | Primary unlock | Prerequisite |
|---|---|---|---|---|
| **Stone Furnace** | Smelting & Forming | Unlocks `smelter` machine type Tier 1; enables all ore→ingot recipes for known materials | Research spend (small) | — |
| **Basic Miner** | Extraction | Unlocks `miner` machine type Tier 1; automatic ore extraction from surface deposits | Production milestone: 50 stone | — |
| **Combustion Generator** | Power | Unlocks `combustion_generator` machine type; burns fuel items for power | Research spend (small) | Stone Furnace |
| **Solar Array** | Power | Unlocks `solar_array` machine type; passive power scaled by planet solar modifier | Research spend (small) | — |
| **Field Analyzer** | Science | Unlocks `analyzer` machine type Tier 1; enables sample analysis → research currency | Research spend (small) | — |
| **Basic Network Node** | Logistics | Unlocks `network_node` Tier 1 cable + basic storage node; enables logistics network | Research spend (small) | Stone Furnace |
| **Land Drone Mk1** | Exploration | Unlocks `land_drone` Tier 1; enables surface scouting and sample collection beyond the aegis field | Production milestone: 20 iron ingots | Basic Miner |

> ⚠️ Both power nodes (Combustion Generator, Solar Array) are in every run. The planet modifier determines which is worth building — a low-solar world pushes players to combustion; a geologically inert world pushes solar. Both exist so the player always has a choice, not a dictated answer.

### Tier 1 unlock ordering examples

**Power-first player:**
Solar Array → Stone Furnace → Basic Miner → Field Analyzer → Basic Network Node → Combustion Generator → Land Drone Mk1

**Production-first player:**
Stone Furnace → Basic Miner → Basic Network Node → Field Analyzer → Combustion Generator → Solar Array → Land Drone Mk1

**Science-first player:**
Field Analyzer → Stone Furnace → Combustion Generator → Basic Miner → Basic Network Node → Land Drone Mk1 → Solar Array

All three hit the Tier 2 gate at roughly the same clock time. The difference is which infrastructure is built out first. The Ore Crusher (now the first T2 forming deepening) is the natural next unlock once the direct-smelt loop has produced its first 100 refined units.

### Tier 1 → 2 gate

**Condition (provisional — TBD):** Produce 100 units of any refined base material (ingot or plate form). This is Tier 1's *exit* gate (see §3 semantics), but the exit-gate set is TBD pending full progression design (§7 Q#1–2); it also differs from the T1 theme's `Gate:` line ("analyze first exotic sample + deploy surface drone") — one is the tier exit, the other an early intra-tier objective, to be reconciled.

If adopted, the broad condition is intentional — the gate should accommodate whichever base material the run's seed placed in the core zone, not assume a specific one.

---

## 6bis. Tiers 3–5 — RON node set *(Phase D, authored)*

The full T3–T5 design node tables live in [`standard-run-design.md §5`](standard-run-design.md#5-tier-by-tier-progression). This section records the **nodes actually authored in RON** (`assets/tech_nodes/`) for the fixed Standard run — the reachable spine that makes `assets path launch_successor` resolve. It builds on the existing T1–T2 nodes; T3+ nodes carry `tier: 3/4/5` and take T2 nodes (`exotic_materials`, `resonite_engineering`, `ore_crusher`, `drone_recon`, `advanced_processing`) as prerequisites.

**Category mapping** (engine `NodeCategory` has no Extraction/Fabrication variants): design *Extraction* → `Exploration`; *Smelting & Forming* and *Fabrication* → `Processing`; *Science (exotic)* → `Science`.

**Research-theme earn order (soft-lock-free).** Material (turn 0) → Engineering (circuits, T2) → **Discovery** (from `analyze_field_sample`, unlocked by `drone_recon`, fed by `field_sample` collected at the xalite site) → **Synthesis** (from `analyze_exotic_reaction`, unlocked by `synthesis_lab` which is *Discovery*-gated, fed by `resonite_shard`). Each theme's generator is unlocked by a node that does **not** cost that theme, so no currency gates its own source.

### Tier 3 — Inheritance (RON)

| Node | Category | Unlock vector | Prereq | Effect (recipes/machines) |
|---|---|---|---|---|
| `steel_alloying` | Processing | ResearchSpend material 150 | basic_smelting | `alloy_steel` |
| `aluminum_extraction` | Exploration | ResearchSpend material 150 | ore_extraction | `crush_aluminum`, `smelt_aluminum_crushed` |
| `ore_washer` | Processing | ProductionMilestone iron_crushed 50 | ore_crusher | `wash_iron/copper/tin/aluminum`, `scrub_slag` |
| `exotic_processing` | Processing | ResearchSpend discovery 150 | exotic_materials | `form_resonite_lattice` |

### Tier 4 — Ascent (RON)

| Node | Category | Unlock vector | Prereq | Effect |
|---|---|---|---|---|
| `plate_roller` | Processing | ProductionMilestone iron_ingot 150 | ore_washer | machine `plate_roller`; `roll_iron_plate`, `roll_aluminum_plate` |
| `titanium_forming` | Processing | ResearchSpend material 200 | plate_roller | `roll_titanium_plate` |
| `precursor_survey` | Exploration | ResearchSpend discovery 120 | drone_recon | — (survey gate; enables Fluxite/Space discovery) |
| `fluxite_studies` | Science | ExplorationDiscovery `fluxite_relic_cache` | precursor_survey | `refine_fluxite` |
| `synthesis_lab` | Science | ResearchSpend discovery 200 | exotic_processing | `analyze_exotic_reaction` (Synthesis generator) |
| `vitreite_synthesis` | Processing | ResearchSpend synthesis 150 | exotic_processing, fluxite_studies | `synth_vitreite` |
| `coolant_reclaim` | Processing | ResearchSpend synthesis 120 | vitreite_synthesis | `reclaim_coolant` |
| `fluxite_generator` | Power | ResearchSpend engineering 300 | fluxite_studies, advanced_processing | machine `fluxite_generator`; `make_fluxite_generator`, `generate_fluxite` |
| `fluxite_coil` | Processing | ResearchSpend engineering 250 | fluxite_studies | `make_fluxite_coil` |
| `advanced_assembler` | Processing | ResearchSpend engineering 400 | resonite_engineering | machine `advanced_assembler`; `make_advanced_assembler` |
| `space_scanner` | Exploration | ResearchSpend discovery 300 | precursor_survey | — (second-site + derelict gate) |

### Tier 5 — Scion / Standard terminal (RON)

| Node | Category | Unlock vector | Prereq | Effect |
|---|---|---|---|---|
| `cryophase_prospecting` | Exploration | ResearchSpend discovery 300 | space_scanner | — (surfaces cryogenic-signature rumor) |
| `cryophase_extraction` | Exploration | ExplorationDiscovery `cryophase_deposit` | cryophase_prospecting | — (second-site; enables mining cryophase) |
| `exotic_fuel_refining` | Processing | ResearchSpend synthesis 250 | cryophase_extraction, coolant_reclaim | `refine_exotic_fuel`, `refine_exotic_fuel__raw` |
| `derelict_salvage` | Exploration | ExplorationDiscovery `derelict_ship` | space_scanner | `make_successor_chassis__salvaged` |
| `successor_core` | Processing | ResearchSpend synthesis 300 | advanced_assembler | `make_successor_core` |
| `successor_chassis` | Processing | ResearchSpend synthesis 300 | advanced_assembler, vitreite_synthesis | `make_successor_chassis` |
| `successor_drive` | Processing | ResearchSpend synthesis 300 | fluxite_coil, titanium_forming | `make_successor_drive` |
| `successor_sensor` | Processing | ResearchSpend synthesis 250 | successor_core, exotic_processing | `make_successor_sensor` |
| `provisioning_module` | Processing | ResearchSpend engineering 350 | advanced_assembler | `make_miner_kit`, `make_generator_kit`, `make_assembler_kit`, `make_provisioning_module` |
| `launch_site_assembly` | Processing | ResearchSpend synthesis 400 | successor_core, chassis, drive, sensor | machine `launch_site`; `make_launch_site` |
| `launch_successor` | **Escape** | ResearchSpend synthesis 500 | launch_site_assembly, synthesis_lab, provisioning_module, exotic_fuel_refining | `launch_successor` (the escape cascade) |

**Deviations from design §5 (flagged).**
- **Optional nodes deferred.** The §5 tables list optional yield/insurance nodes (Gravel Sink exists; Digger Drone, Efficiency Module I/II, Reinforced Scaffold, Fluxite Capacitor, Terraform Router/Provisioning, Deep Survey, Redundant Core, Field Lab, Sustained Power Array, Fuel Depot) that are **not** on the launch spine. They are omitted from RON for now (representative fixed run) and remain design-only.
- **Base-metal extraction folded.** The `smelt_metal` template (unlocked at `basic_smelting`) already yields every metal ingot, so separate Titanium/Aluminum "Extraction" gates have little to unlock; aluminum's crush/wash forms hang off `aluminum_extraction`/`ore_washer`, titanium's plate off `titanium_forming`. Design's Steel Alloying / Ore Washer / Plate Roller also **adopt previously-orphaned Phase B recipes** (`alloy_steel`, `wash_*`, `scrub_slag`, `roll_iron_plate`), wiring them to real gates.
- **Launch gate.** See [`technical/escape-condition.md §7.1`](technical/escape-condition.md#71-standard-escape-successor-launch) for the ProductionMilestone("all 4 systems") → ResearchSpend+recipe-inputs modeling.

---

## 7. Open Questions

| # | Question | Priority |
|---|---|---|
| 1 | Exact Tier 1→2 gate quantity — 100 units too easy/hard? Needs playtesting. | High |
| 2 | T2–T5 node set — not designed yet. Nail T1 feel first. | High |
| 3 | Are 8 fixed Tier 1 nodes the right number, or should some T1 nodes also be seeded? Fewer fixed nodes = more run variance but less reliable onboarding. | Medium |
| 4 | "Smelting & Forming" vs "Processing" — may be confusing distinction for players in the shadow UI. Consider merging into "Material Processing" for display while keeping internal split. | Medium |
| 5 | Exotic material count per Standard run — targeting 3–4. Drives pool selection logic but not yet specified. | Medium |
| 6 | Tier 5/6 themes for Advanced/Pinnacle — deferred until Standard is validated. | Low |
| 7 | Are Combustion Generator and Solar Array (T1) a candidate exclusive group? Currently both always spawn; making them a choice would create a harder power commitment early. | Medium |
| 8 | Optional pool sizing — how many optional nodes per tier? No targets set. | Medium |
| 9 | UI treatment for proactive choice surfacing — modal / sidebar overlay / tree highlight? Needs UX spec. | Medium |

---

*Tech Tree Design v0.2 — T1 (Landfall) designed. T2–T5 pending. T6–T10 deferred. See `gdd.md §7` for design constraints and `technical-design.md §3` for data model.*

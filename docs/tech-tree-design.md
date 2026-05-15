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

- **Choice, not wait.** Research is plentiful enough to always unlock *something*. The constraint is deciding which of several visible options to take, not waiting for currency to accumulate. Scarcity comes from unlock conditions (milestones, exploration, alien science gates) tightening across tiers — not from rate slowdown alone.
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
| T2 Roots | 1 / 8 min | 12 | ~1.6h | |
| T3 Contact | 1 / 10 min | 16 | ~2.7h | **Initiation terminal** |
| T4 Reach | 1 / 12 min | 20 | ~4.0h | |
| T5 Salvage | 1 / 14 min | 15 | ~3.5h | **Standard terminal** (−25% vs T4) |
| T6 Traverse | 1 / 17 min | 22 | ~6.2h | |
| T7 Interface | 1 / 20 min | 16 | ~5.3h | **Advanced terminal** (−27% vs T6) |
| T8 Revelation | 1 / 23 min | 24 | ~9.2h | |
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
- **T6+:** Each unlock requires deliberate investment. Production milestones, exploration triggers, and alien science gates dominate. Research-spend alone very expensive. 3–4 nodes visible, 1–2 affordable at a time.

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

| # | Tier name | Terminal for | Gate condition |
|---|---|---|---|
| 1 | Landfall | — | — (always available) |
| 2 | Roots | — | Produce 100 units of any refined base material |
| 3 | Contact | **Initiation** | Activate alien structure (terminal: gateway; intermediary: ruin/cache unlocking alien material or machine) |
| 4 | Reach | — | Achieve first orbital flight |
| 5 | Salvage | **Standard** | Interact with alien structure (terminal: repair alien vessel + launch; intermediary: access alien fabrication probe, extract fabrication data) |
| 6 | Traverse | — | Reach outer-system zone |
| 7 | Interface | **Advanced** | Interact with alien megastructure (terminal: operate relay; intermediary: extract FTL theory fragments from alien archive) |
| 8 | Revelation | — | Synthesize first exotic material |
| 9 | Forge | — | Produce all FTL drive component types + sustain FTL-grade power |
| 10 | Transcendence | **Pinnacle** | — (escape condition is the terminal) |

*Gate condition = requirement that completes a tier and opens the next. Exit of tier T = entry of tier T+1. T1 has no entry gate; it is always available.*

**Tiers 3, 5, and 7** each have two artifact variants. Terminal variant: the alien structure is the escape objective. Intermediary variant: a different artifact class is present (probe, cache, archive) — intact but not usable for escape, value is what it teaches or produces. This preserves immersion: each run has a different precursor remnant suited to its difficulty.

### Tier themes

Each tier has a thematic identity that drives which categories dominate and what the factory looks like at that stage.

#### Tier 1 — Landfall
*Survive. First power. Basic material loop.*

- 1–2 base materials (surface deposits only)
- Basic extraction, smelting, forming
- First power source (planet-dependent — combustion or solar)
- Land drone: surface scouting begins
- Basic logistics: first network nodes, manual storage
- Basic science: field analyzer unlocked, research loop established
- **Base/alien ratio:** 100% base
- **Gate:** Analyze first alien sample + deploy surface drone

#### Tier 2 — Roots
*Push outward. First alien science. Power begins to strain.*

- 1–2 more base materials + first alien material (shallow, seeded)
- First multi-step processing chains (ore → crushed → dust → ingot)
- Optional second-domain access via amphibious or digger-capable drone, if the run's resource graph needs a water or underground site
- Network design pressure begins: channel limits first felt
- Power efficiency issues emerge; second source type needed
- **Base/alien ratio:** ~70% base, 30% alien
- **Gate:** Produce 100 units of any refined base material (ingot or plate form)

#### Tier 3 — Contact *(Initiation terminal)*
*First alien structure. Gateway or ruin.*

- 2–3 alien materials; base materials begin feeding alien chains
- Processing conditions appear: temperature, pressure, catalysts
- Flight-capable drone *(post-MVP; T3 Initiation escape does not require atmospheric-domain content)*
- Sub-network segmentation becomes economical
- **Base/alien ratio:** ~40% base, 60% alien
- **Gate (terminal):** Construct activation key + sustain gateway power + activate
- **Gate (intermediary):** Activate alien ruin/cache → unlocks alien material or machine type

#### Tier 4 — Reach
*Spacecraft. Orbital access.*

- Deeper alien chains; byproduct routing creates graph interconnections
- Space-capable drone or spacecraft prerequisites unlock
- Orbital/space sites become accessible when the run's objective uses them
- Power Tier 2+ required; first major renegotiation
- **Base/alien ratio:** ~30% base, 70% alien
- **Gate:** Achieve first orbital flight

#### Tier 5 — Salvage *(Standard terminal)*
*Terminal: alien vessel — repair and launch. Intermediary: alien fabrication probe — extract and learn.*

- Final base-material chains complete; alien science dominates
- Orbital fabrication prerequisites
- Network requires sub-network architecture
- **Base/alien ratio:** ~20% base, 80% alien
- **Gate (terminal):** Construct ship systems (hull, nav, engines, life support) + produce alien fuel + launch
- **Gate (intermediary):** Locate alien fabrication probe → extract fabrication data → unlocks alien drive tech or fabrication machines

#### Tier 6 — Traverse
*Outer-system reach.*

- Deep alien science chains; exotic material precursors emerge
- Outer-system capable spacecraft prerequisites
- Power Tier 3+ required
- **Base/alien ratio:** ~10% base, 90% alien
- **Gate:** Reach outer-system zone

#### Tier 7 — Interface *(Advanced terminal)*
*Alien megastructure. Operate or study.*

- Full alien science graph visible; unique escape-component chains begin
- **Base/alien ratio:** ~5% base, 95% alien
- **Gate (terminal):** Collect seeded relay fragments + construct repair components + sustain relay power + activate
- **Gate (intermediary):** Access alien repository, extract FTL theory fragments → unlocks exotic material synthesis routes

#### Tier 8 — Revelation
*FTL theory applied. Exotic science chains.*

- Exotic material synthesis unlocked via T7 knowledge
- New machine types for exotic processing
- **Gate:** Synthesize first exotic material

#### Tier 9 — Forge
*Full exotic manufacturing. Final power renegotiation.*

- All FTL drive component chains available
- FTL-grade power infrastructure required
- **Gate:** Produce all FTL drive component types + sustain FTL-grade power

#### Tier 10 — Transcendence *(Pinnacle terminal)*
*Build the ship. Leave on your own terms.*

- Four major construction tracks: engines, FTL drive, reactor, shielding
- Each track requires deep independent production chain
- **Gate (terminal):** Construct all four ship systems + assemble + launch

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
| **Fabrication** | Assembly machines for composite and alien items | 3–10 |

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

Design uses: architectural forks (two legitimate approaches to the same problem), alien-contact path splits, risk/reward divergences at the same tier slot.

---

## 6. Tier 1 — Foundation

### Design intent

Tier 1 should feel like a fast-moving setup phase. The player is landing, building their first machines, and establishing the research loop. Unlocks come quickly — roughly one every 6 minutes — and there are always 3–5 unlockable nodes visible at once.

The player's choices in Tier 1 determine their early factory shape: power-first vs. production-first vs. science-first. All orderings lead to the same Tier 2 gate, but they produce meaningfully different infrastructure going in.

### Starting state

The player AI body includes a **built-in hand scanner** — not a tech unlock, always present. It can analyze samples manually and generates a small fixed yield of Material Science research per sample analyzed.

The **first alien sample analysis** is the source of initial research — a bootstrapping event within T1, not a tier gate. This analysis produces enough Material Science research to unlock 1–2 "Research spend (small)" nodes — typically Stone Furnace or a power source. From there the standard research loop begins: build Field Analyzer → analyze samples at scale → spend research on remaining nodes.

Simultaneously, the player can collect stone by hand (no machine required). After 50 stone, Basic Miner unlocks via production milestone — no research needed. This creates two parallel opening tracks:

- **Research-first:** Analyze sample → unlock Stone Furnace → build Field Analyzer → research loop live
- **Production-first:** Collect 50 stone by hand → Basic Miner unlocks → mine ore → smelt (after Stone Furnace via research track)

Both tracks converge quickly. Neither can proceed far without the other.

The hand scanner remains available throughout the run as a fallback but is weak — it cannot be automated, has no throughput scaling, and yields less per sample than a built Field Analyzer. Its only purpose is bootstrapping.

### Tier 1 node set (8 nodes, representative)

These are the 8 nodes that appear in *every* Standard run (all Common, Tier 1 only). They form the fixed skeleton; seeded nodes extend this in higher tiers.

| Node | Category | Effect | Primary unlock | Prerequisite |
|---|---|---|---|---|
| **Stone Furnace** | Smelting & Forming | Unlocks `smelter` machine type Tier 1; enables all ore→ingot recipes for known materials | Research spend (small) | — |
| **Ore Crusher** | Smelting & Forming | Unlocks `crusher` machine type Tier 1; enables ore→crushed_ore, crushed_ore→dust | Research spend (small) | Stone Furnace |
| **Basic Miner** | Extraction | Unlocks `miner` machine type Tier 1; automatic ore extraction from surface deposits | Production milestone: 50 stone | — |
| **Combustion Generator** | Power | Unlocks `combustion_generator` machine type; burns fuel items for power | Research spend (small) | Stone Furnace |
| **Solar Array** | Power | Unlocks `solar_array` machine type; passive power scaled by planet solar modifier | Research spend (small) | — |
| **Field Analyzer** | Science | Unlocks `analyzer` machine type Tier 1; enables sample analysis → research currency | Research spend (small) | — |
| **Basic Network Node** | Logistics | Unlocks `network_node` Tier 1 cable + basic storage node; enables logistics network | Research spend (small) | Stone Furnace |
| **Land Drone Mk1** | Exploration | Unlocks `land_drone` Tier 1; enables surface scouting and sample collection beyond the aegis field | Production milestone: 20 iron ingots | Basic Miner |

> ⚠️ Both power nodes (Combustion Generator, Solar Array) are in every run. The planet modifier determines which is worth building — a low-solar world pushes players to combustion; a geologically inert world pushes solar. Both exist so the player always has a choice, not a dictated answer.

### Tier 1 unlock ordering examples

**Power-first player:**
Solar Array → Stone Furnace → Basic Miner → Field Analyzer → Basic Network Node → Ore Crusher → Combustion Generator → Land Drone Mk1

**Production-first player:**
Stone Furnace → Basic Miner → Ore Crusher → Basic Network Node → Field Analyzer → Combustion Generator → Solar Array → Land Drone Mk1

**Science-first player:**
Field Analyzer → Stone Furnace → Combustion Generator → Basic Miner → Basic Network Node → Ore Crusher → Land Drone Mk1 → Solar Array

All three hit the Tier 2 gate (50 refined base-material units) at roughly the same clock time. The difference is which infrastructure is built out first.

### Tier 1 → 2 gate

**Condition:** Produce 100 units of any refined base material (ingot or plate form).

Broad condition intentional — the gate should accommodate whichever base material the run's seed placed in the core zone, not assume a specific one.

---

## 7. Open Questions

| # | Question | Priority |
|---|---|---|
| 1 | Exact Tier 1→2 gate quantity — 100 units too easy/hard? Needs playtesting. | High |
| 2 | T2–T5 node set — not designed yet. Nail T1 feel first. | High |
| 3 | Are 8 fixed Tier 1 nodes the right number, or should some T1 nodes also be seeded? Fewer fixed nodes = more run variance but less reliable onboarding. | Medium |
| 4 | "Smelting & Forming" vs "Processing" — may be confusing distinction for players in the shadow UI. Consider merging into "Material Processing" for display while keeping internal split. | Medium |
| 5 | Alien material count per Standard run — targeting 3–4. Drives pool selection logic but not yet specified. | Medium |
| 6 | Tier 5/6 themes for Advanced/Pinnacle — deferred until Standard is validated. | Low |
| 7 | Are Combustion Generator and Solar Array (T1) a candidate exclusive group? Currently both always spawn; making them a choice would create a harder power commitment early. | Medium |
| 8 | Optional pool sizing — how many optional nodes per tier? No targets set. | Medium |
| 9 | UI treatment for proactive choice surfacing — modal / sidebar overlay / tree highlight? Needs UX spec. | Medium |

---

*Tech Tree Design v0.2 — T1 (Landfall) designed. T2–T5 pending. T6–T10 deferred. See `gdd.md §7` for design constraints and `technical-design.md §3` for data model.*

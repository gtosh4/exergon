# Science & Research System Design

ECS components, system step-by-step logic, events/messages, and edge cases for the Science & Research system. Read `gdd.md §6` for design intent and `technical-design.md §9` for the prose overview. Vertical Slice and MVP differences are noted inline.

---

## Table of Contents

1. [Overview](#1-overview)
2. [ECS Structure](#2-ecs-structure)
3. [Research Pool](#3-research-pool)
4. [Research Recipes](#4-research-recipes)
5. [Unlock Flow](#5-unlock-flow)
6. [Knowledge Visibility](#6-knowledge-visibility)
7. [Systems](#7-systems)
8. [Messages](#8-messages)
9. [Execution Order](#9-execution-order)
10. [Vertical Slice Scope](#10-vertical-slice-scope)
11. [Edge Cases](#11-edge-cases)

---

## 1. Overview

Research is the bridge between factory operation and tech tree progression. Players build research stations, feed them items via the logistics network, and accumulate typed research points. Spending research points to unlock tech nodes is an explicit player choice — not automatic.

Two kinds of research station inputs exist:
- **Production inputs** (ore, ingots) → quantitative research points of a given type
- **Sample inputs** (drone-collected sample items) → knowledge reveals on specific nodes *(MVP only)*

Research stations are standard machines. No special ECS beyond what any machine has.

---

## 2. ECS Structure

### Research Station entity

Research stations use the standard machine component set. There is no research-specific component on the station entity.

```
Research station entity
├── Machine { machine_type: "research_station", tier: u8, ... }
├── MachineState
├── MachineActivity  (optional — present when recipe is running)
├── MachineEnergyPorts    ← relationship target: energy port entities
└── MachineLogisticsPorts ← relationship target: logistics port entities
```

Items flow in and out via the logistics network the same way as any other machine. The player connects logistics cables to the station's ports; `recipe_start_system` checks input availability and starts recipes automatically.

**In-progress experiment = `MachineActivity` on the station entity.** There is no separate experiment entity.

### TechTreeProgress resource

```rust
#[derive(Resource, Default, Debug)]
pub struct TechTreeProgress {
    /// Nodes the player has spent research to fully unlock (FullyRevealed).
    pub unlocked_nodes: HashSet<NodeId>,
    /// Nodes that have been partially revealed through gameplay — broad parameters visible.
    /// Populated by knowledge triggers. Unused in Vertical Slice.
    pub partially_revealed: HashSet<NodeId>,
    /// Recipe IDs gated by tech nodes that are now unlocked.
    pub unlocked_recipes: HashSet<String>,
    /// Machine type IDs unlocked via tech nodes.
    pub unlocked_machines: HashSet<String>,
}
```

Knowledge visibility state per node is derived from these two sets:

| State | Condition |
|---|---|
| **Shadow** | Not in `unlocked_nodes`, not in `partially_revealed` |
| **PartialReveal** | In `partially_revealed`, not in `unlocked_nodes` *(MVP only)* |
| **FullyRevealed** | In `unlocked_nodes` |

The GDD requires Shadow → PartialReveal → FullyRevealed; VS only uses Shadow and FullyRevealed.

---

## 3. Research Pool

```rust
#[derive(Resource, Default, Debug)]
pub struct ResearchPool {
    /// Accumulated research points by type. Keys are content-defined string IDs.
    pub amounts: HashMap<String, u32>,
}

impl ResearchPool {
    pub fn add(&mut self, type_id: &str, amount: u32) {
        *self.amounts.entry(type_id.to_string()).or_default() += amount;
    }

    pub fn get(&self, type_id: &str) -> u32 {
        self.amounts.get(type_id).copied().unwrap_or(0)
    }

    pub fn has(&self, type_id: &str, amount: u32) -> bool {
        self.get(type_id) >= amount
    }

    /// Returns false and does nothing if insufficient balance.
    pub fn spend(&mut self, type_id: &str, amount: u32) -> bool {
        let balance = self.amounts.entry(type_id.to_string()).or_default();
        if *balance < amount {
            return false;
        }
        *balance -= amount;
        true
    }
}
```

Research types are **content-defined strings** — no hardcoded enum. The base content pack defines the four types from `technical-design.md §9`:

| Type ID | Earned from | Gates |
|---|---|---|
| `material_science` | Mineral/ore/ingot inputs to research station | Recipe reveals, machine tier unlocks |
| `field_research` | Biological/ecosystem sample inputs | Exploration-gated nodes, biome knowledge |
| `engineering` | Production milestone recipes, machine operation recipes | Module unlocks, logistics upgrades |
| `discovery` | Exploration find / site interaction recipes | Exploration-only nodes, tier unlocks |

**Vertical Slice uses only `material_science`.**

---

## 4. Research Recipes

Research recipes are standard recipe assets with one convention: research point outputs use item IDs with the prefix `"research."`.

```
"research.material_science"   → 10u32 added to ResearchPool
"research.field_research"     → 5u32 added to ResearchPool
```

The suffix after `"research."` is the type ID passed to `ResearchPool.add`. Amount is the item quantity in the recipe output stack.

`recipe_progress_system` on recipe completion checks each output item: if its ID starts with `"research."`, route to `ResearchPool.add(suffix, quantity)` instead of the logistics network. This is the only special-casing needed. All other recipe logic is identical to any other machine.

### Content format

Research recipes follow the same RON format as all recipes. Example:

```ron
// assets/recipes/basic_mineral_analysis.ron
ConcreteRecipe(
    id: "basic_mineral_analysis",
    machine_type: "research_station",
    machine_tier: 1,
    inputs: [("iron_ore", 3)],
    outputs: [("research.material_science", 10)],
    processing_time: 5.0,
    energy_cost: 100,
    min_voltage_tier: 1,
)
```

Research station recipes are **tech-tree gated** like any other recipe. `recipe_start_system` checks `TechTreeProgress.unlocked_recipes` before starting a recipe.

### Sample recipes *(MVP)*

Sample items (collected via drone) are regular item IDs (e.g. `"xalite_sample"`). Sample-processing recipes take them as inputs and may produce both research points and knowledge triggers. In VS, sample items exist as item IDs but have no collection mechanic yet.

```ron
// assets/recipes/xalite_sample_analysis.ron  (MVP)
ConcreteRecipe(
    id: "xalite_sample_analysis",
    machine_type: "research_station",
    machine_tier: 1,
    inputs: [("xalite_sample", 1)],
    outputs: [("research.material_science", 50)],
    processing_time: 10.0,
    energy_cost: 200,
    min_voltage_tier: 1,
)
```

The knowledge trigger for this recipe is declared on the tech node that gets revealed — not on the recipe itself (see §6).

---

## 5. Unlock Flow

Unlocking a `ResearchSpend` node is **player-initiated**. Other unlock vectors remain automatic.

### UnlockVector change

`UnlockVector::ResearchSpend` gains a `type_id` field:

```rust
pub enum UnlockVector {
    ResearchSpend { type_id: String, amount: u32 },
    PrerequisiteChain,
    ProductionMilestone { material: String, quantity: f32 },
    ExplorationDiscovery(String),
    Observation(String),
}
```

All existing VS tech node assets update `ResearchSpend(N)` to `ResearchSpend(type_id: "material_science", amount: N)`.

### Player-initiated unlock

1. Player opens tech tree UI.
2. Player selects a node. Detail panel shows current balance for the node's research type alongside the cost: `"material_science: 47 / 100"`.
3. If prerequisites met and sufficient balance: UNLOCK button is active. Otherwise greyed.
4. Player clicks UNLOCK → UI fires `UnlockNodeRequest(node_id)`.
5. `check_research_unlocks` processes the request: re-validates prerequisites + balance, calls `ResearchPool.spend(type_id, amount)`, inserts node into `TechTreeProgress.unlocked_nodes`, applies `NodeEffect`s.
6. `TechTreeProgress` change triggers UI rebuild.

### Automatic unlocks

| Vector | Trigger |
|---|---|
| `PrerequisiteChain` | All prerequisites in `unlocked_nodes` → auto-unlock in same frame, no cost |
| `ExplorationDiscovery(key)` | Matching `DiscoveryEvent(key)` fired → auto-unlock |
| `ProductionMilestone` | *(not implemented in VS or MVP — architecture reserved)* |
| `Observation` | *(not implemented — architecture reserved)* |

`PrerequisiteChain` unlocks loop until stable within a single `check_research_unlocks` call. A chain of three `PrerequisiteChain` nodes all unlock in one frame once the root prerequisite is satisfied.

---

## 6. Knowledge Visibility

### Shadow and FullyRevealed (Vertical Slice)

Every node in `TechTree.nodes` is visible as a shadow: category, tier, rarity shown; effects and unlock cost hidden. When in `unlocked_nodes`, the node is fully revealed.

The tech tree UI renders node state from these two sets. Shadow nodes show their category and rarity. FullyRevealed nodes show their full name, effects, and unlock conditions.

### PartialReveal (MVP)

`TechTreeProgress.partially_revealed` stores nodes that have crossed to PartialReveal. Broad parameters are exposed: approximate input types, rough output range. Full details remain hidden until the player spends research.

### Knowledge triggers (MVP)

Knowledge triggers move a node from Shadow → PartialReveal. They are declared on the **tech node definition**, not on recipes:

```rust
// Added to NodeDef for MVP
pub partial_reveal_trigger: Option<PartialRevealTrigger>,
```

```rust
pub enum PartialRevealTrigger {
    /// Node becomes partially revealed when the named recipe completes.
    OnRecipeComplete(String),
    /// Node becomes partially revealed when the named item is consumed as a recipe input.
    OnItemConsumed(String),
}
```

On recipe completion, `recipe_progress_system` checks all nodes with a matching `PartialRevealTrigger` and inserts them into `TechTreeProgress.partially_revealed`. The check is done against:
- `OnRecipeComplete(id)` — if `completed_recipe_id == id`
- `OnItemConsumed(item_id)` — if `item_id` was among the recipe's inputs

Partial reveal is **earned through gameplay**, never purchased. Moving from PartialReveal → FullyRevealed still requires `ResearchPool.spend`. Players can skip PartialReveal entirely (shadow → full reveal at higher cost).

**VS:** No `partial_reveal_trigger` is set on any VS tech node asset. The `partially_revealed` set is never populated. The check in `recipe_progress_system` is a no-op when no node has a trigger.

---

## 7. Systems

### `check_research_unlocks`

Runs in `GameSystems::Simulation` while in `GameState::Playing`. Processes unlock requests and auto-unlock events.

**Step-by-step:**

1. Read all `UnlockNodeRequest` messages for this frame.
2. For each request:
   a. Look up node in `TechTree`. If not found, log warning, skip.
   b. If already in `unlocked_nodes`, skip (idempotent).
   c. Check all prerequisites are in `unlocked_nodes`. If any missing, skip (do not deduct RP).
   d. Match on `primary_unlock`:
      - `ResearchSpend { type_id, amount }` → call `ResearchPool.spend(type_id, amount)`. If returns false (insufficient), log and skip.
      - Other vectors: player should not be sending unlock requests for non-ResearchSpend nodes; log warning and skip.
   e. Insert node into `unlocked_nodes`. Apply `NodeEffect`s (`UnlockRecipes` → extend `unlocked_recipes`, `UnlockMachine` → insert into `unlocked_machines`).
   f. Log: `"Tech node '{id}' unlocked"`.
3. Read all `DiscoveryEvent` messages for this frame. Collect keys into a local set.
4. For each node in `TechTree` with `primary_unlock == ExplorationDiscovery(key)`:
   - If key is in the collected discovery set and node not already unlocked and prerequisites met → unlock (no cost). Apply effects.
5. Loop until stable: scan all nodes with `primary_unlock == PrerequisiteChain`, not yet unlocked. If all prerequisites in `unlocked_nodes` → unlock (no cost), apply effects. Repeat until no new unlocks.
6. *(Production milestone and Observation vectors: no-op for now — architecture reserved.)*

**Trigger:** This system currently runs every frame (`Update`). It should be changed to run only when `UnlockNodeRequest` or `DiscoveryEvent` messages are pending — use `MessageReader::is_empty()` guard or Bevy's `run_if` with a message-pending condition. Performance optimization, not a correctness concern.

### `recipe_progress_system` additions

On recipe completion, after normal output handling:

1. For each output item with ID starting with `"research."`:
   - Extract suffix: `type_id = item_id.strip_prefix("research.").unwrap()`
   - Call `ResearchPool.add(type_id, quantity)`.
   - Do **not** route to logistics storage.

2. *(MVP only)* For each node in `TechTree` with `partial_reveal_trigger` set:
   - `OnRecipeComplete(id)`: if completed recipe's ID matches → insert node into `TechTreeProgress.partially_revealed` if not already in `unlocked_nodes`.
   - `OnItemConsumed(item_id)`: if item_id was among this recipe's input items → same.

### Tech tree UI additions

In `rebuild_detail`:

- For `ResearchSpend { type_id, amount }` nodes that are not yet unlocked and have prerequisites met:
  - Show current balance: `"{type_id}: {pool.get(type_id)} / {amount}"`
  - Spawn UNLOCK button (active if `pool.has(type_id, amount)`, greyed otherwise).
  - Button click handler fires `UnlockNodeRequest(node_id)`.
- For nodes with `primary_unlock` other than `ResearchSpend`: show description of how to unlock (as before), no UNLOCK button.

---

## 8. Messages

**`UnlockNodeRequest(pub NodeId)`** — fired by UI when player clicks UNLOCK on a ResearchSpend node. Consumed by `check_research_unlocks`. One message per click; `check_research_unlocks` handles duplicates idempotently.

**`DiscoveryEvent(pub String)`** — existing message. Consumed by `check_research_unlocks` to trigger `ExplorationDiscovery` unlocks. Unchanged.

Both are Bevy `Message` types (one-frame broadcast via `MessageReader`).

---

## 9. Execution Order

```
PowerSimSystems                    // generators fill buffers
  → NetworkSystems::of::<Logistics>()
    → LogisticsSimSystems
        recipe_start_system        // starts research station recipes
        recipe_progress_system     // advances, on completion:
                                   //   routes "research.*" outputs → ResearchPool
                                   //   checks partial_reveal_trigger (MVP)
                                   //   fires NetworkStorageChanged
  → GameSystems::Simulation
      check_research_unlocks       // processes UnlockNodeRequest + DiscoveryEvent
                                   //   modifies TechTreeProgress
                                   //   modifies ResearchPool (deducts on unlock)
```

`check_research_unlocks` runs after `recipe_progress_system` in the same frame. Research points accumulated by a completed recipe are available for the `UnlockNodeRequest` handler in the same frame if the player clicks unlock immediately.

---

## 10. Vertical Slice Scope

VS implements:
- `ResearchPool` with typed `HashMap<String, u32>` (single type: `"material_science"`)
- `TechTreeProgress` with `unlocked_nodes`, `unlocked_recipes`, `unlocked_machines` (no `partially_revealed`)
- Research station machine type with VS content recipes (ore inputs → `"research.material_science"` outputs)
- Player-initiated UNLOCK button in tech tree detail panel
- Auto-unlock for `ExplorationDiscovery` and `PrerequisiteChain`
- `UnlockVector::ResearchSpend { type_id, amount }` in asset format

VS does **not** implement:
- Multiple research types (architecture exists, only one type active)
- `partially_revealed` population (field exists, never written)
- `partial_reveal_trigger` on `NodeDef` (field added for MVP; VS assets leave it absent)
- Sample item collection mechanic (sample items exist as IDs; no drone collection)
- `ProductionMilestone` or `Observation` unlock vectors

---

## 11. Edge Cases

**Player sends `UnlockNodeRequest` for a non-ResearchSpend node.** Log warning, skip. UI should not generate this request for other unlock vectors, but the system guards regardless.

**Player sends `UnlockNodeRequest` for a node whose prerequisites are not yet met.** Validation in `check_research_unlocks` catches this. No RP deducted, no unlock. UI greys out the button when prerequisites are unmet, so this should not happen in normal play.

**Player sends `UnlockNodeRequest` for a node they cannot afford.** `ResearchPool.spend` returns false. No partial deduction. Node remains locked.

**Multiple `UnlockNodeRequest` messages for the same node in one frame.** The second request hits the `if already in unlocked_nodes, skip` check. No double-spend.

**`DiscoveryEvent` fires for a node whose prerequisites are not met.** Skip — prerequisites still block the unlock. The event is not queued; if prerequisites later become met, the node will not auto-unlock without a new `DiscoveryEvent`. Design implication: `ExplorationDiscovery` nodes should be designed so the player can only reach the discovery site after prerequisites are naturally satisfied, or the node should have a `ResearchSpend` alternative vector.

**Research station recipe completes but `ResearchPool` resource does not exist.** `recipe_progress_system` should check `Option<ResMut<ResearchPool>>` and log a warning if missing. Outputs that cannot be routed are silently discarded in current implementation — this should be an explicit warning.

**`PartialRevealTrigger::OnRecipeComplete` fires for a node already in `unlocked_nodes`.** Skip — no action, no downgrade. FullyRevealed cannot revert to PartialReveal.

**Node has `partial_reveal_trigger` but is processed by a recipe in a different station tier.** The trigger fires regardless of which station tier ran the recipe — the trigger is on the node, not on the recipe. If this is undesired, the trigger should instead point to a specific recipe ID (which implies a specific tier requirement via the recipe's `machine_tier` field).

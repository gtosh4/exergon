# Crafting System Design

ECS components, system step-by-step logic, events/messages, and edge cases for the crafting system. Read `gdd.md §8` and `technical-design.md §5–6` for design intent. Covers recipe execution, catalyst inputs, module effects on recipes, auto-crafting job dispatch, and recipe graph runtime integration.

---

## Table of Contents

1. [Overview](#1-overview)
2. [ECS Structure](#2-ecs-structure)
3. [Recipe Graph Runtime](#3-recipe-graph-runtime)
4. [Machine Job Policy](#4-machine-job-policy)
5. [Recipe Execution](#5-recipe-execution)
6. [Catalyst Inputs](#6-catalyst-inputs)
7. [Module Effects on Recipe Execution](#7-module-effects-on-recipe-execution)
8. [Auto-crafting Job Dispatch](#8-auto-crafting-job-dispatch)
9. [Systems](#9-systems)
10. [Messages](#10-messages)
11. [Execution Order](#11-execution-order)
12. [Vertical Slice Scope](#12-vertical-slice-scope)
13. [Edge Cases](#13-edge-cases)

---

## 1. Overview

Crafting is the execution layer for the recipe graph. A machine runs a recipe by consuming input items, holding power, advancing a progress timer, and producing output items on completion. The auto-crafting network (logistics layer) dispatches crafting jobs to capable machines and resolves multi-step dependency chains automatically.

Three todos from `design-todos.md` are resolved here: **Recipe Graph Runtime Integration**, **Catalyst Inputs**, and **Auto-crafting Job Dispatch**. Module slot mechanics and snap detection are out of scope (separate module system spec); this document covers only how module components alter recipe execution.

---

## 2. ECS Structure

### Machine entity extensions (from `technical-design.md §5`)

```
Machine entity
├── RecipeProcessor             ← execution state; see below
├── MachineJobPolicy            ← dispatch behavior; see §4
├── MachineCapability           ← cached set of runnable recipes; see §3
└── ... (existing Machine, Transform, PowerConsumer, etc.)
```

### RecipeProcessor component

```rust
#[derive(Component)]
pub struct RecipeProcessor {
    pub slots: Vec<RecipeSlot>,  // len 1 normally; 2 with a parallel-slot module
}

pub struct RecipeSlot {
    pub job: Option<Entity>,  // CraftingJob assigned to this slot; None = idle
    pub progress: f32,        // 0.0–1.0; fraction of processing_time elapsed
    pub state: SlotState,
}

pub enum SlotState {
    Idle,
    Running,
    PowerPaused,  // insufficient power; amps held, progress halted
}
```

`RecipeProcessor` replaces `MachineActivity` and `MachineState` from `networks.md §2`. `SlotState::Running` = `MachineState::Running`; `SlotState::Idle` = `MachineState::Idle`. Do not use `MachineActivity` or `MachineState` on machines that carry `RecipeProcessor`.

Slot count is set at machine spawn (1) and updated when a parallel-slot module is attached or detached (see §7). The parallel-slot module can only be added or removed while all slots are idle.

### CraftingJob entity

One entity per dispatched job. Spawned by `crafting_plan_resolver_system` (auto-craft) or `manual_recipe_system` (manual mode).

```
CraftingJob entity
├── CraftingJob { recipe_id, quantity, priority, status: JobStatus }
├── JobPrerequisites { blocking: Vec<Entity> }   ← absent when no prerequisites
└── CatalystReservations { slots: Vec<CatalystSlot> }  ← absent when recipe has no catalysts
```

```rust
pub enum JobStatus {
    Blocked,                                       // waiting on prerequisite jobs
    Queued,                                        // prerequisites met; waiting for a machine
    Dispatched { machine: Entity, slot: u8 },      // assigned; machine setting up
    InProgress { machine: Entity, slot: u8 },      // actively running
    Complete,
}
```

### CraftingPlan entity

Spawned by `crafting_plan_resolver_system` per auto-craft request. Groups all jobs belonging to one plan. A plan is scoped to a single logistics network — the dispatcher only assigns jobs to machines on that network.

```
CraftingPlan entity
├── CraftingPlan { target_item: ItemId, target_quantity: u32 }
├── PlanNetwork(Entity)        ← points to the logistics network; see relationship below
└── PlanJobs(Vec<Entity>)      ← auto-maintained by Bevy relationship; do not mutate directly
```

### PlanNetwork relationship

```rust
/// On each CraftingPlan entity — the logistics network this plan operates within.
#[derive(Component)]
#[relationship(relationship_target = NetworkPlans)]
pub struct PlanNetwork(pub Entity);

/// On each LogisticsNetwork entity — auto-maintained list of active plans on this network.
#[derive(Component)]
#[relationship_target(relationship = PlanNetwork)]
pub struct NetworkPlans(Vec<Entity>);
```

Default cascade despawn applies: if the logistics network entity is despawned, all its plans are despawned too (their jobs are handled by `plan_cancellation_system` via observer on `OnRemove<PlanNetwork>`). All reservations on the network are already gone with the network entity.

`RequestCraft` carries the network entity so the resolver knows which network to scope to (machines, available storage, reservations all scoped to that network).

### JobOf relationship

```rust
/// On each job entity — points to its owning plan.
#[derive(Component)]
#[relationship(relationship_target = PlanJobs, despawn_dependents_with_target = false)]
pub struct JobOf(pub Entity);

/// On the plan entity — auto-maintained list of all jobs in this plan.
#[derive(Component)]
#[relationship_target(relationship = JobOf)]
pub struct PlanJobs(Vec<Entity>);
```

`despawn_dependents_with_target = false` disables cascade despawn — `InProgress` jobs must run to completion even when their plan is cancelled. Cleanup is handled explicitly by `plan_cancellation_system`.

```rust
pub struct CraftingJob {
    pub recipe_id: RecipeId,
    pub quantity: u32,
    pub priority: i32,
    pub status: JobStatus,
    // plan entity accessed via JobOf relationship query — no redundant field
}
```

### NetworkReservations component

Attached to each `LogisticsNetwork` entity. Scopes all reservation accounting to the network where the physical items reside — a reservation on network A cannot affect availability on network B.

```rust
#[derive(Component, Default)]
pub struct NetworkReservations {
    /// item_id → total quantity held by active jobs as catalysts (consumed: false inputs)
    pub catalyst: HashMap<ItemId, u32>,
    /// (plan_entity, item_id) → quantity produced by a prerequisite job and reserved for
    /// downstream jobs in that plan; invisible to other plans and passive machines
    pub plan_output: HashMap<(Entity, ItemId), u32>,
}
```

`LogisticsNetworkMembers.has_items` (defined in `networks.md §2`) is extended to subtract `NetworkReservations` from that network's total storage:

- `catalyst[item_id]` subtracted for all callers.
- `plan_output[(plan, item_id)]` subtracted for callers outside plan P; a query with plan context P skips its own plan's entries — the downstream job sees its reserved items as available and pullable. Passive machines and other plans see plan output reservations as unavailable.

---

## 3. Recipe Graph Runtime

### RecipeGraph resource

Inserted at run start from the generated recipe graph. Never mutated during a run.

```rust
#[derive(Resource)]
pub struct RecipeGraph {
    pub recipes: HashMap<RecipeId, ConcreteRecipe>,
    // Lookup indexes — all derived from `recipes` at build time
    pub by_output:  HashMap<ItemId, Vec<RecipeId>>,
    pub by_input:   HashMap<ItemId, Vec<RecipeId>>,
    pub by_machine: HashMap<(MachineType, u8), Vec<RecipeId>>,  // (type, min_tier)
    pub terminal_item: ItemId,
}
```

`by_machine` keys on `(MachineType, min_tier)` — a recipe requiring tier 2 is stored under `(type, 2)`. When querying for a tier-3 machine, the dispatcher checks `(type, 1)`, `(type, 2)`, and `(type, 3)` — a machine can always run recipes designed for its tier or lower.

### TechTreeProgress unlock gating

`TechTreeProgress` (in `src/research/mod.rs`) carries `unlocked_recipes: HashSet<RecipeId>`. This set is the source of truth for recipe availability.

`unlocked_recipes` is updated by `tech_node_unlock_system` when a node is unlocked:

- **Material node** → expand all recipe templates that involve this material's form groups; insert resulting `ConcreteRecipeId`s for all already-unlocked machine types.
- **Machine/process node** → expand all recipe templates for this machine type across all already-unlocked materials; insert resulting `ConcreteRecipeId`s.
- **Special recipe node** → insert the specific `RecipeId` directly.

Standard (template-derived) recipes never appear in `unlocked_recipes` until both their material node and their machine node are unlocked. Special recipes require their own explicit node.

### MachineCapability component

Cached per-machine set of recipes it can currently run, derived from machine type + tier and `TechTreeProgress.unlocked_recipes`. Rebuilt by `machine_capability_register_system` on `TechNodeUnlocked` and on machine placement.

```rust
#[derive(Component)]
pub struct MachineCapability {
    pub capable: HashSet<RecipeId>,
}
```

The dispatcher reads `MachineCapability` directly; it does not re-query `RecipeGraph` and `TechTreeProgress` per dispatch cycle.

---

## 4. Machine Job Policy

Each machine entity carries a `MachineJobPolicy` component. Configured through the machine's UI panel (see machine UI mock). The dispatcher reads this component when selecting machines for job assignment.

```rust
#[derive(Component)]
pub struct MachineJobPolicy {
    pub mode: JobMode,
    pub overrides: HashMap<RecipeId, RecipeOverride>,
}

pub enum JobMode {
    Auto {
        priority: i32,                        // higher = preferred by dispatcher
        category_filter: Option<RecipeCategory>,  // None = all categories
    },
    PassiveLoop(RecipeId),   // run this recipe continuously; auto-restart on completion
    PassiveOnce(RecipeId),   // run this recipe once; go idle afterward
    Excluded,                // never accept auto-craft jobs
}

pub struct RecipeOverride {
    pub priority: Option<i32>,  // overrides Auto priority for this specific recipe
    pub excluded: bool,         // exclude this recipe from this machine even in Auto mode
}
```

**Auto mode:** machine is eligible for auto-crafting job dispatch. Priority determines preference when multiple machines can run the same job — higher priority machines are assigned first. Category filter restricts the machine to jobs of matching recipe category. Recipe-level overrides further tune per-recipe priority or exclusion.

**PassiveLoop / PassiveOnce:** machine is not eligible for auto-craft dispatch. The player pins a recipe; the machine runs it passively and automatically without dispatcher involvement. `passive_recipe_system` handles these machines directly. Inputs are pulled from the logistics network; outputs are pushed back. No `CraftingJob` entity is created for passive-mode runs — progress is tracked directly on the `RecipeSlot`.

**Excluded:** machine is invisible to the dispatcher; useful for machines reserved for specific workflows.

`overrides` only applies in `Auto` mode. In `ManualLoop`/`ManualOnce`/`Excluded`, overrides are ignored.

---

## 5. Recipe Execution

### recipe_start_system

**Trigger:** `JobDispatched { job, machine, slot }` (auto-craft path) or `PassiveRecipeStart { machine, slot, recipe_id }` (passive-mode path). `passive_recipe_system` emits `PassiveRecipeStart` and is itself triggered by `MachineSlotIdle` and `NetworkStorageChanged` — the latter handles newly available inputs unblocking a stalled passive machine.

Step by step:

1. Read `ConcreteRecipe` from `RecipeGraph`.
2. Check power — voltage and amp checks per `networks.md §2` and `technical-design.md §7`:
   a. Network voltage tier ≥ `recipe.min_voltage_tier` — hard block if not met; emit `RecipeBlockedVoltage { machine, slot }`.
   b. Available amp headroom ≥ draw rate — block if at amp capacity; emit `RecipeBlockedAmps { machine, slot }`. Job stays `Dispatched` and retries when `AmpHeadroomRestored` fires.
3. Check input availability — for each `recipe.inputs` where `consumed == true`, resolve the machine's input-eligible logistics ports (via `PortPolicy`) and call `has_items` on each port's network with this job's plan as context (plan-aware: plan output reservations for this plan are visible as available on that network). If any item is unavailable in sufficient quantity across reachable networks, block: emit `RecipeBlockedInputs { machine, slot, missing }`. Retries on `NetworkStorageChanged`.
4. Check output routing — for each `recipe.outputs`, verify at least one of the machine's logistics ports (via `PortPolicy`) allows output for that item and has a connected network. If any output has no valid destination, block: emit `RecipeBlockedOutputs { machine, slot }`. Retries on `NetworkChanged<Logistics>`.
5. Check catalyst availability — for each `recipe.inputs` where `consumed == false`, call `has_items` on the item's source network (subtracts `NetworkReservations.catalyst[item]` for that network). If `available < required`, block: emit `RecipeBlockedCatalysts { machine, slot }`. Retries on `NetworkStorageChanged` and on `CatalystReservationReleased`.
6. All checks pass:
   a. Pull `recipe.inputs` where `consumed == true` from the plan's logistics network.
   b. For auto-craft path: for each pulled item, decrement `network.NetworkReservations.plan_output[(plan, item)]`.
   c. Reserve `recipe.inputs` where `consumed == false`: increment `network.NetworkReservations.catalyst[item]`; add `CatalystReservations` component to job entity (records item and quantity — network is always the plan's network).
   d. Allocate amp draw from power network.
   e. Set `RecipeSlot.state = Running`, `progress = 0.0`, `job = Some(job_entity)`.
   f. Set job status to `InProgress { machine, slot }`.
   g. Emit `RecipeStarted { machine, slot, recipe_id }`.

### recipe_progress_system

**Trigger:** every tick, for all machines with `RecipeProcessor`.

Step by step (per slot):

1. Skip slots where `state != Running`.
2. Attempt to withdraw `draw_rate * delta_seconds` joules from power network.
3. If joules unavailable:
   a. Set `state = PowerPaused`. Amps remain allocated (machine is still online).
   b. Emit `RecipePowerPaused { machine, slot }` (once per transition; not every tick).
   c. Skip progress advance this tick.
4. If joules available and state was `PowerPaused`:
   a. Set `state = Running`.
   b. Emit `RecipePowerResumed { machine, slot }`.
5. Advance `progress += delta_seconds / effective_processing_time`.
   - `effective_processing_time = recipe.processing_time * speed_modifier` (see §7).
6. If `progress >= 1.0`:
   a. Clamp `progress = 1.0`.
   b. Emit `RecipeComplete { machine, slot }`.

### recipe_completion_system

**Trigger:** `RecipeComplete { machine, slot }`.

Step by step:

1. Read the job entity from `RecipeSlot.job`.
2. Push `recipe.outputs` to logistics network storage via the machine's output-eligible logistics ports (per `PortPolicy`).
3. Emit `NetworkStorageChanged { network }` for each logistics network connected to the machine's output ports — triggers `recipe_start_system` evaluation for idle machines on those networks.
4. Release amp draw from power network.
5. Release catalyst reservations: for each entry in `CatalystReservations`, decrement `network.NetworkReservations.catalyst[item]` on the recorded network; remove `CatalystReservations` component from job entity; emit `CatalystReservationReleased { item_id, quantity }` per item.
6. Set `RecipeSlot.state = Idle`, `job = None`, `progress = 0.0`.
7. Set job status to `Complete`.
8. Emit `JobComplete { job, machine, slot }`.
9. Emit `MachineSlotIdle { machine, slot }`.
10. For `PassiveLoop` machines: emit `PassiveRecipeStart { machine, slot, recipe_id }` immediately to restart.

---

## 6. Catalyst Inputs

### Data model

`ConcreteRecipe` uses a unified `RecipeInput` type for all inputs (from `technical-design.md §2`):

```rust
pub struct RecipeInput {
    pub item: ItemId,
    pub quantity: u32,
    pub consumed: bool,  // false = catalyst; physically present and reserved for job duration, not pulled from network
}

pub struct ConcreteRecipe {
    // ... existing fields ...
    pub inputs: Vec<RecipeInput>,   // consumed:true = regular input; consumed:false = catalyst
    pub outputs: Vec<ItemStack>,    // all outputs; no separate byproduct concept
}
```

Inputs with `consumed: false` are passed through unchanged from the recipe asset — they are not a seeded variance axis and do not receive parameter variance multipliers.

Asset format:

```ron
ConcreteRecipe(
    // ...
    inputs: [
        RecipeInput(item: "raw_ore",         quantity: 3, consumed: true),
        RecipeInput(item: "activation_lens", quantity: 1, consumed: false),  // catalyst
    ],
)
```

Template recipes may also define catalyst inputs; they are copied identically to every instantiation.

### Reservation semantics

Catalyst inputs (`consumed: false`) are **not pulled** from the logistics network during job execution — they remain in place throughout the job. They are **reserved**: unavailable to other jobs, other auto-craft requests, or passive pulls until the reservation is released.

Reservations are stored on the `NetworkReservations` component of the specific network where the catalyst physically resides:

- **Reserve:** when `recipe_start_system` passes all checks, for each `consumed: false` input, increment `network.NetworkReservations.catalyst[item_id] += quantity`. `has_items` on that network subtracts this from available total.
- **Release:** when `recipe_completion_system` runs, decrement `network.NetworkReservations.catalyst[item_id]` on the recorded network; emit `CatalystReservationReleased { item_id, quantity }`.

`CatalystReservations` on the job entity records `(item_id, quantity)` per catalyst. The network is always the plan's network (via `JobOf` → `PlanNetwork`), so no redundant network field is needed.

Catalyst items cannot be removed mid-job because the reservation makes them unavailable to all other systems. The scenario of mid-job catalyst loss is therefore unreachable in normal play.

### Multi-job contention

Two jobs requiring the same catalyst each need independent reserved copies. For each candidate job start, `recipe_start_system` evaluates per network:

```
available = network_total(item) - network.NetworkReservations.catalyst[item]
```

If `available < required_quantity`, the job blocks (`RecipeBlockedCatalysts`). It retries when `CatalystReservationReleased` fires (another job completed and freed a copy) or when `NetworkStorageChanged` fires (new items arrived in storage).

---

## 7. Module Effects on Recipe Execution

Module attachment and slot snap detection are specified in the module system doc. This section covers only the runtime effects on `RecipeProcessor`.

### Parallel slot module

A machine with a parallel-slot module attached has `RecipeProcessor.slots` expanded to length 2. Each slot operates independently:

- Each slot can hold a different recipe (or the same recipe running in parallel).
- Each slot consumes its own input items and produces its own outputs.
- Power draw = sum of both slots' active draw rates. Amp allocation is per-slot; a slot that is blocked or idle holds no amps.
- Both slots share the machine's single `MachineJobPolicy`.

The module may only be attached or detached while all slots are idle (`SlotState::Idle`). Attachment: `RecipeProcessor.slots.push(RecipeSlot::default())`. Detachment: remove the last slot only if `slots[1].state == Idle`.

### Speed and efficiency modules

Speed and efficiency module effects are applied as multipliers on recipe execution:

```rust
#[derive(Component)]
pub struct MachineModifierState {
    pub speed_multiplier: f32,       // default 1.0; <1.0 = faster (shorter time)
    pub efficiency_multiplier: f32,  // default 1.0; <1.0 = less energy per recipe
}
```

`MachineModifierState` is recomputed by `module_effect_system` whenever modules are attached or detached. `recipe_progress_system` reads it to compute `effective_processing_time`:

```
effective_processing_time = recipe.processing_time * speed_multiplier
draw_rate = (recipe.energy_cost / recipe.processing_time) * efficiency_multiplier
```

Note: reducing `speed_multiplier` shortens processing time but does not change total energy cost per recipe — the draw rate adjusts proportionally so energy per recipe = `energy_cost * efficiency_multiplier` regardless of speed.

The tradeoff between speed and efficiency modules is a function of their specific values, defined in module asset data.

---

## 8. Auto-crafting Job Dispatch

### Crafting plan resolution

**Trigger:** `RequestCraft { item_id: ItemId, quantity: u32 }` — emitted by UI (player request) or by another system.

**System:** `crafting_plan_resolver_system`

Step by step:

1. Look up producing recipes for `item_id` in `RecipeGraph.by_output`. Filter by `TechTreeProgress.unlocked_recipes`. If no recipe found, emit `PlanResolutionFailed { item_id, reason: NoRecipe }` and return.
2. Select best recipe (priority: explicitly pinned > machine policy priority > recipe graph ordering).
3. Recurse for each `recipe.input_items`:
   a. Check logistics network available count (subtracting reservations).
   b. If available ≥ required: mark as sourced from storage. No job needed for this input.
   c. If not: recurse — find producing recipe, generate a prerequisite `CraftingJob`.
4. If any input has no producing recipe and is unavailable in storage: emit `PlanResolutionFailed` and return.
5. Spawn `CraftingPlan` entity and all `CraftingJob` entities (leaf jobs have no `JobPrerequisites`; downstream jobs list their prerequisite job entities). Jobs at the bottom of the chain start as `Queued`; jobs with unsatisfied prerequisites start as `Blocked`.
6. Emit `CraftingPlanCreated { plan: Entity }`.
7. Emit `JobQueued` for each `Queued` job — triggers the dispatcher.

### Job dispatcher system

**Trigger:** `MachineSlotIdle { machine, slot }` and `JobQueued { job }`.

**System:** `job_dispatcher_system`

Step by step:

1. **On `MachineSlotIdle { machine, slot }`:**
   a. Skip if machine's `MachineJobPolicy.mode` is not `Auto`.
   b. Collect all `Queued` jobs (status = Queued, no blocking prerequisites).
   c. Filter to jobs whose `recipe_id` is in `machine.MachineCapability.capable`.
   d. Apply `MachineJobPolicy.overrides`: remove excluded recipes; collect priority values.
   e. If no candidates: machine stays idle.
   f. Select highest-priority job (tiebreak: FIFO by job creation order).
   g. Set job status to `Dispatched { machine, slot }`.
   h. Emit `JobDispatched { job, machine, slot }` → triggers `recipe_start_system`.

2. **On `JobQueued { job }`:**
   a. Collect all idle slots across all `Auto`-mode machines.
   b. For each idle slot, check if `recipe_id` is in that machine's `MachineCapability.capable` and not overridden-excluded.
   c. Among eligible machines, select highest-priority machine (by `MachineJobPolicy.mode.priority`, tiebreak FIFO).
   d. If a match found: proceed as step 1f–h.
   e. If no match: job remains `Queued` until a machine becomes idle.

### Machine capability auto-registration

**System:** `machine_capability_register_system`

**Trigger:** `TechNodeUnlocked { node }` and machine entity creation.

Step by step:

1. For each machine entity (or just the new machine on creation):
   a. Read machine type and tier.
   b. Query `RecipeGraph.by_machine` for `(type, tier)` and all lower tiers of the same type.
   c. Intersect with `TechTreeProgress.unlocked_recipes`.
   d. Write result to `MachineCapability.capable`.
2. Emit `MachineCapabilityUpdated { machine }`.

On `TechNodeUnlocked`, re-run for all machines of the types affected by the newly unlocked node — not all machines.

### Plan cancellation

**Trigger:** `CancelCraftingPlan { plan }` — emitted by UI when the player cancels a pending or in-progress plan.

**System:** `plan_cancellation_system`

Step by step:

1. Read `PlanNetwork` to get the logistics network entity. Read `PlanJobs` to get all job entities.
2. On the network's `NetworkReservations`: remove all `plan_output[(plan, *)]` entries for this plan. Items become visible to other crafts immediately.
3. For each job where `status` is `Queued` or `Blocked`:
   a. If the job has `CatalystReservations`: for each entry, decrement `network.NetworkReservations.catalyst[item]` on the recorded network; emit `CatalystReservationReleased` per item.
   b. Despawn the job entity (Bevy automatically removes it from `PlanJobs`).
4. For each job where `status` is `Dispatched` (assigned but not yet started): treat as Queued — release catalyst reservations if any, despawn.
5. For each job where `status` is `InProgress`:
   a. Remove `JobOf` component — detaches job from plan without despawning; job runs to completion normally.
   b. On completion, `recipe_completion_system` reads plan context from `JobOf` — absent, so no plan output reservation is added. Outputs land unreserved in the logistics network.
6. Despawn the plan entity.

`InProgress` jobs are not aborted — inputs are already consumed, and aborting yields nothing.

---

### Job prerequisite resolution

**Trigger:** `JobComplete { job }`.

**System:** `job_prerequisite_system`

Step by step:

1. Query all `Blocked` jobs that list the completed job in `JobPrerequisites.blocking`.
2. For each: remove the completed job from `blocking`.
3. If `blocking` is now empty:
   a. Reserve the completed job's outputs for this plan: for each output item and quantity, increment `network.NetworkReservations.plan_output[(plan, item)] += quantity` on the plan's network (via `PlanNetwork`).
   b. Set status to `Queued`; emit `JobQueued { job }`.

---

## 9. Systems

| System | Trigger | Purpose |
|---|---|---|
| `crafting_plan_resolver_system` | `RequestCraft` | Walk recipe graph; spawn plan + job entities |
| `machine_capability_register_system` | Machine placed, `TechNodeUnlocked` | Rebuild `MachineCapability` for affected machines |
| `job_dispatcher_system` | `MachineSlotIdle`, `JobQueued` | Assign queued jobs to idle machine slots |
| `recipe_start_system` | `JobDispatched`, `PassiveRecipeStart` | Check power/inputs/catalysts; consume inputs; release plan output reservations; reserve catalysts; start slot |
| `recipe_progress_system` | Every tick | Advance progress on running slots; withdraw power |
| `recipe_completion_system` | `RecipeComplete` | Produce outputs; release catalysts; set job complete; emit idle |
| `job_prerequisite_system` | `JobComplete` | Unblock dependent jobs; reserve plan outputs; emit `JobQueued` |
| `plan_cancellation_system` | `CancelCraftingPlan` | Release reservations; despawn non-InProgress jobs; despawn plan |
| `passive_recipe_system` | `MachineSlotIdle`, `NetworkStorageChanged` (passive-mode machines) | Start PassiveLoop/PassiveOnce recipe without job dispatch |
| `module_effect_system` | Module attached/detached | Recompute `MachineModifierState` |

---

## 10. Messages

| Message | Payload | Emitted by |
|---|---|---|
| `RequestCraft` | `item_id, quantity, network: Entity` | UI / player input |
| `CancelCraftingPlan` | `plan: Entity` | UI / player input |
| `CraftingPlanCreated` | `plan: Entity` | `crafting_plan_resolver_system` |
| `PlanResolutionFailed` | `item_id, reason` | `crafting_plan_resolver_system` |
| `JobQueued` | `job: Entity` | `crafting_plan_resolver_system`, `job_prerequisite_system` |
| `JobDispatched` | `job, machine, slot` | `job_dispatcher_system` |
| `JobComplete` | `job, machine, slot` | `recipe_completion_system` |
| `MachineSlotIdle` | `machine, slot` | `recipe_completion_system` |
| `MachineCapabilityUpdated` | `machine: Entity` | `machine_capability_register_system` |
| `RecipeStarted` | `machine, slot, recipe_id` | `recipe_start_system` |
| `RecipeBlockedVoltage` | `machine, slot` | `recipe_start_system` |
| `RecipeBlockedAmps` | `machine, slot` | `recipe_start_system` |
| `RecipeBlockedInputs` | `machine, slot, missing` | `recipe_start_system` |
| `RecipeBlockedOutputs` | `machine, slot` | `recipe_start_system` |
| `RecipeBlockedCatalysts` | `machine, slot` | `recipe_start_system` |
| `RecipeComplete` | `machine, slot` | `recipe_progress_system` |
| `RecipePowerPaused` | `machine, slot` | `recipe_progress_system` |
| `RecipePowerResumed` | `machine, slot` | `recipe_progress_system` |
| `CatalystReservationReleased` | `item_id, quantity` | `recipe_completion_system` |
| `PassiveRecipeStart` | `machine, slot, recipe_id` | `passive_recipe_system`, `recipe_completion_system` (loop restart) |

---

## 11. Execution Order

All crafting systems belong to `LogisticsSimSystems` and run after `PowerSimSystems` per `networks.md §4`.

```
[PowerSimSystems]                    // generator_tick — fill buffers; may emit NetworkChanged<Power>

[LogisticsSimSystems]
├── passive_recipe_system            (NetworkStorageChanged, MachineSlotIdle — passive machines)
│       └─ emit PassiveRecipeStart
│
├── recipe_progress_system           (every tick — all machines with RecipeProcessor)
│       └─ (progress = 1.0) → RecipeComplete
│
├── recipe_completion_system         (on RecipeComplete)
│       ├─ produce outputs → emit NetworkStorageChanged
│       ├─ release catalysts → emit CatalystReservationReleased
│       ├─ emit JobComplete + MachineSlotIdle
│       └─ (PassiveLoop) emit PassiveRecipeStart → recipe_start_system same frame
│
├── job_prerequisite_system          (on JobComplete)
│       └─ unblock dependent jobs → emit JobQueued
│
├── job_dispatcher_system            (on MachineSlotIdle + JobQueued)
│       └─ emit JobDispatched
│
└── recipe_start_system              (on JobDispatched / PassiveRecipeStart)
        └─ checks pass → start slot; emit RecipeStarted

[Event-driven, not tick-ordered]
├── crafting_plan_resolver_system    (on RequestCraft)
├── machine_capability_register_system  (on machine placed, TechNodeUnlocked)
└── module_effect_system             (on module attached/detached)
```

`recipe_progress_system` runs before `recipe_completion_system`. `job_prerequisite_system` runs before `job_dispatcher_system` — a job unblocked this frame can be dispatched in the same frame. `passive_recipe_system` runs before `recipe_progress_system` so a machine idled by the previous frame's completion can begin its next recipe without a one-frame delay.

Power systems complete before `LogisticsSimSystems` — generator buffers are filled before `recipe_start_system` checks them. For power-blocked retries, `AmpHeadroomRestored` (from `networks.md`) triggers `recipe_start_system` after the power tick that freed headroom.

---

## 12. Vertical Slice Scope

| Feature | VS | MVP |
|---|---|---|
| Passive recipe execution (PassiveLoop) | ✓ | ✓ |
| RecipeProcessor with single slot | ✓ | ✓ |
| recipe_progress_system + recipe_completion_system | ✓ | ✓ |
| Input consumption, output production | ✓ | ✓ |
| Power integration (voltage + amp check, progress pause) | ✓ | ✓ |
| MachineJobPolicy (default mode only) | ✓ | ✓ |
| RecipeGraph resource + lookup indexes | ✓ | ✓ |
| TechTreeProgress unlock gating | — | ✓ |
| MachineCapability auto-registration | — | ✓ |
| Auto-crafting job dispatch (CraftingPlan + dispatcher) | — | ✓ |
| Catalyst inputs + plan output isolation (NetworkReservations) | — | ✓ |
| Job prerequisite resolution (multi-step plans) | — | ✓ |
| Recipe-level policy overrides | — | ✓ |
| Parallel slot module | — | ✓ |
| Speed/efficiency module effects | — | ✓ |
| PassiveOnce mode | — | ✓ |

For VS: one machine type, one tier. `MachineJobPolicy` defaults to `PassiveLoop` with a hardcoded recipe. No dispatcher, no plans, no catalysts. `RecipeGraph` resource exists but contains only the VS recipe set. `TechTreeProgress.unlocked_recipes` is not gated.

---

## 13. Edge Cases

| Case | Behavior |
|---|---|
| Two jobs require the same catalyst; only one copy in network | First job increments `network.NetworkReservations.catalyst[item]`; second job's `has_items` check sees available = 0 and blocks with `RecipeBlockedCatalysts`. Retries when `CatalystReservationReleased` fires on first job's completion. |
| Machine tier upgraded while job in progress | Higher tiers can run recipes for their tier and below. Recipe remains valid; `RecipeSlot` continues uninterrupted. `MachineCapability` is rebuilt on upgrade event. |
| Parallel slot module detached while slot 1 is running | Detachment blocked: module system rejects detach while any slot `state != Idle`. Player must wait for slot 1 to complete. |
| Parallel slot module attached while slot 0 is running | Allowed: slot 0 continues; slot 1 starts idle. `RecipeProcessor.slots.push(RecipeSlot::default())`. |
| PassiveLoop machine: inputs unavailable at completion | `recipe_completion_system` emits `PassiveRecipeStart`. `recipe_start_system` checks input availability, blocks on `RecipeBlockedInputs`. Machine idles until `NetworkStorageChanged` fires with sufficient inputs. |
| Auto-craft plan requested for item with no unlocked recipe | `crafting_plan_resolver_system` emits `PlanResolutionFailed { reason: NoRecipe }`. No jobs created. |
| Auto-craft plan requested; recipe exists but no capable machine exists | Plan resolves and jobs are spawned as `Queued`. Jobs remain queued indefinitely until a capable machine is placed and `MachineCapabilityUpdated` triggers the dispatcher. |
| Job dispatched to machine; power voltage too low at start | `recipe_start_system` emits `RecipeBlockedVoltage`. Job stays `Dispatched`. No retry is automatic — player must upgrade the power network. The dispatcher does not reassign the job; the blocked start is a signal to the player. |
| Job dispatched; amp capacity full at start | `recipe_start_system` emits `RecipeBlockedAmps`. Job stays `Dispatched`. `recipe_start_system` retries when `AmpHeadroomRestored` fires (another recipe completes and frees amps). |
| CraftingPlan cancelled mid-execution | `plan_cancellation_system` runs (see §8). |
| Two plans both need item B, which itself requires a recipe | Each plan independently generates a `CraftingJob` for item B. Both dispatch to capable machines as headroom allows. No deduplication — plans are independent. |
| Priority tie between multiple queued jobs | Dispatcher uses job entity creation order as tiebreak (earlier creation = higher priority). Stable and deterministic. |
| Prerequisite job A completes; downstream job B (same plan) not yet started; another craft targets the same output items | `job_prerequisite_system` increments `network.NetworkReservations.plan_output[(plan_P, item)]` on the output network. Other crafts and passive machines call `has_items` without plan context — all `plan_output` entries are subtracted, blocking them. Job B's `recipe_start_system` queries with plan P context — its own plan's `plan_output` entries are skipped, so it sees the items as available. On job B start, the `plan_output` entry is decremented and items consumed. |
| `TechNodeUnlocked` fires for a machine type with no machines placed | `machine_capability_register_system` updates no entities (no machines of that type exist). When a machine of that type is placed later, it gets the correct `MachineCapability` from the standard placement trigger. |

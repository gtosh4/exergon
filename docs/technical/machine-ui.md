# Machine UI Technical Design

Side-rail overlay for machine configuration and status monitoring. Primary surface for job policy, port binding, and passive recipe management. Read `ui.md §Machine UI` for layout and mockups, `crafting.md §4–5` for recipe execution, `networks.md §2–3` for port and power structure.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Opening the Machine UI](#2-opening-the-machine-ui)
3. [ECS Structure](#3-ecs-structure)
4. [Left Rail](#4-left-rail)
5. [Right Pane — Recipe Table](#5-right-pane--recipe-table)
6. [Systems](#6-systems)
7. [Messages](#7-messages)
8. [Execution Order](#8-execution-order)
9. [Vertical Slice Scope](#9-vertical-slice-scope)
10. [Edge Cases](#10-edge-cases)

---

## 1. Overview

The Machine UI is a side-rail overlay opened when the player interacts with a placed machine in the world. It does not pause simulation — the machine continues running while the UI is open.

**Left rail** — machine identity (type, tier, player name), per-slot recipe progress, power status, module slots, port binding configuration.

**Right pane** — recipe table: all recipes in `MachineCapability`, with per-recipe C (craft) and P (passive) flags and priority editing.

This document also defines the **revised `MachineJobPolicy`** structure that replaces the single-enum form in `crafting.md §4`. The per-recipe model (`per_recipe: HashMap<RecipeId, RecipePolicy>`) carries independent C/P flags per recipe, with machine-level `CraftingJobMode` and `passive: bool` as defaults. `crafting.md §4` defers to this document for the canonical `MachineJobPolicy` definition.

---

## 2. Opening the Machine UI

### Interaction model

Player triggers `{kbd:interact}` (default `E`; see `input.md §3.2`) while aimed at a machine within reach → `MachineInteractInput { machine: Entity }`.

Closes when:
- Player triggers `{kbd:cancel}` or clicks the close button
- The focused machine entity is despawned
- Player re-interacts with the same machine (toggle)

### FocusedMachine resource

```rust
#[derive(Resource, Default)]
pub struct FocusedMachine {
    pub entity: Option<Entity>,
}
```

`machine_ui_open_system` sets `FocusedMachine.entity` on `MachineInteractInput`. Observes `OnRemove<Machine>` to clear if the focused machine is despawned.

### Alert jump-to

The alerts dropdown (HUD top bar) lists machines with active block reasons. Clicking an entry emits `MachineInteractInput { machine }` to open that machine's UI directly.

---

## 3. ECS Structure

### Components the UI reads (display)

| Component | Entity | Used for |
|---|---|---|
| `MachineDef` | Registry lookup | Type display name, tier label, port names, module slot count |
| `Machine` | Machine entity | `machine_type`, `machine_tier` |
| `Name` | Machine entity | Player-assigned name (bevy standard `Name` component) |
| `RecipeProcessor` | Machine entity | Per-slot progress and state |
| `MachineCapability` | Machine entity | Recipe list for the recipe table |
| `MachineJobPolicy` | Machine entity | Current auto/passive configuration |
| `MachineModifierState` | Machine entity | Speed/efficiency from installed modules |
| `MachineLogisticsPorts` | Machine entity | Lists logistics port entities |
| `MachineEnergyPorts` | Machine entity | Lists energy port entities |
| `PortPolicy` | Port entity | Per-port and per-item flow mode |
| `LogisticsNetworkMember` | Port entity | Which network this port is on |
| `NetworkLabel` | Network entity | Network name for port binding display |
| `PowerNetworkMember` | Port entity | Which power network this port is on |
| `PowerNetworkMembers` | Power network entity | `voltage_tier()`, `amp_capacity()`, `amps_in_use()` |
| `SlotBlockReason` | Machine entity | Reason text for blocked slots |
| `RecipeGraph` | Resource | Recipe names, inputs, outputs, rates |
| `TechTreeProgress` | Resource | `unlocked_recipes` for lock icon display |

### Components the UI writes

| Component | Entity | Action |
|---|---|---|
| `MachineJobPolicy` | Machine entity | C/P flag, priority edits |
| `PortPolicy` | Port entity | Flow mode and item override edits |
| `Name` | Machine entity | Player rename |

### MachineJobPolicy — revised structure

This definition supersedes `crafting.md §4`. The single `JobMode` enum is replaced with a per-recipe model.

```rust
#[derive(Component)]
pub struct MachineJobPolicy {
    pub crafting_mode: CraftingJobMode,
    pub passive: bool,                        // machine-wide passive default
    pub per_recipe: HashMap<RecipeId, RecipePolicy>,
}

/// Per-recipe overrides. None on a field = use machine-level default.
pub struct RecipePolicy {
    pub passive: Option<bool>,                   // None = use MachineJobPolicy.passive
    pub crafting_mode: Option<CraftingJobMode>,  // None = use MachineJobPolicy.crafting_mode
                                                 // Some(Craft { priority, .. }) = force-include with per-recipe priority
                                                 // Some(Excluded) = force-exclude even if machine is Craft
}

pub enum CraftingJobMode {
    Craft {
        priority: i32,
        category_filter: Option<RecipeCategory>,
    },
    Excluded,
}
```

**C flag effective state** (per recipe):
- `per_recipe[id].crafting_mode == Some(Craft { .. })` → **ON** (per-recipe override, uses its own priority)
- `per_recipe[id].crafting_mode == Some(Excluded)` → **OFF** (force-excluded)
- `per_recipe[id].crafting_mode == None` and machine `crafting_mode == Craft` → **ON** (inherits machine priority)
- `per_recipe[id].crafting_mode == None` and machine `crafting_mode == Excluded` → **OFF**

**P flag effective state** (per recipe):
- `per_recipe[id].passive == Some(true)` → **ON** (force passive on)
- `per_recipe[id].passive == Some(false)` → **OFF** (force passive off)
- `per_recipe[id].passive == None` → inherits `MachineJobPolicy.passive`

**Passive runtime:** `MachineJobPolicy` is **pure config** — no runtime state. `RecipePolicy.passive: Option<bool>` declares intent. Actual slot state lives in `RecipeProcessor.slots`. `passive_recipe_system` each tick: cross-references passive-flagged recipes against occupied slots; starts a passive recipe on any free slot where it is not already running.

**Slot assignment:** Slots are parallel processors of the machine's single policy — no per-slot configuration. `passive_recipe_system` fills free slots with passive recipes first (any recipe where effective passive = true that is not currently running, ordered by effective priority). `job_dispatcher_system` fills remaining free slots: checks `crafting_mode` and per-recipe overrides, dispatches to any available slot.

**Passive limit (network-wide, future scope):** Level-maintainer behavior ("keep N items in the network") is deferred to a future `ProductionGoal` component that operates network-wide rather than per-machine. The passive P flag is a simple boolean for now.

### Machine naming

Machines use Bevy's standard `Name` component. An `Observer` on `OnAdd<Machine>` inserts `Name("{TypeDisplayName} #{entity_generation}")` as the default. Player renames write directly to `Name`. Persisted on the machine entity in run save.

### SlotBlockReason component

Written by `slot_block_reason_system` on block events. Cleared per-slot on `RecipeStarted`.

```rust
#[derive(Component, Default)]
pub struct SlotBlockReason {
    pub reasons: Vec<Option<BlockReason>>,  // indexed by slot; len matches RecipeProcessor.slots
}

pub enum BlockReason {
    VoltageTooLow { required: u8, current: u8 },
    AmpsFull,
    MissingInputs { items: Vec<ItemId> },
    NoOutputRoute { items: Vec<ItemId> },
    CatalystUnavailable { items: Vec<ItemId> },
}
```

---

## 4. Left Rail

### 4.1 Identity section

| Field | Source |
|---|---|
| Type name | `MachineDef.display_name` for `Machine.machine_type` |
| Tier badge | `Machine.machine_tier` → "Tier N" label |
| Machine name | `Name` component |
| Rename | `{kbd:ui_rename}` on name field → inline text edit → `MachineRenamed { machine, name }` |

### 4.2 Progress section

One progress bar per slot in `RecipeProcessor.slots`.

| Slot state | Display |
|---|---|
| `Idle` | "Idle" · empty bar |
| `Running` | Recipe display name · progress % · fill bar · effective cycle rate (items/min) |
| `PowerPaused` | Recipe display name · "⚡ Power paused" · frozen bar |
| Blocked | Recipe display name (last attempted) · block reason from `SlotBlockReason.reasons[slot]` |

Effective cycle rate = `recipe.primary_output_qty / (recipe.processing_time * speed_multiplier) * 60.0` items/min.

Block reason display examples:
- `VoltageTooLow { required: 2, current: 1 }` → "Voltage too low (need Tier 2, grid is Tier 1)"
- `MissingInputs { items }` → "Missing: iron_ore (×3)"
- `AmpsFull` → "Amp capacity full — waiting for headroom"

### 4.3 Power section

| Field | Source |
|---|---|
| Connection status | Any energy port in `MachineEnergyPorts` has `PowerNetworkMember` |
| Grid voltage tier | `PowerNetworkMembers.voltage_tier()` on connected power network |
| Amps held | Sum of draw rates for all `Running` and `PowerPaused` slots |
| Amp headroom | `PowerNetworkMembers.amp_capacity() - amps_in_use()` |
| Current draw | Sum of active slot draw rates (display with SI metric prefix, e.g. `560 EU/t`, `1.2 kEU/t`) |
| Voltage requirement | Max `min_voltage_tier` across all non-excluded recipes in `MachineCapability.capable` |

All numeric quantities (EU/t, item counts) display with SI metric prefixes (k/M/G…). Use a crate (e.g. `uom`, `unit-prefix` or similar on crates.io) or a small custom formatter; do not hand-roll per call-site.

Status line:

| Condition | Status |
|---|---|
| No energy port has `PowerNetworkMember` | **Disconnected** |
| `grid_voltage < voltage_requirement` | **Voltage blocked** — recipes requiring higher voltage cannot start |
| `amp_headroom == 0` | **Amp full** — waiting for a running recipe to complete |
| All checks pass | **OK** |

### 4.4 Module slots section

One entry per module slot defined in `MachineDef.module_slots` for the current tier. Read-only — module attachment and detachment are handled by the module snap system.

| Field | Source |
|---|---|
| Slot label | `MachineDef.module_slots[i].label` |
| Module installed | Query for module entity snapped to this slot (module system) |
| Speed modifier | `MachineModifierState.speed_multiplier` (aggregate across all installed modules) |
| Efficiency modifier | `MachineModifierState.efficiency_multiplier` |

Empty slots show a placeholder with the slot kind (speed / efficiency / parallel).

### 4.5 Port binding section

`PortPolicy` component shape and runtime semantics are defined in `networks.md §2` (canonical source). This section covers the editing UI only.

Lists all logistics ports from `MachineLogisticsPorts`. Energy ports shown separately as a read-only list.

**Per logistics port entry:**

| Field | Source |
|---|---|
| Port name | `MachineDef` port label at this port position |
| Connected network | `LogisticsNetworkMember` → `NetworkLabel.name`; "Unconnected" if absent |
| Default flow mode | `PortPolicy.default_mode`: use icons from ui.md Machine UI mockup — `+` In / `−` Out / `%` Both / `⊙` None (4-button toggle row as shown in mock) |

Clicking a flow mode icon emits `PortPolicyEdit { port, edit: SetDefaultMode(mode) }`. Use mock icons, not text symbols.

**Per-item override rows** (expand/collapse per port):

| Field | Source / Write target |
|---|---|
| Item name | Content display name |
| Flow mode | `PortPolicy.item_overrides[item_id]` |

Adding an override: player types item name or drags from Terminal → creates entry.
Removing an override: X button → emits `PortPolicyEdit { port, edit: RemoveItemOverride { item_id } }`.

Changes take effect immediately. The next `recipe_start_system` and `recipe_completion_system` invocations read the updated `PortPolicy`.

**Energy ports** (read-only):

| Field | Source |
|---|---|
| Port name | `MachineDef` port label |
| Connected power network | `PowerNetworkMember` → `PowerNetworkMembers` network entity → `NetworkLabel.name`; "Unconnected" if absent |

---

## 5. Right Pane — Recipe Table

Populated from `MachineCapability.capable`. One row per recipe. Default sort: **Passive** (P = true, by priority desc) → **Craft** (C effectively ON, by priority desc) → **Disabled** (C and P both off, alphabetical) → **Locked** (not in `TechTreeProgress.unlocked_recipes`, greyed, lock icon, alphabetical).

### 5.1 Columns

| Column | Source |
|---|---|
| Recipe name | `RecipeGraph.recipes[recipe_id].display_name` |
| Inputs → Outputs | Abbreviated item stacks, e.g. "3× ore → 2× ingot + 1× slag" |
| Cycle rate | `primary_output_qty / processing_time * 60.0` items/min (base, no modifiers) |
| C | Autocraft eligibility flag (see §5.2) |
| P | Passive toggle: Off / On (see §5.2) |
| Priority | Auto-craft priority integer; blank if C is off |

Locked recipes (not in `TechTreeProgress.unlocked_recipes`): greyed row, lock icon, C and P non-interactive.

### 5.2 Mode flags

**C flag (craft eligible):**

- **ON** — `per_recipe[id].crafting_mode == Some(Craft { .. })`, OR `crafting_mode == None` and machine `crafting_mode == Craft`.
- **OFF** — `per_recipe[id].crafting_mode == Some(Excluded)`, or both `None` and machine `crafting_mode == Excluded`.
- Setting C ON when recipe has `Some(Excluded)`: clears to `None` (reverts to machine default).
- Setting C ON on any recipe when machine `crafting_mode == Excluded` with no per-recipe override: switches machine `crafting_mode` to `Craft { priority: 0, category_filter: None }`. UI shows a one-time hint: "Machine now accepts craft jobs."

**P flag (passive):**

- **Off** — effective passive = false (per-recipe `Some(false)` or `None` with machine `passive == false`).
- **On** — effective passive = true (per-recipe `Some(true)` or `None` with machine `passive == true`).
- Multiple recipes can be P-flagged simultaneously. `passive_recipe_system` fills free slots with P-flagged recipes ordered by effective priority. If passive recipes outnumber slots, lower-priority ones wait for a slot to free.

**Override indicator:**

C and P are two-state toggles (ON/OFF). When a recipe has an explicit per-recipe override (`RecipePolicy` field is `Some(...)`), the flag renders with a small indicator dot. No dot = value inherited from machine default. `{kbd:ui_context_menu}` on a flag with a dot shows a "Reset to default" action, which emits `SetRecipeCraftingMode { mode: None }` or `SetRecipePassive { passive: None }` to clear the override. `{kbd:ui_context_menu}` on an inherited flag has no action.

**Priority field:**

Editable integer. Sets `per_recipe[id].crafting_mode = Some(Craft { priority: value, category_filter: None })` (upserts). Clearing it sets `crafting_mode = None` (reverts to machine default). Higher integer = preferred by `job_dispatcher_system` and `passive_recipe_system`.

### 5.3 Edits

All recipe table edits emit `MachinePolicyEdit` events consumed by `machine_policy_edit_system`.

```rust
pub enum MachinePolicyEdit {
    SetRecipePassive { machine: Entity, recipe_id: RecipeId, passive: Option<bool> },
    SetMachinePassive { machine: Entity, passive: bool },
    SetRecipeCraftingMode { machine: Entity, recipe_id: RecipeId, mode: Option<CraftingJobMode> },
    SetMachineCraftingMode { machine: Entity, mode: CraftingJobMode },
}
```

After each edit, `machine_policy_edit_system` emits `MachineJobPolicyChanged { machine }`.

---

## 6. Systems

| System | Trigger | Purpose |
|---|---|---|
| `machine_ui_open_system` | `MachineInteractInput`, `OnRemove<Machine>` | Set/clear `FocusedMachine`; toggle on re-interact |
| `machine_ui_display_system` | `FocusedMachine` change; `RecipeStarted`, `RecipeComplete`, `RecipePowerPaused`, `RecipePowerResumed`, `RecipeBlocked*`; `NetworkChanged<Power>`, `NetworkChanged<Logistics>`; `MachineCapabilityUpdated`, `MachineJobPolicyChanged` | Rebuild display state for focused machine; push to UI layer |
| `slot_block_reason_system` | `RecipeBlocked*` events | Write `SlotBlockReason.reasons[slot]`; clear on `RecipeStarted` for that slot |
| `machine_policy_edit_system` | `MachinePolicyEdit` | Mutate `MachineJobPolicy`; emit `MachineJobPolicyChanged` |
| `port_policy_edit_system` | `PortPolicyEdit` | Mutate `PortPolicy`; emit `NetworkChanged<Logistics>` for the port's network |
| `machine_rename_system` | `MachineRenamed` | Update `Name` component |

`machine_ui_display_system` is UI-only. It runs in the UI schedule after `LogisticsSimSystems` each frame so display reflects same-frame simulation results.

---

## 7. Messages

| Message | Payload | Emitted by |
|---|---|---|
| `MachineInteractInput` | `machine: Entity` | Player interact input, alert jump-to |
| `MachineRenamed` | `machine: Entity, name: String` | Machine UI inline rename |
| `MachinePolicyEdit` | `MachinePolicyEdit` enum | Machine UI C/P/priority/limit controls |
| `MachineJobPolicyChanged` | `machine: Entity` | `machine_policy_edit_system` |
| `PortPolicyEdit` | `port: Entity, edit: PortPolicyEditKind` | Machine UI port binding controls |

```rust
pub enum PortPolicyEditKind {
    SetDefaultMode(PortMode),
    SetItemOverride { item_id: ItemId, mode: PortMode },
    RemoveItemOverride { item_id: ItemId },
}
```

`MachineJobPolicyChanged` consumed by:
- `job_dispatcher_system` — re-evaluate queued jobs against updated policy
- `passive_recipe_system` — re-evaluate whether passive should start or stop

---

## 8. Execution Order

```
[LogisticsSimSystems]
├── recipe_start_system     → RecipeStarted / RecipeBlocked*
├── recipe_progress_system  → RecipePowerPaused / RecipePowerResumed / RecipeComplete
├── recipe_completion_system → RecipeComplete → NetworkStorageChanged
│
└── slot_block_reason_system   (RecipeBlocked* → write SlotBlockReason)
                                (RecipeStarted → clear SlotBlockReason for slot)

[UI Systems — after LogisticsSimSystems]
└── machine_ui_display_system  (state-change events → push display data to UI)

[Event-driven, any frame — before LogisticsSimSystems in same frame]
├── machine_ui_open_system       (MachineInteractInput → FocusedMachine)
├── machine_policy_edit_system   (MachinePolicyEdit → MachineJobPolicy)
│       └─ MachineJobPolicyChanged → job_dispatcher_system (same frame)
│                                 → passive_recipe_system (same frame)
├── port_policy_edit_system      (PortPolicyEdit → PortPolicy + NetworkChanged<Logistics>)
└── machine_rename_system        (MachineRenamed → Name)
```

`machine_policy_edit_system` runs before `job_dispatcher_system` and `passive_recipe_system` — a policy edit takes effect within the same frame's dispatch cycle.

---

## 9. Vertical Slice Scope

| Feature | VS | MVP |
|---|---|---|
| `FocusedMachine` resource + open/close | ✓ | ✓ |
| Identity section (type, tier display) | ✓ | ✓ |
| `Name` (bevy standard) + player rename | — | ✓ |
| Progress bar (single slot) | ✓ | ✓ |
| Slot state: Running / Idle / PowerPaused | ✓ | ✓ |
| `SlotBlockReason` + block reason display | — | ✓ |
| Power section (connection, voltage, amp) | — | ✓ |
| Module slots section | — | ✓ |
| Port binding section — read-only (connected network name) | — | ✓ |
| Port binding section — flow mode editing (`PortPolicyEdit`) | — | ✓ |
| Recipe table — full (C/P flags, priority editing) | — | ✓ |
| Recipe table — VS stub (P flag only, hardcoded recipe, no C flag) | ✓ | — |
| Craft mode category filter | — | ✓ |
| Alert jump-to machine UI | — | ✓ |

**VS simplifications:**
- One hardcoded recipe is P-flagged at machine spawn (`per_recipe[id].passive = true`). No recipe table editing.
- Progress section only: type/tier label, recipe name, progress bar, Running/Idle/PowerPaused state.
- No port binding UI. `PortPolicy` defaults (Both) apply; no editing surface.
- No power section UI. Power blocking still functions; diagnosis via debug output.
- No player rename (`Name` gets default only).

---

## 10. Edge Cases

| Case | Behavior |
|---|---|
| Machine despawned while UI open | `machine_ui_open_system` observes `OnRemove<Machine>` for `FocusedMachine.entity`; clears `FocusedMachine`, closes UI. |
| Player sets P flag on locked recipe | P selector is non-interactive for locked recipes. No event emitted. |
| Player disables P on a recipe while it is running | `per_recipe[id].passive = false`. In-progress run completes normally. `passive_recipe_system` does not restart that recipe on next slot idle. |
| Player disables C on recipe currently InProgress | In-progress job runs to completion. `per_recipe[id].crafting_mode = Some(Excluded)` only affects future dispatch. |
| Player disables C on all recipes while a job is `Dispatched` (not yet started) | `MachineJobPolicyChanged` fires. `job_dispatcher_system` does not un-assign already-dispatched jobs. Job stays assigned; `recipe_start_system` retries per its normal triggers. Player must wait or cancel the plan via Terminal. |
| More passive recipes than slots | `passive_recipe_system` fills each free slot with the highest-priority passive recipe not already running. Lower-priority passive recipes wait. If all slots are busy with passive, no auto jobs are dispatched until a passive completes and a slot frees. |
| Machine has 1 slot; passive + auto both configured | Passive fills the single slot. `job_dispatcher_system` finds no free slot and skips machine. Machine UI shows hint: "Install parallel slot module to enable concurrent craft jobs." |
| Port policy edited while recipe is running | Change takes effect immediately on the component. Inputs were already consumed at start (`recipe_start_system`). New `PortPolicy` affects output routing in `recipe_completion_system` for the current run and all subsequent ones. |
| Player opens UI for a machine on a network the player's body is not on | UI opens normally — no network restriction. Port binding shows connected network names for that machine's ports regardless of player body location. |

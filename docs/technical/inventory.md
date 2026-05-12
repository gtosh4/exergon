# Inventory System Design

ECS components, system step-by-step logic, events/messages, and edge cases for the inventory layer. Read `gdd.md §10` (factory layer, unified storage), `technical-design.md §6` (logistics network), `networks.md §2` (StorageUnit), and `ui.md §Terminal` for context. Covers the hotbar, drone inventory, storage units, Terminal screen runtime data, and the goal tracker.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Hotbar](#2-hotbar)
3. [Drone Inventory](#3-drone-inventory)
4. [Storage Units](#4-storage-units)
5. [Terminal Screen Runtime Data](#5-terminal-screen-runtime-data)
6. [Goal Tracker](#6-goal-tracker)
7. [Messages](#7-messages)
8. [Execution Order](#8-execution-order)
9. [Vertical Slice Scope](#9-vertical-slice-scope)
10. [Edge Cases](#10-edge-cases)

---

## 1. Overview

The player AI has **no personal inventory**. Items exist either in a logistics network's unified storage or in a drone's local buffer — never "in the player's pockets." The hotbar is a **network-linked shortcut panel**: each slot is a configured reference to an item type whose quantity is read live from the active logistics network. Using a hotbar slot either consumes an item from that network (placeables) or equips a tool that remains in storage.

This model flows directly from `gdd.md §10` ("storage is a necessary system but not a primary design constraint") and Pillar 2 — the player's attention is on the production graph, not on inventory management.

Three subsystems form the inventory layer:

- **Hotbar** — 9 visible slots across 3 banks; each slot is a configured item reference pointing into the active logistics network. Holds tools and placeables only.
- **Drone inventory** — per-drone item buffer filled during Remote mode (manual mining, sample collection). Deposited into a logistics network by interacting with a logistics port.
- **Terminal screen** — full read/write view of a logistics network's unified storage; see `ui.md §Terminal`. This document covers the runtime data it queries, not the UI layout.

---

## 2. Hotbar

### Design model

The hotbar contains **tools and placeable objects** (machines, cables, modules). Raw materials and items-in-transit are not hand-carried — they live in the network. Each slot stores a configured `ItemId`; quantities are read live from the active network each time the slot is displayed or used.

Selecting a hotbar slot has two behaviors by item kind:

- **Placeable** (machine, cable, beacon, etc.): enters placement preview mode. On confirm, consumes one unit from the active network.
- **Tool** (mining drill, scanner, etc.): equips the tool for active use. Does not consume from storage — the tool remains in the network, logically checked out.

The active network is determined by the player body's current outpost. When the player switches network tabs in the Terminal, the hotbar's active network updates to match.

### ECS structure

```
Player entity
└── ActiveHotbarSlot { bank: u8, slot_index: u8 }  ← which slot is highlighted (for UI styling)

Body entity
├── HotbarConfig { banks: [HotbarBank; 3], active_bank: u8 }
└── HotbarNetworkLink(Entity)   ← the LogisticsNetwork this body's hotbar reads from
```

```rust
pub struct HotbarBank {
    pub slots: [HotbarSlot; 9],
}

pub struct HotbarSlot {
    pub item_id: Option<ItemId>,  // None = unconfigured
}
```

`HotbarConfig` stores configuration only — no quantities or item copies. Quantities are read from the linked logistics network on `NetworkStorageChanged` and on tab switch.

`ActiveHotbarSlot` records which bank/slot index is currently selected. The UI uses this to highlight the active slot. Systems that need the currently equipped tool resolve it via `HotbarConfig.banks[active.bank].slots[active.slot_index]`. Cleared (set to an invalid/sentinel value) when the player selects an empty slot or switches bodies.

`HotbarNetworkLink(Entity)` — points to the `LogisticsNetwork` entity this body's hotbar reads from. Updated when the player changes the active tab in the Terminal (`TerminalTabChanged` event). Defaults to the primary network of the body's outpost on spawn.

`HotbarConfig` and `HotbarNetworkLink` are components on the **body entity**, not the player entity. Each body retains its own hotbar configuration and active network link independently. When the player switches bodies, `hotbar_display_system` reads from the new body's components — no migration needed. `HotbarConfig` and `HotbarNetworkLink` are persisted in the run save on the body entity.

### Bank switching

Banks A/B/C map to `banks[0]`, `banks[1]`, `banks[2]`. `active_bank` is updated on Shift+Scroll. The 9 visible slots are always `banks[active_bank].slots`. Bank switching fires `HotbarBankChanged { active_bank }` so the UI layer redraws.

### Slot configuration

Players configure slots by dragging an item from the Terminal item table into a hotbar slot. Drag-to-slot emits `HotbarSlotSet { bank, slot_index, item_id }`. Right-clicking a slot emits `HotbarSlotCleared { bank, slot_index }`. Configuration is persisted in the run save on the body entity.

An unconfigured slot shows empty. A configured slot shows the item name and live quantity. Quantity = 0 renders the slot as depleted (item name greyed, qty = 0) but the configuration is preserved — the slot will repopulate when items are produced.

### Placement behavior

**System:** `hotbar_place_system`

**Trigger:** Player selects a slot configured to a placeable item and triggers placement.

Step by step:

1. Read `HotbarNetworkLink` to get the active network entity.
2. Call `LogisticsNetworkMembers.count_items(item_id)` on the active network (subtracts `NetworkReservations`). Check result >= 1.
3. If unavailable: block with "insufficient stock" feedback. Emit `PlacementBlocked { item_id, reason: InsufficientStock }`. No automatic craft initiation — player opens Terminal to queue a craft manually.
4. If available: enter placement preview. Preview renders a ghost of the object at cursor position.
5. On confirm: call `take_items(item_id, 1)` on the active network. Spawn the placed world object. Emit `NetworkStorageChanged { network }`.
6. On cancel: exit preview. No items consumed.

### Tool equip behavior

**System:** `hotbar_equip_system`

**Trigger:** Player selects a slot configured to a tool item.

Step by step:

1. Read `HotbarNetworkLink`, call `count_items(item_id)` on the active network. Check result >= 1.
2. If unavailable: block; emit `EquipBlocked { item_id, reason: NotInNetwork }`.
3. If available: set `ActiveHotbarSlot { bank, slot_index }`. Emit `ActiveHotbarSlotChanged { bank, slot_index }`. The tool is not pulled from storage — it remains in the network, checked out by reference only. No reservation is placed.
4. De-equip (select empty slot or select a different tool): clear `ActiveHotbarSlot`. Emit `ActiveHotbarSlotChanged` with sentinel. Systems needing the equipped item resolve it via `HotbarConfig.banks[slot.bank].slots[slot.slot_index]`.

### System: `hotbar_display_system`

**Trigger:** `NetworkStorageChanged { network }` where `network == HotbarNetworkLink.0`, and `HotbarBankChanged`.

Step by step:

1. For each slot in all three banks (not just the active bank — other banks may be visible via Terminal):
   - If `item_id` is Some: call `count_items(item_id)` on the active network (non-destructive read; does not subtract reservations — shows physical qty for display).
   - If `item_id` is None: record quantity = 0.
2. Push updated quantities to the UI layer. No ECS mutation — quantities are display-only, not stored on components.

---

## 3. Drone Inventory

### Design model

Drones carry a local item buffer filled during Remote mode. Manual mining deposits ore into this buffer; drone sample collection adds samples. The buffer is not connected to any logistics network until the player explicitly flies the drone to a logistics port and triggers a deposit. The player chooses which network receives the items by choosing which port to interact with.

This is intentionally deliberate: the player must pilot back to a logistics port, making exploration range a real cost (see `gdd.md §11`). It preserves the tension between "go further" and "unload what I have."

Drone ECS structure (non-inventory components) is in `drone.md`. This section covers only `DroneInventory` and deposit mechanics.

### ECS components

```rust
#[derive(Component)]
pub struct DroneInventory {
    pub items: HashMap<ItemId, u32>,
    pub max_slots: u32,   // max distinct item types; set from drone tier data
    pub max_mass_kg: f32, // mass capacity; set from drone tier data
}
```

`DroneInventory` is present on all drone entities, empty on spawn. `max_slots` and `max_mass_kg` come from the drone tier's content asset. Higher-tier drones have larger buffers.

Mass in use = `sum of (item_mass_kg(item_id) * qty)` across all entries. This is computed on each add — not stored.

### Collection

**System:** `drone_collection_system`

**Trigger:** `DroneInteractEvent { drone, target }` while player is in Remote mode on that drone.

Step by step:

1. Determine item type and quantity from the target (deposit distribution sample for ore; sample type for biome samples — see `mining.md` and `drone.md`).
2. Capacity check:
   - If `DroneInventory.items.len() >= max_slots` and `item_id` not already present: block; emit `DroneCollectionBlocked { drone, reason: SlotsFull }`. Player must deposit before collecting more distinct types.
   - If `current_mass + (item_mass_kg(item_id) * quantity) > max_mass_kg`: reduce quantity to what fits; if zero fits, emit `DroneCollectionBlocked { drone, reason: MassFull }`.
3. Increment `DroneInventory.items[item_id] += quantity`.
4. Emit `DroneInventoryChanged { drone }` (for UI update in Remote mode HUD).

Sample items are regular `ItemId`s (e.g. `"sample_rock_geological"`, `"sample_flora_biological"`). They occupy drone inventory slots and are deposited into the logistics network like any other item. Analysis stations pull them as recipe inputs from the network.

### Deposit

**System:** `drone_deposit_system`

**Trigger:** Player triggers deposit action while drone is within interaction range of a logistics port collider (same proximity check as cable placement).

Step by step:

1. Resolve target network: read `LogisticsNetworkMember` on the targeted port entity.
2. If port has no `LogisticsNetworkMember` (not connected to any network): emit `DroneDepositFailed { drone, reason: PortNotConnected }`. Inventory unchanged.
3. For each `(item_id, qty)` in `DroneInventory.items`, call `give_items(item_id, qty)` on the target network's `LogisticsNetworkMembers`. Track overflow (qty that could not be stored due to network mass capacity).
4. Remove deposited entries from `DroneInventory.items`. If overflow occurred: leave overflow quantities in inventory.
5. Emit `NetworkStorageChanged { network }` for the target network.
6. If no overflow: emit `DroneDeposited { drone, network, items }`.
7. If overflow: emit `DroneDepositPartial { drone, network, deposited, overflow }`. Player must expand storage or deposit to a different network.

The player selects which network receives items by choosing which port to interact with — a port on the main network deposits to the main network, a port on a subnet deposits to that subnet. This is the same selection mechanism used for cable placement.

---

## 4. Storage Units

### StorageUnit component

`StorageUnit { items: HashMap<String, u32> }` from `networks.md §2`. One per storage machine entity. `LogisticsNetworkMembers` aggregates across all `StorageUnit`s in the network — items are not centralized, the network is the read/write interface.

### Capacity model

Each storage machine type defines in its content asset:

```
StorageDef {
    max_mass_kg: f32,   // total mass this unit can hold
}
```

Mass in use = `sum of (item_mass_kg(item_id) * qty)` across all entries in the unit's `StorageUnit.items`. `give_items` on `LogisticsNetworkMembers` distributes items across member `StorageUnit`s in priority order (lower-indexed units fill first) and returns any overflow that exceeds all units' mass capacity combined.

Network-level capacity aggregates:

- **Total mass capacity** = `sum of StorageDef.max_mass_kg` across all storage machines in the network.
- **Total mass used** = `sum of (item_mass_kg(item_id) * qty)` across all items in the network.
- **Unique types** = count of distinct `ItemId`s with qty > 0 across all `StorageUnit`s in the network.

These aggregates power the Terminal header bars. They are recomputed on `NetworkStorageChanged`, not per-tick.

### Unique types — display metric

The "unique cells" bar in the Terminal header is a **display metric only** — there is no hard cap on distinct item types stored per unit or per network. The count informs players of storage diversity but does not block item acceptance. This is consistent with `gdd.md §10`: "storage is a necessary system but not a primary design constraint."

The architecture records the count from day one. If a hard type-slot mechanic is introduced post-MVP, it requires only adding a cap check in `give_items` — no structural changes.

### Item mass metadata

Each item type carries `mass_kg: f32` in its `ItemDef` content asset. The item registry (not `RecipeGraph`) is the authority for this value — mass is an item property independent of any recipe. Systems read it as `item_registry.get(item_id).mass_kg`. This value populates:

- The "kg/ea" and "kg total" columns in the Terminal item table.
- Mass capacity consumption on `give_items` and `drone_deposit_system`.

---

## 5. Terminal Screen Runtime Data

The Terminal screen (see `ui.md §Terminal`) queries live ECS state. This section covers what data it reads and how — not the UI layout.

### Item table

The item table shows one row per distinct `ItemId` held in the active network. All rows are recomputed on `NetworkStorageChanged` for the active network. No per-tick polling.

| Column | Source |
|---|---|
| Item name | Content asset display name for `ItemId` |
| Qty | `LogisticsNetworkMembers.count_items(item_id)` — non-destructive read; subtracts `NetworkReservations` so reserved units show as unavailable |
| Δ/min | Computed from `NetworkFlowLedger` on the active network (see §5.1) |
| kg/ea | `item_registry.get(item_id).mass_kg` (from `ItemDef` content asset) |
| kg total | qty × kg/ea |
| Craftable flag | 1. Find all unlocked recipes with `item_id` as output. 2. For each, check if any machine in the active network has a job policy that accepts that recipe. True if any match found. |

Items with qty = 0 (reserved by active crafting plans) are shown with qty = 0 and greyed, not hidden, so players can see what is pending production.

#### 5.1 NetworkFlowLedger

Δ/min requires a rolling window of net item flow. This is tracked by `NetworkFlowLedger`:

```rust
#[derive(Component)]
pub struct NetworkFlowLedger {
    pub window_seconds: f32,               // rolling window length (default: 60.0)
    pub events: VecDeque<FlowEvent>,
}

pub struct FlowEvent {
    pub timestamp: f32,  // game time in seconds
    pub item_id: ItemId,
    pub delta: i32,      // positive = produced/deposited, negative = consumed
}
```

`NetworkFlowLedger` lives on each `LogisticsNetwork` entity. It is inserted by `LogisticsSimPlugin` when a logistics network is spawned.

**Writers** (all push to the network's ledger):

| Writer | Delta direction | Event |
|---|---|---|
| `recipe_completion_system` | Positive (outputs produced) | Fired after `give_items` on output |
| `miner_tick_system` | Positive (ore deposited) | Fired after `give_items` |
| `drone_deposit_system` | Positive (drone items deposited) | Fired after `give_items` |
| `recipe_start_system` | Negative (inputs consumed) | Fired after `take_items` on inputs |

**Read:** On `NetworkStorageChanged`, `hotbar_display_system` and the Terminal item table query the ledger for each displayed item:

```
delta_per_window = sum of ledger.events
    where timestamp >= (now - window_seconds)
    and item_id == query_item_id
delta_per_min = delta_per_window * (60.0 / window_seconds)
```

The ledger trims events outside the window at query time, not per-tick. Ledger growth is bounded: max events = `tick_rate * unique_item_flows_per_tick * window_seconds`. On `NetworkChanged<Logistics>` (network split/merge), the ledger on the surviving network entity is preserved; new fragment networks start with empty ledgers.

### Network tabs

Tabs at the Terminal top correspond to logistics networks reachable from the player's current body. "Reachable" means: the network the body's outpost is on (main), plus any networks bridged via interface blocks to that network. Tab names come from `NetworkLabel { name: String }` on the network entity, defaulting to `"Network N"` (auto-incrementing). Players rename networks by double-clicking a tab header; this sets `NetworkLabel.name` and is persisted in the run save.

Switching tabs emits `TerminalTabChanged { network: Entity }`, which updates `HotbarNetworkLink` on the player entity so the hotbar reflects the selected network.

### Header strip

| Element | Source |
|---|---|
| Mass capacity bar | `(total_mass_used / total_mass_capacity)` across all `StorageUnit`s in active network |
| Unique cells bar | Count of distinct `ItemId`s with qty > 0 in active network (display metric; see §4) |
| Power bar | `PowerNetworkMembers.amps_in_use() / PowerNetworkMembers.amp_capacity()` for the power network at the player's current body location |

The power bar reads the power network of the outpost the player is currently inhabiting, not the network of the Terminal's active tab. It reflects "how loaded is my local power grid right now," not the selected subnet.

### CRAFT button

The CRAFT button in the Terminal item table opens the craft modal for the selected item. On confirm, emits `RequestCraft { item_id, quantity, network: HotbarNetworkLink.0 }`. This is consumed by `crafting_plan_resolver_system`; see `crafting.md §8` for the full plan resolution flow.

### Saved filters

Saved filters (shown in the Terminal left sidebar) are UI-only state — a `Vec<TerminalFilter>` persisted in the run save on the player entity. They are not ECS components and have no effect on simulation systems. Each filter is a name string + predicate (item name substring, category tag, or craftable-only flag). Applying a filter restricts the Terminal item table rows to matching items; it does not affect `has_items` or any network query.

---

## 6. Goal Tracker

### Design model

Goals are created by **pinning** an item or node from another screen. There is no freeform text entry. The goal tracker shows in the Terminal left sidebar with a progress bar per goal.

Supported pin sources:

| Source screen | Pin action | Goal type created |
|---|---|---|
| Terminal item table | Right-click row → "Pin as goal…" (prompts for target qty) | `ItemQuantity` |
| Index (recipe browser) | Right-click item → "Set production goal…" (prompts for qty) | `ItemQuantity` |
| Tech Tree | Right-click unlocked or partially-revealed node → "Watch prerequisites" | `NodeUnlock` |

### ECS structure

`GoalTracker` is a `Component` on the player entity — not a `Resource`. Save-game data belongs on entities, not resources.

```rust
#[derive(Component, Default)]
pub struct GoalTracker {
    pub goals: Vec<GoalEntry>,
}

pub enum GoalEntry {
    ItemQuantity {
        item_id: ItemId,
        target_qty: u32,
        network: Entity,   // the LogisticsNetwork to read qty from
        label: String,     // display name; defaults to item display name
    },
    NodeUnlock {
        node_id: NodeId,
        label: String,     // display name; defaults to node display name
    },
}
```

`goal_tracker_system` queries the player entity for `GoalTracker`. `GoalTracker` is persisted in the run save as part of the player entity.

### Progress display

Goal progress bars are recomputed on:

- `NetworkStorageChanged { network }` — for `ItemQuantity` goals where `goal.network == network`.
- `TechNodeUnlocked { node }` — for `NodeUnlock` goals where `goal.node_id == node`.

Computation per goal type:

- **ItemQuantity**: `progress = min(current_qty / target_qty, 1.0)`. Display: `"current / target"` with progress bar. `current_qty` = `LogisticsNetworkMembers.count_items(item_id)` on `goal.network` (does not subtract reservations — shows physical qty so the player sees real stock including reserved items).
- **NodeUnlock**: binary — unlocked (100%) or not (0%). No intermediate. The goal shows the node name and its current visibility state (shadow / partial / full) as a status label.

### Goal lifecycle

**Pin:** Appends a `GoalEntry` to `GoalTracker.goals`. Emits `GoalPinned { goal_index }`.

Duplicate prevention:
- `ItemQuantity`: if an entry already exists for the same `(item_id, network)` pair, update `target_qty` instead of appending. Emits `GoalUpdated { goal_index }`.
- `NodeUnlock`: if an entry for the same `node_id` already exists, silently ignore (idempotent pin).

**Dismiss:** Player clicks X on a goal entry. Removes from `GoalTracker.goals` by index. Emits `GoalDismissed { goal_index }`. No other effect — dismissal does not cancel crafts or affect the network.

**Auto-completion:** Goals are **not auto-removed** when complete (progress = 1.0). The player sees the goal at 100% and dismisses manually. This is intentional — reaching a production target doesn't necessarily mean the player is done tracking it (they may want to maintain a stockpile above that level).

---

## 7. Messages

| Message | Payload | Emitted by |
|---|---|---|
| `HotbarSlotSet` | `bank: u8, slot_index: u8, item_id: ItemId` | UI drag-to-slot action |
| `HotbarSlotCleared` | `bank: u8, slot_index: u8` | UI right-click slot |
| `HotbarBankChanged` | `active_bank: u8` | Shift+Scroll input |
| `ActiveHotbarSlotChanged` | `bank: u8, slot_index: u8` (or sentinel for cleared) | `hotbar_equip_system` |
| `PlacementBlocked` | `item_id: ItemId, reason: PlacementBlockReason` | `hotbar_place_system` |
| `EquipBlocked` | `item_id: ItemId, reason: EquipBlockReason` | `hotbar_equip_system` |
| `TerminalTabChanged` | `network: Entity` | Terminal tab click |
| `DroneInventoryChanged` | `drone: Entity` | `drone_collection_system` |
| `DroneDeposited` | `drone: Entity, network: Entity, items: Vec<(ItemId, u32)>` | `drone_deposit_system` |
| `DroneDepositPartial` | `drone: Entity, network: Entity, deposited: Vec<(ItemId, u32)>, overflow: Vec<(ItemId, u32)>` | `drone_deposit_system` |
| `DroneDepositFailed` | `drone: Entity, reason: DepositFailReason` | `drone_deposit_system` |
| `DroneCollectionBlocked` | `drone: Entity, reason: CollectionBlockReason` | `drone_collection_system` |
| `GoalPinned` | `goal_index: usize` | UI pin action |
| `GoalUpdated` | `goal_index: usize` | UI pin action (duplicate item qty update) |
| `GoalDismissed` | `goal_index: usize` | UI dismiss action |

---

## 8. Execution Order

Inventory systems are event-driven, not tick-ordered, with two exceptions:

```
[LogisticsSimSystems]
├── ... (recipe_progress, recipe_completion, miner_tick — see crafting.md §11)
│       └─ NetworkStorageChanged → hotbar_display_system
│                                → Terminal item table redraw (UI layer)
│                                → goal_tracker_system (ItemQuantity goals)

[Event-driven, outside tick ordering]
├── hotbar_place_system          (player placement input)
├── hotbar_equip_system          (player slot selection input)
├── drone_collection_system      (DroneInteractEvent)
├── drone_deposit_system         (player deposit input)
└── goal_tracker_system          (NetworkStorageChanged, TechNodeUnlocked)
```

`hotbar_display_system` runs after `LogisticsSimSystems` complete each frame so it reflects the same-frame recipe outputs and miner deposits. It does not run mid-tick.

`drone_deposit_system` emits `NetworkStorageChanged` after deposit, which triggers the standard `recipe_start_system` evaluation for machines on the receiving network — deposited samples may unblock a stalled analysis station.

---

## 9. Vertical Slice Scope

| Feature | VS | MVP |
|---|---|---|
| HotbarConfig (single slot, tool only) | ✓ | ✓ |
| Multi-bank hotbar (A/B/C, shift+scroll) | — | ✓ |
| HotbarNetworkLink (hotbar reads from network) | — | ✓ |
| Hotbar slot configuration via Terminal drag | — | ✓ |
| hotbar_place_system (placement consumes from network) | ✓ | ✓ |
| hotbar_equip_system (tool equip by reference) | ✓ | ✓ |
| hotbar_display_system (live qty from network) | — | ✓ |
| DroneInventory + drone_collection_system | ✓ | ✓ |
| DroneInventory max_slots + max_mass_kg enforcement | — | ✓ |
| drone_deposit_system (port-targeted deposit) | — | ✓ |
| VS-simplified deposit (proximity auto-deposit within aegis) | ✓ | — |
| StorageUnit + mass capacity tracking | ✓ | ✓ |
| NetworkFlowLedger (Δ/min computation) | — | ✓ |
| Terminal item table (qty, Δ/min, kg, craftable) | — | ✓ |
| Network tabs (multi-network Terminal) | — | ✓ |
| NetworkLabel (tab renaming) | — | ✓ |
| Goal tracker (pinned goals, progress bars) | — | ✓ |
| Saved filters | — | ✓ |

**VS simplifications:**

- One hardcoded hotbar slot for the mining drill tool. No bank switching, no slot configuration UI.
- `DroneInventory` exists for sample and ore collection; deposit is proximity-triggered within the aegis field (no logistics port interaction needed — simplified to "return to base and auto-deposit to the single storage crate").
- No `NetworkFlowLedger` — Δ/min column omitted from any debug UI.
- No Terminal screen — raw debug readout of `StorageUnit.items` is sufficient for VS validation.
- No goal tracker.

---

## 10. Edge Cases

| Case | Behavior |
|---|---|
| Hotbar slot configured to an item; item qty drops to 0 in network | Slot displays qty = 0, rendered as depleted. Configuration preserved. Placement/equip blocks with "insufficient stock" on selection. |
| Player switches bodies while a tool is equipped (`ActiveHotbarSlot` set) | `BodySwitched` event clears `ActiveHotbarSlot`. `hotbar_display_system` reads the new body's `HotbarConfig` and `HotbarNetworkLink`. Old body retains its own `HotbarConfig` and `HotbarNetworkLink` unchanged. |
| `TerminalTabChanged` switches to a network where a hotbar-configured item has qty = 0 | `HotbarNetworkLink` updates; `hotbar_display_system` fires; slot shows qty = 0. No configuration change. |
| Drone deposit: port has no `LogisticsNetworkMember` | `DroneDepositFailed { reason: PortNotConnected }`. Inventory unchanged. Player must connect the port to a network before depositing. |
| Drone deposit: target network mass capacity full | `DroneDepositPartial` emitted. Items that fit are deposited; overflow remains in `DroneInventory`. Player must add storage machines or deposit to a second network. |
| Two drones deposit to the same logistics port in the same frame | `drone_deposit_system` processes both events sequentially in system ordering. Both `give_items` calls apply; `NetworkStorageChanged` fires once per deposit (both fire). No contention — `StorageUnit.items` is not written concurrently. |
| `DroneInventory.max_slots` exceeded when collecting a new item type | `DroneCollectionBlocked { reason: SlotsFull }`. Existing slots unaffected. Player must deposit (freeing all slots) or collect a type already in inventory. |
| Hotbar placement: item qty = 1 but that unit is reserved by an active crafting plan | `count_items` subtracts `NetworkReservations` — returns 0 for the reserved unit. Placement blocked. Player must wait for plan to complete or cancel the plan via Terminal. |
| Goal tracker: `ItemQuantity` goal's `network` entity is despawned (network split removes it) | On next `NetworkStorageChanged`, `goal_tracker_system` checks `Commands::get_entity(goal.network).is_some()` before calling `count_items`; if missing, displays progress as "?" and adds a "disconnected" status label. Goal is not auto-removed — network may be restored. |
| `NodeUnlock` goal: node was never present in this run's tech tree | Goal shows "not in this run" label, progress = 0%, non-dismissible until player explicitly dismisses. No error — the player may have pinned a node from a previous run's Codex view. |
| `NetworkFlowLedger` accumulation over a long run | Ledger trims events outside the `window_seconds` rolling window at query time. Max live events = `tick_rate * peak_unique_item_flows_per_tick * window_seconds`. For a 60s window and 60 tps this is bounded. No periodic trim needed. |
| Player tries to deposit drone samples to a subnet, not the main network | Deposit succeeds to whichever network the targeted port belongs to. Analysis stations on that subnet can consume the samples. If the station is on a different network, the samples must be transferred via interface blocks or the player can re-deposit to the correct port. Player is responsible for port selection. |

# Drone System Design

ECS components, system step-by-step logic, events/messages, and edge cases for the Drone system. Read `gdd.md §8` for design intent and `technical-design.md §8` for the prose overview.

---

## Table of Contents

1. [Overview](#1-overview)
2. [ECS Structure](#2-ecs-structure)
3. [Local↔Remote Mode Transition](#3-localremote-mode-transition)
4. [Fog-of-War Reveal](#4-fog-of-war-reveal)
5. [Sample Collection](#5-sample-collection)
6. [Range Scanning](#6-range-scanning)
7. [Multiple Drone Switching](#7-multiple-drone-switching)
8. [Systems](#8-systems)
9. [Messages](#9-messages)
10. [Execution Order](#10-execution-order)
11. [Vertical Slice Scope](#11-vertical-slice-scope)
12. [Edge Cases](#12-edge-cases)

---

## 1. Overview

Drones are player-piloted exploration and interaction tools — player *attention* travels, not the character body. Two mutually exclusive play modes exist: **Local** (character control) and **Remote** (one drone controlled). Only the active drone receives input; all others are inert at their last positions.

---

## 2. ECS Structure

### Drone entity

```
Drone entity
├── Drone                        ← marker; always present
├── DroneType                    ← tier + domain access/capability
├── DroneState                   ← Idle | ActivelyControlled
├── DroneInventory               ← items collected during piloting
├── DroneTools                   ← equipped tool capabilities (mining drill, etc.)
├── FogRevealRadius              ← proximity reveal radius (meters)
├── ScanRadius                   ← range-scan radius (meters, XZ plane)
├── ScanYTolerance               ← altitude band for range scan (meters above/below)
├── Transform                    ← world position and orientation
├── RigidBody::Dynamic
├── Collider
├── TnuaController<DroneScheme>
├── TnuaConfig<DroneScheme>
├── TnuaAvian3dSensorShape
└── LockedAxes::ROTATION_LOCKED
```

```rust
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DroneType {
    Land,
    Amphibious,
    Digger,
    Flying,
    Space,
}
```

`DroneType` names are capability tiers, not a guarantee that every run contains a complete matching domain. Digger, Flying, and Space content should appear only when the run's tech tier, resource graph, or escape objective includes underground, atmospheric, or orbital/space domains worth accessing.

```rust
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DroneState {
    #[default]
    Idle,
    ActivelyControlled,
}

#[derive(Component, Default, Debug)]
pub struct DroneInventory {
    pub items: HashMap<String, u32>,
}

/// Tools equipped to this drone. Occupies fixed left hotbar slots when drone is active.
/// Tools are capabilities (e.g. mining drill), not inventory items.
#[derive(Component, Default, Debug)]
pub struct DroneTools {
    pub tools: Vec<String>, // tool IDs, ordered = hotbar slot order
}

#[derive(Component, Debug)]
pub struct FogRevealRadius(pub f32);

#[derive(Component, Debug)]
pub struct ScanRadius(pub f32);
```

Default values on spawn: `FogRevealRadius(12.0)`, `ScanRadius(80.0)`. Default `DroneTools` for land drone: `["mining_drill"]`.

### Player entity

```
Player entity
├── PlayerCharacter              ← marker; always present
├── InputBlocked                 ← optional; present while in Remote mode
└── ...
```

```rust
#[derive(Component)]
pub struct PlayerCharacter;

/// Suppresses movement and interaction input for this entity.
#[derive(Component)]
pub struct InputBlocked;
```

### Active drone resource

```rust
/// Entity of the currently controlled drone. None when in Local mode.
#[derive(Resource, Default)]
pub struct ActiveDrone(pub Option<Entity>);
```

---

## 3. Local↔Remote Mode Transition

### Trigger

Player presses **F** while in `PlayMode::Exploring` → enter Remote mode on the nearest deployable drone, or the previously active drone if one is parked.

Player presses **F** while in `PlayMode::DronePilot` → return to Local mode.

In Local mode with multiple deployed drones, pressing **F** always activates the drone last selected (stored in `ActiveDrone`). If `ActiveDrone` is `None`, activates the closest drone to the player character.

### `toggle_drone_mode` system (existing, extend)

**Entering Remote (Exploring → DronePilot):**

1. Read `ActiveDrone` resource to find target entity. If `None`, query nearest `Drone` entity to the `PlayerCharacter` entity; set `ActiveDrone` to that entity.
2. On the target drone entity: set `DroneState` to `ActivelyControlled`.
3. On the player entity: insert `InputBlocked`.
4. Swap hotbar to drone hotbar (see below).
5. Set `NextState<PlayMode>` to `PlayMode::DronePilot`.
6. Write `DroneActivatedEvent { drone: Entity }`.

**Exiting Remote (DronePilot → Exploring):**

1. Read `ActiveDrone`. On that entity: set `DroneState` to `Idle`.
2. On the player entity: remove `InputBlocked`.
3. Restore player hotbar (see below).
4. Set `NextState<PlayMode>` to `PlayMode::Exploring`.
5. Write `DroneDeactivatedEvent { drone: Entity }`.
6. Do **not** clear `ActiveDrone` — it caches the last active drone for next re-entry.

### Drone hotbar

When entering Remote mode the player's normal hotbar is replaced by the active drone's hotbar. The drone hotbar has two regions:

| Region | Source | Contents |
|---|---|---|
| Left (tool slots) | `DroneTools.tools` | Capabilities: mining drill and future tools. One slot per tool, in order. |
| Right (item slots) | `DroneInventory.items` | Placeable items the drone is carrying (machines, cables, etc.). Filled dynamically. |

Tool slots show capability icons (not consumable — using a tool does not remove it). Item slots behave like normal inventory slots: using one deploys/places the item and decrements the count.

On exit Remote, the player's original hotbar state is restored exactly. The hotbar swap is purely a display concern — the underlying `DroneTools` and `DroneInventory` components are the source of truth.

### What changes on each entity

| Component | Player entity | Drone entity |
|---|---|---|
| `InputBlocked` | Inserted on Remote enter; removed on exit | — |
| `DroneState` | — | Set `ActivelyControlled` on enter; `Idle` on exit |
| Active hotbar | Swapped to drone hotbar on enter; restored on exit | — |
| Camera follows | Player body (Local) | Active drone (Remote) |

Camera transfer is handled by `drone_pilot_input` which reads `ActiveDrone` and moves `MainCamera` to drone position each frame. No camera component swap is needed.

---

## 4. Fog-of-War Reveal

### Data structure

```rust
/// Grid-based fog of war. One cell per 4m × 4m world tile.
/// Indexed by (chunk_x, chunk_z) → bitmask of 4×4 sub-tiles within chunk.
#[derive(Resource, Default)]
pub struct FogOfWar {
    pub revealed: HashMap<IVec2, u16>,
}
```

Cell size: 4 m. Chunk size: 16 m (4×4 cells per chunk → 16 bits per chunk). A bit is 1 when revealed.

### `fog_reveal_system`

Runs every frame while `PlayMode::DronePilot` is active.

1. Query `ActiveDrone` resource → get drone `Transform`.
2. Read drone's `FogRevealRadius` component.
3. Compute the set of 4m cells within radius of drone position (circle test in XZ plane).
4. For each cell: set corresponding bit in `FogOfWar.revealed`. No un-reveal.
5. For any newly revealed cell: write `FogCellRevealedEvent { cell: IVec2 }` (used by renderer to update fog mesh).

Fog reveal runs in Local mode too for the player character, but with a smaller fixed radius (4.0 m) and no `FogRevealRadius` component lookup — this is a separate `character_fog_reveal_system`. The drone system only handles Remote mode.

---

## 5. Sample Collection

### Trigger

Left-click (`MouseButton::Left`) while `PlayMode::DronePilot` is active **and** the active hotbar slot holds the `mining_drill` tool.

### `drone_mine_system` (existing, extend)

1. On `just_pressed(MouseButton::Left)`: check active drone's selected hotbar slot. If not `mining_drill`, skip.
2. Cast ray from `MainCamera` forward, max distance `MINE_REACH` (4.0 m).
3. Check hit entity for `OreDeposit` component.
4. If hit:
   a. Compute `yield_factor(deposit.total_extracted, deposit.depletion_seed)`.
   b. Sample ore type via `sample_ore(&deposit.ores, &mut rng)`.
   c. Apply yield as a probabilistic gate: `rng.gen::<f32>() < yield_factor` → collect.
   d. If collected: query active drone entity via `ActiveDrone`; add ore item to its `DroneInventory`.
   e. Increment `deposit.total_extracted` by 1.0.
   f. Write `OreSampledEvent { drone: Entity, ore_id: String, deposit: Entity }`.
5. If hit entity has no `OreDeposit`: no-op (future: other interactable types).

**Where item lands:** `DroneInventory` on the drone entity. Items do **not** transfer automatically when switching to Local mode. The drone must physically return to the aegis field or a logistics-linked dropoff point. On proximity to a `DroneDropoff` entity, `drone_dropoff_system` moves all `DroneInventory` contents into the connected logistics network. Items are never silently lost — they remain in `DroneInventory` until a valid dropoff is reached.

### Item produced

The ore ID string from `sample_ore` maps directly to an `ItemId` in the item registry. No transformation. Sample items produced this way are tagged as "raw sample" for research station consumption — this is item metadata in the asset definition, not a separate component.

---

## 6. Range Scanning

### Trigger

Left-click (`MouseButton::Left`) while `PlayMode::DronePilot` is active **and** the active hotbar slot holds the `scanner` tool.

### `drone_scan_system`

Runs on `just_pressed(MouseButton::Left)` when scanner tool is selected.

1. Read `ActiveDrone` → get drone `Transform`, `ScanRadius`, and `ScanYTolerance`.
2. Query all `OreDeposit` entities within `ScanRadius` in the XZ plane **and** within `ScanYTolerance` in the Y axis (i.e. `|deposit.y - drone.y| ≤ ScanYTolerance`). This excludes aerial and underground deposits outside the drone's altitude band.
3. For each deposit in range:
   - If already `Discovered`: skip (player already has precise data).
   - Otherwise: determine `ScanResult` — biome affinity of deposit (from deposit metadata) + broad resource category (`Mineral`, `Fluid`, `Biological`, `Energy`). Do **not** expose: exact position, quantity, ore type, or depletion state.
4. Write `ScanCompletedEvent { results: Vec<ScanResult> }`.
5. Do **not** mark deposits as `Discovered` — scan gives coarse data only.

```rust
pub struct ScanResult {
    /// Approximate bearing from drone (8-directional compass).
    pub direction: CardinalDir8,
    /// Rough distance bucket: Near (<20m), Mid (20–50m), Far (>50m).
    pub distance: DistanceBucket,
    pub biome_affinity: String,
    pub category: ResourceCategory,
}

pub enum ResourceCategory {
    Mineral,
    Fluid,
    Biological,
    Energy,
}

pub enum CardinalDir8 { N, NE, E, SE, S, SW, W, NW }
pub enum DistanceBucket { Near, Mid, Far }
```

6. Scan has a **cooldown of 5 seconds** (real time). Track last scan time on the drone entity:

```rust
#[derive(Component, Default)]
pub struct ScanCooldown(pub f32); // seconds remaining
```

`drone_scan_system` checks `ScanCooldown > 0` → skip. A separate `scan_cooldown_tick_system` decrements by `Time::delta_secs()` each frame.

Add `ScanYTolerance` to drone entity:

```rust
#[derive(Component, Debug)]
pub struct ScanYTolerance(pub f32); // meters above/below drone altitude
```

Default on spawn: `ScanYTolerance(6.0)` (covers surface-level variation; excludes flying or deep-underground deposits).

---

## 7. Multiple Drone Switching

### Deploying a drone

Future content: drones are spawned as items and placed in world by the player. For Vertical Slice: one land drone spawned at game start.

### Selection mechanic

With multiple deployed drones (MVP+): player holds **Alt** and presses **1–9** to select by slot, or opens a drone selection panel. For now: **no UI switching** — only F-key re-enter activates the last-used drone.

### `switch_active_drone_system` (MVP+)

1. Detect slot key press (1–9) while `PlayMode::DronePilot` is active.
2. Collect all drones related to the player via `DroneOf` relationship, sorted by `DroneSlot`. Map key press to that sorted list.
3. Deactivate current: set `DroneState::Idle` on current active drone.
4. Activate new: set `DroneState::ActivelyControlled`; update `ActiveDrone` resource.
5. Camera will follow new drone next frame via `drone_pilot_input`.
6. Write `DroneActivatedEvent` for new, `DroneDeactivatedEvent` for old.

```rust
/// Relationship: drone → player entity. Player entity gets DroneFleet (auto-populated).
#[derive(Component)]
#[relationship(relationship_target = DroneFleet)]
pub struct DroneOf(pub Entity);

/// Auto-populated on the player entity. Gives access to all related drone entities.
#[derive(Component)]
#[relationship_target(relationship = DroneOf)]
pub struct DroneFleet;

/// Slot index for hotkey selection (0-based). Drones sorted by this for 1–9 keys.
#[derive(Component, Default)]
pub struct DroneSlot(pub u8);
```

On spawn, each drone gets `DroneOf(player_entity)` and an assigned `DroneSlot`. To query all drones owned by the player: `query.get::<DroneFleet>(player_entity)`.

Invariant: at most one drone entity has `DroneState::ActivelyControlled` at any time.

---

## 8. Systems

| System | Schedule | Run condition |
|---|---|---|
| `toggle_drone_mode` | `Update` | `in_state(GameState::Playing)` |
| `drone_pilot_input` | `Update` (`TnuaUserControlsSystems`) | `in_state(PlayMode::DronePilot)` |
| `drone_mine_system` | `Update` | `in_state(PlayMode::DronePilot)` |
| `deposit_discovery_system` | `Update` | `in_state(PlayMode::DronePilot)` |
| `fog_reveal_system` | `Update` | `in_state(PlayMode::DronePilot)` |
| `drone_scan_system` | `Update` | `in_state(PlayMode::DronePilot)` |
| `scan_cooldown_tick_system` | `Update` | `in_state(GameState::Playing)` |
| `switch_active_drone_system` | `Update` | `in_state(PlayMode::DronePilot)` |
| `drone_dropoff_system` | `Update` | `in_state(PlayMode::DronePilot)` |

---

## 9. Messages

```rust
/// Fired when a drone becomes the active controlled drone.
pub struct DroneActivatedEvent {
    pub drone: Entity,
}

/// Fired when a drone is released from active control.
pub struct DroneDeactivatedEvent {
    pub drone: Entity,
}

/// Fired when an ore sample is collected into a drone's inventory.
pub struct OreSampledEvent {
    pub drone: Entity,
    pub ore_id: String,
    pub deposit: Entity,
}

/// Fired per newly revealed fog cell.
pub struct FogCellRevealedEvent {
    pub cell: IVec2,
}

/// Fired when a range scan completes.
pub struct ScanCompletedEvent {
    pub results: Vec<ScanResult>,
}
```

---

## 10. Execution Order

Within a single frame:

```
FixedUpdate:
  TnuaUserControlsSystems (drone_pilot_input)

Update:
  toggle_drone_mode
    → writes DroneActivatedEvent / DroneDeactivatedEvent
  [after toggle_drone_mode]
  drone_mine_system
  deposit_discovery_system
  fog_reveal_system
  drone_scan_system
  scan_cooldown_tick_system
  switch_active_drone_system
  drone_dropoff_system
```

`toggle_drone_mode` must run before input systems so mode is correct when they execute. Use `.after(toggle_drone_mode)` or a system set.

---

## 11. Vertical Slice Scope

Vertical Slice implements:
- Single land drone, spawned at game start
- F-key Local↔Remote toggle
- WASD + mouse-look piloting
- Left-click ore sample collection into `DroneInventory` (mining drill tool selected)
- Proximity deposit discovery (`deposit_discovery_system`, already present)
- Fog-of-war reveal (grid structure + `fog_reveal_system`)

Deferred to MVP:
- Range scanning (`drone_scan_system`, `ScanCompletedEvent`)
- Multiple drone switching (`DeployedDrones`, `switch_active_drone_system`)
- Drone inventory dropoff (`drone_dropoff_system`, `DroneDropoff` entity)
- Drone construction from factory components
- Drone tiers beyond land drone

---

## 12. Edge Cases

**Drone inventory dropoff.** Items in `DroneInventory` do not transfer on Local↔Remote toggle. The drone must physically fly back to a `DroneDropoff` entity (aegis field or logistics-linked station). `drone_dropoff_system` triggers on proximity and moves all inventory into the connected logistics network. Items persist in `DroneInventory` indefinitely until a valid dropoff is reached — never silently lost.

**Player switches drone while mining animation plays.** `drone_mine_system` is instantaneous (single frame, raycast). No partial-mining state. No edge case.

**Two drones at same position.** `DroneSlot` index selects drones, not proximity, so two co-located drones are distinguishable. No collision between inactive drones (they are physics-simulated but `Idle` — Tnua controller produces zero desired motion when `InputBlocked` or `Idle`). Actually: inactive drones should have physics disabled to avoid unexpected drift. Set `RigidBody::Static` on `DroneState::Idle` drones, restore `RigidBody::Dynamic` on activation.

**Fog-of-war reveal while in Local mode.** Character fog reveal is a separate fixed-radius system and does not use `FogRevealRadius`. Drone fog reveal only runs in `DronePilot` mode — character reveal runs unconditionally in `Playing` state.

**Scan during cooldown.** `drone_scan_system` silently ignores left-click (scanner) when `ScanCooldown > 0`. No event, no feedback (feedback is a UI concern). The scan system only writes `ScanCompletedEvent` on a successful scan.

**`ActiveDrone` points to despawned entity.** On drone despawn (not in Vertical Slice), clear `ActiveDrone` and force transition to Local mode if currently in Remote. Guard all `ActiveDrone` lookups with `entities.contains(entity)`.

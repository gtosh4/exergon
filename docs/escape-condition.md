# Escape Condition Design

Specifies ECS structure, system logic, events, and edge cases for the escape condition. Vertical Slice covers Initiation (alien gateway); MVP adds Standard/Advanced/Pinnacle. Design these only in terms of other docs; do not reference current code state.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Recipe System Extension — Catalyst Inputs](#2-recipe-system-extension--catalyst-inputs)
3. [ECS Structure](#3-ecs-structure)
4. [Systems](#4-systems)
5. [Events](#5-events)
6. [Edge Cases](#6-edge-cases)
7. [Execution Order](#7-execution-order)
8. [Integration Tests](#8-integration-tests)
9. [MVP Escape Types](#9-mvp-escape-types)

---

## 1. Overview

Each run has one escape objective. It has three sequential phases:

| Phase | What it requires | Who drives it |
|---|---|---|
| **Construction** | Craft all required components | Recipe execution system (no special escape logic) |
| **Field Requirement** | Non-production prerequisite (power level, fragment count, fuel stockpile) | Escape-type-specific system |
| **Activation** | Player explicitly triggers; all conditions met simultaneously | Player input → activation system |

Phase transitions are tracked on the escape objective entity via an `EscapePhase` component. The player must complete Construction before triggering Activation — the system enforces this by checking phase state on interact.

### Initiation escape: alien gateway

The player finds the alien gateway ruins via drone. They craft a Gateway Key in an assembler (Construction). They then interact with the gateway while the key is in the connected logistics network, starting a power-sustained charge (Field Requirement + Activation combined). If power drops, the charge resets and must be restarted explicitly. When charge completes, the run ends.

The gateway activation uses a **catalyst input** — the key must be present in the logistics network for the duration of the charge but is not consumed.

---

## 2. Recipe System Extension — Catalyst Inputs

Catalyst inputs are a general recipe concept introduced here and used first by gateway activation. A catalyst item must be present in the logistics network at job start; it is reserved (unavailable for other jobs) for the job's duration; it is released — not consumed — when the job completes or is cancelled.

### Data model

Each concrete recipe gains an optional field:

```
catalyst_inputs: Vec<(ItemId, u32)>
```

A recipe with no `catalyst_inputs` behaves exactly as before. A recipe with catalyst inputs:

1. At job creation: checks that all catalyst items are available in the network (quantity ≥ required). If not, job cannot start.
2. At job start: marks catalyst items as reserved in the logistics network. Reserved items appear as present in network inventory but are flagged unavailable for other jobs.
3. At job complete: releases reservation. Items remain in network.
4. At job cancel: releases reservation. Items remain in network.

Catalyst reservation is per-job. Two simultaneous jobs requiring the same catalyst item each need independent copies: if two jobs both require 1× catalyst_X, 2× catalyst_X must be available.

### Asset format addition

Concrete recipe assets and template recipes gain a `catalyst_inputs` list (default: empty). Generation algorithm passes catalyst fields through unchanged — they are not seeded variance axes.

---

## 3. ECS Structure

### Gateway entity

Spawned once at run generation at the gateway ruins site. Not player-placed.

| Component | Fields | Notes |
|---|---|---|
| `Machine` | `machine_type: "gateway"`, `tier: 1` | Existing machine component |
| `GatewayActivationSpec` | `power_demand_watts: f32`, `charge_duration_secs: f32` | Loaded from gateway machine asset. Defines required power draw and total charge time. |
| `EscapePhase` | `phase: EscapePhaseState` | Tracks current phase: `Construction`, `FieldRequirement(FieldReqState)`, `Complete` |
| `Discovered` | (marker) | Absent until `DiscoveryEvent("gateway_ruins")` fires. Systems gate interaction on this. |
| Power port entities | (existing) | Machine has at least one energy port for cable connection |
| Logistics port entities | (existing) | Machine has at least one logistics port for network connection |

### `EscapePhaseState` enum

```
Construction
    — waiting for player to initiate activation (key not yet inserted)
FieldRequirement(FieldReqState)
    — activation started; charge accumulating or stalled
Complete
    — charge filled; EscapeEvent fired
```

### `FieldReqState` enum

```
Charging { elapsed_secs: f32 }
    — power is sufficient; charge accumulating
Stalled
    — power lost after charge began; waiting for player to restart
```

`Stalled` is a distinct state from `Construction` so the UI can distinguish "never started" from "started and lost power."

### `GatewayActivationSpec` (in gateway machine asset)

```
GatewayActivationSpec {
    power_demand_watts: f32,   // minimum network power speed_factor * machine_draw >= this value
    charge_duration_secs: f32, // seconds of continuous sufficient power to complete
}
```

`power_demand_watts` is compared against the power network's delivered watts to the gateway port. The gateway asset defines its `machine_draw` as a standard machine field; `power_demand_watts` in the spec is the threshold the network must meet (may equal or exceed `machine_draw`).

### `RunState` resource

```
RunState {
    seed: u64,
    status: RunStatus,         // InProgress | Escaped
    start_time_secs: f32,
    escape_time_secs: Option<f32>,
}
```

Initialized at game start from `RunSeed`. `escape_time_secs` set on `EscapeEvent`.

---

## 4. Systems

### 4-1. `gateway_interact_system`

**Trigger:** `GatewayInteractEvent { gateway: Entity }` (fired by input system on player right-click of gateway in `Exploring` state).

**Step-by-step:**

1. Read `EscapePhase` on the gateway entity.
2. If `phase == Complete`: no-op (charge already done, escape already triggered).
3. If `phase == FieldRequirement(Charging { .. })`: show diagnostic "Gateway already charging." No-op.
4. Check `gateway_theory` unlocked in `TechTreeProgress`. If not: show diagnostic "Undiscovered — you don't understand this structure yet." No-op.
5. Read `GatewayActivationSpec` from gateway entity.
6. Check logistics network connected to gateway contains ≥ 1 `gateway_key` (not already reserved). If not: show diagnostic "Missing activation key." No-op.
7. Check power network connected to gateway delivers ≥ `spec.power_demand_watts`. If not: show diagnostic "Insufficient power — {delivered:.0} / {required:.0} W." No-op.
8. All checks pass:
   - Reserve 1× `gateway_key` in the logistics network (mark as reserved by this gateway entity).
   - Set `EscapePhase` to `FieldRequirement(Charging { elapsed_secs: 0.0 })`.

**Note:** The system allows restarting from `Stalled` — step 3 only blocks on `Charging`, not `Stalled`. A `Stalled` gateway goes through steps 4–8 normally. This lets the player re-initiate after restoring power.

### 4-2. `gateway_charge_system`

**Trigger:** Runs every tick. Queries gateway entities with `EscapePhase` in `FieldRequirement` state.

**Step-by-step (per gateway):**

1. Read `FieldReqState`.
2. Read current power delivery to gateway from power network (`delivered_watts`).
3. **If `Stalled`:** Power check — if `delivered_watts >= spec.power_demand_watts`, do nothing (wait for explicit player restart). Stay `Stalled`. (Player must right-click again — no auto-resume.)
4. **If `Charging { elapsed_secs }`:**
   a. If `delivered_watts < spec.power_demand_watts`:
      - Release `gateway_key` reservation.
      - Set state to `Stalled`.
      - Show notification "Gateway power lost — charge reset. Restore power and re-activate."
   b. Else: increment `elapsed_secs += delta_time`.
   c. If `elapsed_secs >= spec.charge_duration_secs`:
      - Release `gateway_key` reservation (key not consumed).
      - Set `EscapePhase` to `Complete`.
      - Fire `EscapeEvent`.

### 4-3. `gateway_status_ui_system`

**Trigger:** Runs every tick. Active in `Playing` and `DronePilot` states.

**Condition:** Player (or active drone) within display range of gateway entity (e.g. 20 units). Range is checked against player/drone position vs. gateway `Transform`.

**Output (shown in machine status panel or world-space HUD element):**

| Phase | `FieldReqState` | Display |
|---|---|---|
| `Construction` | — | "Inactive — craft and insert activation key to begin" |
| `FieldRequirement` | `Charging` | Progress bar: `elapsed / charge_duration`, time remaining, current power reading |
| `FieldRequirement` | `Stalled` | "Charge lost — power interrupted. Restore power and re-activate." |
| `Complete` | — | (escape sequence active; this panel not visible) |

If `gateway_theory` not yet unlocked, display nothing (structure appears inert and unrecognized).

---

## 5. Events

### `GatewayInteractEvent`

```
GatewayInteractEvent { gateway: Entity }
```

**Fired by:** Input system when player right-clicks a gateway machine entity in `Exploring` state.  
**Consumed by:** `gateway_interact_system`.

### `EscapeEvent`

```
EscapeEvent { escape_time_secs: f32 }
```

**Fired by:** `gateway_charge_system` when charge completes.  
**Consumed by:**
- Run state system: sets `RunState.status = Escaped`, `RunState.escape_time_secs = Some(escape_time_secs)`.
- Game state system: transitions to `GameState::Escaped`.

---

## 6. Edge Cases

**Player leaves logistics network range during charge.**  
The key reservation holds regardless of player position. The charge continues as long as power is delivered. Logistics network connectivity is the constraint, not player proximity.

**Logistics network splits during charge (cable destroyed).**  
If the network segment containing the reserved key disconnects from the gateway, power and item delivery both break. Power check fails → `Stalled`. Key reservation is released on transition to `Stalled`. On network reconnect and player re-activation, key must still be available (not consumed).

**Player right-clicks gateway while `Stalled`.**  
System treats `Stalled` the same as `Construction` for interaction purposes — steps 4–8 run normally. If conditions are met, charge restarts from `elapsed_secs: 0.0`. Charge does not resume from where it stalled.

**`gateway_theory` not yet unlocked when player approaches gateway.**  
`gateway_status_ui_system` shows nothing. `gateway_interact_system` shows "Undiscovered" diagnostic on attempt. The gateway entity exists and has physics collision, but gives no feedback until discovery.

**Player attempts to use two gateway keys simultaneously (two in network).**  
Only 1× key is required and reserved. The second key remains unreserved and available. No special handling needed.

**Power fluctuates exactly at threshold during charge.**  
`gateway_charge_system` checks delivered watts each tick. A single tick below threshold triggers `Stalled`. No hysteresis or debounce. This is intentional — the power requirement is strict, and the player must build a stable supply.

**`EscapeEvent` fires while game is paused.**  
`EscapeEvent` is processed on resume. `gateway_charge_system` does not run while paused (simulation systems are disabled in `GameState::Paused`), so charge cannot complete while paused.

---

## 7. Execution Order

Within each simulation tick, systems run in this order:

```
1. power_delivery_system        — updates delivered_watts on all power network members
2. gateway_charge_system        — reads delivered_watts, advances or resets charge
3. gateway_interact_system      — processes GatewayInteractEvent (PostUpdate or after charge tick)
4. gateway_status_ui_system     — reads EscapePhase for display (runs after charge system)
5. escape_event_handler         — updates RunState, fires GameState transition
```

`gateway_charge_system` must run after `power_delivery_system` so it reads the current tick's power state, not last tick's.

`gateway_interact_system` runs after `gateway_charge_system` so that an interaction in the same tick as a power-loss event sees the `Stalled` state, not the `Charging` state that was just invalidated.

---

## 8. Integration Tests

All tests use `World` directly, no `App`.

**Test 1 — Full charge completes**  
Setup: gateway entity with `GatewayActivationSpec { power_demand_watts: 100.0, charge_duration_secs: 5.0 }`, `EscapePhase::FieldRequirement(Charging { elapsed_secs: 4.9 })`, power network delivering 100.0 W.  
Run: `gateway_charge_system` with `delta_time = 0.2`.  
Assert: `EscapePhase::Complete`, `EscapeEvent` in world events.

**Test 2 — Power loss resets charge**  
Setup: gateway charging with `elapsed_secs: 3.0`, power network delivering 50.0 W, `power_demand_watts: 100.0`. Key reserved.  
Run: `gateway_charge_system`.  
Assert: `EscapePhase::FieldRequirement(Stalled)`. Key reservation released. No `EscapeEvent`.

**Test 3 — Interact from Stalled re-charges when conditions met**  
Setup: gateway `Stalled`. Power now 100.0 W. Key available (unreserved). `gateway_theory` unlocked.  
Run: `gateway_interact_system` with `GatewayInteractEvent`.  
Assert: `EscapePhase::FieldRequirement(Charging { elapsed_secs: 0.0 })`. Key reserved.

**Test 4 — Interact blocked without key**  
Setup: gateway `Construction`. `gateway_theory` unlocked. Power sufficient. No `gateway_key` in logistics network.  
Run: `gateway_interact_system`.  
Assert: phase unchanged. No key reservation. Diagnostic event fired with `MissingKey` reason.

**Test 5 — Interact blocked without sufficient power**  
Setup: as Test 4 but key present. Power delivers 50.0 W, demand 100.0 W.  
Assert: phase unchanged. No key reservation. Diagnostic event with `InsufficientPower { delivered: 50.0, required: 100.0 }`.

**Test 6 — Interact blocked when gateway_theory not unlocked**  
Setup: gateway entity, `Discovered` marker absent from `TechTreeProgress`.  
Assert: phase unchanged. Diagnostic event with `Undiscovered` reason.

**Test 7 — Catalyst reservation: key not consumed on completion**  
Setup: full charge run to completion (as Test 1).  
Assert: `gateway_key` quantity in logistics network equals pre-run quantity. Reservation released.

**Test 8 — Auto-resume does not occur when Stalled**  
Setup: gateway `Stalled`. Power restored to above threshold.  
Run: `gateway_charge_system` (3 ticks).  
Assert: still `Stalled` after all ticks. `elapsed_secs` does not increment.

---

## 9. MVP Escape Types

All escape types share the same three-phase structure and `EscapePhase` component. Each type introduces a different `FieldReqState` variant and a corresponding field-requirement system. Construction phase is always handled by the standard recipe/crafting system with no escape-specific code.

| Difficulty | Field Requirement | Activation |
|---|---|---|
| Initiation | Sustained power to gateway for `charge_duration_secs` | Player interact while charging |
| Standard | All ship components installed + alien fuel loaded | Player interact with ship launch console |
| Advanced | All relay fragments collected (count tracked in `RelayRepairState`) + sustained power | Player interact with relay array |
| Pinnacle | All four ship systems assembled + FTL-grade power sustained | Player interact with launch sequence |

Each MVP escape type will be specced separately before implementation.

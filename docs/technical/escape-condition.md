# Escape Condition Design

Specifies ECS structure, system logic, events, and edge cases for the escape condition. Vertical Slice covers Initiation (alien gateway); MVP adds Standard/Advanced/Pinnacle. Design these only in terms of other docs; do not reference current code state.

---

## Table of Contents

1. [Overview](#1-overview)
2. [ECS Structure](#2-ecs-structure)
3. [Systems](#3-systems)
4. [Events](#4-events)
5. [Edge Cases](#5-edge-cases)
6. [Integration Tests](#6-integration-tests)
7. [MVP Escape Types](#7-mvp-escape-types)

---

## 1. Overview

Each run has one escape objective — a machine the player constructs, enables, and sustains until its recipe job completes. Three informal phases, each handled by existing systems with no escape-specific code:

| Phase | What it requires | Who drives it |
|---|---|---|
| **Construction** | Craft all required components | Recipe execution system |
| **Activation** | Player enables the escape machine | Machine enabled/disabled state (general feature) |
| **Completion** | Machine runs recipe job to end | Recipe execution + power systems |

When the recipe on an escape machine completes, `EscapeEvent` fires.

### Initiation escape: alien gateway

The player finds the alien gateway ruins via drone. They craft a Gateway Key in an assembler (Construction — standard recipe). They enable the gateway machine (Activation). The gateway draws power and runs a timed recipe; if power drops, the job stalls until power is restored and the machine is re-enabled. When the job completes, the run ends.

The gateway recipe uses the Gateway Key as a **catalyst input** — present for the job duration, not consumed on completion. Catalyst inputs are a recipe system feature; see `technical/crafting.md §catalyst-inputs`.

---

## 2. ECS Structure

### Gateway entity

Spawned once at run generation at the gateway ruins site. Not player-placed.

| Component | Fields | Notes |
|---|---|---|
| `Machine` | `machine_type: "gateway"`, `tier: 1` | Existing machine component |
| `EscapeObjective` | (marker) | Recipe completion on this machine fires `EscapeEvent` |
| `Discovered` | (marker) | Absent until `DiscoveryEvent("gateway_ruins")` fires; systems gate interaction on this |
| Power port entities | (existing) | Machine has at least one energy port for cable connection |
| Logistics port entities | (existing) | Machine has at least one logistics port for network connection |

`EscapeObjective` is the only escape-specific component. Machine enabled/disabled state, job status, and power delivery are tracked by existing machine and power systems.

### Run outcome

Run outcome (seed, escape time, status) is **save file metadata** — read and written at load/save time, not held as a runtime resource. `EscapeEvent` carries the data needed to write the outcome on save.

---

## 3. Systems

### 3-1. `escape_objective_system`

**Trigger:** Reads `RecipeJobCompleted` events each tick.

**Step-by-step:**

1. For each `RecipeJobCompleted { machine: Entity, .. }`:
2. Check whether `machine` has `EscapeObjective` marker.
3. If yes: fire `EscapeEvent { escape_time_secs: current_time }`.

No other escape-specific systems. Machine enable/disable, power delivery, and job execution are handled by existing systems.

---

## 4. Events

### `EscapeEvent`

```
EscapeEvent { escape_time_secs: f32 }
```

**Fired by:** `escape_objective_system` when a recipe job completes on an `EscapeObjective` machine.  
**Consumed by:**
- Save system: writes run outcome to save file.
- Game state system: transitions to `GameState::Escaped`.

---

## 5. Edge Cases

**Power loss during gateway charge.**  
Handled by power and machine systems: insufficient power stalls the job. Player must re-enable the machine after restoring power. No escape-specific behavior.

**`gateway_theory` not yet unlocked when player approaches gateway.**  
`gateway_status_ui_system` shows nothing. Machine interaction shows "Undiscovered" diagnostic. Entity exists with physics collision but gives no feedback until discovery.

**Logistics network splits during charge (cable destroyed).**  
Power and item delivery both break; machine job stalls. Catalyst key reservation behavior is specified in `technical/crafting.md §catalyst-inputs`.

**`EscapeEvent` fires while game is paused.**  
`EscapeEvent` is processed on resume. Recipe execution systems do not run while paused (`GameState::Paused`), so jobs cannot complete while paused.

---

## 6. Integration Tests

All tests use `World` directly, no `App`.

**Test 1 — Recipe completion on EscapeObjective machine fires EscapeEvent**  
Setup: gateway entity with `EscapeObjective` marker. `RecipeJobCompleted` event for that entity.  
Run: `escape_objective_system`.  
Assert: `EscapeEvent` in world events.

**Test 2 — Recipe completion on non-escape machine does not fire EscapeEvent**  
Setup: assembler entity without `EscapeObjective`. `RecipeJobCompleted` event for that entity.  
Run: `escape_objective_system`.  
Assert: no `EscapeEvent`.

---

## 7. MVP Escape Types

All escape types are machines with `EscapeObjective`. Construction uses the standard recipe/crafting system. Activation uses machine enabled/disabled state. Each type's power and logistics requirements are defined in the machine asset and recipe.

| Difficulty | Machine | Recipe requirement | Field condition |
|---|---|---|---|
| Initiation | Alien gateway | Gateway Key (catalyst) | Sustained power for `charge_duration_secs` |
| Standard | Ship launch console | All ship components + alien fuel | None beyond recipe inputs |
| Advanced | Relay array | All relay fragments | Sustained power while job runs |
| Pinnacle | FTL launch system | All four ship systems | FTL-grade sustained power |

Each MVP escape type will be specced separately before implementation, covering machine asset details and recipe structure.

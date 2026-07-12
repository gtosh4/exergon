# Escape Condition Design

Specifies ECS structure, system logic, events, and edge cases for the escape condition. Vertical Slice covers Initiation (precursor gateway); MVP adds Standard/Advanced/Pinnacle. Design these only in terms of other docs; do not reference current code state.

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

### Initiation escape: precursor gateway

The player finds the **precursor** gateway ruins via drone — a transit structure an earlier probe generation left (seeded near the lineage trunk; a frontier run has none and scratch-builds the launch instead, GDD §12). They craft the Gateway Key — the minimal **successor** payload — in an assembler (Construction — standard recipe). They enable the gateway machine (Activation). The gateway draws power and runs a timed recipe; if power drops, the job stalls until power is restored and the machine is re-enabled. When the job completes, the copy is sent through and the run ends.

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

**Trigger:** Reads `JobComplete` events each tick.

**Step-by-step:**

1. For each `JobComplete { job, machine, slot }`:
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
Setup: gateway entity with `EscapeObjective` marker. `JobComplete` event for that entity.  
Run: `escape_objective_system`.  
Assert: `EscapeEvent` in world events.

**Test 2 — Recipe completion on non-escape machine does not fire EscapeEvent**  
Setup: assembler entity without `EscapeObjective`. `JobComplete` event for that entity.  
Run: `escape_objective_system`.  
Assert: no `EscapeEvent`.

---

## 7. MVP Escape Types

All escape types are one machine with `EscapeObjective`. Construction uses the standard recipe/crafting system. Activation uses machine enabled/disabled state. **Difficulty scales the *successor* (how complete a copy, and how many — GDD §12), and scale lives in the recipe inputs, not in new machinery — the win is a single climactic cascade.** A **precursor** structure, when the world seeds one (near the lineage trunk), is a **catalyst/discount** on part of the launch recipe; a frontier world seeds none and the successor is fabricated whole.

| Difficulty | Machine (`EscapeObjective`) | Recipe requirement (the successor) | Field condition |
|---|---|---|---|
| Initiation | Launch site — precursor **gateway** discounts the transit step, if seeded | 1 minimal successor (the Gateway Key payload when gateway-discounted) | Sustained power for `charge_duration_secs` |
| Standard | Launch site — precursor **derelict** discounts the hull, if seeded | Fuller successor (4 systems) + provisioning module + exotic fuel | Sustained power |
| Advanced | Replication line — precursor **relay** discounts the range/boost, if seeded | Sustained successor output + terraform-products | Sustained input rate + power |
| Pinnacle | Self-expanding forge (always frontier — scratch) | Four successor systems + provisioning, fully self-fabricated | Forge-grade sustained power |

Each MVP escape type will be specced separately before implementation, covering machine asset details and recipe structure.

### 7.1 Standard escape: successor launch *(specced — content shipped Phase D)*

The Standard escape is the successor-launch cascade (`standard-run-design.md §8`). **No new engine** — it reuses the Initiation mechanism: an `EscapeObjective` machine running one recipe; `JobComplete → EscapeEvent → win`. Scale lives entirely in the recipe inputs (four successor systems + provisioning + exotic fuel), not in new machinery.

**Machine — `launch_site`** (`assets/machines/launch_site.ron`): a tier-2, single-interaction ground machine. It is the Standard analog of the gateway. Like the gateway, it must carry the `EscapeObjective` marker for `escape_objective_system` to fire the win on its `JobComplete`.

> **Engine hook (landed).** The gateway receives `EscapeObjective` at run-gen (`world::ruins::spawn_gateway_ruins_system`). A **player-built** `launch_site` receives it from `escape::tag_escape_machines_system`, which tags any newly-added `Machine` whose `machine_type` is in `ESCAPE_MACHINE_TYPES` (`["launch_site"]`) — option (a) above. Regression-tested (`escape/mod.rs::launch_site_machine_gets_escape_objective`) and end-to-end (`standard_full_run.rs`: a real placed launch_site runs `launch_successor` → `RunState::Completed`).

**Recipe — `launch_successor`** (`assets/recipes/launch_successor.ron`), the single launch cascade:

| Field | Value |
|---|---|
| machine | `launch_site` (tier 2) |
| inputs | 1 `successor_core`, 1 `successor_chassis`, 1 `successor_drive`, 1 `successor_sensor`, 1 `provisioning_module`, 20 `exotic_fuel` |
| outputs | — (empty; the escape is the completion event, as with `activate_gateway`) |
| processing_time | 180 s (sustained-power field requirement) |
| energy_cost | 8000 |

Each system pulls a **different exotic line** so the launch needs the whole graph: core ← Resonite (`resonite_circuit`) + silicon; chassis ← Vitreite + aluminum plate; drive ← Fluxite (`fluxite_coil`) + titanium plate; sensor ← Resonite lattice + silicon; fuel ← Cryophase (second-site, `standard-run-design.md §7`). Numbers are representative/unvalidated (`standard-run-design.md §9 #4`).

**The derelict discount** (`standard-run-design.md §8.3`, fixed run = derelict present): the `derelict_ship` site (drone-discovered) yields `salvaged_hull`. `derelict_salvage` (ExplorationDiscovery: `derelict_ship`) unlocks `make_successor_chassis__salvaged` — `2 aluminum_plate + 1 vitreite + 1 salvaged_hull → 1 successor_chassis`, roughly half the aluminum+Vitreite of the scratch `make_successor_chassis` (`4 aluminum_plate + 2 vitreite`). Both produce the same `successor_chassis`, so a run with the derelict has a lighter chassis burden; a frontier run (no derelict) scratch-builds. The frontier variant is a deferred second config (`standard-run-design.md §9 #7`).

**Tech gating.** The launch machine unlocks at `launch_site_assembly` (ResearchSpend synthesis — modeling "all 4 systems built" pragmatically via the recipe's own inputs; see below). The launch recipe unlocks at the terminal `launch_successor` node (category `Escape`), whose prerequisites are `launch_site_assembly`, `synthesis_lab`, `provisioning_module`, and `exotic_fuel_refining` — i.e. all six input lines must be researched before the cascade recipe is available.

> **ProductionMilestone deviation (flag).** Design §5 gates Launch Site Assembly on "all 4 successor systems built". `ProductionMilestone` tracks a **single** material id, so it cannot express "4 distinct systems". Modeled pragmatically per the design's own fallback: `launch_site_assembly` gates on `ResearchSpend(synthesis)` and lists the four `successor_*` nodes as **prerequisites**, and the `launch_successor` **recipe itself** requires all four systems as inputs — so the "whole graph" requirement is enforced by the recipe, not the unlock vector.

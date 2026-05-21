# Aegis System Design

ECS components, system step-by-step logic, events/messages, and edge cases for the Aegis system. Read `gdd.md §10` for design intent.

---

## Table of Contents

1. [Overview](#1-overview)
2. [ECS Structure](#2-ecs-structure)
3. [Aegis Boundary Check]((#3-aegis-boundary-check))
4. [Local Mode Constraint](#4-local-mode-constraint)
5. [Atmospheric Exposure](#5-atmospheric-exposure)
6. [Aegis Emitter Upgrades]((#6-aegis-emitter-upgrades))
7. [Outpost Beacon (MVP)](#7-outpost-beacon-mvp)
8. [Systems](#8-systems)
9. [Messages](#9-messages)
10. [Execution Order](#10-execution-order)
11. [Vertical Slice Scope](#11-vertical-slice-scope)
12. [Edge Cases](#12-edge-cases)

---

## 1. Overview

The open environment is hostile to the AI's hardware. The hazard type (EM interference, corrosive particulates, exotic radiation, etc.) is a planet property — seeded per run, fixed for the run, revealed through early scanning. The hazard type affects lore framing and warning feedback only; the underlying mechanics are identical across all types. Two infrastructure types project aegis fields:

- **Aegis Emitter** — spawned with the escape pod; always-on; no power dependency
- **Outpost Beacon** — built and placed by the player; requires continuous power

The player body can only be in **Local mode** (direct control) when inside an active aegis field. Outside any active envelope, the body sustains hardware damage from environmental exposure. Drones are ruggedized expendable hardware — they operate freely in the open environment.

Aegis fields are non-overlapping by design intent but the system does not enforce adjacency; two zones may be physically separate (the gap between them is drone-only territory).

---

## 2. ECS Structure

### Aegis Emitter entity

```
AegisEmitter entity
├── AegisEmitter          ← marker; always present; never depowered
├── AegisRadius(f32)        ← sphere radius in meters; expanded by upgrade
└── Transform                 ← world position = aegis field center
```

### Outpost beacon entity (MVP)

```
OutpostBeacon entity
├── OutpostBeacon             ← marker
├── AegisRadius(f32)        ← sphere radius in meters; fixed per tier
├── AegisActive             ← marker; present iff power is sufficient
├── PowerConsumer             ← wattage draw; shared with power network system
└── Transform                 ← world position = aegis field center
```

`AegisActive` is the canonical signal that an aegis field is live. `AegisEmitter` entities always carry it (never removed). `OutpostBeacon` entities gain/lose it as power delivery changes.

### Player body entity

```
Player body entity
├── PlayerBody                ← marker
├── PrimaryBody               ← marker; only on the starting body; its aegis emitter never fails
├── BodySlot(u8)              ← optional; binds body to a {kbd:body_quick_switch_N} quick-switch slot (1–9, 0; default Alt+digit per input.md §3.4)
├── InAegis                 ← marker; present iff body is inside ≥1 active aegis field
├── AtmosphericExposure       ← only present when NOT InAegis; see §5
└── Transform                 ← world position
```

`InAegis` is derived each tick from position vs. active zones — not stored persistently.

### ActiveBody resource

```rust
#[derive(Resource)]
pub struct ActiveBody(pub Entity);
```

Points to the player body entity currently under Local mode control. Changed by the body-switching mechanic (see §7). All living body entities are found by querying `With<PlayerBody>` — no separate registry needed.

---

## 3. Aegis Boundary Check

**System:** `aegis_boundary_check_system`

Runs every tick. For the entity in `ActiveBody`, determines whether it is inside any active aegis field and updates the `InAegis` marker accordingly.

Step by step:

1. Read `ActiveBody` to get the current player body entity.
2. Read the body's `Transform`.
3. Query all entities with `(AegisRadius, Transform, AegisActive)` — these are the live aegis fields.
4. For each aegis field, compute `distance = body_pos.distance(aegis_center)`.
5. If `distance <= aegis_radius` for any aegis field → body is inside.
6. If inside and `InAegis` absent → insert `InAegis` on body entity; emit `EnteredAegis`.
7. If not inside and `InAegis` present → remove `InAegis`; emit `LeftAegis`.

Drones are not checked — `DroneState` and drone movement have no aegis dependency.

---

## 4. Local Mode Constraint

In Local mode, the active body's movement is bounded by the aegis field it currently occupies. The body cannot be moved outside the aegis radius via player input.

**System:** `aegis_movement_constraint_system`

Runs after `aegis_boundary_check_system`, before physics integration.

Step by step:

1. If `PlayMode` is not `Local`, skip.
2. If active body does not have `InAegis`, skip (body is already outside all aegis fields — power failure scenario handled by §7 countdown).
3. Read active body's `Transform` and all active aegis fields (`AegisRadius + Transform + AegisActive`).
4. Find the aegis field whose center is closest to the body (the "current" field).
5. If the body's proposed next-frame position would exceed that field's radius, clamp the movement delta so the body stays on the inner surface of the sphere.

The player cannot move the body outside an active aegis field while in Local mode. If the aegis field collapses mid-play (power failure), the body is already outside all fields — movement is no longer constrained, but the power-failure countdown (§7) governs what happens next.

Any input requesting Local mode while body lacks `InAegis` is silently ignored (body must be inside an aegis to inhabit it in Local mode).

---

## 5. Atmospheric Exposure

When the active body is outside all active aegis fields, it sustains hardware damage from environmental exposure.

### AtmosphericExposure component

```rust
#[derive(Component)]
pub struct AtmosphericExposure {
    pub elapsed_secs: f32,
}
```

Added to the body when `LeftAegis` fires. Removed when `EnteredAegis` fires. Timer does not persist across exposure gaps — re-entering an aegis field resets the clock.

### atmospheric_exposure_system

Step by step:

1. Query `(Entity, &mut AtmosphericExposure)` on the active body.
2. Increment `elapsed_secs` by `delta_seconds`.
3. If `elapsed_secs >= EXPOSURE_LETHAL_THRESHOLD_SECS`:
   - Emit `BodyDestroyed { body: Entity }`.
   - Despawn the body entity.
   - If the destroyed body was `ActiveBody`, set `ActiveBody` to the next available body entity (see §7 body registry). If no body remains, emit `RunFailed`.

`EXPOSURE_LETHAL_THRESHOLD_SECS` is a tunable constant, not seed-derived. Target: long enough that a power interruption gives the player time to manually switch to Remote mode and recall a drone; short enough that ignoring it means body loss.

---

## 6. Aegis Emitter Upgrades

The Aegis Emitter's radius expands when the machine is upgraded in-place (standard machine upgrade path — not specific to the aegis system). When radius changes:

**System:** `aegis_expansion_system` — triggered by `MachineUpgraded { entity }` event.

Step by step:

1. Check if the upgraded entity has `AegisEmitter`.
2. If yes, read the new `AegisRadius` from the upgrade definition asset.
3. Write the new radius to the `AegisRadius` component.
4. Emit `AegisExpanded { generator: Entity, new_radius: f32 }`.

**Ore deposit removal:** `AegisExpanded` is consumed by the world/deposit system (outside this spec). Any ore deposit whose center is now within the new radius is removed.

The boundary check system picks up the new radius automatically on the next tick — no additional wiring needed.

---

## 7. Outpost Beacon (MVP)

### Placement

Outpost Beacon is a buildable item (asset id: `outpost_beacon`). Placed by drone from `DroneInventory` using the standard machine-placement flow. Spawns an `OutpostBeacon` entity with `AegisRadius` from its asset definition. Does **not** spawn with `AegisActive` — power must be connected first.

### Power requirement

`OutpostBeacon` carries a `PowerConsumer` component. The power network system (defined in `networks.md §3`) delivers power to consumers each tick. When delivered watts ≥ required watts:

- **System:** `outpost_power_check_system` (runs after power network tick)
  1. Query all `OutpostBeacon` entities.
  2. For each, read `PowerConsumer.delivered_watts` vs. `PowerConsumer.required_watts`.
  3. If delivered ≥ required and `AegisActive` absent → insert `AegisActive`; emit `OutpostActivated { beacon }`.
  4. If delivered < required and `AegisActive` present → remove `AegisActive`; emit `OutpostDeactivated { beacon }`.

### Outpost independence

Each outpost has its own power network and may have its own logistics network. Outposts do not need a continuous link to the main base or to each other. Once a body is activated at an outpost with an active aegis, the player can switch to it freely regardless of inter-outpost network connectivity.

### Body fabrication

Body chassis is a craftable item (asset id: `body_chassis`). Standard recipe — fabricated at an Assembler. Chassis item can be loaded into drone inventory and transported to an outpost.

**Activating a body at an outpost:** player selects "Upload" on the `OutpostBeacon` entity while the body chassis item is physically inside the beacon's aegis field (in drone inventory or a logistics container within range). The system:

1. Consumes one `body_chassis` item from inventory.
2. Spawns a `PlayerBody` entity at the beacon's `Transform`.
3. The new body does not become `ActiveBody` automatically — player switches deliberately (see below).

### Body switching

Player can switch `ActiveBody` when:

- Current `PlayMode` is `Local` (player is actively inhabiting the current body).
- Target body has `InAegis` (target is inside an active aegis field at its outpost).

No inter-outpost network link is required.

**System:** `body_switch_system` — triggered by `RequestBodySwitch { target: Entity }`.

Step by step:

1. Validate conditions above. If either fails, emit `BodySwitchFailed { reason }` and return.
2. Set `ActiveBody` to target entity.
3. Emit `BodySwitchComplete { from, to }`.

The camera and input system bind to `ActiveBody` — no additional wiring needed on switch.

### Body/Drone quick-switch UI

A panel (toggled by a dedicated key) lists all living `PlayerBody` entities and all active drones. Each can be bound to a slot numbered 1–9 and 0 (ten slots total).

**Input:** `{kbd:body_quick_switch_N}` (default `Alt+digit`; see `input.md §3.4`) — switches `ActiveBody` if the slot holds a body; activates drone focus if the slot holds a drone.

Slot bindings persist for the run. Bodies and drones may be rebound freely. Unbound entities appear in the panel list but have no quick-switch key until assigned.

The `BodySlot(u8)` component on `PlayerBody` entities stores the bound slot index. Drones use the equivalent `DroneSlot(u8)` component (defined in `drone.md`).

### Power interruption collapse

When `OutpostDeactivated` fires for a beacon hosting the active body:

1. `aegis_boundary_check_system` removes `InAegis` from the body (aegis gone → body now outside).
2. `atmospheric_exposure_system` begins ticking `AtmosphericExposure` on the body.
3. `power_failure_countdown_system` emits `OutpostPowerFailureWarning` and starts a countdown (`POWER_FAILURE_COUNTDOWN_SECS`). UI shows a hazard alert and countdown timer.

If the player restores power before the countdown expires: `AegisActive` returns, `InAegis` returns, `AtmosphericExposure` removed, countdown cancelled.

If the countdown expires without power restored: `power_failure_countdown_system` emits `RequestBodySwitch { target: PrimaryBody }`. `ActiveBody` automatically switches to the `PrimaryBody` entity at the main base, whose aegis emitter has no power requirement and can never fail. The original body remains at the outpost taking atmospheric exposure damage — the player can switch back once power is restored, if the body has not been destroyed.

`POWER_FAILURE_COUNTDOWN_SECS` is tunable. Target: long enough to give the player a realistic chance to restore power; short enough that ignoring the warning means losing control of the outpost body.

---

## 8. Systems

| System | Trigger | Purpose |
|---|---|---|
| `aegis_boundary_check_system` | Every tick | Add/remove `InAegis` on active body |
| `aegis_movement_constraint_system` | After boundary check, before physics | Clamp Local-mode body movement to aegis radius |
| `atmospheric_exposure_system` | Every tick (body lacks `InAegis`) | Increment exposure timer; destroy body at threshold |
| `aegis_expansion_system` | `MachineUpgraded` event | Update `AegisRadius`; emit `AegisExpanded` |
| `outpost_power_check_system` | After power network tick | Add/remove `AegisActive` on beacons |
| `power_failure_countdown_system` | `OutpostDeactivated` event / every tick | Start countdown; auto-switch to `PrimaryBody` on expiry |
| `body_switch_system` | `RequestBodySwitch` event | Validate and execute `ActiveBody` change |

---

## 9. Messages

| Message | Payload | Emitted by |
|---|---|---|
| `EnteredAegis` | `body: Entity` | `aegis_boundary_check_system` |
| `LeftAegis` | `body: Entity` | `aegis_boundary_check_system` |
| `AegisExpanded` | `generator: Entity, new_radius: f32` | `aegis_expansion_system` |
| `OutpostActivated` | `beacon: Entity` | `outpost_power_check_system` |
| `OutpostDeactivated` | `beacon: Entity` | `outpost_power_check_system` |
| `OutpostPowerFailureWarning` | `beacon: Entity, body: Entity, remaining_secs: f32` | `power_failure_countdown_system` |
| `RequestBodySwitch` | `target: Entity` | UI / player input / `power_failure_countdown_system` |
| `BodySwitchFailed` | `reason: SwitchFailReason` | `body_switch_system` |
| `BodySwitchComplete` | `from: Entity, to: Entity` | `body_switch_system` |
| `BodyDestroyed` | `body: Entity` | `atmospheric_exposure_system` |
| `RunFailed` | — | `atmospheric_exposure_system` (no bodies remain) |

---

## 10. Execution Order

```
[Power network tick]
    └─ outpost_power_check_system
           └─ (emits OutpostActivated / OutpostDeactivated)
                  └─ power_failure_countdown_system   (on OutpostDeactivated; also ticks each frame)

[Per-tick boundary phase]
    └─ aegis_boundary_check_system
           └─ aegis_movement_constraint_system        (before physics integration)
           └─ atmospheric_exposure_system

[Event-driven]
    └─ aegis_expansion_system        (on MachineUpgraded)
    └─ body_switch_system              (on RequestBodySwitch)
```

`aegis_movement_constraint_system` must run after `aegis_boundary_check_system` and before physics integration — ordering enforced via `after()`/`before()` constraints. `atmospheric_exposure_system` runs after `aegis_boundary_check_system`.

---

## 11. Vertical Slice Scope

| Feature | VS | MVP |
|---|---|---|
| Aegis Emitter with fixed radius | ✓ | ✓ |
| Starting zone prevents ore deposits | ✓ | ✓ |
| Boundary check → `InAegis` | ✓ | ✓ |
| Local mode movement clamped to aegis radius | ✓ | ✓ |
| Atmospheric exposure timer + body destruction | ✓ | ✓ |
| Aegis Emitter upgrade / radius expansion | — | ✓ |
| Outpost Beacon placement | — | ✓ |
| Outpost Beacon power requirement | — | ✓ |
| Body fabrication + activation | — | ✓ |
| Body switching (marker-component based) | — | ✓ |
| Power failure countdown + auto-switch to PrimaryBody | — | ✓ |
| Body/Drone quick-switch panel (`{kbd:body_quick_switch_N}`, slots 1–9, 0) | — | ✓ |

For VS: one `AegisEmitter` spawned at run start with a fixed radius constant. No upgrades, no beacons, no body switching. `AtmosphericExposure` applies if the player body somehow ends up outside (debug/test only in normal play).

---

## 12. Edge Cases

| Case | Behavior |
|---|---|
| Body inside beacon aegis field; beacon loses power mid-frame | `outpost_power_check_system` removes `AegisActive` → `aegis_boundary_check_system` removes `InAegis` → `power_failure_countdown_system` starts countdown; all same tick |
| Power restored before countdown expires | `AegisActive` re-added → `InAegis` re-added → `AtmosphericExposure` removed; countdown cancelled |
| Countdown expires without power restored | `power_failure_countdown_system` emits `RequestBodySwitch { target: PrimaryBody }`; `ActiveBody` switches to main-base body; former body left at outpost with `AtmosphericExposure` ticking |
| Player requests body switch while in Remote mode | `body_switch_system` rejects: must be in Local mode to switch |
| Player requests body switch; target body lacks `InAegis` | `body_switch_system` emits `BodySwitchFailed { reason: TargetNotInAegis }`; no state change |
| Last body destroyed | `atmospheric_exposure_system` emits `RunFailed`; no `ActiveBody` target; game ends run |
| Two beacons in the same location | Both project independent aegis fields; `InAegis` is true if inside either — either losing power does not affect the other |
| Body chassis item outside any aegis field when player selects Upload | Upload rejected — chassis must be accessible within the target beacon's aegis field (in logistics range or drone inventory within zone) |
| `AegisExpanded` covers an active ore deposit | `AegisExpanded` is observed by the deposit system; deposit removed on same event; no deferred cleanup |
| Player bound a body to a `{kbd:body_quick_switch_N}` slot; body is destroyed | Slot becomes empty; panel shows "(destroyed)"; slot reusable for rebind |

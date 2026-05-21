# Power System Design

Generator kinds, energy production, batteries, and environmental coupling. Sits on top of the cable graph defined in `networks.md §3`.

`networks.md` owns the cable network mechanics — topology, `GeneratorUnit` buffer pooling via `PowerNetworkMembers`, the `give_energy`/`take_energy`/`has_energy` surface, voltage/amp gating at recipe start, and the non-destructive failure modes. This doc covers **how generators produce the energy** that flows through that network.

Generators are machines. They go through the same placement flow as any other machine (`machines.md §4`), expose IO ports, and run recipes (`crafting.md §5`). The only thing special about a generator is that its recipe output is **energy** rather than items, and that some generators consume **virtual items** sourced from an environmental port rather than real items routed through a logistics network.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Generator Kinds](#2-generator-kinds)
3. [Asset Schema](#3-asset-schema)
4. [ECS Structure](#4-ecs-structure)
5. [Environmental Ports](#5-environmental-ports)
6. [Virtual Items](#6-virtual-items)
7. [Energy as Recipe Output](#7-energy-as-recipe-output)
8. [Throttle](#8-throttle)
9. [Batteries](#9-batteries)
10. [Weather & Day/Night Coupling](#10-weather--daynight-coupling)
11. [Lightning Rods & Burst Generators](#11-lightning-rods--burst-generators)
12. [Run Variance](#12-run-variance)
13. [Systems & Messages](#13-systems--messages)
14. [Execution Order](#14-execution-order)
15. [Vertical Slice Scope](#15-vertical-slice-scope)
16. [Edge Cases](#16-edge-cases)

---

## 1. Overview

A generator is a `Machine` with a `GeneratorUnit` buffer (see `networks.md §3`). It runs one or more recipes that produce **energy** as output. Recipe inputs differ by kind:

- **Active generator** — recipe consumes real items routed through a logistics port (coal, refined fuel, fuel rods).
- **Passive generator** — recipe consumes virtual items produced by a built-in environmental port (sunlight, heat, lightning pulses). Virtual items never enter the logistics graph.
- **Burst generator** — passive generator whose env port emits in stochastic bursts (e.g. lightning strikes).

All three kinds share the same recipe-execution machinery from `crafting.md §5`. Energy output is written into the machine's `GeneratorUnit` buffer; consumers draw from it via the pooled `PowerNetworkMembers.take_energy` surface defined in `networks.md §3`.

```
Active   :  fuel item ─[logistics port]─► RecipeProcessor ─► joules ─► GeneratorUnit ─► network pool
Passive  :  env       ─[env port]──────► RecipeProcessor ─► joules ─► GeneratorUnit ─► network pool
Burst    :  storm     ─[env port pulse]─► RecipeProcessor ─► joules ─► GeneratorUnit ─► network pool
```

---

## 2. Generator Kinds

| Kind | Input source | Examples | Key property |
|---|---|---|---|
| **Active** | Logistics port (real items) | Combustion, nuclear, catalyst-gated exotic | Fuel routed via factory logistics; emits byproduct items where defined |
| **Passive** | Built-in env port (virtual items) | Solar panel, geothermal coupler, wind turbine | Tied to environmental rate; placement-location dependent |
| **Burst** | Built-in env port (event-driven pulses) | Lightning rod | Stochastic energy spikes; storage saturation can drop pulses |

`GeneratorDef.kind` selects which port wiring is used at spawn time. From the recipe system's perspective all three are identical — recipe inputs name items, the source of those items differs.

---

## 3. Asset Schema

Generators reuse `MachineDef` / `MachineTierDef` (`machines.md §3`). Generator-specific fields live in an optional `generator: Option<GeneratorDef>` block on `MachineTierDef`:

```rust
pub struct GeneratorDef {
    pub kind: GeneratorKind,             // Active | Passive | Burst
    pub watts: f32,                      // rated output for buffer-fill scheduling/UI
    pub max_buffer_joules: f32,          // GeneratorUnit buffer cap
    pub voltage_tier: u8,                // emit tier; must match cable voltage
    pub env_port: Option<EnvPortDef>,    // present iff kind == Passive | Burst
    pub throttle: ThrottleMode,          // OnBufferFull | NeverThrottle
}

pub enum GeneratorKind { Active, Passive, Burst }

pub enum ThrottleMode {
    OnBufferFull,    // pause fuel/env consumption when buffer == max
    NeverThrottle,   // burn constantly; excess discarded (advanced gens, design-flag)
}

pub struct EnvPortDef {
    pub virtual_item: ItemId,            // e.g. "sunlight_tick", "heat_tick", "energy_pulse"
    pub env_source: EnvSource,           // Solar | Thermal | Lightning | Wind | Custom(...)
    pub rated_rate: f32,                 // virtual items per second at env_factor = 1.0
    pub burst_mode: Option<BurstMode>,   // present iff kind == Burst
}

pub enum EnvSource {
    Solar,                               // env_factor = planet.sun_factor * sky_visibility * time_of_day
    Thermal,                             // env_factor from local geothermal site lookup
    Lightning,                           // env_factor = 1.0, but emission is event-driven (see BurstMode)
    Wind,                                // env_factor from planet wind_factor * altitude_bonus
    Custom(String),                      // post-VS hook for alien sources
}

pub struct BurstMode {
    pub trigger_event: EventKind,        // e.g. LightningStrike
    pub range_m: f32,                    // distance from event origin that activates this generator
    pub items_per_event: u32,            // virtual items injected per qualifying event
}
```

Recipes for generators live in the recipe graph (`recipe-graph.md`) with `machine_type == generator_def.id` and outputs that include an `Energy(joules)` entry. See §7.

---

## 4. ECS Structure

Generator-specific components attached to the machine entity (in addition to standard `Machine` components from `machines.md §3`):

| Component | Entity | Purpose |
|---|---|---|
| `GeneratorUnit { pos, voltage_tier, watts, buffer_joules, max_buffer_joules }` | Machine entity | Buffer + voltage; defined in `networks.md §3`. Pooled via `PowerNetworkMembers` |
| `Throttle(ThrottleMode)` | Machine entity | Drives the throttle gate on `recipe_start_system` for this machine |
| `EnvPort { source, virtual_item, rated_rate, burst_mode }` | Env port entity (child of machine) | Present iff generator is Passive or Burst |
| `EnvPortOf(Entity)` | Env port entity | Relationship back to the owning machine (parallels `LogisticsPortOf` / `EnergyPortOf`) |
| `MachineEnvPorts` | Machine entity | Relationship-target listing env port entities |
| `LocalVirtualStock(HashMap<ItemId, u32>)` | Machine entity | Local-only counter that `recipe_start_system` reads instead of `LogisticsNetworkMembers.has_items` when an input is a virtual item |

Env ports are spawned by the `generator_observer` on `OnAdd<Machine>` when `MachineRegistry.tier_def(item_id).generator.kind != Active`. The observer matches the machine type filter (`"generator"` umbrella, dispatched by `GeneratorDef.kind`) — same pattern as the other special-case observers in `machines.md §4 — Special-case machines`.

`LocalVirtualStock` exists so that recipe input checks (`has_items` / `take_items` in `crafting.md §5`) have a uniform integer count to consume against. Virtual item emission writes into this stock; recipe start drains from it.

---

## 5. Environmental Ports

An env port is a port entity owned by the machine but **not** attached to any cable network. It produces virtual items at a rate driven by an environmental input.

### Spawn

`generator_observer` runs on `OnAdd<Machine>` for any machine whose `GeneratorDef.env_port` is `Some`:
1. Spawns a child entity carrying `EnvPort { source, virtual_item, rated_rate, burst_mode }`, `EnvPortOf(machine)`, and `Transform` at the asset-declared offset.
2. Inserts `MachineEnvPorts` on the machine listing the new env port.
3. Initializes `LocalVirtualStock` on the machine with a 0 count for the env port's `virtual_item`.

Env ports have no `Collider`. They cannot be targeted by cable placement. They cannot be edited by `PortPolicy`.

### Tick

`env_port_tick_system` runs in `PowerSimSystems` once per tick. For each env port:

1. **Continuous sources** (`Solar`, `Thermal`, `Wind`) — compute `env_factor` via `EnvFactorRegistry` lookup (see §10). Add `rated_rate * env_factor * dt` to a per-port accumulator. When the accumulator ≥ 1.0, transfer `floor(accumulator)` virtual items into the owning machine's `LocalVirtualStock` and subtract the integer part. This produces a smooth rate regardless of frame timing.
2. **Burst sources** (`Lightning`) — no per-tick emission. Burst handler observes the trigger event and pushes virtual items directly into `LocalVirtualStock` (see §11).
3. **Throttle gate.** If the owning machine's `Throttle == OnBufferFull` and `GeneratorUnit.buffer_joules == GeneratorUnit.max_buffer_joules`, skip emission this tick. Virtual items are not produced while the buffer is full — env input is not wasted on throttled passives. Lightning bursts arriving while throttled are **dropped at the rod** (see §11 — design intent for burst risk).

### Stock cap

`LocalVirtualStock` is clamped to a small ceiling per virtual item (default `64`) to prevent runaway accumulation during long buffer-full periods that precede a sudden demand surge. Reaching the cap behaves like the buffer-full case: no further emission until the recipe consumes stock.

---

## 6. Virtual Items

Virtual items are normal `ItemDef`s in the item registry, but flagged `virtual: true`. They:

- Cannot be picked up by the player or placed into a hotbar slot.
- Are not routed through logistics networks; `LogisticsNetworkMembers.has_items` / `give_items` reject them (defensive check; in practice `LocalVirtualStock` is the only producer/consumer).
- Have no in-world model; they exist as counters only.
- Appear in the Terminal item codex with a "virtual" tag so players can read recipe inputs that name them.

Reserved virtual item ids for VS:

| `ItemId` | Source | Notes |
|---|---|---|
| `sunlight_tick` | Solar panels | 1 tick = baseline solar quantum |
| `heat_tick` | Geothermal couplers | 1 tick = baseline heat quantum |
| `energy_pulse` | Lightning rods | 1 pulse = one lightning strike's worth |
| `wind_tick` | Wind turbines (post-VS) | 1 tick = baseline wind quantum |

Recipe authors size inputs/outputs around these quanta. A solar recipe might be `[sunlight_tick × 1] → [Energy(10kJ)]` running on a `1s` duration; tuning lives in the recipe def, not the env port.

---

## 7. Energy as Recipe Output

Generator recipes produce energy via `RecipeOutput::Energy { joules, chance }`. The variant is defined in `recipe-graph.md §6`:

```rust
pub enum RecipeOutput {
    Item   { item: ItemId, quantity: u32, chance: f32 },
    Energy { joules: f32,  chance: f32 },
}
```

Generator-recipe invariants from `recipe-graph.md §10`:
- **#16** — `outputs[0]` of any recipe on a generator machine must be `RecipeOutput::Energy` (primary output = energy yield).
- **#17** — only generator machines may produce energy.

### Completion routing

`recipe_completion_system` (`crafting.md §5`) iterates `recipe.outputs`, rolls each entry's `chance`, and dispatches by variant:
- `Item` → `give_items` on the machine's output-eligible logistics ports (existing path; ash, depleted_rod, and any other byproducts route here as ordinary items).
- `Energy` → `GeneratorUnit.give_energy(joules)` on the machine entity. Clamped to `max_buffer_joules`; surplus discarded and `GeneratorOverflow { machine, joules_lost }` fired (§13).

Energy is **not withdrawn per-tick during generator progress**. The full `joules` value lands at completion in a single write. This matters for cancellation: per `machines.md §5`, an in-progress generator recipe interrupted by removal/cancellation refunds its input items and emits **zero joules**. There is no partial energy production — the recipe either completes (full yield) or is cancelled (no yield).

### Generator recipes that also consume energy

A generator recipe may set `energy_cost > 0` — e.g. a nuclear pre-heater stage that draws network power to initiate the fuel cycle, then produces a larger amount of energy on completion. This is allowed and goes through the standard `recipe_progress_system` per-tick withdrawal (`crafting.md §5`). The two flows are independent:
- `energy_cost` (per-tick withdrawal during progress) is unrelated to `RecipeOutput::Energy` (completion-time deposit).
- A net-positive generator recipe satisfies `output_energy_joules > energy_cost`. The runtime does not enforce net-positivity — it's a recipe-design concern. A net-negative generator recipe is legal but pointless.

This is why `RecipeOutput::Energy` is a distinct variant rather than a negative `energy_cost`: the cadence is different (completion vs per-tick), and a recipe may need both directions simultaneously.

### Voltage gating

Voltage and amp checks at start use the **recipe's `min_voltage_tier`** for the energy-cost (per-tick) draw, exactly as for any recipe (`crafting.md §5`). For the **output side**, the generator's `GeneratorUnit.voltage_tier` must match the cable network the generator is joined to: a generator on a network below its emit tier refuses to start with `BlockReason::VoltageMismatch`. You cannot pour T2 energy into a T1 network.

### Byproduct items

Item byproducts (ash, depleted_rod, slag) are ordinary `RecipeOutput::Item` entries after `outputs[0]`. They route through the standard logistics output path. `GeneratorDef` does not gate byproducts — clean-burn vs ash-emitting is a recipe-level authoring decision. Multiple alternative combustion recipes (clean and ash-emitting) may coexist in the graph; `recipe_start_system` selects among them per the machine's policy.

---

## 8. Throttle

Two modes; designer chooses per `GeneratorDef`:

- **`OnBufferFull` (default)** — when `GeneratorUnit.buffer_joules == max_buffer_joules`:
  - Active generators: `recipe_start_system` refuses to start a new recipe; running recipes complete and write joules (overflow discarded — but throttle prevents starting next recipe, so steady-state runaway is bounded by one in-flight recipe).
  - Passive/burst generators: `env_port_tick_system` skips virtual item emission (§5).
- **`NeverThrottle`** — emission and recipe selection ignore the buffer state. Energy completing into a full buffer is **discarded** (lost). Designer flag to encourage solving the logistics problem (consume the energy or waste it).

In-flight recipes for `OnBufferFull` active generators are **not interrupted** when the buffer fills mid-recipe; they complete and their joules are discarded if the buffer is full at completion. Interrupting mid-recipe would either waste the partially-consumed input or require partial refund — neither is appealing. The buffer-overflow loss is small in practice because the next recipe won't start.

`Throttle` is its own component (not a `GeneratorDef` field on the machine) so save/load reflects the designer's choice without re-reading assets, and so post-VS modifiers can change throttle mode dynamically.

---

## 9. Batteries

A battery is a non-generator machine that **stores and re-emits** energy. It participates in the `GeneratorUnit` buffer pool but never runs recipes.

### Components

| Component | Entity | Purpose |
|---|---|---|
| `BatteryUnit { buffer_joules, max_buffer_joules, voltage_tier, max_charge_watts, max_discharge_watts }` | Machine entity | Storage + rate caps |

`BatteryUnit` is **distinct** from `GeneratorUnit`. Both participate in `PowerNetworkMembers` accounting, but only `BatteryUnit` accepts charging.

### Pool participation

`PowerNetworkMembers` exposes the pooled surface (`networks.md §3`). For `take_energy`:
- Iterates all members with `GeneratorUnit` or `BatteryUnit` and draws from their buffers (priority: generators first, batteries second — favor consuming live production before stored).
- For batteries, draw is rate-capped to `max_discharge_watts * dt` per tick across all consumers (network-wide, not per-consumer).

For `give_energy` (called only by generator completion and the battery charge tick):
- Generators always write to their **own** `GeneratorUnit`. Surplus does not spill to batteries directly.
- Batteries charge via a dedicated system, not via `give_energy`.

### Charging

`battery_charge_system` runs in `PowerSimSystems` after `env_port_tick_system` and after any generator recipe completions for the tick. For each `BatteryUnit` whose owning network has **surplus** (sum of `GeneratorUnit.buffer_joules` > 0 and no demand drew it this tick):

1. Compute network surplus joules this tick (snapshot taken after generator emission and before consumer demand).
2. For each battery, draw up to `max_charge_watts * dt` joules from the surplus pool and add to `buffer_joules` (clamped to `max_buffer_joules`).
3. Battery acts as the "consumer of last resort" — drawn after all consumer recipes have taken their per-tick energy.

This implies the per-tick order is:
```
env_port_tick (passives produce virtual items)
generator recipe completion (joules into generator buffers)
recipe_progress (consumers take_energy from pooled buffers)
battery_charge_system (batteries absorb remaining surplus)
```

### Discharge

`take_energy` draws from generator buffers first, then battery buffers (with `max_discharge_watts` rate cap). No explicit "discharge" event — batteries are transparent to consumers.

### Voltage

Batteries have a voltage tier. They participate in `PowerNetworkMembers.voltage_tier()` calculation (the network voltage is the minimum tier across all members; placing a T1 battery on a T2 network downgrades it). Players must choose battery tier to match network tier.

### Save

`BatteryUnit` is fully persisted (`#[require(Save)]` via `Machine` per `save.md`). Buffer state survives save/load.

---

## 10. Weather & Day/Night Coupling

Env factor for passive generators comes from `EnvFactorRegistry`, a Bevy resource. Each `EnvSource` has a registered evaluator:

```rust
pub trait EnvFactorEvaluator: Send + Sync {
    fn factor(&self, world: &World, pos: Vec3) -> f32;
}

pub struct EnvFactorRegistry {
    evaluators: HashMap<EnvSource, Box<dyn EnvFactorEvaluator>>,
}
```

Default evaluators (VS):

| `EnvSource` | Factor formula |
|---|---|
| `Solar` | `planet.sun_factor * sky_visibility(pos) * time_of_day_factor()` |
| `Thermal` | `geothermal_site_lookup(pos)` — returns 0.0 if not on/adjacent to a vent; else site-specific value scaled by `planet.geothermal_factor` |
| `Lightning` | Always `1.0` for continuous emission (which is zero); driven entirely by `BurstMode` events |
| `Wind` | `planet.wind_factor * altitude_bonus(pos.y)` |

### Day/Night

`TimeOfDay` resource (post-Planet-Identity addition) holds a `0.0..1.0` value cycling at planet-specific rate. `time_of_day_factor()` returns a piecewise function: 0 from 0.0–0.2, ramps to 1.0 by 0.3, holds until 0.7, ramps to 0 by 0.8, 0 from 0.8–1.0 (dusk/dawn ramps). Exact curve is tunable.

### Weather

`WeatherState` resource (post-Planet-Identity) carries the current weather event (clear, cloudy, storm, dust). It multiplies the relevant env factors:

| Weather | Solar mult | Wind mult | Lightning rate |
|---|---|---|---|
| Clear | 1.0 | normal | 0 |
| Cloudy | 0.4 | normal | 0 |
| Storm | 0.1 | 1.5 | high |
| Dust | 0.5 | normal | 0 |

Weather transitions are seeded events from `WeatherSchedule` (seeded per run, similar to `WeatherEvent` rolling per game-hour). VS may use a single-state baseline weather; full dynamic weather is post-VS — but the multipliers live in `EnvFactorRegistry` from VS so the upgrade path is just enabling `WeatherSchedule`.

### Sky visibility

`sky_visibility(pos)` raycasts upward from the panel and returns 1.0 if unobstructed by terrain/machine, 0.0 if fully obstructed, with a small step gradient for partial occlusion (count of obstructed sample rays). Refreshed at most once per second per env port to amortize cost.

---

## 11. Lightning Rods & Burst Generators

`LightningStrikeEvent { pos: Vec3, energy: f32 }` is emitted by the weather system at a seeded rate during storms. `lightning_burst_system` runs in `PowerSimSystems` on this event:

1. Find all `EnvPort`s with `source == Lightning` within their respective `burst_mode.range_m` of the strike position.
2. For each in-range rod, **target the nearest one only** (a strike isn't shared) — sort by distance, pick the closest.
3. Inject `burst_mode.items_per_event` `energy_pulse` virtual items into that rod's owning machine's `LocalVirtualStock`, clamped to the stock cap.
4. **If the rod's `Throttle == OnBufferFull` and its `GeneratorUnit.buffer_joules == max_buffer_joules`, drop the pulses entirely** (pulses do not queue, do not refill later). This is a deliberate design tension: undersized storage during storms loses energy.
5. If no rod is in range, the strike's energy is lost (purely flavor — the strike still plays its visual/audio).

`recipe_start_system` then picks up the `energy_pulse` stock the next tick and runs the rod's recipe (e.g. `[energy_pulse × 1] → [Energy(500kJ)]`).

Range checks use a flat horizontal distance; vertical doesn't matter for strike attraction in this model.

---

## 12. Run Variance

What varies per run, layered:

**Layer 1 — Param variance (always on).** Common generator recipes vary inputs/yield/time/energy within the bounds declared in `recipe-graph.md` (`§ — variance bounds`). A combustion recipe in run A burns 1 coal/1.0s → 200kJ; in run B burns 1 coal/0.7s → 280kJ. Within the per-recipe envelope, no run logic needed.

**Layer 2 — Generator availability (seeded).** Whether a given generator's tech-tree node spawns is a per-run roll, owned by `tech-tree-design.md §374`. Combustion and Solar both always spawn at T1 (every run needs power); higher-tier or unique generators are gated.

**Layer 3 — Unique per-run generators (post-VS).** Generator definitions that exist only in some seeds. Power.md leaves the schema hook (`GeneratorDef.unique_run_id: Option<String>`) and defers the spawning logic to the generator-graph generator (post-VS).

**Fuel identity does not vary.** Coal remains coal across runs. The strategic shift comes from param variance (how much energy per coal) and from which tech nodes exist (do you have nuclear this run?), not from fuel substitution.

**Env factor variance.** `planet.sun_factor`, `planet.geothermal_factor`, `planet.wind_factor` are per-run constants set by `planet-identity.md`. A low-solar world (`sun_factor = 0.3`) forces combustion strategy; a geologically inert world (`geothermal_factor = 0`) eliminates geothermal entirely.

---

## 13. Systems & Messages

| System | Schedule | Purpose |
|---|---|---|
| `generator_observer` | `OnAdd<Machine>` | Inserts `GeneratorUnit`, `Throttle`, env port child entity, `MachineEnvPorts`, `LocalVirtualStock` for generator-kind machines |
| `env_port_tick_system` | `PowerSimSystems` | Continuous virtual item emission for Passive sources; respects throttle |
| `lightning_burst_system` | `PowerSimSystems` (event-driven) | Targets nearest in-range rod and injects pulses |
| `battery_charge_system` | `PowerSimSystems`, after consumers | Absorbs surplus joules into batteries within rate caps |
| `generator_recipe_complete_observer` | Observer on `RecipeCompleted` | Routes `RecipeOutput::Energy` to the machine's `GeneratorUnit.give_energy`; routes byproducts via existing item path. Generator recipes are discoverable via `RecipeGraph.energy_producers()` / `by_energy_output` — see `recipe-graph.md §7`. |
| (existing) `generator_system` from `networks.md §3` | `PowerSimSystems` | Cable connection/disconnection; fires `NetworkChanged<Power>` |
| (existing) `generator_tick_system` from `networks.md §3` | `PowerSimSystems` | Currently fills buffer at `watts`; **superseded** by recipe-driven completion (see migration note below) |

### Migration note: `generator_tick_system`

`networks.md §3` describes a `generator_tick_system` that fills generator buffers at `watts * dt`. That fill model is replaced by the recipe-completion path in this spec — generators fill their buffers via `RecipeOutput::Energy` written at recipe completion, not at constant `watts`. The `watts` field on `GeneratorUnit` is retained for UI display (rated output) and for `EnvFactorRegistry` consumers that key off it, but the per-tick fill behavior moves to `recipe_completion_system`.

`networks.md` should be updated to reference this doc for the production model; this doc is the canonical source for **how energy enters the buffer**.

### Messages

| Event | Fields | Producers | Consumers |
|---|---|---|---|
| `LightningStrikeEvent` | `pos: Vec3, energy: f32` | `weather_system` (post-VS) | `lightning_burst_system` |
| `GeneratorOverflow` | `machine: Entity, joules_lost: f32` | `generator_recipe_complete_observer` (when buffer full and throttle == NeverThrottle) | Telemetry (`telemetry.md`) |
| `BatteryChargeChanged` | `battery: Entity, delta: f32` | `battery_charge_system` | (Optional) UI |
| `NetworkChanged<Power>` | (from `networks.md §3`) | This doc's systems on buffer 0→positive transition | `recipe_start_system` in logistics |

`NetworkChanged<Power>` is fired by `generator_recipe_complete_observer` when the generator's buffer transitions 0 → positive, and by `battery_charge_system` when a battery transitions 0 → positive — both to unblock paused consumer recipes per `networks.md §3 — Power as a Consumable Resource`.

---

## 14. Execution Order

```
NetworkSystems::of::<Power>()           // cable_placed, cable_removed
  → PowerSimSystems                     // ordered substages:
      env_port_tick_system              // passives produce virtual items
      lightning_burst_system            // event-driven burst injection
      [generator machines run recipes via crafting.md §5]
        recipe_start_system  (subset for generators)
        recipe_progress_system
        recipe_completion_system → generator_recipe_complete_observer
      battery_charge_system             // absorb remaining surplus
    → NetworkSystems::of::<Logistics>()
      → LogisticsSimSystems             // consumers draw via take_energy
```

Note: `recipe_start_system` and `recipe_progress_system` are shared with non-generator machines (`crafting.md §11`). The generator sub-ordering above shows where generator-specific input gathering and output routing fit; the underlying systems themselves run once and process all machines.

`battery_charge_system` runs **after** the consumer pass in `LogisticsSimSystems` is logically complete, but since both belong to the same frame's tick the implementation orders `battery_charge_system` last in `PowerSimSystems` and relies on the fact that consumer `take_energy` calls during recipe_progress have already debited the relevant buffers. (Alternative: split into two `PowerSimSystems` stages with the battery charge in a post-stage. Implementation detail; either is correct as long as batteries see the post-consumer buffer state.)

---

## 15. Vertical Slice Scope

**In VS:**
- `GeneratorKind::Active` (combustion at T1)
- `GeneratorKind::Passive` with `EnvSource::Solar` (solar panel at T1)
- `GeneratorDef` asset schema, env port spawning, virtual items (`sunlight_tick`), `LocalVirtualStock`
- `BatteryUnit` machine with charge/discharge rate caps, voltage tier participation
- `ThrottleMode::OnBufferFull` (default for all VS generators)
- `EnvFactorRegistry` resource with Solar evaluator wired to `planet.sun_factor` and a static `sky_visibility` (no day/night cycle in VS — `time_of_day_factor` returns 1.0)
- Recipe-driven energy production (`RecipeOutput::Energy`)
- Per-recipe param variance (`recipe-graph.md`)

**Out of VS (post-VS):**
- `GeneratorKind::Burst` and lightning rods
- `EnvSource::Thermal`, `EnvSource::Wind`
- `TimeOfDay` resource and day/night cycle
- `WeatherState`, `WeatherSchedule`, `LightningStrikeEvent`
- `ThrottleMode::NeverThrottle` advanced generators
- Nuclear / catalyst-gated exotic active generators
- Unique per-run generators (`GeneratorDef.unique_run_id`)
- Transformer machines / voltage step-up (deferred pending playtest; see `gdd.md §387`)

This means in VS, solar generators run at `planet.sun_factor * sky_visibility` (a per-run constant once the panel is placed), and combustion is the only fuel-burning kind. The full env coupling machinery exists structurally (registry, env ports, virtual items) so post-VS upgrades flip on additional evaluators without restructuring.

---

## 16. Edge Cases

1. **Generator on T1 network when it emits T2.** Recipe start refuses with `BlockReason::VoltageMismatch`. UI displays "Network voltage too low for this generator." No damage. Player must upgrade the cable to T2.

2. **Solar panel covered by player-built roof mid-game.** `sky_visibility` raycast detects the obstruction within ≤1s. Env factor drops, virtual item emission slows. Recipe pauses naturally when stock depletes.

3. **Battery placed on network with no generators.** Battery sits at 0 joules indefinitely. No error — it's a valid (if useless) configuration. Once a generator is added, charge begins on the next tick where the network has surplus.

4. **Battery full + generator buffer full + downstream demand zero.** Generator with `OnBufferFull` halts at end of current recipe. Battery stops accepting (at max). System reaches steady state with zero waste. Generator with `NeverThrottle` continues completing recipes; `GeneratorOverflow` events fire and joules are discarded.

5. **Lightning rod hit while buffer full.** Pulse dropped (§11 step 4). Player must size batteries for storm capture. Design-intent: this is a logistics puzzle, not a bug.

6. **Two lightning rods in range of one strike.** Only the nearest gets the pulse. Stacking rods does not multiply yield.

7. **Solar panel placed underground.** `sky_visibility = 0`. Env factor = 0. No emission. Stock stays at 0. UI shows "No sky exposure." Player must relocate.

8. **`LocalVirtualStock` cap reached during a buffer-full period.** Emission halts at the stock cap. The cap protects against runaway accumulation; it does not punish well-built setups (in normal play, recipes consume the stock faster than the cap matters).

9. **Generator recipe with `energy_cost > 0`.** Allowed but unusual — a generator that draws network power to run (e.g. a nuclear pre-heater stage). The recipe goes through normal voltage/amp/energy checks per `crafting.md §5`. If the network is too drained for the pre-heater, the generator can't start its production recipe. Power-positive-net rule isn't enforced by the system — it's a recipe-design concern. Recipe authors must ensure `output_energy > input_energy` over the recipe's duration, or the gen is a net consumer (which is allowed but pointless).

10. **Mid-recipe generator removal.** `machines.md §5 step 5` already emits `MachineRemoved`; the generator observer in `power.md` releases buffered joules to nothing (joules are lost — buffer doesn't transfer to the network on removal, and partial recipes refund input items per `machines.md §5 step 1`). UI message: "Generator dismantled; X kJ buffered energy lost."

11. **Battery removed while charged.** Buffered joules are lost. No item dropped. Player should drain the battery (by running consumers) before removal if they care. Telemetry event for joules lost is post-VS.

12. **Multiple solar panels placed adjacent; shadow check overlap.** Each panel runs its own `sky_visibility` raycast independently. If panel A shadows panel B, B sees lower visibility. This is correct behavior — players designing dense arrays must account for self-shadowing. (Visual: panels are flat and thin; in practice self-shadowing is minor unless they're stacked vertically.)

13. **Generator placed but no cable connected.** `GeneratorUnit` exists; `PowerNetworkMember` not inserted (no network). `env_port_tick_system` still produces virtual items into `LocalVirtualStock`. Recipe runs and writes joules into `GeneratorUnit.buffer_joules`. Buffer fills. Throttle kicks in. **Result: an isolated generator silently accumulates buffer up to cap and then idles.** This is correct — player connects cable, network re-evaluates, downstream consumers draw stored energy.

14. **Catalyst-gated generator recipe.** Per `crafting.md §6`, catalyst reservation applies. A combustion generator requiring a rare "ignition coil" item runs only while the coil is in the network. The coil is reserved (not consumed) during the recipe; multiple coil-gated machines share the pool via the reservation book. This works without generator-specific code — it's just a recipe with a catalyst input.

15. **Save/load with mid-recipe generator.** Per `machines.md §6 — Save tagging`, mid-recipe state is persisted. `RecipeProcessor.progress`, `LocalVirtualStock`, `GeneratorUnit.buffer_joules`, `Throttle`, env port child entities are all saved (`#[require(Save)]` cascades from `Machine`). On load, the recipe resumes from saved progress; env port emission resumes on the next tick.

16. **Battery rate cap during burst.** Lightning rod produces a large joule completion in one tick. Generator buffer fills. Network surplus is huge. Battery's `max_charge_watts * dt` is small compared to the burst. Result: most of the strike's energy stays in the rod's `GeneratorUnit.buffer_joules` until consumed; battery sips at its rate cap. This is correct — batteries are not infinite sinks. Burst storage is the rod's own buffer, not the battery's.

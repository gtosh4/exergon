# Mining & Deposit System

Ore extraction from surface deposits. Two extraction modes: manual (drone-piloted, early-game) and automatic (miner machine, primary method). Both write to the same depletion counter on the deposit entity.

---

## Table of Contents

1. [ECS Components](#1-ecs-components)
2. [Deposit Placement and Lifecycle](#2-deposit-placement-and-lifecycle)
3. [Miner Placement and Linking](#3-miner-placement-and-linking)
4. [Automatic Mining — `miner_tick_system`](#4-automatic-mining--miner_tick_system)
5. [Depletion Curve](#5-depletion-curve)
6. [Weighted Ore Sampling](#6-weighted-ore-sampling)
7. [Manual Mining — `drone_mine_system`](#7-manual-mining--drone_mine_system)
8. [Deposit Discovery](#8-deposit-discovery)
9. [System Execution Order](#9-system-execution-order)
10. [Content Data Format](#10-content-data-format)
11. [Post-MVP](#11-post-mvp)

---

## 1. ECS Components

### Deposit entity

| Component | Fields | Notes |
|---|---|---|
| `OreDeposit` | `chunk_pos: IVec2` | Chunk this deposit belongs to; used by despawn system |
| | `ores: Vec<(String, f32)>` | Weighted ore blend, normalised to sum 1.0 at spawn |
| | `total_extracted: f32` | Cumulative units extracted; drives depletion curve |
| | `depletion_seed: u64` | Per-deposit seed; determines floor and decay rate |
| | `miner: Option<Entity>` | Miner currently occupying this deposit. `None` = open |
| `Transform` | | World position (surface height + 0.75 m) |
| `Discovered` | (marker) | Inserted by proximity; gates ore-blend visibility in UI |

### Miner machine entity

`MinerMachine` is inserted in addition to the standard `Machine` bundle when `machine_type == "miner"`.

| Component | Fields | Notes |
|---|---|---|
| `MinerMachine` | `deposit: Entity` | The `OreDeposit` entity this miner targets |
| | `accumulator: f32` | Fractional progress toward next ore output |
| `Machine` | `machine_type`, `tier`, ... | Standard machine components |
| `LogisticsNetworkMember` | | Set when player cables the miner's output port |

### `MachineTierDef` additions

Two new fields on `MachineTierDef` (content data, not ECS components):

| Field | Type | Default | Notes |
|---|---|---|---|
| `cycle_time` | `f32` | `1.0` | Seconds between ore outputs at 100% yield |
| `output_count` | `u32` | `1` | Ore samples emitted per completed cycle |

Higher-tier miners increase `output_count`. `cycle_time` may be tuned per tier through content files; exact values are balance parameters requiring playtesting.

---

## 2. Deposit Placement and Lifecycle

### Spawn

`spawn_deposit_markers` runs when a `TerrainChunk` is first added (via `Added<TerrainChunk>`). For each new chunk:

1. Look up ore distribution for the chunk's world position via `DepositRegistry::ore_at`
2. If no deposit rolls for this cell (33% spawn probability): skip
3. **Check for existing deposit** — if an `OreDeposit` already exists with `chunk_pos == chunk.chunk_pos`, skip (prevents double-spawn when player re-enters a chunk whose deposit persisted due to an active miner)
4. Derive `depletion_seed` from world seed + chunk position via `xxh64`
5. Spawn deposit entity at terrain surface height + 0.75 m with `OreDeposit { miner: None, total_extracted: 0.0, ... }`

### Despawn

`despawn_deposit_markers` runs each frame. For each `OreDeposit` whose `chunk_pos` is not in `SpawnedChunks`:

- If `deposit.miner.is_none()` → despawn the entity
- If `deposit.miner.is_some()` → **keep alive**; the deposit persists as long as a miner occupies it, regardless of whether the terrain chunk is loaded

This ensures the miner continues extracting when the player moves away from the area. When the player returns, `spawn_deposit_markers` skips the chunk because the deposit entity still exists.

---

## 3. Miner Placement and Linking

### `MINER_LINK_RADIUS`

```
const MINER_LINK_RADIUS: f32 = 16.0;
```

Deposit cells are 64×64 m (one per chunk); a miner placed anywhere within the chunk will fall within this radius of the deposit marker at chunk center.

### Placement sequence

In `place_machine_system`, when `def.id == "miner"`:

1. Scan all `OreDeposit` entities; find nearest to placement position within `MINER_LINK_RADIUS`
2. **No deposit found** → log warning; skip spawn (`continue`). Machine is not placed.
3. **Deposit found but `deposit.miner.is_some()`** → log warning; skip spawn. Machine is not placed. (One miner per deposit enforced at placement.)
4. **Valid unoccupied deposit found** → spawn `MachineBundle` + insert `MinerMachine { deposit, accumulator: 0.0 }` → set `deposit.miner = Some(machine_entity)`

Placement still fires `MachineNetworkChanged` only on successful spawn.

### Removal sequence

In `remove_placed_objects_system`, when a removed machine entity has `MinerMachine`:

1. `deposit_q.get_mut(miner.deposit)` → set `deposit.miner = None`
2. Proceed with standard machine despawn (port markers, network changed)

If `deposit_q.get_mut` returns `Err` (deposit entity already gone), skip the clear — no panic.

---

## 4. Automatic Mining — `miner_tick_system`

Runs every frame in `LogisticsSimSystems` (after `NetworkSystems::of::<Logistics>()`).

For each `(MinerMachine, LogisticsNetworkMember)` pair:

```
1. Get deposit entity from miner.deposit
   → if deposit missing: continue (miner idles silently)

2. yf = yield_factor(deposit.total_extracted, deposit.depletion_seed)

3. miner.accumulator += yf * dt / cycle_time

4. if miner.accumulator < 1.0: continue

5. miner.accumulator -= 1.0

6. for _ in 0..output_count:
     rng_seed = deposit.depletion_seed ^ deposit.total_extracted.to_bits() as u64
     ore_id = sample_ore(&deposit.ores, rng)
     if Some(ore_id):
         deposit.total_extracted += 1.0
         give_items(network_members, ore_id, 1)
         fire NetworkStorageChanged { network }
```

`cycle_time` and `output_count` come from the `MachineTierDef` for the miner's current tier, resolved at tick time from `MachineRegistry`.

The miner does not require a running recipe or `MachineActivity`. It runs continuously as long as it has a valid deposit and a connected logistics network.

### Network output

`give_items` is called on the `LogisticsNetworkMembers` of the miner's output-eligible port. Port eligibility follows `PortPolicy` (default: `Both`). If the miner has no connected network (`LogisticsNetworkMember` absent), ore is silently discarded — no buffer, no backpressure.

---

## 5. Depletion Curve

*Implementation: `drone::yield_factor`*

```rust
pub fn yield_factor(total_extracted: f32, depletion_seed: u64) -> f32 {
    let floor = 0.1 + (depletion_seed % 100) as f32 * 0.001;  // [0.100, 0.199]
    let k     = 0.02 + (depletion_seed % 50)  as f32 * 0.001; // [0.020, 0.069]
    floor + (1.0 - floor) * (-k * total_extracted).exp()
}
```

- **Floor** in [0.1, 0.199]: deposit never reaches zero yield. Seeded per deposit.
- **Decay rate k** in [0.02, 0.069]: steeper k = faster depletion. Seeded per deposit.
- At `total_extracted = 0`: yield = 1.0 exactly.
- Yield is asymptotic toward `floor` — no hard cutoff.

Depletion is shared between manual and automatic extraction. Both increment `total_extracted` on the same component.

---

## 6. Weighted Ore Sampling

*Implementation: `drone::sample_ore`*

```rust
pub fn sample_ore<R: Rng>(ores: &[(String, f32)], rng: &mut R) -> Option<String>
```

One probabilistic draw per call. Weights are normalised at `DepositDef` load time (sum to 1.0). A copper-dominant deposit with `[("copper_ore", 0.7), ("tin_ore", 0.2), ("zinc_ore", 0.1)]` produces a copper-ore result ~70% of the time per draw.

Each automatic cycle at `output_count > 1` draws independently — no attempt to balance the distribution within a cycle. Over many cycles the blend converges to the weights.

The RNG seed for each draw is derived from `deposit.depletion_seed ^ deposit.total_extracted.to_bits()`. This is not reproducible across runs with different extraction sequences, but is deterministic within a single tick's multiple draws (all happen before `total_extracted` increments again — see tick loop above).

---

## 7. Manual Mining — `drone_mine_system`

Trigger: right mouse button press while in `PlayMode::DronePilot`.

```
1. Raycast from camera forward, max MINE_REACH = 4.0 m
2. If hit entity has OreDeposit:
     rng_seed = deposit.depletion_seed ^ deposit.total_extracted.to_bits()
     ore_id = sample_ore(&deposit.ores, rng)
     if Some(ore_id):
         drone_storage.items[ore_id] += 1
         deposit.total_extracted += 1.0
```

Output target: the `StorageUnit` component on the `Drone` entity currently being piloted. The `Drone` entity carries a `StorageUnit` added at spawn alongside the other drone components.

Manual mining does not check:
- `Discovered` status (player can mine before formally discovering)
- `deposit.miner` occupancy (manual + automatic coexist; both extract from same counter)

---

## 8. Deposit Discovery

`deposit_discovery_system` runs each frame in `PlayMode::DronePilot`.

```
DISCOVERY_RADIUS = 8.0 m

For each OreDeposit without Discovered, within DISCOVERY_RADIUS of drone:
    commands.entity(deposit).insert(Discovered)
    events.write(DiscoveryEvent("ore_deposit".to_string()))
```

Once `Discovered` is on the entity, the UI can read `OreDeposit.ores` to display the full ore blend. Before discovery, the UI shows only that a deposit exists (position known from visual marker in world).

`DiscoveryEvent` fires once per deposit (the `Without<Discovered>` filter prevents re-processing). The generic string payload is used by the tech tree system for unlock triggers; ore-specific data lives on the entity itself.

The xalite hard-code (`if !deposit.ores.iter().any(|(id, _)| id == "xalite")`) is removed. All deposits trigger discovery on proximity.

---

## 9. System Execution Order

```
NetworkSystems::of::<Power>()         // cable_placed, cable_removed
  → PowerSimSystems                   // generator_tick
    → NetworkSystems::of::<Logistics>()
      → LogisticsSimSystems
          → storage_unit_system
          → miner_tick_system         // ← runs here
          → recipe_start_system
          → recipe_progress_system

Update (any order, DronePilot only):
  → drone_mine_system
  → deposit_discovery_system
```

`miner_tick_system` fires `NetworkStorageChanged` which triggers `recipe_start_system` for the affected network in the same frame (messages are drained within `LogisticsSimSystems`).

---

## 10. Content Data Format

### `DepositDef` (assets/deposits/*.ron)

```
DepositDef(
    id: "copper_iron_mix",
    ores: [
        ("copper_ore", 0.7),
        ("iron_ore",   0.2),
        ("zinc_ore",   0.1),
    ],
)
```

Weights need not sum to 1.0; `DepositRegistry::new` normalises them.

### `MachineTierDef` additions (assets/machines/*.ron)

```
MachineTierDef(
    tier: 1,
    cycle_time: 1.0,      // seconds per ore output at 100% yield
    output_count: 1,      // ore samples per cycle
    energy_io_offsets: [...],
    logistics_io_offsets: [...],
)
```

---

## 11. Post-MVP

- **Void miners** — bypass `yield_factor`; output at constant 1.0 regardless of `total_extracted`. Requires a new miner tier flag in `MachineTierDef`.
- **Augments** — modules that increase base extraction rate (multiplier on effective cycle speed).
- **Reactivity contribution** — active miners contribute continuous reactivity to their region (tracked per-source; see `World Reactivity` spec).
- **Per-source depletion breakdown** — UI showing manual vs. automatic extraction contribution.

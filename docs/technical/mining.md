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
| | `progress: f32` | Fractional progress toward next ore output |
| `Machine` | `machine_type`, `tier`, ... | Standard machine components |
| `LogisticsNetworkMember` | | Set when player cables the miner's output port |

### `MachineTierDef` as trait

`MachineTierDef` is a trait rather than a struct, so each machine type defines only the fields it needs. The miner implements it via `MinerTierDef`:

```rust
pub trait MachineTierDef {
    fn tier(&self) -> u32;
    fn energy_io_offsets(&self) -> &[IoOffset];
    fn logistics_io_offsets(&self) -> &[IoOffset];
}

pub struct MinerTierDef {
    pub tier: u32,
    pub cycle_time: f32,   // seconds per ore output at 100% yield
    pub output_count: u32, // ore samples emitted per completed cycle
    pub energy_io_offsets: Vec<IoOffset>,
    pub logistics_io_offsets: Vec<IoOffset>,
}

impl MachineTierDef for MinerTierDef { ... }
```

Higher-tier miners increase `output_count`. `cycle_time` may be tuned per tier through content files; exact values are balance parameters requiring playtesting.

---

## 2. Deposit Placement and Lifecycle

### Spawn

`spawn_deposit_markers` runs when a `TerrainChunk` is first added (via `Added<TerrainChunk>`). For each new chunk:

1. Look up ore distribution for the chunk's world position via `DepositRegistry::ore_at`
2. If no deposit rolls for this cell (33% spawn probability): skip
3. **Check for existing deposit** — if an `OreDeposit` already exists with `chunk_pos == chunk.chunk_pos`, skip (prevents double-spawn when player re-enters a chunk whose deposit persisted)
4. Derive `depletion_seed` from world seed + chunk position via `xxh64`
5. Spawn deposit entity at terrain surface height + 0.75 m with `OreDeposit { miner: None, total_extracted: 0.0, ... }`

### Despawn

`despawn_deposit_system` reacts to `ChunkUnloaded { chunk_pos: IVec2 }` events emitted by the terrain system when a chunk is unloaded. For each matching `OreDeposit` with `chunk_pos == event.chunk_pos`:

- If `deposit.total_extracted == 0.0 && deposit.miner.is_none()` → despawn the entity (pristine, unoccupied)
- Otherwise → **keep alive**

A deposit that has been mined — even partially — persists indefinitely. This prevents players from resetting diminishing returns by unloading and reloading a chunk. Miners also keep their target alive even without prior extraction.

When the player returns, `spawn_deposit_markers` skips the chunk because the deposit entity still exists.

---

## 3. Miner Placement and Linking

### `MINER_LINK_RADIUS`

```
const MINER_LINK_RADIUS: f32 = 16.0;
```

Deposit cells are 64×64 m (one per chunk); a miner placed anywhere within the chunk will fall within this radius of the deposit marker at chunk center.

### Collider interaction

The deposit's collider is **non-blocking** for machine placement. Players must position the miner such that it physically intersects the deposit marker. The collider exists only for visual selection and discovery raycasting, not placement obstruction.

### Placement sequence

In `place_machine_system`, when `def.id == "miner"`:

1. Scan all `OreDeposit` entities; find nearest to placement position within `MINER_LINK_RADIUS`
2. **No deposit found** → log warning; skip spawn (`continue`). Machine is not placed.
3. **Deposit found but `deposit.miner.is_some()`** → log warning; skip spawn. Machine is not placed. (One miner per deposit enforced at placement.)
4. **Valid unoccupied deposit found** → spawn `MachineBundle` + insert `MinerMachine { deposit, progress: 0.0 }` → set `deposit.miner = Some(machine_entity)`

Placement still fires `MachineNetworkChanged` only on successful spawn.

### Removal sequence

`remove_placed_objects_system` emits a `MachineRemoved { entity: Entity, machine_type: String }` event for every removed machine, then proceeds with standard despawn (port markers, network changed).

The miner plugin registers `on_machine_removed_system` which reads `MachineRemoved` events, filters for miners, and clears `deposit.miner = None`. If `deposit_q.get_mut` returns `Err` (deposit entity already gone), skip — no panic.

This pattern applies to any plugin that needs to react to machine removal; those plugins register their own reader system rather than embedding logic in the generic removal path.

---

## 4. Automatic Mining — `miner_tick_system`

The miner system lives in the **miner plugin**, not the logistics module. The miner reads deposit state and pushes ore directly into the logistics network; it does not modify logistics internals.

Runs every frame in `LogisticsSimSystems` (after `NetworkSystems::of::<Logistics>()`).

For each `(MinerMachine, LogisticsNetworkMember)` pair:

```
1. Get deposit entity from miner.deposit
   → if deposit missing: continue (miner idles silently)

2. yf = yield_factor(deposit.total_extracted, deposit.depletion_seed)

3. miner.progress += yf * dt / cycle_time

4. if miner.progress < 1.0: continue

5. miner.progress -= 1.0

6. for draw_index in 0..output_count:
     ore_id = deposit.sample_ore(draw_index)
     if Some(ore_id):
         give_items(network_members, ore_id, 1)
         fire NetworkStorageChanged { network }
```

`cycle_time` and `output_count` come from the `MinerTierDef` for the miner's current tier, resolved at tick time from `MachineRegistry`.

The miner does not require a running recipe or `MachineActivity`. It runs continuously as long as it has a valid deposit and a connected logistics network.

### Network output

`give_items` is called on the `LogisticsNetworkMembers` of the miner's output-eligible port. Port eligibility follows `PortPolicy` (default: `Both`). If the miner has no connected network (`LogisticsNetworkMember` absent), ore is silently discarded — no buffer, no backpressure.

---

## 5. Depletion Curve

*Implementation: `mining::yield_factor`*

```rust
pub fn yield_factor(total_extracted: f32, depletion_seed: u64) -> f32 {
    let floor    = 0.1 + (depletion_seed % 100) as f32 * 0.001; // [0.100, 0.199]
    let half_life = /* derived from DepositDef */ ...;
    let k        = std::f32::consts::LN_2 / half_life;
    floor + (1.0 - floor) * (-k * total_extracted).exp()
}
```

- **Floor** in [0.1, 0.199]: deposit never reaches zero yield. Seeded per deposit.
- **`half_life`**: units of ore that must be extracted before yield drops to 50% of its full range (from floor to 1.0). Defined in `DepositDef`; directly interpretable by content authors — "after extracting 300 units, this deposit is half-spent."
- At `total_extracted = 0`: yield = 1.0 exactly.
- Yield is asymptotic toward `floor` — no hard cutoff.

Depletion is shared between manual and automatic extraction. Both call `deposit.sample_ore(...)` which increments `total_extracted` on the same component.

---

## 6. Weighted Ore Sampling

*Implementation: `OreDeposit::sample_ore`*

```rust
impl OreDeposit {
    pub fn sample_ore(&mut self, draw_index: u32) -> Option<String> {
        let seed = self.depletion_seed
            ^ self.total_extracted.to_bits() as u64
            ^ draw_index as u64;
        let mut rng = SmallRng::seed_from_u64(seed);
        let result = weighted_sample(&self.ores, &mut rng);
        if result.is_some() {
            self.total_extracted += 1.0;
        }
        result
    }
}
```

`draw_index` varies per draw within a cycle, ensuring multiple draws in the same tick produce independent (potentially different) results even though `total_extracted` has not yet changed between them. Without `draw_index`, all draws in a cycle would use the same seed and return the same ore.

Weights are normalised at `DepositDef` load time (sum to 1.0). A copper-dominant deposit with `[("copper_ore", 0.7), ("tin_ore", 0.2), ("zinc_ore", 0.1)]` produces a copper-ore result ~70% of the time per draw. Over many cycles the blend converges to the weights.

---

## 7. Manual Mining — `drone_mine_system`

Trigger: right mouse button press while in `PlayMode::DronePilot`.

```
1. Raycast from camera forward, max MINE_REACH = 4.0 m
2. If hit entity has OreDeposit:
     ore_id = deposit.sample_ore(0)
     if Some(ore_id):
         drone_storage.items[ore_id] += 1
```

`sample_ore` increments `total_extracted` internally; the caller does not manage it.

Output target: the `DroneInventory` component on the `Drone` entity currently being piloted.

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

Event-driven (any frame a machine is removed):
  → on_machine_removed_system         // miner plugin; clears deposit.miner
```

`miner_tick_system` fires `NetworkStorageChanged` which triggers `recipe_start_system` for the affected network in the same frame (messages are drained within `LogisticsSimSystems`).

---

## 10. Content Data Format

### `DepositDef` (assets/deposits/*.ron)

```
DepositDef(
    id: "copper_iron_mix",
    half_life: 300.0,    // units extracted until yield drops to ~50%
    ores: [
        ("copper_ore", 0.7),
        ("iron_ore",   0.2),
        ("zinc_ore",   0.1),
    ],
)
```

Weights need not sum to 1.0; `DepositRegistry::new` normalises them. `half_life` is the extraction volume at which yield falls to 50% of its full range; the runtime derives `k = ln(2) / half_life`.

### `MinerTierDef` (assets/machines/miner*.ron)

```
MinerTierDef(
    tier: 1,
    cycle_time: 1.0,      // seconds per ore output at 100% yield
    output_count: 1,      // ore samples per cycle
    energy_io_offsets: [...],
    logistics_io_offsets: [...],
)
```

Each machine type provides its own `*TierDef` struct implementing `MachineTierDef`. Fields are specific to that machine type; no shared union struct.

---

## 11. Post-MVP

- **Void miners** — bypass `yield_factor`; output at constant 1.0 regardless of `total_extracted`. Requires a new miner tier flag in `MinerTierDef`.
- **Augments** — modules that increase base extraction rate (multiplier on effective cycle speed).
- **Reactivity contribution** — active miners contribute continuous reactivity to their region (tracked per-source; see `World Reactivity` spec).
- **Per-source depletion breakdown** — UI showing manual vs. automatic extraction contribution.

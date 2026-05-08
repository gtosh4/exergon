# Network System Design

Cable-based infrastructure for logistics and power. Two independent network kinds share one generic ECS implementation.

---

## Table of Contents

1. [Generic Network System](#1-generic-network-system)
2. [Logistics Network](#2-logistics-network)
3. [Power Network](#3-power-network)
4. [Interplay](#4-interplay)

---

## 1. Generic Network System

### Overview

The generic layer in `src/network/` parameterizes topology management over a `NetworkKind` type parameter. Both `Power` and `Logistics` implement `NetworkKind`; they get identical cable placement, removal, split/merge, and machine membership logic for free.

### ECS Structure

**Network entity** — one per connected component. Carries:
- `N::Members` — relationship-target component listing all member entities in this network
- A kind-specific marker (`PowerNetwork`, `LogisticsNetwork`)

**Cable segment entity** — one per placed cable connection. Carries:
- `N::CableSegment` — stores `from: Vec3`, `to: Vec3`, `path: Vec<IVec3>` (routed voxel path)
- `N::Member` — relationship to the network entity this segment belongs to

**Port entity** — one per machine IO port. Separate from the machine entity. Carries:
- `N::PortOf` — relationship back to the owning machine entity (e.g. `EnergyPortOf(machine)`, `LogisticsPortOf(machine)`)
- `Transform` — world position of the port
- `Collider` — physics collider used as raycast target for cable placement
- `N::Member` (optional) — present when this port is an endpoint of a placed cable

**Machine entity** carries:
- `Machine` — lists `energy_ports: Vec<Vec3>` and `logistics_ports: Vec<Vec3>` as world positions
- `MachineEnergyPorts` / `MachineLogisticsPorts` — relationship-target components giving the machine a handle to its port entities

### NetworkKind Trait

```rust
pub trait NetworkKind: Send + Sync + 'static {
    const CABLE_ITEM_ID: &'static str;

    type CableSegment: Component + HasEndpoints;
    type Member: NetworkMemberComponent;
    type Members: NetworkMembersComponent;
    type PortOf: PortOfMachine + Component;

    fn io_ports(machine: &Machine) -> &[Vec3];
    fn new_cable_segment(from, to, is_blocked) -> Self::CableSegment;
    fn spawn_network(commands) -> Entity;
}
```

Implementing this trait and calling `app.add_plugins(NetworkPlugin::<N>::default())` gives a network kind the full topology pipeline.

### Topology Systems

`NetworkPlugin<N>` registers two systems in `NetworkSystems::of::<N>()`, chained:

#### `cable_placed_system`

Triggered by `CableConnectionEvent` matching `N::CABLE_ITEM_ID`.

1. Builds `endpoint → network` map from existing cable segments
2. Finds all networks adjacent to the new cable's endpoints (via existing cables **and** the port entities explicitly targeted in `CableConnectionEvent.from_port` / `to_port`)
3. **No adjacent networks** → spawns a new network entity
4. **One or more adjacent networks** → picks the one with the most members as survivor; merges others into it by reinserting `N::Member` on all their member entities and despawning the absorbed network entities
5. Spawns the cable segment entity with membership in the target network
6. Inserts `N::Member` on the port entities explicitly specified in `from_port`/`to_port` (resolved at placement time by raycasting the player's view to each port's `Collider`); ignores ports not named in the event
7. Fires `NetworkChanged<N>` for the affected network

#### `cable_removed_system`

Triggered by `WorldObjectEvent` with `kind == Removed`. Two removal modes:
- **Typed** (`item_id == N::CABLE_ITEM_ID`) — removes all cable segments touching the specified grid position
- **Generic** (`item_id.is_empty()`) — finds the nearest cable segment to the click position by point-to-segment distance; removes it if within 2 units

After removal:
1. Despawns removed cable entities
2. Runs BFS (`find_segment_components`) on remaining segments to find connected components
3. **Zero remaining** → removes membership from all affected ports; despawns network entity
4. **One component** → network survives; removes membership from ports no longer near any cable endpoint
5. **Multiple components** → keeps largest as the original network; spawns new network entities for smaller fragments; reassigns cable and port memberships; fires `NetworkChanged<N>` for all resulting networks

#### Machine Removal

When a machine is removed, all cables whose endpoints target any of that machine's ports are removed first. Each removal goes through `cable_removed_system`, which handles network splitting, port membership cleanup, and `NetworkChanged<N>` events. Port entities are then despawned with the machine.

### Cable Routing

Two routing strategies, both producing `Vec<IVec3>` paths:

**`auto_route`** — Manhattan path stepping X → Y → Z. Fallback when A* fails.

**`route_avoiding`** — A* with a turn penalty (`TURN_PENALTY = 3`) to prefer straight runs. Avoids machine positions and terrain (positions below terrain height). Falls back to `auto_route` when no path is found within budget.

### Messages

**`NetworkChanged<N>`** — fired whenever a network's membership changes. Consumed by kind-specific simulation systems to recompute state (capacity, brownout, recipes).

**`CableConnectionEvent`** — carries `from`, `to`, `item_id`, `kind`, `from_port: Option<Entity>`, `to_port: Option<Entity>`. Ports are resolved by raycasting the player's view to port colliders at placement time. Input to `cable_placed_system`.

**`WorldObjectEvent`** — carries `pos`, `item_id`, `kind`. Input to `cable_removed_system` and kind-specific systems (generator placement, etc.).

---

## 2. Logistics Network

*Implementation: `src/logistics/`*

### Components

| Component | Entity | Purpose |
|---|---|---|
| `LogisticsNetwork` | Network entity | Marker; no data |
| `LogisticsNetworkMember(Entity)` | Cable segment or port entity | Points to owning network |
| `LogisticsNetworkMembers(Vec<Entity>)` | Network entity | Lists all member entities; exposes `has_items`, `take_items`, `give_items` with priority-ordered iteration across member `StorageUnit`s |
| `LogisticsCableSegment` | Cable segment entity | `from`, `to`, `path` |
| `LogisticsPortOf(Entity)` | Port entity | Points to owning machine |
| `PortPolicy { default_mode: PortMode, item_overrides: HashMap<String, PortMode> }` | Port entity | Controls which items flow in which direction through this port |
| `StorageUnit { items: HashMap<String, u32> }` | Machine entity | Item inventory for storage crates |

`PortMode` — `None`, `Input`, `Output`, or `Both`. `item_overrides` takes precedence over `default_mode` for a specific item id. Ports default to `Both` if no `PortPolicy` is present.

### Plugin Split

**`LogisticsSimPlugin`** — simulation only, usable in integration tests with `MinimalPlugins`. Registers `NetworkPlugin::<Logistics>` plus the simulation systems.

**`LogisticsPlugin`** — adds visual systems and state gating on top of `LogisticsSimPlugin`.

### Simulation Systems

Run after `NetworkSystems::of::<Logistics>()` and after `PowerSimSystems`:

**`storage_unit_system`** — watches for newly spawned machines with `machine_type == "storage_crate"` and inserts `StorageUnit` on them.

**`miner_tick_system`** — advances miner timers each tick; when a cycle completes, picks ore from the deposit distribution and calls `give_items` on the networks of the miner's output-eligible ports.

**`recipe_start_system`** — triggered by `NetworkStorageChanged` or `NetworkChanged<Power>`. For each affected logistics network, scans idle machines:
1. Finds matching recipes by `machine_type` and `machine_tier`
2. Checks tech tree lock (`TechTreeProgress.unlocked_recipes`) if progress resource exists
3. **Power check** — if `recipe.energy_cost > 0`, calls `PowerNetworkMembers.has_energy(energy_per_tick)` where `energy_per_tick = energy_cost / processing_time * dt`; no withdrawal at start
4. **Input check** — for each recipe input item, resolves the machine's logistics ports that allow input for that item (via `PortPolicy`), then calls `has_items` across the `LogisticsNetworkMembers` of those ports' networks
5. **Output check** — for each recipe output/byproduct item, verifies at least one of the machine's logistics ports allows output for that item and has a connected network; if any output has no valid destination, recipe does not start
6. If all checks pass: calls `take_items` on each input-port network accordingly, inserts `MachineActivity` and `MachineState::Running` on the machine

**`recipe_progress_system`** — each tick:
1. If `recipe.energy_cost > 0`: calls `PowerNetworkMembers.take_energy(energy_cost / processing_time * dt)`; if insufficient energy, skips progress advance for this tick (recipe pauses until buffer refills)
2. Advances `MachineActivity.progress` by `dt`

On completion:
- For each output/byproduct item, resolves the machine's logistics ports that allow output for that item (via `PortPolicy`), calls `give_items` on those ports' networks
- Special-cases `RESEARCH_POINTS_ID` outputs → adds to `ResearchPool` resource instead of storage
- Removes `MachineActivity`, sets `MachineState::Idle`
- Fires `NetworkStorageChanged` for the network (triggers next recipe evaluation)

### Unified Storage

`LogisticsNetworkMembers` exposes `has_items`, `take_items`, and `give_items` as methods. It owns the iteration order across member `StorageUnit` entities, allowing priority ordering between storage units to be encapsulated here. Items are not centralized — each `StorageUnit` holds its own `HashMap<String, u32>`; `LogisticsNetworkMembers` is the index and the access point.

### Messages

**`NetworkStorageChanged { network: Entity }`** — fired when storage contents change (recipe output deposited, miner deposit). Triggers `recipe_start_system` for that network.

---

## 3. Power Network

*Implementation: `src/power/mod.rs`*

### Components

| Component | Entity | Purpose |
|---|---|---|
| `PowerNetwork` | Network entity | Marker; no data |
| `PowerNetworkMember(Entity)` | Cable segment, port entity, or generator entity | Points to owning network |
| `PowerNetworkMembers(Vec<Entity>)` | Network entity | Lists all member entities; exposes `has_energy`, `take_energy`, `give_energy` with priority-ordered iteration across member `GeneratorUnit` buffers |
| `PowerCableSegment` | Cable segment entity | `from`, `to`, `path` |
| `EnergyPortOf(Entity)` | Port entity | Points to owning machine |
| `GeneratorUnit { pos: Vec3, watts: f32, buffer_joules: f32, max_buffer_joules: f32 }` | Standalone entity | Represents a placed generator; fills its buffer at `watts` joules/sec up to `max_buffer_joules` |

### Simulation Systems

Run immediately after `NetworkSystems::of::<Power>()`:

**`generator_system`** — reacts to `WorldObjectEvent` with `item_id == "generator"`:
- **Placed** → spawns `GeneratorUnit` entity with an empty buffer; inserts `PowerNetworkMember` when the player cables to its port; fires `NetworkChanged<Power>`
- **Removed** → despawns `GeneratorUnit`, fires `NetworkChanged<Power>` for the previously connected network

**`generator_tick_system`** — runs every tick. For each `GeneratorUnit` in a power network:
1. Calls `PowerNetworkMembers.give_energy(watts * dt)` to fill generator buffers (clamped to `max_buffer_joules` per generator)
2. If `give_energy` reports a 0→positive transition, fires `NetworkChanged<Power>` to unblock waiting recipes

### Generator Placement Timing

Generators connect to power networks the same way as any other machine: the player explicitly targets the generator's energy port collider when placing a power cable. There is no auto-connect on proximity. Both cable-first and generator-first placement require the player to draw a cable to the port.

---

## 4. Interplay

### Separate Physical Infrastructures

Power and logistics are **completely independent cable graphs**. A machine participates in both via separate port entities:
- Energy ports (`EnergyPortOf`) snap to power cables → join a `PowerNetwork`
- Logistics ports (`LogisticsPortOf`) snap to logistics cables → join a `LogisticsNetwork`

A machine with no power cable connection has no `PowerNetworkMember` on its energy ports. A machine with no logistics cable has no `LogisticsNetworkMember` on its logistics ports. The two graphs never share entities.

### Power as a Consumable Resource

Power mirrors the item storage model. Generators produce energy into a buffer; recipes withdraw a lump of joules on start — exactly like miners produce items into storage and recipes withdraw inputs.

```
NetworkStorageChanged  ─┐
                         ├─► recipe_start_system ─► starts recipe (or doesn't)
NetworkChanged<Power>  ─┘
```

When a power network's buffer becomes non-zero (`generator_tick_system` fires `NetworkChanged<Power>`), the system traverses:
```
power network → PowerNetworkMembers → EnergyPortOf → machine → MachineLogisticsPorts → logistics network
```
…and re-evaluates logistics networks whose machines have connected energy ports. This allows a newly charged generator to unblock a recipe waiting on energy.

Power is withdrawn per-tick during recipe execution, unlike logistics inputs which are consumed at recipe start. Each tick `recipe_progress_system` calls `take_energy(energy_cost / processing_time * dt)`; if the buffer is insufficient the recipe pauses until generators refill it. The start check (`has_energy`) only gates recipe initiation — no upfront withdrawal.

### Execution Order

```
NetworkSystems::of::<Power>()        // cable_placed, cable_removed
  → PowerSimSystems                  // generator, generator_tick
    → NetworkSystems::of::<Logistics>()
      → LogisticsSimSystems          // storage_unit, miner_tick, recipe_start, recipe_progress
```

Power systems complete before logistics recipe evaluation begins each frame. This ensures generator buffers are filled before `recipe_start_system` checks and withdraws from them.

### Machine Component Summary

```
Machine entity
├── Machine { energy_ports: Vec<Vec3>, logistics_ports: Vec<Vec3> }
├── MachineState
├── MachineActivity (optional, when Running)
├── MachineEnergyPorts    ← relationship target: lists EnergyPortOf entities
├── MachineLogisticsPorts ← relationship target: lists LogisticsPortOf entities
└── GeneratorUnit (optional, if this machine is a generator)

Energy port entity
├── EnergyPortOf(machine_entity)
├── Transform (world position)
└── PowerNetworkMember(network_entity)  ← optional, when connected

Logistics port entity
├── LogisticsPortOf(machine_entity)
├── Transform (world position)
└── LogisticsNetworkMember(network_entity)  ← optional, when connected
```

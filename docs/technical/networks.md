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

For power networks, after topology is updated: if any resulting network's `amps_in_use()` exceeds its new `amp_capacity()`, the system pauses all running machines on that network (removes `MachineActivity`, releases their amp allocations, sets `MachineState::Idle`) and fires `NetworkChanged<Power>` so `recipe_start_system` can re-evaluate them in priority order once headroom exists. No cable or machine damage occurs.

#### Machine Removal

When a machine is removed, all cables whose endpoints target any of that machine's ports are removed first. Each removal goes through `cable_removed_system`, which handles network splitting, port membership cleanup, and `NetworkChanged<N>` events. Port entities are then despawned with the machine.

`remove_placed_objects_system` emits `MachineRemoved { entity: Entity, machine_type: String }` after the generic despawn. Machine-type plugins (e.g. the miner plugin) register their own reader systems for this event to handle type-specific teardown. Logic that depends on which kind of machine was removed belongs in those plugin systems, not in the generic removal path.

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
| `LogisticsNetworkMembers(Vec<Entity>)` | Network entity | Lists all member entities; exposes `has_items`, `take_items`, `give_items` with priority-ordered iteration across member `StorageUnit`s; exposes `channel_capacity()` and `channels_in_use()` for the throughput check (see Channel Capacity below) |
| `LogisticsCableSegment` | Cable segment entity | `from`, `to`, `path`, `channel_capacity: u8` |
| `LogisticsPortOf(Entity)` | Port entity | Points to owning machine |
| `PortPolicy { default_mode: PortMode, item_overrides: HashMap<String, PortMode> }` | Port entity | Controls which items flow in which direction through this port |
| `StorageUnit { items: HashMap<String, u32> }` | Machine entity | Item inventory for storage crates |
| `SubnetRouter` | Router machine entity | Marker; the machine is a non-conducting cable boundary. Owns one port per side (`LogisticsPortOf` → this router), each a member of a different `LogisticsNetwork` (see Sub-network Routers below) |
| `RouterFilter { crossings: Vec<Crossing> }` where `Crossing { item_id: String, from: u8, to: u8 }` | Router machine entity | The explicit item crossings this router relays; `from`/`to` index the router's per-side ports. Items absent from the list do not cross |

`PortMode` — `None`, `Input`, `Output`, or `Both`. `item_overrides` takes precedence over `default_mode` for a specific item id. Ports default to `Both` if no `PortPolicy` is present.

### Plugin Split

**`LogisticsSimPlugin`** — simulation only, usable in integration tests with `MinimalPlugins`. Registers `NetworkPlugin::<Logistics>` plus the simulation systems.

**`LogisticsPlugin`** — adds visual systems and state gating on top of `LogisticsSimPlugin`.

### Simulation Systems

Run after `NetworkSystems::of::<Logistics>()` and after `PowerSimSystems`:

**`storage_unit_system`** — watches for newly spawned machines with `machine_type == "storage_crate"` and inserts `StorageUnit` on them.

**`miner_tick_system`** — advances miner timers each tick; when a cycle completes, picks ore from the deposit distribution and calls `give_items` on the networks of the miner's output-eligible ports.

**`recipe_start_system`** — triggered by `NetworkStorageChanged` or `NetworkChanged<Power>` (manual-mode machines; see `crafting.md §4`) or by `JobDispatched` (auto-craft machines; see `crafting.md §8`). The full recipe execution specification — `RecipeProcessor` component, job dispatch path, catalyst reservation, module effects — is in `crafting.md §5`. The logistics-layer checks this system performs:
1. Finds matching recipes by `machine_type` and `machine_tier` (using `MachineCapability` from `crafting.md §3`)
2. Checks tech tree lock (`TechTreeProgress.unlocked_recipes`) if progress resource exists
3. **Power check** — if `recipe.energy_cost > 0`:
   - **Voltage** — `PowerNetworkMembers.voltage_tier()` ≥ `recipe.min_voltage_tier`; cannot start if not met
   - **Amps** — `(energy_cost / processing_time) / network_voltage` ≤ `PowerNetworkMembers.amp_capacity() - amps_in_use()`; cannot start if at amp capacity
   - **Buffer** — calls `PowerNetworkMembers.has_energy(energy_per_tick)` where `energy_per_tick = (energy_cost / processing_time) * dt`; no withdrawal at start
4. **Input check** — for each recipe input item, resolves the machine's logistics ports that allow input for that item (via `PortPolicy`), then calls `has_items` across the `LogisticsNetworkMembers` of those ports' networks
5. **Catalyst check** — if `recipe.catalyst_inputs` is non-empty: for each item, `available = has_items(item) - CatalystReservationBook.reserved[item]`; if `available < required`, recipe does not start (see `crafting.md §6`)
6. **Output check** — for each recipe output/byproduct item, verifies at least one of the machine's logistics ports allows output for that item and has a connected network; if any output has no valid destination, recipe does not start
7. If all checks pass: calls `take_items` on each input-port network accordingly; reserves catalyst inputs in `CatalystReservationBook`; sets `RecipeSlot.state = Running` and `progress = 0.0` (see `crafting.md §2` for `RecipeProcessor` definition)

**`recipe_progress_system`**, **`recipe_completion_system`** — see [`crafting.md §5`](crafting.md#5-recipe-execution). The only network-layer interactions are `PowerNetworkMembers.take_energy` per tick and `give_items` on output ports at completion; all progress tracking, catalyst release, and output production are crafting-layer concerns.

### Unified Storage

`LogisticsNetworkMembers` exposes `has_items`, `take_items`, and `give_items` as methods. It owns the iteration order across member `StorageUnit` entities, allowing priority ordering between storage units to be encapsulated here. Items are not centralized — each `StorageUnit` holds its own `HashMap<String, u32>`; `LogisticsNetworkMembers` is the index and the access point. When the `CatalystReservationBook` resource is present, `has_items` subtracts `CatalystReservationBook.reserved[item_id]` from the total available count before returning — items with active catalyst reservations are physically present in storage but treated as unavailable (see `crafting.md §6`).

### Channel Capacity

The item-network's throughput lever is **discrete channel capacity** (AE2-style, GDD §10) — the mirror of amperage on the power network (§3). It is **not** a per-tick flow rate.

- **Unit — per port.** Each connected machine `LogisticsPortOf` member consumes **1 channel**. `channels_in_use()` = the count of connected machine ports on the network. (Cable segments and storage-crate ports themselves are the transport; the count is of machine-serving ports, matching AE2's "one channel per device.")
- **Capacity — cable tier.** `channel_capacity()` = the **minimum** `channel_capacity` among the network's `LogisticsCableSegment`s (weakest-link, so raising a network's ceiling means upgrading its cable, not just one segment). Higher cable tiers carry more channels; tier ratings are authored content (cable-tier tech nodes in `tech-tree-design.md` Logistics category, authored in `assets/tech_nodes/`).
- **Over-budget behavior — non-destructive.** When `channels_in_use() > channel_capacity()`, the ports past the budget are **inactive**: excluded from `has_items` / `take_items` / `give_items` iteration, so their machines are neither fed nor drained (they block at `recipe_start_system`'s input/output checks like any unconnected machine). No cable or machine damage — the mirror of power's amp-overload pause (§3, and the topology-recompute rule at the amp check). The drop order is deterministic and priority-ordered (least-priority ports shed first), recomputed on `NetworkChanged<Logistics>` when membership changes.
- **Resolution — upgrade or segment.** A player over budget either lays higher-tier cable (raises the floor) or **segments** the network into sub-networks joined via router/interface boundaries — each sub-net carries its own channel budget. Segmentation is the discovered solution the GDD intends, not a forced constraint; the Sub-network Router I / II nodes (`tech-tree-design.md` Logistics category) unlock it.

> **Status:** design-locked (design-decisions.md 2026-07-12), not yet implemented. `src/logistics/` today is an uncapped shared pool — `channel_capacity()`/`channels_in_use()` and the over-budget shed are pending engine work, sequenced to Demo scope after the Vertical Slice playtest gate.

### Sub-network Routers

A **Sub-network Router** is the segmentation device that resolves a channel-over-budget network (§Channel Capacity). It is *not* a channel source (channels come from cable tier); it is a semi-permeable boundary that divides one `LogisticsNetwork` into independent sub-nets, each carrying its own channel budget. Unlocked by the **Sub-network Router I / II** tech nodes (`tech-tree-design.md` Logistics category).

**Topology — the router breaks cable connectivity.** A router (`machine_type == "subnet_router"`) *terminates* cable runs instead of conducting them. During the `NetworkPlugin::<Logistics>` build pass it is treated as a non-conducting node, so cables on one face form one connected graph and cables on another face form a separate graph — the single net becomes N distinct `LogisticsNetwork` entities. The router owns one port per side (`LogisticsPortOf` → the router), and each port is a member of that side's network. `NetworkChanged<Logistics>` fires for every affected sub-net when a router is placed or removed (membership and both budgets recompute).

**Channels — a boundary costs one channel per side.** Each router port consumes **1 channel** on its own sub-net (it appears in that sub-net's `channels_in_use()` like any machine port). A 2-sided Router I therefore costs **2 channels total** — one in each sub-net. This is the deliberate price of segmentation: cut along **low-traffic seams** (few item types cross smelting↔processing), not arbitrarily, or the boundary cost eats the budget you freed. Router ports are **priority-pinned** (highest priority) so the over-budget shed severs leaf machine ports before it ever severs a boundary; a sub-net sheds its own least-priority leaf ports first. **Isolation:** a machine port on sub-net B does *not* count against sub-net A's budget, and each sub-net's `channel_capacity()` = the min cable tier of *its own* segments — the budgets are fully independent.

**Transfer — filtered, not free.** The router carries a `RouterFilter`: an explicit list of `Crossing { item_id, from, to }` where `from`/`to` are side indices. `router_transfer_system` runs each tick — for each crossing, if the `from` sub-net `has_items(item)` and the `to` sub-net has a valid sink, it `take_items` from the source and `give_items` to the destination. There is **no per-tick rate cap** — channels are the only throughput lever (GDD §10, no flow-rate model); the router is a membrane, not a throttle. Because crossings are explicit, cross-net dependencies (smelting-net copper feeding processing-net circuits) are a **declared** planning decision — unlisted items stay isolated to their sub-net. This is the Pillar-2 friction: network *architecture*, not the runtime clock.

**Router I vs II (capability ladder — II is a strict superset of I).**
- **Sub-network Router I:** a 2-sided boundary (splits one net into two); `RouterFilter` capped at N crossings (N = authored content, TBD).
- **Sub-network Router II:** multi-sided routers (≥3 sub-nets meeting at one hub) and **nested** sub-nets (a sub-net may contain its own router → a tree/mesh of sub-nets), plus a higher crossing cap. Everything Router I does, plus depth. Tier naming is internal convention; display names may re-flavor later.

**Failure modes (non-destructive, mirrors power §3).**
- *Missing crossing:* an item a downstream machine needs but which no `Crossing` relays simply never arrives — that machine blocks at `recipe_start_system`'s input check, like any starved machine. No damage, no cable/router harm.
- *Sub-net over budget:* the affected sub-net sheds its own least-priority leaf ports (router ports pinned), so the boundary stays intact and the rest of the mesh keeps flowing.

> **Status:** design-locked (design-decisions.md 2026-07-14), not implemented. Depends on the channel-capacity engine work above; same Demo-scope sequencing after the Vertical Slice playtest gate.

### Messages

**`NetworkStorageChanged { network: Entity }`** — fired when storage contents change (recipe output deposited, miner deposit). Triggers `recipe_start_system` for that network.

---

## 3. Power Network

*Implementation: `src/power/mod.rs`*

> **Scope:** this section owns the **cable-graph mechanics** for power — `GeneratorUnit` buffer pooling, `PowerNetworkMembers` surface (`has_energy`/`take_energy`/`give_energy`, `voltage_tier`, `amp_capacity`, `amps_in_use`), voltage/amp gating at recipe start, and non-destructive failure modes. The **production model** — how generators actually fill those buffers (recipe-driven, env ports, virtual items, batteries, weather, throttle, burst generators) — is in [`power.md`](power.md). The `generator_tick_system` constant-watts fill described below is **superseded** by the recipe-completion fill in `power.md §7`; the description here is retained for context but implementations follow `power.md`.

### Components

| Component | Entity | Purpose |
|---|---|---|
| `PowerNetwork` | Network entity | Marker; no data |
| `PowerNetworkMember(Entity)` | Cable segment, port entity, or generator entity | Points to owning network |
| `PowerNetworkMembers(Vec<Entity>)` | Network entity | Lists all member entities; exposes `has_energy`, `take_energy`, `give_energy` across member `GeneratorUnit` buffers; exposes `voltage_tier()`, `amp_capacity()`, `amps_in_use()` for voltage and throughput checks |
| `PowerCableSegment` | Cable segment entity | `from`, `to`, `path`, `voltage_tier: u8`, `max_amps: f32` |
| `EnergyPortOf(Entity)` | Port entity | Points to owning machine |
| `GeneratorUnit { pos: Vec3, voltage_tier: u8, watts: f32, buffer_joules: f32, max_buffer_joules: f32 }` | Standalone entity | Outputs at `voltage_tier`; fills its own internal buffer at `watts` joules/sec up to `max_buffer_joules`; `PowerNetworkMembers.take_energy` draws across all member buffers |

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

Each generator fills its own internal buffer at its rated wattage. `PowerNetworkMembers.take_energy` draws across all member buffers in the network (generators and batteries). This mirrors the item storage model — multiple `StorageUnit`s, not a single pool.

```
NetworkStorageChanged  ─┐
                         ├─► recipe_start_system ─► starts recipe (or doesn't)
NetworkChanged<Power>  ─┘
```

When any generator buffer transitions from empty to non-empty (`generator_tick_system` fires `NetworkChanged<Power>`), the system traverses:
```
power network → PowerNetworkMembers → EnergyPortOf → machine → MachineLogisticsPorts → logistics network
```
…and re-evaluates logistics networks whose machines have connected energy ports. This allows a refilling generator to unblock a paused recipe.

Recipe start checks voltage tier compatibility and amp headroom before the energy check — a machine that requires Medium Voltage cannot start on a Low Voltage network regardless of available joules. Amps are allocated on start and held until the recipe finishes or is cancelled; a paused recipe (generator buffers empty) continues to hold its amp allocation.

All power failure modes are non-destructive. Voltage mismatch blocks the machine with a displayed reason. Amp capacity reached blocks new starts. Generator shortage pauses in-progress recipes. Cable removal that causes amp overload pauses all affected running machines, releases their amp allocations, and lets them resume once headroom restores — no cable damage, no machine loss.

Power is withdrawn per-tick during recipe execution, unlike logistics inputs which are consumed at recipe start. Each tick `recipe_progress_system` calls `take_energy((energy_cost / processing_time) * dt)`; if generator buffers are insufficient the recipe pauses until they refill. No upfront energy withdrawal; no proportional throttle.

### Execution Order

```
NetworkSystems::of::<Power>()        // cable_placed, cable_removed
  → PowerSimSystems                  // generator, generator_tick
    → NetworkSystems::of::<Logistics>()
      → LogisticsSimSystems          // storage_unit, miner_tick, manual_recipe,
                                     // recipe_progress, recipe_completion, job_prerequisite,
                                     // job_dispatcher, recipe_start
                                     // (see crafting.md §11 for sub-ordering)
```

Power systems complete before logistics recipe evaluation begins each frame. This ensures generator buffers are filled before `recipe_start_system` checks and withdraws from them.

### Machine Component Summary

```
Machine entity
├── Machine { energy_ports: Vec<Vec3>, logistics_ports: Vec<Vec3> }
├── RecipeProcessor    ← recipe execution state; replaces MachineState + MachineActivity (see crafting.md §2)
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

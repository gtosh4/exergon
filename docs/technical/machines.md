# Machine System Design

Core machine entity model and data format. Covers `MachineDef` asset schema, placement flow, tier upgrade (state transfer), IO port spawning, orientation, and persistence tagging. Read `gdd.md §10` for design intent.

Scope boundary:
- `machine-ui.md` — side-rail panel UI and `MachineJobPolicy` editing.
- `crafting.md` — `RecipeProcessor`, recipe execution, job dispatch, `MachineCapability`.
- `networks.md` — `LogisticsPortOf` / `EnergyPortOf` port entities, `PortPolicy`, cable membership.
- This doc — `MachineDef`, `Machine` component, port spawn on placement, tier upgrade, save tagging.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Machine Asset Data](#2-machine-asset-data)
3. [ECS Structure](#3-ecs-structure)
4. [Placement Flow](#4-placement-flow)
5. [Removal Flow](#5-removal-flow)
6. [Tier Upgrade](#6-tier-upgrade)
7. [IO Port Routing](#7-io-port-routing)
8. [Orientation](#8-orientation)
9. [Persistence](#9-persistence)
10. [Modules (Deferred)](#10-modules-deferred)
11. [Systems](#11-systems)
12. [Messages](#12-messages)
13. [Execution Order](#13-execution-order)
14. [Vertical Slice Scope](#14-vertical-slice-scope)
15. [Edge Cases](#15-edge-cases)

---

## 1. Overview

A machine is a placed entity that hosts recipe execution, IO ports, and (post-VS) module attachments. Each tier of a machine is a distinct item with its own prefab (scene, collider, port layout). Higher tiers run higher `min_voltage_tier` recipes and (post-VS) carry more module slots.

Machines, ports, modules, and the machine items themselves are content-driven via RON assets in `assets/machines/` and `assets/items/`. Engine knows only the schema; concrete machine types are data.

Placement and removal flow through `building.md` via `WorldObjectEvent { Placed | Removed, item_id, orientation }`. This doc covers machine-specific ECS bits: `MachineDef` asset schema, per-tier item↔def mapping, port entity spawn on `OnAdd<Machine>`, tier upgrade (kit + replace paths), persistence.

Tier upgrades use `MachineUpgradeRequest` triggered either by applying an upgrade-kit item to a placed machine or by deconstruct-and-replace.

**Port/module positions in assets are dev-time placeholders.** The schema below uses explicit `IVec3` offsets so VS can iterate without art. Real GLTF models will expose port/module anchors as named child entities (`Port_LogisticsIn_0`, `ModuleSlot_0`, …); a later pass swaps the registry loader to extract anchor transforms from the scene at load time. Schema field shape is preserved across the swap — only the data source changes.

---

## 2. Machine Asset Data

### MachineDef schema

```rust
#[derive(Deserialize, Clone, Debug)]
pub struct MachineDef {
    pub machine_type: MachineTypeId,    // e.g. "smelter" — NOT an item id
    pub display_name: String,
    pub tiers: Vec<MachineTierDef>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct MachineTierDef {
    pub tier: u8,                       // 1-based; matches ItemKind::Machine.tier
    pub item_id: ItemId,                // e.g. "smelter_mk1" — the item that places this tier
    pub scene: String,                  // e.g. "models/machines/smelter_mk2.glb#Scene0"
    pub energy_ports: Vec<PortOffset>,
    pub logistics_ports: Vec<PortOffset>,
    // Post-VS:
    pub module_slots: Vec<ModuleSlotDef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PortOffset {
    pub offset: IVec3,                  // canonical-space offset from the machine anchor (dev placeholder; real models source from GLTF child entities — see §1)
    pub label: String,                  // e.g. "Top", "Left input" — surfaced in machine-ui port binding
}
```

`MachineDef.machine_type` is the `MachineTypeId` used by recipes (`ConcreteRecipe.machine` resolves to a machine item whose `ItemKind::Machine.machine_type` matches one of these). Tier items are distinct (`smelter_mk1`, `smelter_mk2`, …); each `MachineTierDef.item_id` names the item that places it. See `recipe-graph.md §4` for the items-as-machines model.

No `footprint_radius` field. Placement validation derives the footprint collider from the loaded scene mesh (ghost-preview path in `building.md`).

`ModuleSlotDef` schema is in §10 — VS-era asset authors leave `module_slots: []`.

### Asset format

`assets/machines/<machine_type>.ron`:

```ron
(
    machine_type: "smelter",
    display_name: "Smelter",
    tiers: [
        (
            tier: 1,
            item_id: "smelter_mk1",
            scene: "models/machines/smelter_mk1.glb#Scene0",
            energy_ports: [
                (offset: (3, 0, 0), label: "Right"),
            ],
            logistics_ports: [
                (offset: (0, 0,  3), label: "Front"),
                (offset: (0, 0, -3), label: "Back"),
                (offset: (-3, 0, 0), label: "Left input"),
                (offset: (-3, 0, 2), label: "Left output"),
            ],
            module_slots: [],
        ),
    ],
)
```

### MachineRegistry resource

```rust
#[derive(Resource)]
pub struct MachineRegistry {
    by_type: HashMap<MachineTypeId, MachineDef>,
    by_item: HashMap<ItemId, (MachineTypeId, u8)>,  // item_id -> (machine_type, tier) index
}

impl MachineRegistry {
    pub fn machine_def(&self, machine_type: &MachineTypeId) -> Option<&MachineDef>;
    pub fn tier_def(&self, machine_type: &MachineTypeId, tier: u8) -> Option<&MachineTierDef>;
    pub fn tier_def_by_item(&self, item_id: &ItemId) -> Option<(&MachineDef, &MachineTierDef)>;
}
```

Loaded at `Startup` from `assets/machines/`. The registry is the single read source for all machine type/tier metadata. `by_item` is populated during load by walking each `MachineDef.tiers[*].item_id`. Recipe graph and dispatcher resolve placed-machine metadata via `tier_def_by_item` keyed on `Machine.item_id`.

### Link to ItemKind::Machine

Each `MachineTierDef.item_id` must correspond to an `ItemDef` with `ItemKind::Machine { machine_type, tier }` where `machine_type` matches the parent `MachineDef.machine_type` and `tier` matches `MachineTierDef.tier` (see `recipe-graph.md §3`).

Asset-consistency invariant (checked at load): every `ItemKind::Machine { machine_type, tier }` item has a matching `MachineTierDef` in the registry, and vice versa. The recipe-graph builder asserts bidirectional consistency.

---

## 3. ECS Structure

### Machine entity

```rust
#[derive(Component, Debug)]
#[require(Transform, Save)]
pub struct Machine {
    pub item_id: ItemId,         // resolves machine_type + tier via MachineRegistry.tier_def_by_item
    pub orientation: Orientation,
}
```

`Save` (from `save.md §3`) is required so machine entities persist by default. `item_id` is the single authoritative key; `machine_type` and `tier` are looked up through `MachineRegistry`, never duplicated on the component.

Port positions are derived from `MachineRegistry.tier_def_by_item(machine.item_id).1.{energy_ports, logistics_ports}` and `orientation` at need (placement, upgrade, save load). Source of truth = registry + orientation, not duplicated state.

### Port entities

One entity per port, spawned by `on_machine_placed_observer` on `OnAdd<Machine>`. Each carries:

```
Port entity
├── Transform                       (world position = anchor + orientation.transform(offset))
├── IoPortMarker { owner: Entity }  (back-pointer; cleanup convenience)
├── EnergyPortOf(Entity) | LogisticsPortOf(Entity)   (one or the other; relationship to machine)
├── PortLabel(String)               (asset-authored label; used by machine-ui)
├── Collider::sphere(0.4)           (raycast target for cable placement)
├── Sensor
├── #[require(Save)]
└── PortPolicy::default()           (for logistics ports only; see networks.md §2)
```

`MachineLogisticsPorts` / `MachineEnergyPorts` relationship targets on the machine auto-populate from `LogisticsPortOf` / `EnergyPortOf` (already defined in `src/machine/mod.rs`).

### Item-side machine handle

The hotbar carries machine items by `ItemId`. The build system emits `WorldObjectEvent { item_id, pos, kind: Placed }`; `place_machine_system` filters items whose `ItemKind` is `Machine` and spawns the matching `MachineTierDef` via `MachineRegistry.tier_def_by_item(item_id)`.

---

## 4. Placement Flow

> Generic placement (ghost preview, footprint validation via scene collider, surface support, build-mode input) belongs to `building.md`. This section covers only machine-specific spawn after the build system commits a placement.

### place_machine_system

**Trigger:** `WorldObjectEvent { kind: Placed, item_id, pos, orientation }`.

Step by step:

1. Look up `item_id` via `MachineRegistry.tier_def_by_item(item_id)`. If absent, skip (not a machine — other placement systems handle).
2. Spawn the machine entity:
   ```
   Machine { item_id, orientation }
   + Transform::from_translation(pos)
   + RigidBody::Static
   + Collider (derived from scene mesh; cached in MachineColliders)
   + SceneRoot(asset_server.load(tier_def.scene))
   ```
3. `on_machine_placed_observer` (on `OnAdd<Machine>`) runs immediately:
   a. For each `energy_ports[i]`, spawn a port entity with `EnergyPortOf(machine_entity)` and `Transform::from_translation(pos + orientation.transform(offset.as_vec3()))`.
   b. For each `logistics_ports[i]`, spawn a port entity with `LogisticsPortOf(machine_entity)` and the same transform formula, plus `PortPolicy::default()`.
   c. Insert `RecipeProcessor::default()`, `MachineCapability::default()`, `MachineJobPolicy::default()`, `Name` (default `"{display_name} #{generation}"`).
   d. Emit `MachinePlaced { machine: Entity, item_id }`.
4. `machine_capability_register_system` (see `crafting.md §3`) reacts to `MachinePlaced` and populates `MachineCapability.capable` from `RecipeGraph` ∩ `TechTreeProgress.unlocked_recipes`.

Placement validation (footprint clear, surface support) happens upstream in the building system before `WorldObjectEvent { Placed }` is emitted. Machines.md trusts the event.

### Special-case machines

Machines whose `machine_type` matches a kind handled by another plugin receive additional components via type-specific observers:
- `"generator"` → `GeneratorUnit` (see `networks.md §3`)
- `"miner"` → `MinerMachine`, link to nearest `OreDeposit` (see `mining.md`)
- `"storage_crate"` → `StorageUnit` (see `networks.md §2`)
- `"outpost_beacon"` → `OutpostBeacon`, `AegisEmitter` (see `aegis.md`)

These observers all key on `OnAdd<Machine>`, resolve `machine_type` via `MachineRegistry.tier_def_by_item(machine.item_id).0.machine_type`, and gate on the type string. The base placement system has no knowledge of which machine types are special — special behavior belongs in the owning plugin.

---

## 5. Removal Flow

> Building-system removal selection (raycast against placeables, ghost-highlight) belongs to `building.md`. This section covers only machine-specific cleanup after the build system commits a removal targeting a machine entity.

Removal is initiated by `WorldObjectEvent { kind: Removed, pos }` and resolved by `remove_placed_objects_system` (the building plugin's umbrella system).

Machine-specific cleanup (when the target is a `Machine` entity):

1. If the machine has a `RecipeProcessor` with any `Running` or `PowerPaused` slot:
   a. For each running slot, return consumed inputs to the network: `give_items` of `recipe.inputs[i]` where `consumed == true`, on the slot's logistics port network.
   b. Release reserved catalysts (decrement `NetworkReservations.catalyst[item]`).
   c. Release amp allocation on the power network.
2. Despawn all cables whose endpoints target any of the machine's ports (each despawn flows through `cable_removed_system` for split/merge — see `networks.md §1`).
3. Despawn all port entities owned by the machine (found via `MachineLogisticsPorts` and `MachineEnergyPorts`).
4. Despawn the machine entity. Default Bevy cascade removes `Transform`-attached children.
5. Emit `MachineRemoved { machine: Entity, item_id: ItemId }` — consumed by type-specific cleanup observers (miner unlinks `OreDeposit`, generator releases buffered joules, etc.).
6. Return the machine item (`Machine.item_id`) to the player's hotbar. Modules attached to the machine drop to the connected logistics network or to the hotbar if no network is connected (post-VS).

Item return is **lossless** by design: machine deconstruction never destroys the item. Inputs in flight at removal are returned to the network in step 1; produced outputs already in storage are unaffected.

---

## 6. Tier Upgrade

Two upgrade paths. Both end at the same state — machine entity bound to the higher-tier item with preserved configuration.

### 6.1 Recipe model: tier N machine is an input to the tier N+1 machine recipe

The producer recipe for `{machine_type}_mk{N+1}` lists `{machine_type}_mk{N}` as a `RecipeInput` with `consumed: true`, alongside the other ingredients (alloys, circuits, etc.). This anchors tier progression in the recipe graph: you can't craft a tier-N+1 machine without consuming a tier-N machine. Wildcard expansion authors this once per tier ladder (e.g. `$machine_type_mk$N+1` consumes `$machine_type_mk$N`).

The **upgrade kit** for `{machine_type}` from tier N→N+1 is then the same recipe with the tier-N machine input removed (since the placed tier-N machine satisfies that input in-world). Kit item: `ItemKind::MachineUpgradeKit { machine_type, from_tier, to_tier }`. The graph builder may auto-derive the kit's producer recipe by stripping the machine input from the parent recipe (open question — see `recipe-graph.md` for template/wildcard support).

VS: hand-author the upgrade-kit recipes alongside the tier ladder. Auto-derivation is a recipe-graph optimization, not a machines-layer concern.

### 6.2 Upgrade-kit path

**Trigger:** Player interacts (E) with a placed machine while holding an upgrade kit → `MachineUpgradeRequest { machine: Entity, kit_item: ItemId }`.

**System:** `machine_upgrade_system`

Step by step:

1. Resolve the target `Machine` and the kit's `from_tier`/`to_tier`/`machine_type` from `ItemRegistry`.
2. Validate:
   a. `kit.machine_type == registry.tier_def_by_item(machine.item_id).0.machine_type` — else `MachineUpgradeFailed { reason: WrongType }`.
   b. `kit.from_tier == registry.tier_def_by_item(machine.item_id).1.tier` — else `WrongTier`.
   c. `MachineRegistry.tier_def(machine_type, to_tier).is_some()` — else `UnknownTier`.
   d. New tier's scene-derived collider fits in the existing footprint, or the additional area is clear — else `Obstructed`. (Building system's ghost-preview check; if the new tier needs more space, the player must use the deconstruct-and-replace path.)
   e. No slot is `Running` or `PowerPaused`. If any slot is busy: `MachineUpgradeFailed { reason: SlotBusy }`. Mid-job upgrade rejected — in-flight recipe is bound to tier-N parameters.
3. Consume the kit item from the player's inventory.
4. Update `Machine.item_id = new_tier_def.item_id`. Orientation, `Name`, `MachineJobPolicy`, `MachineModifierState`, hotbar binding all unchanged.
5. Despawn the old port entities (without despawning cables — the cables stay).
6. Swap the scene: `commands.entity(machine).insert(SceneRoot(new_tier_def.scene.load()))` and update the collider from the `MachineColliders` cache for the new tier.
7. Run port-respawn logic (same as placement §4.3a–b) using the new tier's `energy_ports` and `logistics_ports`. Each new port gets `PortPolicy::default()`.
8. **Cable reattachment** — for each old port that had a `LogisticsNetworkMember` / `PowerNetworkMember`: find the cable segment whose endpoint was nearest the old port's world position; rebind the cable to the **nearest matching-kind port** on the new tier (`EnergyPortOf` for power cables, `LogisticsPortOf` for logistics cables) within `0.75 m`. Insert the new membership and emit `NetworkChanged<N>`. If no port within tolerance, the cable is orphaned and `cable_removed_system` drops it as an item at its midpoint.

   **Authoring guidance:** tier ladders should keep port positions consistent across tiers (same offsets at each face for the common kinds). Players reusing upgrade kits expect cables not to fall off.
9. Rebuild `MachineCapability` from the new tier (`crafting.md §3`).
10. Emit `MachineUpgraded { machine, from_item_id, to_item_id }`.

Modules attached to the old tier: see §10 — module retention is a post-VS concern.

### 6.3 Deconstruct-and-replace path

Player removes the machine (§5) and places a new-tier machine in the same spot. The tier-N machine is consumed by the tier-N+1 producer recipe (§6.1) — same item cost as the kit path, just paid up-front in the crafting recipe instead of via a kit. State transfer is **manual**: none of `MachineJobPolicy`, port `PortPolicy`, custom `Name`, or installed modules carry over. Cables drop as items per the standard removal path. Use this path when the new tier needs a footprint the old tier did not, or when the player wants to reconfigure from scratch.

### 6.4 Downgrade

Not supported. A player wanting tier-N from a tier-N+1 machine must deconstruct and place a tier-N machine. Downgrade has no use case worth the complexity.

---

## 7. IO Port Routing

Per `gdd.md §10`, port routing is a software policy, not a physical hatch model. `PortPolicy` semantics, edit events (`PortPolicyEdit { SetDefaultMode | SetItemOverride | RemoveItemOverride }`), and UI affordances are defined in **`machine-ui.md §4.5`** (editing surface) and **`networks.md §2`** (component shape + runtime semantics). This doc does not redefine them — refer to the canonical sources.

Two machine-side rules that the placement observer enforces:

1. **Port → kind binding.** Asset-authored on `MachineTierDef.energy_ports` vs `logistics_ports`. Spawned as `EnergyPortOf` or `LogisticsPortOf` accordingly; the kind cannot change at runtime. No physical conversion ports.
2. **Default on spawn.** New logistics ports get `PortPolicy::default()` (i.e. `default_mode: Both, item_overrides: {}` per `networks.md §2`). Energy ports carry no `PortPolicy`. Recipe-derived defaults are post-VS.

---

## 8. Orientation

`Orientation = Rotation` — 4 cardinal Y-axis rotations (N/E/S/W). VS does **not** support mirroring.

| Operation | Result |
|---|---|
| `orientation.transform(IVec3)` | Rotates a canonical-space offset around Y by 0/90/180/270°. |
| Port world position | `machine.transform.translation + orientation.transform(offset.as_vec3())` |
| Scene rotation | Quaternion equivalent of `orientation`; applied via `Transform::rotation` on the `SceneRoot`. |

`BuildOrientation` resource holds the player's current pick (owned by the building system per `building.md`). `{kbd:rotate_cw}` / `{kbd:rotate_ccw}` cycle rotation (defaults `R` / `Shift+R`; see `input.md §3.2`). Mirror support is post-VS and may not land — re-evaluate when asymmetric machines arrive.

---

## 9. Persistence

Machines are saved by `moonshine_save` (see `save.md §3`) via `#[require(Save)]` on `Machine`. Saved fields:

| Component | Saved | Notes |
|---|---|---|
| `Machine { item_id, orientation }` | ✓ | Authoritative — drives tier lookup, scene, and port positions on load. |
| `Transform` | ✓ | World position. |
| `Name` | ✓ | Player rename. |
| `MachineJobPolicy` | ✓ | Per-recipe C/P/priority. |
| `RecipeProcessor` | ✓ | Full slot state — `state` (Idle/Running/PowerPaused/Blocked), `progress`, `recipe_id`, reserved catalysts. Players must be able to take breaks mid-recipe. |
| `MachineModifierState` | — | Recomputed on load by `module_effect_system`. |
| `MachineCapability` | — | Recomputed on load by `machine_capability_register_system`. |
| Port entities | ✓ | Each port carries `Save`; restored as standalone entities with their `EnergyPortOf` / `LogisticsPortOf` relationship. `PortPolicy` saved per port. |

`RecipeProcessor` save policy: **slot state persists in full.** A machine that was 47% through a recipe at save is 47% through it on load, with the same catalyst reservations and consumed-input deductions intact. Energy debt and amp allocations are network-side concerns that the network re-derives from the union of `Running`/`PowerPaused` slots after load — see `save.md §3` for the rebuild-on-load resource pattern.

On load, `on_machine_placed_observer` does **not** re-spawn ports (ports are saved entities). Instead, `machine_load_finalize_system` runs after `moonshine_save` finishes loading:
1. Rebuild `MachineColliders` cache entry if missing.
2. Insert `SceneRoot` from the tier's scene path (looked up via `MachineRegistry.tier_def_by_item(machine.item_id)`).
3. Rebuild `MachineCapability` for every loaded machine entity.
4. Recompute `MachineModifierState` from any restored module attachments.
5. Replay catalyst reservations into the `CatalystReservationBook` resource from saved `RecipeProcessor` slots.

---

## 10. Modules (Deferred)

Modules are explicitly out of scope for the vertical slice (`vertical_slice.md §8.4: "Full module system"`). The asset schema reserves `MachineTierDef.module_slots: Vec<ModuleSlotDef>` so machine assets do not need to be rewritten when the module system lands; VS-era assets leave the field empty.

Design intent captured for the post-VS spec (do not implement until VS ships):

- **Slot positions sourced from GLTF.** Like ports (§1), real-model module slot positions will be GLTF named child entities (`ModuleSlot_Speed_0`, etc.); the schema's `IVec3` positions are dev placeholders only.
- **Attachment model:** physical snap. Player places a module item in the world adjacent to a machine; placement validation snaps it to the nearest free `ModuleSlotDef` position within `module_snap_radius`. The module is spawned as its own entity, parented to the machine via `ModuleOf(Entity)` relationship.
- **Detachment:** module is removed via `WorldObjectEvent { kind: Removed }` targeting the module's world position; returned to the player.
- **Module kinds:** speed multiplier, efficiency multiplier, parallel-slot (the three already referenced by `crafting.md §7`). Additional kinds (buffer / cooling / yield) deferred — content axis, not engine axis.
- **Slot constraints:** `ModuleSlotDef` declares accepted module-kind whitelist per slot. Mixed-kind machines (e.g. "two speed slots and one parallel slot") supported.
- **Upgrade retention:** when a machine is upgraded via the kit path (§6.1), modules at slots whose `IVec3` position is identical between `from_tier` and `to_tier` are retained; modules at slots whose position changed are detached and returned to the player's inventory. Tier upgrade does not auto-route modules to compatible new slots — that is an explicit player choice.
- **Save:** modules are independent entities tagged `Save`, restored by `moonshine_save` with their `ModuleOf` relationship intact.

A full `modules.md` will be written before the module system is implemented.

---

## 11. Systems

| System | Trigger | Purpose |
|---|---|---|
| `load_machines` | `Startup` | Load `assets/machines/*.ron` into `MachineRegistry` |
| `place_machine_system` | `WorldObjectEvent { Placed, item_id }` (machine items) | Spawn `Machine`; visuals/collider attached |
| `on_machine_placed_observer` | `OnAdd<Machine>` | Spawn port entities; insert `RecipeProcessor`, `MachineCapability`, `MachineJobPolicy`, `Name`; emit `MachinePlaced` |
| `remove_placed_objects_system` | `WorldObjectEvent { Removed }` | Cable cleanup; in-flight input/catalyst return; despawn machine + ports; emit `MachineRemoved` |
| `machine_upgrade_system` | `MachineUpgradeRequest` | Validate; respawn ports; swap scene; reattach cables; rebuild `MachineCapability`; emit `MachineUpgraded` |
| `machine_load_finalize_system` | After `moonshine_save` load | Reattach scenes; rebuild `MachineCapability` and `MachineModifierState` |
| `build_orientation_input_system` (in `building.md`) | `{kbd:rotate_cw}` / `{kbd:rotate_ccw}` / `{kbd:rotate_fine}` | Update `BuildOrientation` resource |

---

## 12. Messages

| Message | Payload | Emitted by |
|---|---|---|
| `MachinePlaced` | `machine: Entity, item_id: ItemId` | `on_machine_placed_observer` |
| `MachineRemoved` | `machine: Entity, item_id: ItemId` | `remove_placed_objects_system` |
| `MachineUpgradeRequest` | `machine: Entity, kit_item: ItemId` | UI interact handler |
| `MachineUpgraded` | `machine: Entity, from_item_id: ItemId, to_item_id: ItemId` | `machine_upgrade_system` |
| `MachineUpgradeFailed` | `machine: Entity, reason: UpgradeFailReason` | `machine_upgrade_system` |

```rust
pub enum UpgradeFailReason  { WrongType, WrongTier, UnknownTier, Obstructed, SlotBusy }
```

Placement-fail messages are emitted by the building system (`building.md`), not by machines.md — machines.md trusts a `Placed` event has already passed validation.

`MachinePlaced` and `MachineUpgraded` are consumed by `machine_capability_register_system` (`crafting.md §3`) and by type-specific observers (generator, miner, storage_crate, outpost_beacon).

---

## 13. Execution Order

```
[Startup]
└── load_machines → MachineRegistry inserted

[Update — input phase (building.md)]
└── build_orientation_input_system

[Update — MachineScanSet (under GameSystems::Simulation)]
├── place_machine_system        (WorldObjectEvent Placed, machine items only)
│       └─ spawn Machine
│       └─ OnAdd<Machine> ⟶ on_machine_placed_observer (immediate)
│              ├─ spawn ports
│              ├─ insert RecipeProcessor / MachineCapability / MachineJobPolicy
│              └─ emit MachinePlaced
│
├── machine_upgrade_system      (MachineUpgradeRequest)
│       └─ swap item_id + scene; respawn ports; rebind cables; emit MachineUpgraded
│
└── remove_placed_objects_system (WorldObjectEvent Removed)
        ├─ return inputs / catalysts / amps
        ├─ despawn cables → cable_removed_system (networks.md §1)
        └─ emit MachineRemoved

[After MachineScanSet — see networks.md §4]
├── NetworkSystems::of::<Power>()
├── PowerSimSystems
├── NetworkSystems::of::<Logistics>()
└── LogisticsSimSystems   (crafting.md §11)
```

`MachineScanSet` runs before all network systems each frame — port entities exist before cable placement / network membership systems read them. The single-frame ordering guarantee is what allows place-then-cable in the same tick.

---

## 14. Vertical Slice Scope

| Feature | VS | MVP |
|---|---|---|
| `MachineDef` / `MachineTierDef` registry (with `tier_def_by_item`) | ✓ | ✓ |
| Machine placement (validation in building.md) | ✓ | ✓ |
| Port entity spawn on placement | ✓ | ✓ |
| Orientation (4 rotations, no mirror) | ✓ | ✓ |
| Machine removal with in-flight input return | ✓ | ✓ |
| Generator / storage_crate / miner type observers | ✓ | ✓ |
| `Save`/load round-trip (machine + ports + Name + MachineJobPolicy + RecipeProcessor mid-recipe) | — | ✓ |
| Tier upgrade — upgrade-kit path | — | ✓ |
| Tier upgrade — deconstruct-and-replace path | ✓ | ✓ |
| Cable reattachment on upgrade (nearest matching-kind port within tolerance) | — | ✓ |
| Module slots (asset field reserved, no runtime) | — | — (full system post-VS) |
| Outpost beacon / aegis emitter machine types | — | ✓ |
| Mirror orientation | — | — (re-evaluate post-VS) |

**VS simplifications:**
- One tier per machine type (smelter/assembler/refinery/analysis_station/generator/storage_crate/gateway, all tier 1). Upgrade UI is not exposed.
- `MachineDef.display_name` may be missing → fall back to the item id.
- `MachineTierDef.module_slots` always empty.

---

## 15. Edge Cases

| Case | Behavior |
|---|---|
| `WorldObjectEvent { Placed }` arrives with unknown `item_id` (no `tier_def_by_item` match) | `place_machine_system` skips silently (not a machine). Log at debug. |
| Place machine, then immediately cable to a port in the same frame | `MachineScanSet` runs before all network systems — ports exist before `cable_placed_system` reads them. Cable lands on the port. |
| Place machine, then save in the next frame | Port entities, `MachineJobPolicy`, and `RecipeProcessor` (idle) are saved. `MachineCapability` not saved; rebuilt on load. |
| Upgrade kit applied while one slot is running and another is idle | `MachineUpgradeFailed { SlotBusy }`. Upgrade rejected wholesale. Player must wait or cancel jobs. |
| Upgrade kit applied to machine of wrong type | `MachineUpgradeFailed { WrongType }`. Kit not consumed. |
| Upgrade kit consumed but new tier scene asset fails to load | Scene loads by handle; placeholder remains visible until ready. Upgrade is otherwise complete — collider and ports set from `MachineRegistry`, simulation runs normally. |
| Upgrade respawns ports; previously-connected cable cannot find a matching-kind port within `0.75 m` | Cable detaches from the machine side, becomes orphaned, `cable_removed_system` drops it as a cable item at its midpoint. `NetworkChanged<N>` fires; the network may split. |
| Upgrade respawns ports; new tier has the same port at the same offset | Cable rebinds losslessly — net player-visible change is just the visual model swap. (This is the authoring goal for well-designed tier ladders.) |
| Machine removed mid-recipe; output network is on a different port than input network | Inputs returned to input-network storage. Reserved catalysts decremented on the catalyst's network. No outputs produced. |
| Machine removed while another machine on the same logistics network is `Blocked` on its outputs | `MachineRemoved` triggers `NetworkChanged<Logistics>`; `recipe_start_system` re-evaluates the blocked machine. If removal freed a route, recipe may resume. |
| Save triggered while machine is at 47% on a recipe | `RecipeProcessor` persists in full (state, progress, recipe_id, reserved catalysts). On load, `machine_load_finalize_system` replays reservations into `CatalystReservationBook`; network re-derives amp/energy debt from the union of `Running`/`PowerPaused` slots. Recipe resumes at 47%. |
| Two upgrade requests submitted in the same frame for the same machine | `machine_upgrade_system` processes them sequentially. The second sees the new `item_id` and likely fails with `WrongTier` (kit was for old tier). Kit not consumed. |

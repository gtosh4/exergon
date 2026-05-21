# Building System Design

Generic placement and removal flow for every placeable: machines, platforms, cables, decorations (post-VS), modules (post-VS). Owns the ghost preview, footprint validation, surface support, rotation input, hotbar→placement dispatch, and emits the `WorldObjectEvent { Placed | Removed }` + `CableConnectionEvent` contracts that downstream plugins react to.

Read `gdd.md §10` for design intent, `machines.md` for the machine-specific spawn that follows a placed event, `networks.md` for cable network topology that follows a cable placement.

Scope boundary:
- `machines.md` — `Machine` spawn + port spawn after a machine `Placed` event.
- `networks.md` — cable segment + network membership after a `CableConnectionEvent`.
- `inventory.md` — `Hotbar` resource (selected slot, `active_item_id`).
- This doc — `PlaceableDef` asset schema, `PlaceableRegistry`, ghost preview pipeline, validation, orientation input, multi-stage interaction (cable two-click, platform two-corner), removal selection, and the event contracts emitted to downstream systems.

---

## Table of Contents

1. [Overview](#1-overview)
2. [PlaceableDef Asset Data](#2-placeabledef-asset-data)
3. [ECS Structure](#3-ecs-structure)
4. [Single-Point Placement Flow](#4-single-point-placement-flow)
5. [Multi-Stage Placement](#5-multi-stage-placement)
6. [Removal Flow](#6-removal-flow)
7. [Orientation](#7-orientation)
8. [Ghost Preview](#8-ghost-preview)
9. [Validation](#9-validation)
10. [Systems](#10-systems)
11. [Messages](#11-messages)
12. [Execution Order](#12-execution-order)
13. [Vertical Slice Scope](#13-vertical-slice-scope)
14. [Edge Cases](#14-edge-cases)

---

## 1. Overview

Building system is single entry-point for "player puts something in the world." Every placeable item (anything with `ItemDef.placeable: Some(PlaceableDef)`) routes through here. Player holds item in hotbar → ghost preview tracks look target → click commits → building system validates → emits `WorldObjectEvent { Placed }` (or `CableConnectionEvent` for cables) → downstream plugin (machines, networks, etc.) reacts.

Placement is **always-on while in `PlayMode::Exploring`** with a placeable hotbar item selected and inventory closed. No explicit build-mode toggle. Holding `{kbd:place_extend_modifier}` (default `Shift`; see `input.md §3.2`) swaps the ghost to the red removal preview; `{kbd:primary_action}` removes nearest placeable under the cursor.

Three interaction shapes are supported, declared per placeable in the RON asset:
- **Single** — one click → one placed entity (machines, decorations, storage_crate, generator).
- **TwoEndpoint** — two clicks → one cable segment between the two click points (logistics_cable, power_cable).
- **AreaRect** — two clicks → rectangle of tiles, one entity per tile (platform, future floor tiles).

The building plugin owns:
- `PlaceableRegistry` (loaded from `assets/placeables/`)
- `PlaceableColliderCache` (scene-mesh AABB per item)
- `PlacementGhost` entity (placement preview)
- `RemovalHover` marker component (attached to the target entity, drives a shader tint — no separate ghost entity)
- `BuildOrientation` resource (Satisfactory-style continuous Y via `{kbd:rotate_cw}` / `{kbd:rotate_fine}` for `AxisY`, `{kbd:rotate_free_drag}` for `Free`)
- `PendingPlacement` resource (in-progress multi-stage state)
- `placement_input_system`, `update_ghost_preview`, `update_removal_hover`, `place_entity_system`, `remove_entity_system`

The building plugin emits, never spawns. Spawning the placed entity is the consuming plugin's job (`place_machine_system`, `place_platform_system`, `cable_placed_system`). This keeps the building system small and lets new placeable kinds add themselves with a RON file and a `WorldObjectEvent` reader.

`WorldObjectEvent` carries a full `Transform` (replaces the prior `pos: Vec3` field). Translation, rotation, and scale come from the placement state; this is what gets applied to the spawned entity. Scale stays `Vec3::ONE` in VS; post-VS mirror sets a component to `-1`.

```rust
pub struct WorldObjectEvent {
    pub transform: Transform,            // translation + rotation + scale (scale = ONE in VS)
    pub item_id: ItemId,
    pub kind: WorldObjectKind,           // Placed | Removed
}
```

---

## 2. PlaceableDef Asset Data

### Schema

```rust
#[derive(Deserialize, Clone, Debug)]
pub struct PlaceableDef {
    pub item: ItemSpec,                     // full inline item metadata — ItemRegistry derives ItemDef from this
    pub interaction: InteractionShape,      // Single | TwoEndpoint | AreaRect
    pub surface: SurfaceRule,               // Ground | Anywhere | Port
    pub snap: SnapRule,                     // None | Tile(Horizontal|Vertical) | PortRaycast
    pub orientation: OrientationSupport,    // None | AxisY | Free
    pub ghost: GhostHint,                   // Scene | TiledScene | Routed
    pub footprint: Vec3,                    // required — collider half-extents; no silent fallback
    pub max_reach: Option<f32>,             // override of default MAX_REACH (16.0)
}

/// Item-generation block. The placeable RON is the source of truth for the
/// item that places it — `ItemRegistry` is populated from `PlaceableDef.item`
/// alongside non-placeable items (ingredients, fluids, etc.). This guarantees
/// every placeable item exists and stays in sync with its placement metadata.
#[derive(Deserialize, Clone, Debug)]
pub struct ItemSpec {
    pub id: ItemId,
    pub name: String,
    pub description: String,
    pub icon: AssetPath,
    pub stack_size: u32,
    pub kind: ItemKind,                     // Machine { tier } | Cable { network } | Platform | Decoration | ...
    pub tags: HashSet<TagId>,
}

#[derive(Deserialize, Clone, Debug)]
pub enum InteractionShape {
    Single,
    TwoEndpoint { item_id: ItemId },        // matches own item_id (sanity field)
    AreaRect { tile_size: f32 },
}

#[derive(Deserialize, Clone, Debug)]
pub enum SurfaceRule {
    Ground,         // requires solid surface raycast hit (machines, platforms, decorations)
    Anywhere,       // permits air projection from camera onto fixed plane (cables in air, platform second corner)
    Port,           // raycast must hit a port collider of the right kind (cables)
}

#[derive(Deserialize, Clone, Debug)]
pub enum SnapRule {
    None,                                   // free 3D position from raycast
    Tile(TileSnap),                         // tile to a repeating step along the chosen axes
    PortRaycast,                            // snap to port-entity transform if raycast hits one
}

#[derive(Deserialize, Clone, Debug)]
pub enum TileSnap {
    /// Tile along the XZ plane (floors, platforms). `step` is the tile pitch.
    /// `axis` rotates the tile basis so non-grid-aligned tiling works (e.g.
    /// platforms tiled along a 30° corridor). Default basis = world X/Z.
    Horizontal { step: f32, axis: Option<Quat> },
    /// Tile along a vertical plane (walls, fences). `normal` selects the
    /// plane (default = world X axis → tile on YZ). `step` is the tile pitch.
    Vertical { step: f32, normal: Option<Vec3> },
}

#[derive(Deserialize, Clone, Debug)]
pub enum OrientationSupport {
    None,                                   // ignores BuildOrientation; placed identity-rotated
    AxisY,                              // continuous Y rotation (machines); {kbd:rotate_cw} / {kbd:rotate_fine} adjust
    Free,                                   // arbitrary Quat from mouse-drag rotate gesture (platforms, decorations)
}

#[derive(Deserialize, Clone, Debug)]
pub enum GhostHint {
    /// Single instance of the entity's scene, tinted (machines, generators, crates).
    Scene,
    /// Scene tiled across the area-rect footprint, one instance per tile.
    /// The ghost mirrors what the final placed result looks like — the same
    /// scene asset that the per-tile spawn will use.
    TiledScene,
    /// Cable mesh rendered along the actual routing path (Bezier/catenary —
    /// whatever the placed cable uses), not a straight line cuboid.
    Routed,
}
```

### Asset format

`assets/placeables/<item_id>.ron`:

```ron
(
    item: (
        id: "smelter_mk1",
        name: "Smelter Mk1",
        description: "Melts ore into ingots.",
        icon: "icons/smelter_mk1.png",
        stack_size: 50,
        kind: Machine(tier: 1),
        tags: ["machine", "smelter"],
    ),
    interaction: Single,
    surface: Ground,
    snap: None,
    orientation: AxisY,
    ghost: Scene,
    footprint: (1.0, 1.0, 1.0),
    max_reach: None,
)
```

```ron
(
    item: (
        id: "platform",
        name: "Platform",
        description: "A flat tile to build on.",
        icon: "icons/platform.png",
        stack_size: 200,
        kind: Platform,
        tags: ["platform", "building"],
    ),
    interaction: AreaRect(tile_size: 8.0),
    surface: Ground,
    snap: Tile(Horizontal(step: 8.0, axis: None)),
    orientation: Free,
    ghost: TiledScene,
    footprint: (4.0, 0.125, 4.0),
    max_reach: None,
)
```

```ron
(
    item: (
        id: "logistics_cable",
        name: "Logistics Cable",
        description: "Routes items between ports.",
        icon: "icons/logistics_cable.png",
        stack_size: 100,
        kind: Cable(network: "logistics"),
        tags: ["cable", "logistics"],
    ),
    interaction: TwoEndpoint(item_id: "logistics_cable"),
    surface: Port,
    snap: PortRaycast,
    orientation: None,
    ghost: Routed,
    footprint: (0.1, 0.1, 0.1),
    max_reach: None,
)
```

### PlaceableRegistry resource

```rust
#[derive(Resource, Default)]
pub struct PlaceableRegistry {
    by_item: HashMap<ItemId, PlaceableDef>,
}

impl PlaceableRegistry {
    pub fn get(&self, item_id: &ItemId) -> Option<&PlaceableDef>;
    pub fn is_placeable(&self, item_id: &ItemId) -> bool;
}
```

Loaded at `Startup` from `assets/placeables/`. The registry is the single read source for placement metadata **and the source of truth for the items that place them** — `ItemRegistry` is built by combining every `PlaceableDef.item` with the non-placeable items from `assets/items/`. If an item is in the hotbar but not in `PlaceableRegistry`, the ghost and click-to-place are no-ops (item is non-placeable; e.g. an ingot).

Consistency invariant (checked at load): every `PlaceableDef.item.id` is unique across the registry and no non-placeable `ItemDef` collides with it. The item-generation step panics on collision — duplicate item ids are a build error, not a runtime warning.

### PlaceableColliderCache resource

```rust
#[derive(Resource, Default)]
pub struct PlaceableColliderCache {
    by_item: HashMap<ItemId, PlaceableCollider>,
}

pub struct PlaceableCollider {
    pub aabb_half_extents: Vec3,            // derived from scene mesh AABB at load
    pub collider: Collider,                 // avian3d cuboid built from half_extents
}
```

Populated by `build_placeable_collider_cache` on `Startup` after `MachineRegistry` and scene assets load:
1. For each `PlaceableDef`: use `def.footprint` directly to build the cuboid collider. `footprint` is required — no scene-AABB derivation, no default fallback. A missing or zero footprint is a load-time panic (`PlaceableDef "<item_id>" has invalid footprint <vec>`).
2. Insert the resulting `PlaceableCollider` keyed by `item_id`.

Rationale for the panic: silent `Vec3::splat(2.0)` fallback masks authoring mistakes and lets ghosts/placements diverge from the actual visual. VS surfaces these as crashes immediately; post-VS may relax to scene-AABB derivation when machine models are final and assets settle, but until then explicit-or-panic is the right contract.

`PlaceableCacheReady` flips true after the registry is fully resolved (one pass; no deferred async).

---

## 3. ECS Structure

### Resources

```rust
/// Current build-rotation state — stored as a Quat so it can represent
/// continuous Y rotation (machines, Satisfactory-style) and free arbitrary
/// rotation (platforms, decorations) uniformly. The input system constrains
/// the rotation axis per the active `OrientationSupport` — it does not
/// discretize; rotation is continuous.
#[derive(Resource, Default)]
pub struct BuildOrientation(pub Quat);

#[derive(Resource, Default)]
pub enum PendingPlacement {
    #[default]
    Idle,
    TwoEndpoint { item_id: ItemId, anchor: Vec3, anchor_port: Option<Entity> },
    AreaRect    { item_id: ItemId, corner_a: Vec3 },
}

#[derive(Resource)]
pub struct GhostPreview { entity: Entity, last_item_id: String }

#[derive(Resource, Default)]
pub struct PlaceableCacheReady(pub bool);
```

`BuildOrientation` persists across hotbar swaps. Player rotates `AxisY` placeables (machines) continuously around world Y — `{kbd:rotate_cw}` (default `R`) for a small step (default 10°), `{kbd:rotate_ccw}` (default `Shift+R`) for the reverse, `{kbd:rotate_fine}` (default scroll wheel) for finer control while holding the placeable. `Free` placeables (platforms, decorations) rotate arbitrarily around the surface normal via `{kbd:rotate_free_drag}` (default `R` held + mouse drag). Non-rotatable placeables ignore the resource entirely. The resource always stores a valid `Quat`; input handlers constrain the axis per `OrientationSupport` (Y-only for `AxisY`, arbitrary for `Free`).

`PendingPlacement` is `Idle` between placements and between sessions (not saved). `{kbd:secondary_action}`, hotbar swap, or `{kbd:cancel}` clear it back to `Idle` without consuming inputs.

### Marker components

```rust
/// On the persistent placement-preview entity owned by the building plugin.
#[derive(Component)] pub struct PlacementGhost;

/// Attached at runtime to the actual world entity currently under the
/// shift-hover cursor. A render-layer override / material tint system reads
/// this marker and applies the red removal indicator directly to the live
/// scene — no separate red-ghost entity to keep in sync with model swaps,
/// orientation, or scale. The marker is added/removed each frame by
/// `update_removal_hover`.
#[derive(Component)] pub struct RemovalHover;
```

`update_ghost_preview` repositions and re-skins the single `PlacementGhost` entity. `update_removal_hover` adds `RemovalHover` to the resolved target entity (machine / platform / cable) and removes it from any entity that's no longer the hover target. A separate `removal_tint_system` (rendering concern) watches `Added<RemovalHover>` / `RemovalRemoved` and swaps the entity's material (or pushes a tint to a shader parameter) to indicate the red preview state. The tint system is described as a contract — its implementation (extracted material override vs. shader parameter on a shared material) is decided when rendering work lands; the building plugin only owns the marker.

### LookTarget

Unchanged from the current `world/interaction.rs` — building system reads it, doesn't own it:

```rust
pub enum LookTarget {
    Nothing,
    Surface { pos: Vec3, normal: Vec3, entity: Entity },
}
```

Port-aware variant is not needed: when the raycast hits a port entity, `LookTarget::Surface.entity` is the port entity and `pos` is its world position. `placement_input_system` checks `IoPortMarker` membership of `entity` to decide port-snap behavior.

---

## 4. Single-Point Placement Flow

`InteractionShape::Single`. Used by machines, generators, storage crates, decorations.

`placement_input_system` step by step on `Started<PrimaryAction>` — i.e. `{kbd:primary_action}` — in `LocalBodyContext` (no `{kbd:place_extend_modifier}` held, hotbar selection is placeable, look target is `Surface`):

1. Resolve `def = PlaceableRegistry.get(item_id)`. If `None`, no-op.
2. Resolve `collider = PlaceableColliderCache.by_item[item_id]`.
3. Compute `translation`:
   - `SurfaceRule::Ground` → `look_target.pos + look_target.normal * collider.aabb_half_extents.y` (anchor on the surface, displaced up by half-height along the surface normal).
   - `SurfaceRule::Anywhere` → `look_target.pos` (no displacement).
   - `SurfaceRule::Port` → invalid for `Single`; load-time validation rejects this combo.
   - Apply `SnapRule::Tile` if declared: snap to the tile basis (see §2 `TileSnap`).
4. Compute `rotation` per `def.orientation`:
   - `None` → `Quat::IDENTITY`.
   - `AxisY` → `BuildOrientation.0` (a Y-axis Quat — continuous, not snapped).
   - `Free` → `BuildOrientation.0` (arbitrary Quat from the rotate gesture).
5. Validate (see §9): footprint clear, surface supported, within `max_reach`, hotbar has stock. On fail: emit `PlacementRejected { item_id, reason }` and return.
6. Decrement hotbar stock (`take_from_any_storage` against the network the active hotbar is bound to — see `inventory.md`).
7. Emit:
   ```rust
   WorldObjectEvent {
       transform: Transform { translation, rotation, scale: Vec3::ONE },
       item_id,
       kind: Placed,
   }
   ```
8. Downstream plugin (`place_machine_system`, `place_platform_system`, etc.) consumes the event and spawns the actual entity with `transform` applied as-is.

Hotbar item with `ItemKind::Machine { .. }` and no `PlaceableDef` is a load-time error.

### Surface-aligned orientation note

The placed entity's `Transform.rotation` is the one carried by the event. Machines (`AxisY`) rotate around world Y at any angle and stand upright on the ground; the surface normal is used only to displace the anchor, not to orient the placed entity. `Free` placeables (platforms, decorations) author whatever Quat the rotate gesture produced — including out-of-plane rotation for decorations. Wall/ceiling placement is post-VS — if added, it becomes a `SurfaceRule::AnySurface` variant with a separate orientation derivation.

---

## 5. Multi-Stage Placement

### 5.1 TwoEndpoint (cables)

`InteractionShape::TwoEndpoint`, `SurfaceRule::Port`, `SnapRule::PortRaycast`.

State machine driven by `PendingPlacement::TwoEndpoint`:

**First click** (state: `Idle` → `TwoEndpoint`):
1. Look target must be `Surface { entity, pos, .. }` where `entity` has `IoPortMarker` of the matching kind for the cable item (logistics_cable → `LogisticsPortOf`; power_cable → `EnergyPortOf`). The port entity is found by walking `IoPortMarker.owner` for cleanup or used directly as the cable endpoint via `LookTarget.entity`.
2. Set `PendingPlacement::TwoEndpoint { item_id, anchor: pos, anchor_port: Some(entity) }`.
3. No event emitted; ghost switches to "cable from anchor to cursor" mode.

If the first click is not on a port: emit `PlacementRejected { reason: NoPortAtAnchor }`. State stays `Idle`.

**Second click** (state: `TwoEndpoint { item_id, anchor, anchor_port }` → `Idle`):
1. Look target must be `Surface { entity, pos, .. }` where `entity` is a port of the matching kind (same rule as first click).
2. Validate: `anchor.distance(pos) > 0.1`; both endpoints exist; cable item stock available.
3. Decrement hotbar stock.
4. Emit:
   ```rust
   CableConnectionEvent {
       from: anchor,
       to: pos,
       item_id,
       kind: Placed,
       from_port: anchor_port,
       to_port: Some(entity),
   }
   ```
5. Reset `PendingPlacement = Idle`.

**Cancel paths**: `{kbd:secondary_action}`, `{kbd:cancel}`, hotbar swap, or selecting a different item id resets to `Idle` without consuming inputs.

Cables differ from machines in two respects that justify the dedicated event:
- Two world positions, not one — `CableConnectionEvent` already exists in `networks.md` for this.
- Snap target is a port entity, not a free surface — `LookTarget.entity` provides this.

Same building-plugin code path handles both kinds because the kind is in the item id; downstream `cable_placed_system` (in `network/membership.rs`) reads `CableConnectionEvent` directly.

### 5.2 AreaRect (platforms)

`InteractionShape::AreaRect { tile_size }`, `SurfaceRule::Ground` (first corner) and `Anywhere` (second corner via air-projection).

State machine driven by `PendingPlacement::AreaRect`:

**First click** (state: `Idle` → `AreaRect`):
1. Look target must be `Surface { pos, normal, .. }`.
2. Apply `SnapRule::Tile(Horizontal { step, axis })`: snap `pos + normal * half_y` onto the tile basis (`axis` rotates the tile lattice for non-axis-aligned tiling).
3. Set `PendingPlacement::AreaRect { item_id, corner_a: snapped }`.
4. No event emitted.

**Second click** (state: `AreaRect { item_id, corner_a }` → `Idle`):
1. Determine `corner_b`:
   - If look target is `Surface`: snap to the same tile basis as `corner_a`.
   - Else (looking into the sky): project camera ray onto the horizontal plane at `corner_a.y`, then snap (`air_platform_pos` logic from existing code, extended with the tile basis).
2. Compute the tile-index range across the `TileSnap` basis between `corner_a` and `corner_b`.
3. For each tile cell in the range:
   a. Tile center = `corner_a + basis * Vec2(i, j) * step` (basis lifts the 2D index into 3D world space; for `Horizontal`, X/Z; for `Vertical`, the plane's two in-plane axes).
   b. Validate footprint (see §9). On per-tile fail, skip that tile (no global abort).
   c. Decrement one hotbar item; on stockout, stop iterating and reset to `Idle`.
   d. Emit `WorldObjectEvent { transform: Transform { translation: tile_pos, rotation: build_orientation.0, scale: Vec3::ONE }, item_id, kind: Placed }`. For `OrientationSupport::Free` platforms, every tile shares the same rotation — the rotate gesture sets it before the rectangle is committed.
4. Reset `PendingPlacement = Idle`.

Per-tile failures (e.g. one tile already has a machine on it) are individually skipped rather than failing the whole rectangle. Stockout aborts further tiles — the rectangle ends short. Both behaviors are intentional: players paint platforms, and partial paints are better than all-or-nothing.

**Cancel paths**: same as TwoEndpoint.

### 5.3 Pending state hotbar swap

Swapping hotbar selection (`{kbd:hotbar_slot_N}`, `{kbd:hotbar_bank_switch}`) while `PendingPlacement != Idle` resets to `Idle` with no events. This prevents the rectangle starting on platform but finishing on a different item id.

---

## 6. Removal Flow

Two systems split the work: `update_removal_hover` maintains the visual marker every frame; `placement_input_system` commits the removal on click.

### 6.1 Hover marker (per-frame)

`update_removal_hover` runs every frame while `{kbd:place_extend_modifier}` is held and look target is `Surface`:

1. Identify the would-be removal target by resolving `LookTarget::Surface.entity` through the same component-priority chain as commit (machine / platform / port-owner / cable segment).
2. Diff against last frame: insert `RemovalHover` on the new target entity, remove it from any entity that no longer matches.
3. If no target resolves, ensure no entity carries the marker.

`removal_tint_system` (rendering side) watches `Added<RemovalHover>` / `RemovalRemoved<RemovalHover>` and applies the red tint to whatever material the live entity is rendering with. No separate ghost entity exists — orientation, scale, model swaps, animation pose, multi-mesh hierarchies all stay correct automatically because the indicator rides on the real entity.

### 6.2 Commit (on click)

`placement_input_system` on `Started<PrimaryAction>` (the `{kbd:primary_action}` token) **with `{kbd:place_extend_modifier}` held**:

1. Look target must be `Surface { entity, pos, .. }`.
2. Identify the removal target (same chain as the hover system):
   - If `entity` has `Machine` → target the machine entity. `removal_transform = machine_transform`. `item_id` = machine's `Machine.item_id`.
   - Else if `entity` has `Platform` → target the platform entity. `removal_transform = platform_transform`. `item_id = "platform"`.
   - Else if `entity` has `IoPortMarker` → walk to `IoPortMarker.owner` (the machine), same as machine case. (Allows shift-clicking a port to remove the parent machine.)
   - Else if `entity` has `LogisticsCableSegment` or `PowerCableSegment` → target the cable. `removal_transform.translation = pos`. `item_id` = the cable item id from `N::CABLE_ITEM_ID`. This is the "typed" branch that `cable_removed_system` already handles.
   - Else → `PlacementRejected { reason: NoTargetUnderCursor }`.
3. Emit `WorldObjectEvent { transform: removal_transform, item_id, kind: Removed }`.
4. Downstream plugin consumes the event:
   - `remove_placed_objects_system` (in `machine/`) — handles machine + platform removal, in-flight input return, port despawn.
   - `cable_removed_system` (in `network/membership.rs`) — handles cable segment removal, network split/merge.
5. Stock return is the downstream plugin's responsibility — `machines.md §5` covers machine item return.

The current `world/interaction.rs` `{kbd:place_extend_modifier}` + `{kbd:primary_action}` branch sends a generic `WorldObjectEvent { item_id: "", kind: Removed }`, leaving target identification to the receiver. The building system **resolves the target ahead of the event** and sets `item_id` to the specific kind. This eliminates the generic-string-empty path: every `Removed` event carries the resolved item id. Downstream cable-removal still falls back to nearest-cable-by-distance only when the `item_id` is the cable's own (typed-removal mode).

### Multi-removal modifier (post-VS)

`{kbd:place_extend_modifier}` + drag for area removal is post-VS. VS supports one removal per `{kbd:primary_action}`.

---

## 7. Orientation

Orientation is stored as a `Quat` in `BuildOrientation` and authored straight onto the spawned entity's `Transform.rotation`. The placeable's `OrientationSupport` decides how the input gesture maps to that Quat — there is no separate `Orientation` enum carried alongside the transform.

Post-VS, mirror support reuses `Transform.scale` (component-wise `-1` flips). The building system reserves that field but always emits `Vec3::ONE` for now.

`build_orientation_input_system` updates `BuildOrientation.0` from input, gated on the active placeable's `OrientationSupport`:

| `OrientationSupport` | Input handling |
|---|---|
| `None` | Input ignored; resource forced to `Quat::IDENTITY` while this placeable is held. |
| `AxisY` (machines) | Continuous Y rotation, Satisfactory-style. `{kbd:rotate_cw}` rotates CW by `BUILD_ROT_STEP` (default 10°), `{kbd:rotate_ccw}` rotates CCW, `{kbd:rotate_fine}` adjusts continuously while holding the placeable. Resource is always a Y-axis Quat. |
| `Free` (platforms, decorations) | `{kbd:rotate_free_drag}` rotates continuously around the surface normal; a tap of `{kbd:rotate_cw}` snaps to the nearest 15° increment. `{kbd:rotate_ccw}` resets to identity. Resource is an arbitrary Quat. |

Hotbar swap → orientation is sticky **across compatible `OrientationSupport`**. Switching from an `AxisY` item to a `Free` item preserves the rotation directly. Switching from `Free` to `AxisY` projects the held Quat onto the Y axis (keeps the yaw component, drops pitch/roll). Switching to `None` forces identity for the duration of the selection but the previous value is restored when switching back.

Ghost preview applies `BuildOrientation` to its `Transform.rotation` for `AxisY` and `Free`. On placement, the resource value is copied into `WorldObjectEvent.transform.rotation`.

Save: `BuildOrientation` is **not saved**. Resets to `Quat::IDENTITY` on session start.

---

## 8. Ghost Preview

One `PlacementGhost` entity, persistent, re-skinned and repositioned per frame.

`update_ghost_preview` step by step:

1. Show iff `hotbar.active_item_id().is_some() && !inventory_open && !place_extend_modifier_held && PlaceableRegistry.is_placeable(item_id)` (the modifier check is the `{kbd:place_extend_modifier}` token).
2. Match `def.ghost`:
   - `GhostHint::Scene` → load the entity's own scene asset (the same one its `place_*_system` will spawn) as the ghost; tint via `GhostAssets.materials.get(item_id)` or fall back to `GhostAssets.fallback_material`. One scene instance.
   - `GhostHint::TiledScene` → for `InteractionShape::AreaRect`, spawn one scene instance per tile in the active rectangle and lay them out on the tile basis. The ghost is a parent transform with a child per tile; child meshes are the same scene asset used for placement. Tiling reuses the exact pose math from §5.2 so the preview matches the committed result 1:1.
   - `GhostHint::Routed` → for `InteractionShape::TwoEndpoint` cables, build the cable mesh along the actual routing path (catenary / Bezier / whatever the cable plugin renders for a placed segment). The ghost mirrors the placed visual: same routing curve, same mesh, same cross-section. No straight-line cuboid stand-in.
3. Position:
   - `PendingPlacement::Idle` → as §4 step 3.
   - `PendingPlacement::TwoEndpoint` → routing input is `(anchor, look-target)`; the cable router produces the mesh.
   - `PendingPlacement::AreaRect` → as §5.2, with tile children laid out across the rectangle.
4. Rotation: per §7 — identity for `OrientationSupport::None`; `BuildOrientation.0` for `AxisY` and `Free`. `TiledScene` applies the rotation to every tile child uniformly. `Routed` derives its own orientation from the routing path.
5. Material:
   - Green tint when placement valid (see §9).
   - Red tint when invalid (footprint blocked, no surface, out of stock, port mismatch).
   - Validation runs in-system to color the ghost; same checks re-run on click.

Removal preview is handled by the `RemovalHover` marker + `removal_tint_system` (see §6.1), not a ghost entity. The marker rides directly on the entity that would be removed, so the red preview tracks model swaps, animation, scale, and hierarchy automatically. The hover system gates on `{kbd:place_extend_modifier}` held — releasing it clears any active marker.

---

## 9. Validation

Same validator runs in `update_ghost_preview` (for color) and in `placement_input_system` (for commit). Centralized in `fn validate_placement(...)`:

```rust
pub enum PlacementReason {
    OutOfStock,
    OutOfReach,           // > def.max_reach.unwrap_or(MAX_REACH)
    FootprintBlocked,     // shape_intersections returned a non-sensor entity
    NoSurface,            // SurfaceRule::Ground but look target is Nothing/air
    NoPortAtAnchor,       // SurfaceRule::Port but look_target.entity is not a matching port
    PortKindMismatch,     // power cable click on a logistics port (or vice versa)
    PendingMismatch,      // two-stage second click expects same item_id as anchor
    NoTargetUnderCursor,  // removal: nothing removable under {kbd:place_extend_modifier}+{kbd:primary_action}
}
```

Order (cheap first):
1. **Stock** — hotbar has ≥1 of `item_id` (or, for `AreaRect`, ≥1 for the next tile).
2. **Reach** — `(camera.translation.distance(transform.translation)) <= def.max_reach.unwrap_or(MAX_REACH)`.
3. **Surface rule** — per `def.surface`.
4. **Port rule** — for `SurfaceRule::Port`, look target's entity has the matching port-of-kind component.
5. **Footprint clear** — `SpatialQueryFilter` excluding the player and any pending-stage anchor port; `shape_intersections(collider.collider, transform.translation, transform.rotation, &filter)` returns no non-sensor entities. Sensor entities (port colliders, region triggers) don't block placement.

For `AreaRect`, footprint check runs per tile.

Failure path always emits `PlacementRejected { item_id, reason }` and never decrements stock.

`PlacementRejected` is consumed by the UI plugin to surface a toast/sound and is consumed by `telemetry.md` (post-VS event).

---

## 10. Systems

| System | Trigger | Purpose |
|---|---|---|
| `load_placeables` | `Startup` | Load `assets/placeables/*.ron` into `PlaceableRegistry` and populate `ItemRegistry` from `PlaceableDef.item` |
| `build_placeable_collider_cache` | `Startup` after `load_placeables` | Populate `PlaceableColliderCache` from `PlaceableDef.footprint` — panics on missing/zero footprint |
| `setup_ghost_preview` | `OnEnter(PlayMode::Exploring)` | Spawn the `PlacementGhost` entity |
| `hide_ghost_preview` | `OnExit(PlayMode::Exploring)` | Hide the ghost; clear any `RemovalHover` markers |
| `update_look_target` | `Update`, in `Exploring` | Cast camera ray, populate `LookTarget` (re-uses `world/interaction.rs`) |
| `build_orientation_input_system` | `Update`, in `Exploring` | Update `BuildOrientation` per the active `OrientationSupport` — `{kbd:rotate_cw}` / `{kbd:rotate_ccw}` / `{kbd:rotate_fine}` for continuous Y on `AxisY`, `{kbd:rotate_free_drag}` for arbitrary on `Free` |
| `update_ghost_preview` | `Update`, after `update_look_target` | Position, skin, and color the placement ghost |
| `update_removal_hover` | `Update`, after `update_look_target` | Maintain the `RemovalHover` marker on the current `{kbd:place_extend_modifier}`-hover target |
| `removal_tint_system` | `Update`, after `update_removal_hover` | Apply/remove the red material tint on entities with `RemovalHover` (rendering contract — implementation lives with material system) |
| `placement_input_system` | `Update`, on `Started<PrimaryAction>` (the `{kbd:primary_action}` token) | Single-point + multi-stage placement + removal dispatch; emits `WorldObjectEvent` / `CableConnectionEvent` / `PlacementRejected` |
| `pending_placement_cancel_system` | `Update`, on `{kbd:secondary_action}` / `{kbd:cancel}` / hotbar swap | Reset `PendingPlacement` to `Idle` |

All run in `Update` schedule, in set `BuildingSet`, which runs after `inventory::HotbarSet` (so hotbar selection is current) and before `MachineScanSet` (so emitted `WorldObjectEvent`s are read same-frame by `place_machine_system`).

---

## 11. Messages

| Message | Payload | Emitted by | Consumed by |
|---|---|---|---|
| `WorldObjectEvent` | `transform: Transform, item_id: ItemId, kind: WorldObjectKind` | `placement_input_system` | `place_machine_system` (machines.md), platform placer, `cable_removed_system` (typed-removal branch), generator observer, telemetry |
| `CableConnectionEvent` | unchanged from networks.md | `placement_input_system` (TwoEndpoint path) | `cable_placed_system` |
| `PlacementRejected` | `item_id: ItemId, reason: PlacementReason` | `placement_input_system` and `update_ghost_preview` (validation hint, single emission per click) | UI toast plugin, telemetry |
| `PlaceableCacheReady` | unit (state flip resource) | `build_placeable_collider_cache` | Anything that wants to gate on the cache being warm |

`WorldObjectKind` is unchanged: `Placed | Removed`.

`PlacementRejected` is fired by `placement_input_system` on commit-time validation fail. It is **not** fired by `update_ghost_preview` (the ghost color is enough feedback during preview). One event per rejected click.

---

## 12. Execution Order

```
[Startup]
├── load_placeables                  → PlaceableRegistry + ItemRegistry entries
└── build_placeable_collider_cache   → PlaceableColliderCache (panics on missing footprint) → PlaceableCacheReady

[OnEnter(PlayMode::Exploring)]
└── setup_ghost_preview              → PlacementGhost entity

[Update — BuildingSet, after HotbarSet, before MachineScanSet]
├── update_look_target               → LookTarget resource
├── build_orientation_input_system   → BuildOrientation (snap or free per active OrientationSupport)
├── pending_placement_cancel_system  → resets PendingPlacement on {kbd:secondary_action}/{kbd:cancel}/hotbar-swap
├── update_ghost_preview             ← reads LookTarget, BuildOrientation, PendingPlacement, PlaceableRegistry, PlaceableColliderCache
├── update_removal_hover             ← reads LookTarget, keyboard → maintains RemovalHover marker on target entity
├── removal_tint_system              ← reads Added/Removed<RemovalHover> → tints live entity material
└── placement_input_system           ← reads LookTarget, BuildOrientation, PendingPlacement, Hotbar
                                     → emits WorldObjectEvent / CableConnectionEvent / PlacementRejected

[Update — MachineScanSet (after BuildingSet)]
├── place_machine_system             ← reads WorldObjectEvent (Placed, machine items)
├── place_platform_system            ← reads WorldObjectEvent (Placed, "platform")
├── generator placement observer     ← reads WorldObjectEvent (Placed, "generator")
└── remove_placed_objects_system     ← reads WorldObjectEvent (Removed)

[Update — NetworkSystems::of::<N>() (after MachineScanSet)]
├── cable_placed_system              ← reads CableConnectionEvent
└── cable_removed_system             ← reads WorldObjectEvent (Removed, cable items)
```

`BuildingSet → MachineScanSet → NetworkSystems` is the canonical same-frame chain: place-then-cable in one click sequence is supported because the machine's ports exist by the time `cable_placed_system` runs.

---

## 13. Vertical Slice Scope

| Feature | VS | MVP |
|---|---|---|
| `PlaceableDef` RON registry (item-gen included) | ✓ | ✓ |
| Required-footprint collider cache (panic on missing) | ✓ | ✓ |
| Single-point placement (machines, generator, storage_crate) | ✓ | ✓ |
| TwoEndpoint placement (logistics_cable, power_cable) | ✓ | ✓ |
| AreaRect placement (platform) | ✓ | ✓ |
| `AxisY` continuous-Y rotation (`{kbd:rotate_cw}` / `{kbd:rotate_ccw}` / `{kbd:rotate_fine}`) | ✓ | ✓ |
| `Free` arbitrary rotation (`{kbd:rotate_free_drag}`) | — | ✓ |
| Sticky `BuildOrientation` across hotbar swap | ✓ | ✓ |
| Scene-instance ghost (single + tiled) | ✓ | ✓ |
| Routed cable ghost (real cable mesh, real routing) | — | ✓ |
| Ghost color: green/red by validation | — | ✓ |
| Removal hover via `RemovalHover` marker + tint shader | ✓ | ✓ |
| Resolved removal (machine / platform / cable) by raycast | ✓ | ✓ |
| `PlacementRejected` event + UI toast | — | ✓ |
| Telemetry of place/remove/reject | — | ✓ |
| Mirror via `Transform.scale` sign flip | — | — (post-VS) |
| Wall/ceiling placement | — | — (post-VS) |
| Area removal (`{kbd:place_extend_modifier}` + drag) | — | — (post-VS) |
| Module attachment via building system | — | — (post-VS; see machines.md §10) |
| Decorations | — | — (post-VS) |

**VS simplifications:**
- `OrientationSupport::Free` is authored but `{kbd:rotate_free_drag}` is MVP; VS treats `Free` placeables the same as `AxisY` (`{kbd:rotate_cw}` / `{kbd:rotate_ccw}` continuous Y only, no out-of-plane rotation).
- Validation result coloring is MVP; VS shows the ghost unconditionally and silently fails on click.
- `removal_tint_system` may be the simplest possible material swap in VS (override → flat red) and graduate to a shader parameter in MVP.

---

## 14. Edge Cases

| Case | Behavior |
|---|---|
| Hotbar item has no `PlaceableDef` (e.g. an ingot) | Ghost hidden. `{kbd:primary_action}` is a no-op (other input systems may consume — e.g. drone tool). |
| `PlaceableCacheReady = false` when player clicks | `placement_input_system` skips; emits `PlacementRejected { reason: FootprintBlocked }` (cache-not-ready folded into footprint to avoid a new variant). VS: log warning. |
| Hotbar swap mid two-click cable | `PendingPlacement` cleared; first port "forgotten." No event emitted. Anchor stock not yet decremented, so no refund needed. |
| Second cable click on the same port as the first | `anchor.distance(pos) > 0.1` check fails → `PlacementRejected { reason: FootprintBlocked }`. State stays `TwoEndpoint` so the player can retry the second click on a different port. |
| Platform second corner at far side of the world (huge rect) | Tile loop iterates; per-tile validation rejects tiles outside reach; stockout terminates early. No crash. |
| Player clicks machine while standing on it | `SpatialQueryFilter` excludes the player entity — only the surface raycast and footprint check matter. Placement succeeds if the placed footprint doesn't overlap the player capsule. |
| Player clicks while inventory open | Ghost hidden; click consumed by inventory UI, not building system. |
| Two `Placed` events for the same machine type in same frame (cheat / scripted test) | `place_machine_system` handles both in order; second one may fail footprint if the first overlaps. Building system emits both events; downstream is authoritative. |
| Place machine, then in the next click place a cable to its port | Same-frame ordering (BuildingSet → MachineScanSet → NetworkSystems): if both clicks happen in different frames, machine is spawned in frame 1, cable lands on its port in frame 2. Within one frame, place + cable cannot both happen — `{kbd:primary_action}` is a single edge event per frame. |
| `{kbd:place_extend_modifier}` + `{kbd:primary_action}` on the ground (no entity) | `LookTarget::Surface { entity, .. }` resolves to the ground entity, which is neither `Machine` nor `Platform` nor cable nor port — `PlacementRejected { NoTargetUnderCursor }`. |
| `BuildOrientation` was rotated, then player swaps to a `OrientationSupport::None` placeable | Resource preserved across the swap; ghost forces identity for this placeable; emitted event has `transform.rotation = Quat::IDENTITY`. |
| Swap from `Free` to `AxisY` mid-session | The held arbitrary Quat is projected onto the Y axis (yaw kept, pitch/roll dropped); reverse swap restores the previous `Free` Quat. |
| Cable ghost while no anchor set (`PendingPlacement::Idle`, cable item selected) | `GhostHint::Routed` falls back to a small port-snap indicator at look-target if the cursor is on a valid port; hidden otherwise. |
| `PlaceableDef.footprint` larger than the visual scene AABB | Footprint wins (validation uses it). Visual/collision mismatch is an authoring concern surfaced via the ghost (collider cuboid is rendered in dev builds). |
| Load-time: two `PlaceableDef` files declare the same `item.id` | Panic with both source paths — duplicate item ids are a build error. |
| Load-time: `PlaceableDef.footprint` is zero or NaN | Panic at `build_placeable_collider_cache` with the offending item id. No silent splat-fallback. |
| Load-time: a non-placeable `ItemDef` collides with a `PlaceableDef.item.id` | Panic — placeable RON is the source of truth for its item; declare the item only once. |
| Removed entity carried `RemovalHover` at despawn | Marker disappears with the entity automatically; `removal_tint_system` reverts no material since the entity is gone. Next frame, hover resolves a new target if `{kbd:place_extend_modifier}` is still held. |

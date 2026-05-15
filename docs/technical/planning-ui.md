# Planning UI Technical Design

Cross-factory production *planner*: design hypothetical production additions using a Sankey graph, per-node Inspector rail, Recipe Picker overlay, and 3D network topology overlay. The planner works on **planned** production — it does not read currently-placed machines or show live factory state. For live machine state and runtime alerts, see `machine-ui.md`.

Read `crafting.md §3` for `RecipeGraph` and `§5` for `SlotBlockReason`. Read `networks.md §2–3` for network topology data structures.

**Wireframe:** `ui_mock/planner-wireframes.html`. Screens 01 (Sankey + Inspector) and 05 (Recipe Picker) are the canonical layout references for this document.

**Design inspiration:**
- *Factorio* mod **Factory Planner** — production line planning with per-recipe module configuration, ratio math, and alt-recipe selection
- **Foreman2** (github.com/DanielKote/Foreman2) — node-based production graph with throughput-scaled connections, bottleneck highlighting, and recipe comparison

---

## Table of Contents

1. [Overview](#1-overview)
2. [Opening the Planner](#2-opening-the-planner)
3. [ECS Structure](#3-ecs-structure)
4. [Planner Chrome](#4-planner-chrome)
5. [Sankey Graph](#5-sankey-graph)
6. [Inspector Rail](#6-inspector-rail)
7. [Recipe Picker Overlay](#7-recipe-picker-overlay)
8. [3D Network Topology Overlay](#8-3d-network-topology-overlay)
9. [Systems](#9-systems)
10. [Messages](#10-messages)
11. [Execution Order](#11-execution-order)
12. [Vertical Slice Scope](#12-vertical-slice-scope)
13. [Edge Cases](#13-edge-cases)

---

## 1. Overview

The Planning UI is a production *planning* tool. The player specifies a goal item and target rate, and the planner shows what recipes, machine counts, and ingredient chains would be required to achieve that rate — as a design aid for factory additions. It does not read currently-placed machines; bottleneck indicators are ratio-math only (plan vs. goal), not live factory state.

Multiple named plans can be created, saved, and switched between within a run. Each plan is a separate entity with a `PlanState` component so all plans persist in save data.

**Layout (VS target):**
```
┌──────────────────────────────────────────────────────────────────┐
│ topbar: plan name · goal · units toggle · undo · save            │
├─────┬────────────────────────────────────────┬───────────────────┤
│left │                                        │                   │
│rail │          SANKEY GRAPH                  │   INSPECTOR       │
│     │       (ribbon width = rate)            │     (rail)        │
│     │                                        │                   │
├─────┴────────────────────────────────────────┴───────────────────┤
│ statusbar: machines · power · floors · plan issues hint          │
└──────────────────────────────────────────────────────────────────┘
```

The Sankey and Inspector share one selected node. Clicking a node in the Sankey updates the Inspector. The planner does not pause simulation.

The 3D network topology overlay is a separate, independent toggle — it renders over the 3D world view and can be active with or without the planner panel open.

**VS signals served:**
- §3.5 Recipe Graph and Planner UX — Sankey dep graph, machine count, ratio-based bottlenecks, recipe picker
- §3.6 3D factory readability — network topology overlay

---

## 2. Opening the Planner

Opened from the Field Computer terminal (interact → open planner) or a dedicated hotkey (default: Tab; configurable). Closes on: Escape key, close button, or Field Computer interact toggle. Not proximity-gated.

`PlannerOpen` resource tracks open state. Systems that rebuild planner content skip work when `!PlannerOpen.open`.

The planner UI hierarchy persists in the world with `Visibility::Hidden` while closed — no despawn/respawn on each toggle. Spawn/despawn is too expensive for a complex UI hierarchy that is opened and closed frequently; visibility toggle is the correct Bevy pattern for in-game menus.

Plan management (create, rename, duplicate, switch, delete) is accessible from the Field Computer terminal and from the topbar plan-name dropdown. VS scope: single plan per run (plan switching deferred to MVP).

---

## 3. ECS Structure

### 3.1 Resources

```rust
#[derive(Resource, Default)]
pub struct PlannerOpen {
    pub open: bool,
}

/// Tracks all plan entities. One plan is active at a time (shown in the planner).
#[derive(Resource, Default)]
pub struct PlanList {
    pub plans: Vec<Entity>,
    pub active: Option<Entity>,
}

// ── inspector ───────────────────────────────────────────────────────
#[derive(Resource, Default)]
pub struct InspectorState {
    pub selected: Option<ItemId>,
}

// ── recipe picker ───────────────────────────────────────────────────
#[derive(Resource, Default)]
pub struct RecipePickerState {
    pub open: bool,
    pub node: Option<ItemId>,
    pub search: String,
    pub filter_unlocked: bool,
    pub selected_alt: Option<RecipeId>,
}

// ── topology ─────────────────────────────────────────────────────────
#[derive(Resource, Default)]
pub struct TopologyOverlay {
    pub enabled: bool,
    pub filter: NetworkFilter,
}

#[derive(Default, Clone, Copy)]
pub struct NetworkFilter {
    pub logistics: bool,
    pub power: bool,
    pub drone: bool,
    pub research: bool,
}
```

### 3.2 Components

Plan data lives on plan entities so it can be saved and multiple plans can coexist:

```rust
/// One per plan entity. Saved to disk via moonshine-save.
#[derive(Component)]
pub struct PlanState {
    pub target: ItemId,
    pub target_rate: f32,                  // items/s; internal always /s
    pub rate_unit: RateUnit,               // /s | /min (display conversion only)
    pub nodes: Vec<DepNode>,
    pub edges: Vec<(usize, usize)>,        // parent_idx → child_idx
    pub dirty: bool,
    // Player overrides — persist across dirty rebuilds
    pub locked_counts: HashMap<ItemId, u32>,
    pub alt_recipes: HashMap<ItemId, RecipeId>,
}

#[derive(Component)]
pub struct PlanName(pub String);

#[derive(Clone, Copy, Default, PartialEq)]
pub enum RateUnit { #[default] PerSecond, PerMinute }

#[derive(Clone)]
pub struct DepNode {
    pub item: ItemId,
    pub recipe: Option<RecipeId>,   // None = raw material leaf
    pub required_rate: f32,         // items/s at current target_rate
    pub machine_count: u32,         // 0 for leaves; locked if in PlanState::locked_counts
    pub column: u32,                // 0 = leftmost (raw materials); max = rightmost (goal)
}
```

The planner UI root:

```rust
#[derive(Component)]
struct PlannerRoot;
```

`PlannerRoot` is spawned once at startup. `planner_open_system` toggles `Visibility::Visible` / `Visibility::Hidden` on it — no despawn on close.

### 3.3 Components Read

| System | Reads |
|--------|-------|
| `dep_graph_build_system` | `PlanState` (active plan), `RecipeGraph`, `TechTreeProgress` |
| `inspector_system` | `PlanState` (active plan), `InspectorState`, `RecipeGraph` |
| `topology_overlay_system` | Network resources (`networks.md §2`), `Transform` + `RecipeProcessor` + `SlotBlockReason` of machines |

### 3.4 Resources / Components Written

| System | Writes |
|--------|--------|
| `planner_open_system` | `PlannerOpen`; sets `Visibility` on `PlannerRoot` |
| `plan_management_system` | Spawns/despawns plan entities; updates `PlanList` |
| `dep_graph_build_system` | `PlanState::nodes/edges/dirty` |
| `apply_alt_recipe_system` | `PlanState::alt_recipes`; `dirty = true` |
| `lock_machine_count_system` | `PlanState::locked_counts`; `dirty = true` |

---

## 4. Planner Chrome

### 4.1 Topbar

```
PLANNER · [plan name ▾]  goal: 60.0/s ferro-laminate
                          [/s][/min]  [↶ undo] [save plan]  [balance ⌘B 🔒]
```

- Plan name dropdown: shows all `PlanList` entries; select to switch active plan; [+] creates new plan (MVP; VS shows plan name read-only with no switching)
- Goal label: `target_rate target_item_name`; target rate is editable via Inspector on goal node
- Rate unit toggle: `/s` | `/min` — display only; internal state stays `/s`
- `save plan`: persist `PlanState` to run save (MVP; stub in VS)
- `↶ undo`: revert last alt-recipe or machine-count change (MVP; stub in VS)
- `balance ⌘B`: auto-solve all non-locked machine counts to satisfy target rate — **progression-locked** (requires unlocking auto-balancer in tech tree); shown greyed with lock icon until unlocked; deferred to MVP

**VS scope:** Plan name display (read-only), goal label, unit toggle. Undo/save/balance buttons present but non-functional.

### 4.2 Left Rail

Vertical icon strip. Icons from wireframe (`PlannerLeftRail`):

| Icon | Label | Action |
|------|-------|--------|
| ⌖ | goal | Scroll Sankey to goal node; VS |
| ▦ | recipes | Open recipe browser / picker (§7); VS |
| ⚙ | machines | Show total machine counts list; MVP |
| ◴ | power | Show power summary panel; MVP |
| ≣ | floors | Multi-floor view; MVP |
| ↗ | export | Export plan to clipboard (RON or text); MVP |

### 4.3 Statusbar

```
[32 machines] [980 kW] [2 floors] [3 plan issues ⚠]         click sankey node = inspect · ⌘B balance · ⌘L lock
```

- Machine count: sum of all `DepNode::machine_count` in active plan
- Power: estimated from `machine_count × recipe.energy_cost / recipe.processing_time` (kW) — plan-level estimate, not live draw
- Floors: always 1 in VS
- Plan issues badge: count of nodes where `required_rate > throughput_at_machine_count` (under-planned by ratio math); amber; click → scrolls Sankey to first issue node
- Keyboard hint strip (right side)

**VS scope:** Machine count and plan issues badge functional. Power approximated (static per recipe). Floors reads "1".

---

## 5. Sankey Graph

The main canvas. Fills the space between the left rail and Inspector. Displays active `PlanState` as a Sankey diagram: nodes arranged in columns by production stage, connected by ribbons whose width encodes throughput.

### 5.1 Build Algorithm

`dep_graph_build_system` runs when active plan's `PlanState::dirty == true`.

```rust
fn build_dep_graph(
    target: ItemId,
    target_rate: f32,
    graph: &RecipeGraph,
    progress: &TechTreeProgress,
    locked_counts: &HashMap<ItemId, u32>,
    alt_recipes: &HashMap<ItemId, RecipeId>,
) -> (Vec<DepNode>, Vec<(usize, usize)>)
```

1. Initialize `nodes = []`, `edges = []`, `visited: HashSet<ItemId> = {}`.
2. Push root: `DepNode { item: target, recipe: primary_recipe(target), required_rate: target_rate, machine_count: count_for(target, target_rate), column: 0 }`. Column 0 = rightmost in display (renormalized in step 5).
3. For each node (BFS, queue order = column order):
   - If `node.recipe.is_none()` or `node.item ∈ visited`: skip recursion.
   - Mark `node.item` visited.
   - For each `input` in `recipe.inputs` where `consumed == true`:
     - `child_rate = node.required_rate × (input.quantity / primary_output_qty(recipe))`
     - Push `DepNode { item: input.item, recipe: primary_recipe(input.item), required_rate: child_rate, machine_count: count_for(input.item, child_rate), column: node.column + 1 }`.
     - Push edge `(child_idx, parent_idx)` (child feeds parent).
     - Recurse.
4. Set `PlanState::dirty = false`.
5. **Column flip:** `max_col = nodes.iter().map(|n| n.column).max()`. Each node's display column = `max_col - n.column`. Raw materials (leaves) → column 0 (leftmost). Goal → column `max_col` (rightmost). Production flows left → right.
6. **Apply locked counts:** For each node where `locked_counts[node.item]` exists, override `machine_count`.

**`primary_recipe(item)`:** Check `alt_recipes[item]` first. Fall back to first in `RecipeGraph.by_output[item]` filtered by `TechTreeProgress.unlocked_recipes`. Returns `None` if empty.

**`primary_output_qty(recipe)`:** `recipe.outputs[0].quantity` (primary product per `technical-design.md §2`).

**`count_for(item, rate)`:** If `primary_recipe(item).is_none()`: `0`. Else: `(rate / throughput).ceil() as u32` where `throughput = primary_output_qty / recipe.processing_time` (items/s).

Rebuild triggers: `SetDepGraphTarget`, `PlanState::target_rate` change, `ApplyAltRecipe`, `LockMachineCount`.

### 5.2 Column Layout

Nodes in the same column are stacked vertically, evenly spaced. Column x-positions are evenly distributed across the canvas width. Node card height scales with `max(min_height, rate × k)` for a tunable constant `k` — card height proportional to throughput per Sankey convention.

### 5.3 Ribbons

Each edge `(child_idx, parent_idx)` produces a ribbon:
- Bezier cubic from right edge of child node to left edge of parent node
- Ribbon width encodes throughput linearly (or sqrt for wide range; configurable tweak)
- Rate label centered on ribbon when ribbon is wide enough to fit
- Color:
  - Normal: hatched ink pattern
  - Under-planned (child supply < parent demand at current machine counts): red hatch
  - Selected (either endpoint is selected node): gold/amber hatch

Rendering approach: custom 2D mesh or Bevy UI custom painter. Exact API depends on Bevy 0.18 path rendering support — may require `bevy_egui` custom painter or a dedicated Camera2d mesh pass. Wireframe SVG paths (`planner.jsx Sankey`) are the reference for path math.

### 5.4 Node Cards

Each node renders as a rect on the Sankey canvas:
- Item label (top-left, bold)
- Machine type (small, below label) — omitted in compact/dense mode
- `×N` machine count (top-right, bold) — orange if locked
- Rate (bottom-right, small) — red if under-planned
- Utilization bar at bottom (when card height allows): fill = `required_rate / throughput_at_count`
- Border: ink; gold dashed if selected; red if under-planned; double-weight if goal node

Click → `SelectSankeyNode(item_id)`.

---

## 6. Inspector Rail

Right panel. Shows details for `InspectorState::selected`. Empty state: "Click a node to inspect." Chrome header from wireframe `InspectorHeader`.

### 6.1 Header

- Item icon + name
- Machine type name
- Status badge: GOAL (goal node) | UNDER-PLANNED (supply < demand at current count, ratio math only) | OK

### 6.2 Recipe Section

```
RECIPE                                          [swap (N alts)]
  [input_icon] rate/s  [input_icon] rate/s  →  [output_icon] rate/s
  base cycle 4.0s · effective N.N/s per unit
```

- Inputs from `recipe.inputs` where `consumed == true`; catalyst inputs shown with "(catalyst)" label, not counted in flow
- "swap (N alts)" button: `N = RecipeGraph.by_output[item].len() - 1` (alt count excluding current); fires `OpenRecipePicker { node: item_id }`; disabled if N = 0

### 6.3 Throughput Section

```
THROUGHPUT
target   [60.0]  [/s ▾]
machines [5    ]  auto-solved · ⌘L to lock
```

- Target input: edits `PlanState::target_rate` (goal node only). Non-goal nodes show derived rate read-only.
- Rate unit selector: display only
- Machine count: editable text field. On change: fires `LockMachineCount { node, count }`. Lock icon shown when `locked_counts` contains this node.
- "auto-solved · ⌘L to lock" hint — replaced by lock icon when locked

**Under-planned alert (shown when required_rate > throughput_at_count — ratio math only, not live state):**
```
⚠ supply 4/s, demand 6/s · short 2/s
[add ×1 machine]
```
"add ×1 machine" fires `LockMachineCount { node, count: current + 1 }`.

### 6.4 Modules Section (MVP)

Present in VS but non-interactive: "— no modules (MVP)" placeholder. MVP: shows actual module slots and sweep suggestion per wireframe `InspectorBody` Modules section.

### 6.5 Power & Footprint Section

```
POWER & FOOTPRINT
draw        N kW    (machine_count × recipe.energy_cost / recipe.processing_time)
footprint   N × N tiles   (machine_count × machine_footprint from MachineDef)
```

Power is estimated from recipe data. Footprint from `MachineDef` (MVP: placeholder tile count).

### 6.6 Footer Actions (MVP)

Buttons: `duplicate` | `isolate path` | `to sub-floor` | `delete`. All MVP; stubs in VS.

---

## 7. Recipe Picker Overlay

Modal overlay invoked from Inspector "swap (N alts)" button. Covers the Sankey canvas. Three-column layout per wireframe `PlannerRecipePicker`.

**Left column:** Text search (`/ search recipes…`), category filter list (all, unlocked, by process type), filter checkboxes (unlocked only, fluid recipes, consider modules).

**Center column:** Recipe list. Each entry:
- Radio button + recipe name + tier badge + lock indicator
- Input icons → output icon + output rate
- "use →" button
- Locked recipes dimmed; tier badge shown

**Right column:** Compare panel. At `target_rate` (in /s):
- Current recipe: machines needed, raw input rates, power, pollution
- Selected alt: same fields
- Delta summary: "switching saves ×N machines & flux line, costs +N% power"

Footer: `[apply selected]` | `[cancel]`. Press ↵ to apply selected. Drag from center to canvas: deferred (MVP).

**Apply:** fires `ApplyAltRecipe { node: ItemId, recipe: RecipeId }` → `PlanState::alt_recipes[node] = recipe`, `dirty = true`.

**VS scope:** Full picker UI. Drag-to-canvas deferred (MVP). "consider modules" checkbox present but non-functional in VS.

---

## 8. 3D Network Topology Overlay

Toggled independently of the planner panel (default hotkey: `N`). Draws over the 3D world each frame using Bevy `Gizmos`. `topology_overlay_system` early-exits when `!TopologyOverlay::enabled`.

**Why Gizmos (not entities):** Gizmos are batched single-frame draw calls — no spawn/despawn overhead, no per-frame query cost for overlay elements. Appropriate here because topology lines need no interaction (no picking). Tradeoff: cannot be clicked or hovered.

### 8.1 Network Filter

Showing all networks simultaneously is visually cluttered. `TopologyOverlay::filter` controls which network types are drawn. A compact HUD bar appears at screen edge when the overlay is enabled:

```
[TOPO] Logistics [✓] Power [✓] Drone [ ] Research [ ]
```

Default on first toggle: Logistics only. Each enabled network draws in its own color; multiple can be read simultaneously when contrast is sufficient. Exact RGBA values are designer-tuned constants — not hardcoded in design doc.

### 8.2 Cable Segments

Lines along each cable segment in enabled network types (data source: `networks.md §2`):

| Network | State | Color |
|---------|-------|-------|
| Logistics | Connected | green |
| Logistics | No machine | grey |
| Power | Connected | amber |
| Power | No machine | grey |
| Drone | Active | blue |
| Research | Active | purple |

### 8.3 Machine Nodes

Sphere gizmo at each machine center (radius in world units; TBD by art/feel):

| State | Color |
|-------|-------|
| All slots `Running` | Green |
| Any slot `PowerPaused` | Dark amber |
| Any slot has `BlockReason` | Red |
| All slots `Idle` | Yellow |

Blocked machines also draw a pulsing ring gizmo (alpha 0.3–0.9 at 1 Hz).

### 8.4 Port Connections

Short line from machine center toward each cable port connection point.

### 8.5 Distance Culling

Draw only elements within a configurable radius of the player entity's `Transform`. Keeps per-frame gizmo call count bounded at VS scale. Network filter further reduces draw count when viewing only one network type.

---

## 9. Systems

| System | Trigger | Action |
|--------|---------|--------|
| `planner_open_system` | `OpenPlanner` / `ClosePlanner` | Set `Visibility` on `PlannerRoot`; set `PlannerOpen` |
| `plan_management_system` | `CreatePlan` / `SwitchPlan` / `DeletePlan` | Spawn/despawn plan entities; update `PlanList` |
| `dep_graph_target_system` | `SetDepGraphTarget` | Set `PlanState::target`; `dirty = true` |
| `dep_graph_build_system` | Each frame when active plan's `dirty == true` | Run build algorithm; `dirty = false` |
| `sankey_render_system` | Each frame when open | Render `PlanState` as Sankey on canvas |
| `inspector_system` | Each frame when open | Render Inspector for `InspectorState::selected` |
| `select_sankey_node_system` | `SelectSankeyNode` | Set `InspectorState::selected` |
| `apply_alt_recipe_system` | `ApplyAltRecipe` | Update `PlanState::alt_recipes`; `dirty = true` |
| `lock_machine_count_system` | `LockMachineCount` | Update `PlanState::locked_counts`; `dirty = true` |
| `recipe_picker_open_system` | `OpenRecipePicker` | Set `RecipePickerState::open/node` |
| `recipe_picker_close_system` | `CloseRecipePicker` / Escape | Set `RecipePickerState::open = false` |
| `topology_overlay_system` | Each frame | Early-exit if not enabled; draw gizmos for enabled filter networks |

---

## 10. Messages

```rust
#[derive(Event)]
pub struct OpenPlanner;

#[derive(Event)]
pub struct ClosePlanner;

#[derive(Event)]
pub struct CreatePlan { pub name: String }

#[derive(Event)]
pub struct SwitchPlan { pub entity: Entity }

#[derive(Event)]
pub struct DeletePlan { pub entity: Entity }

// Fired at run start (escape-condition system) to initialize dep graph target
#[derive(Event)]
pub struct SetDepGraphTarget(pub ItemId);

#[derive(Event)]
pub struct SelectSankeyNode(pub ItemId);

#[derive(Event)]
pub struct OpenRecipePicker { pub node: ItemId }

#[derive(Event)]
pub struct CloseRecipePicker;

#[derive(Event)]
pub struct ApplyAltRecipe { pub node: ItemId, pub recipe: RecipeId }

#[derive(Event)]
pub struct LockMachineCount { pub node: ItemId, pub count: u32 }

#[derive(Event)]
pub struct ToggleTopologyOverlay;

#[derive(Event)]
pub struct SetTopologyFilter(pub NetworkFilter);
```

---

## 11. Execution Order

```
[recipe_processor_system]
[slot_block_reason_system]         ← machine state (read by topology overlay only)
    → [dep_graph_build_system]     ← when active plan dirty
    → [sankey_render_system]
    → [inspector_system]
    → [topology_overlay_system]
```

All planner render systems early-exit when `!PlannerOpen.open`, except `topology_overlay_system` (early-exits when `!TopologyOverlay::enabled`).

---

## 12. Vertical Slice Scope

### Required for VS

| Feature | Section |
|---------|---------|
| Sankey graph: columns, ribbons (width = rate), node cards (count, rate, under-planned states) | §5 |
| Click node → Inspector; recipe display, derived rate, machine count | §6.1–6.3 |
| Under-planned inline alert + "add ×1 machine" (ratio math only) | §6.3 |
| Target rate input on goal node; `LockMachineCount` | §6.3 |
| Recipe Picker overlay: recipe list, compare panel, apply | §7 |
| Topology overlay: cable lines, machine spheres, blocked pulse, network filter HUD | §8 |
| Statusbar: machine count + plan issues badge functional | §4.3 |
| Left rail: goal, recipes functional | §4.2 |
| Rate unit toggle (/s \| /min) | §4.1 |
| Single plan per run (no plan management UI in VS) | §3 |

### Deferred to MVP

- `balance ⌘B` auto-solve (progression-locked; topbar stub in VS)
- `save plan` / `undo` (stubs in VS)
- Multiple plans: create, rename, switch, delete (VS: single plan)
- Inspector modules section (placeholder text in VS)
- Inspector footer actions (duplicate, isolate, sub-floor, delete)
- Left rail: machines, power, floors, export panels
- Multi-floor view
- Drag recipe from picker to canvas
- "consider modules" filter in picker

### Integration Test Targets

1. Given `RecipeGraph` with 3-node chain (A → B → C, C = `terminal_item`), `dep_graph_build_system` at `target_rate = 1.0/s` produces 3 `DepNode` entries with correct `required_rate`, `machine_count`, and `column` assignments (C = max_col, A = col 0).
2. `ApplyAltRecipe { node: B, recipe: alt_id }` sets `PlanState::alt_recipes[B] = alt_id` and `dirty = true`; next build uses `alt_id` for B's recipe.
3. `LockMachineCount { node: B, count: 7 }` sets `locked_counts[B] = 7`; next build sets `DepNode[B].machine_count = 7` regardless of ratio math.
4. `SelectSankeyNode(item)` sets `InspectorState::selected = Some(item)`.
5. `ToggleTopologyOverlay` flips `TopologyOverlay::enabled`; gizmos drawn iff enabled.

---

## 13. Edge Cases

- **No recipe produces target item:** Sankey shows single leaf node with "No recipe" badge. Inspector: "No recipe available." Normal during early run.
- **Multiple unlocked recipes for one item:** `primary_recipe` picks first unless `alt_recipes` overrides. Inspector shows "swap (N-1 alts)".
- **`target_rate == 0.0`:** All machine counts 0. Summary: "Set a target rate > 0." Guard: skip throughput calc when rate == 0.
- **Cycle in recipe graph:** `technical-design.md §2` guarantees DAG. Cycle detection via `visited` is defensive. If triggered: stop recursion, mark node "Cycle detected" badge. Do not crash.
- **Player locks count higher than needed:** Gold lock in Sankey. No error — intentional buffer.
- **Player locks count lower than needed:** Node is under-planned. Inline alert shows gap; "add ×1 machine" proposes fix.
- **Recipe Picker open when planner closes (Escape):** Close picker first (`RecipePickerState::open = false`), then close planner. Escape priority: picker close > planner close.
- **`alt_recipes` references recipe no longer unlocked** (future mechanic — tech rollback): `primary_recipe` treats as invalid, falls back to index-first. Clear from `alt_recipes`. Log warning.
- **All network types enabled in topology overlay, large factory:** Distance culling keeps gizmo count bounded. Network filter lets player reduce visual clutter.
- **Topology overlay with zero placed machines:** No spheres or port gizmos; system runs without error.

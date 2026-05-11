# User Interface

## Screens & Modes

### Terminal (Inventory / Item Network)
Menu overlay — opened on demand, not always-on HUD.

Tabs switch between storage networks (main network + subnets). Item table shows qty, Δ/min, kg/ea, kg total, craftable flag. Left sidebar: TODO/goal tracker with target-qty progress bars, quick-access slots, saved filters.

Header strip: mass capacity bar + unique cells bar + power bar for the active network.

**CRAFT button flow:** Opens a qty-input modal → resolves a read-only execution plan from current network config + machine priorities (no decisions in modal) → confirm to enqueue / cancel. Modal footer notes "based on current machine priorities" so players know where to go if the plan looks wrong. Two-phase: (1) qty input + RESOLVE button, (2) dependency tree + machine assignments + ENQUEUE. If inputs are missing, ENQUEUE still available with "will stall" warning.

See: `ui_mock/craft-modal.jsx`

See: `ui_mock/integrated.jsx`

---

### Index (Item Research / Recipe Browser)
Three-pane layout: item list (recent + search) / recipe focus / uses panel.

Recipe focus shows all inputs/outputs with qty, rate, notes (byproduct %, fluid), machine, tier, time, power, yield. Multiple recipes for same item paginated (`RECIPE 1 / 3`). Tabs: RECIPE · USES · CODEX.

**CODEX tab** = cross-run persistent knowledge accumulator. Entries sourced from prior run discoveries. Should feel distinct from other tabs (different tint, provenance label: "discovered on [world] · run #N").

**Actions from index:**
- `▶ auto-craft` → opens CRAFT modal (see Terminal)
- `⊞ open in planner` → opens Factory Planner with this item pre-selected (dismissible banner shown, matching node highlighted in inspector)
- `★ favorite`

Index is closely related to the Factory Planner — it is the item research surface; planner is the machine layout surface.

See: `ui_mock/lookup.jsx`

---

### Factory Planner
Sankey diagram (left) + node inspector (right). Sankey ribbon width = items/sec; red hatch = bottleneck.

Clicking a sankey node opens the inspector: recipe (with "swap — N alts" button), throughput target + machine count (auto-solved, lockable), modules slots.

**Planner scope:** machine layout and module optimization. Not for recipe resolution — that happens in the CRAFT modal. Alternative recipes *do* appear here (via recipe swap in inspector).

**Left rail views:** goal · recipes · machines · power · floors · find · export.

**Recipe Picker overlay** (invoked from inspector "swap" button): filters by category + locked status, shows tier badge, live comparison panel (machines / raw inputs / power / pollution diff). Drag recipe to canvas or press ↵ to apply.

**Beacons: not an Exergon concept.** Removed from mocks. Do not reintroduce.

See: `ui_mock/planner.jsx`

---

### Tech Tree
Tier-paged questbook. Each tier is its own page (tabs T0–T4+). Functions as the primary escape progress indicator — tier gates are the win-condition milestones.

**Fog system — 3 knowledge tiers:**
- T1 known to exist: silhouette only, no params
- T2 partial: shape + tag visible, name redacted, stats shown as ranges (`~7–17/s`)
- T3 fully revealed: exact recipe, all params, buildable

**Layout:** swim lanes by research line (smelt / refine / chem / electric / logic / power), each with its own color. Milestone bridge cards appear on both adjacent tier pages (exit card of tier N = entry card of tier N+1).

**Cross-tier stubs:** nodes depending on prior-tier nodes show colored port stubs at the left margin with source label; click to jump pages.

**Right rail inspector** (selected node): tag, partial/full inputs+outputs, flavour text, cross-tier incoming/outgoing list.

**Reveal panel** (opened from any node): knowledge ladder (T1→T2→T3 cost steps), before/after diff, prereq chain (clickable — click any node to focus and reveal that instead), "also unlocked when revealed" side-effects, wishlist action.

**Research currency** displayed in topbar (`128 R · frontier · exergon core`).

See: `ui_mock/tech-tree-v6.jsx`, `ui_mock/tech-tree-wireframes.html`

---

### Machine UI
Side-rail terminal. Left rail: machine identity (type, tier, ID), current craft progress bar, power draw (with peak/idle/grid status), module slots, port binding. Right pane: recipe table sorted by priority.

**Recipe table columns:** recipe name, inputs → outputs, cycle rate, mode flags (C = autocraft, P = passive), priority integer, passive limit.

**Mode flags:** C and P are independent. Both, either, or neither. Passive (P) only fills to the configured limit. Priority integer controls autocraft tie-breaking.

**Port binding:** named nets assigned per item slot, with flow controls (+/-/%). Net names correspond to terminal network tabs.

See: `ui_mock/machine.jsx`

---

### In-World HUD
Minimal always-on overlay, visible during 3D world navigation. Three zones:

**Top bar** — menu shortcut buttons (T=Terminal, I=Index, P=Planner, Y=Tech Tree) · research pool widget · alerts button with dropdown.

**Research pool widget** (top bar, right of menus): shows all 4 research types (material_science, field_research, engineering, discovery) with current balance. Zero-balance types dimmed. Matches `ResearchPool` resource in `research.md §3`.

**Alerts dropdown** (top bar, far right): machine errors and warnings. Each entry shows icon + machine ID + reason text. Click machine name to jump to machine UI.

**Bottom bar** — player vitals (HP / O₂ / SAT bars, left) · hotbar (centre, 9 slots, 3 banks A/B/C, shift+scroll to switch) · XP level + bar (right).

No subnet, no inventory, no minimap. Overlays are keyboard-triggered from top bar buttons.

See: `ui_mock/hud.jsx`

---

### Autocraft CPU Monitor
htop-style process list. Shows CPU clusters (named α/β/γ/δ), utilization bars, active job per CPU. Process table: pid, cpu, item, count, completion %, ETA, power draw, status.

Status codes: run / sub (subprocess / dependency) / wait (queued) / ERR (blocked). ERR status shows `ERR ⓘ` with a **hover tooltip** giving the specific reason (missing input, locked recipe, machine offline, etc.).

Subprocess tree shown via indented `└` entries — reflects the dependency resolution of the craft job.

See: `ui_mock/autocraft.jsx`

---

## Surface Relationships

```
In-World HUD (always-on)
  └── T → Terminal (item overview)
  └── I → Index (item research)
  └── P → Factory Planner (machine layout)
  └── Y → Tech Tree (unlock + escape progress)
  └── alerts dropdown → jump to machine UI

Terminal (item overview)
  └── CRAFT button → CRAFT modal (qty → plan → enqueue)
  └── INDEX button → Index
  └── research pool strip (read-only, spend in Tech Tree)

CRAFT modal
  └── phase 1: qty input → RESOLVE PLAN
  └── phase 2: dependency tree + machine plan → ENQUEUE / BACK

Index (item research)
  └── auto-craft → CRAFT modal
  └── open in planner → Factory Planner (item pre-selected, dismissible banner)

Factory Planner (machine layout)
  └── inspector "swap" → Recipe Picker overlay

Tech Tree (unlock + escape progress)
  └── node click → Reveal Panel
  └── prereq chain links → jump to other nodes

Machine UI (per-machine config)
  └── ports reference terminal network tab names
```

## Decisions

- **Research currency in Terminal** — yes. Research pool strip added below capacity bars in Terminal, showing all 4 typed buckets. Zero-balance types dimmed. Also shown compact in HUD top bar.
- **Autocraft ERR detail** — hover tooltip (`ERR ⓘ`, `title` attribute gives specific reason).
- **Subnet creation** — no UI. Subnets are formed automatically by physical cable/connection topology. The Terminal shows subnets by name once they exist.

---

## Mockup Images

### Palette

![](images/palette.png)

---

### In-World HUD

Full wireframe (both local and remote variants):

![](images/hud_mock.png)

**Top bar** — menus, research pool, alerts button:

![](images/hud_topbar.png)

**Alerts dropdown** — machine errors with jump-to links:

![](images/hud_alerts.png)

**Bottom bar** — vitals, hotbar (3 banks), XP:

![](images/hud_bottombar.png)

---

### Terminal

Full wireframe:

![](images/terminal_mock.png)

**Header** — mass/cell/power capacity bars + research pool strip:

![](images/terminal_header.png)

**Item table + sidebar** — item list, Δ/min, filters, goal tracker:

![](images/terminal_item_table.png)

---

### Craft Modal

**Phase 1** — qty input + RESOLVE:

![](images/craft_modal_phase1.png)

**Phase 2** — dependency tree + machine plan + ENQUEUE:

![](images/craft_modal_phase2.png)

---

### Index (Item & Recipe Lookup)

Full wireframe:

![](images/lookup_mock.png)

**Recipe focus pane** — inputs/outputs, pagination, CODEX tab:

![](images/lookup_recipe_pane.png)

---

### Autocraft CPU Monitor

![](images/autocraft_mock.png)

---

### Factory Planner

Full wireframe:

![](images/planner_mock.png)

**Main layout** — sankey + inspector side-by-side:

![](images/planner_main.png)

**Sankey pane** — ribbon diagram with bottleneck highlighting and left rail:

![](images/planner_sankey.png)

**Node inspector** — recipe swap, throughput target, machine count, module slots:

![](images/planner_inspector.png)

**Recipe picker overlay** — alt recipes, tier filter, live comparison:

![](images/planner_recipe_picker.png)

---

### Tech Tree

Full wireframe:

![](images/tech_tree_mock.png)

**Tier page** — questbook layout with swim lanes and cross-tier stubs:

![](images/tech_tree_tier_page.png)

**Top bar** — research currency, tier tabs, wishlist/filter:

![](images/tech_tree_topbar.png)

**Node canvas** — swim-lane rows, fog states, milestone bridges:

![](images/tech_tree_canvas.png)

**Reveal panel** — knowledge ladder, prereq chain, before/after diff:

![](images/tech_tree_reveal_panel.png)

---

### Machine UI

Full wireframe:

![](images/machine_mock.png)

**Left rail** — identity, current craft, power draw, module slots, port bindings:

![](images/machine_left_rail.png)

**Recipe table pane** — priority sort, bulk edit, mode flags (C/P):

![](images/machine_recipe_table.png)

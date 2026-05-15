# Tech Tree UI Technical Design

> Content design (tier themes, node pool, unlock rates, rarity): [`tech-tree-design.md`](../tech-tree-design.md).
> Data model and runtime architecture (node fields, unlock vectors, tier gates, validity invariants): [`technical-design.md §3`](technical-design.md#3-tech-tree).
> This document covers the UI implementation surface only.

Wireframe: `ui_mock/tech-tree-wireframes.html`

---

## 1. Overview

The tech tree overlay is a **tier-paged questbook**: one page per tier (T1–T10), each with its own tab. Within a page, nodes are laid out spatially by prerequisite depth (BFS) and grouped loosely by research-line category. Milestone nodes bridge adjacent pages. Cross-tier prerequisite edges become port stubs at the page margins.

Three concerns this doc decides:

1. The five node visual states and what each renders
2. The reveal interaction flow (T1→T2→T3)
3. The exclusive-group choice surface (**resolves `tech-tree-design.md` issue #9**)

---

## 2. ECS Components

### `TechTreePanelState` (Resource)

```rust
pub struct TechTreePanelState {
    pub open: bool,
    pub active_page: u8,                          // 1–10
    pub selected_node: Option<Entity>,
    pub reveal_overlay: RevealOverlayState,
    pub choice_overlay: Option<ExclusiveGroupId>,  // surfaced choice group, if any
}

pub enum RevealOverlayState {
    Closed,
    Open(Entity),  // node being revealed
}
```

The panel reads `TechTreeNode`, `NodeKnowledge`, `NodeUnlockState`, and `ExclusiveGroup` components from entities in the run's tech tree. It writes nothing to those components directly — it emits `RevealRequested` and `ChoiceCommitted` events which the research system processes.

### `TechTreeNode` (Component — canonical definition: [`tech-tree-design.md §5`](../tech-tree-design.md#5-node-pool-design))

Carries: `id`, `category`, `tier`, `rarity`, `effects`, `primary_vector`, `alt_vectors`, `primary_prereq`, `alt_prereqs`, `optional`, `exclusive_group`.

### `NodeKnowledge` (existing)

```rust
pub enum NodeKnowledge {
    Shadow,   // T1: known to exist; category/tier/rarity only
    Partial,  // T2: name visible; ranges for inputs/output/machine class
    Revealed, // T3: exact recipe, all params, buildable
}
```

### `NodeUnlockState` (existing)

```rust
pub enum NodeUnlockState {
    Locked,       // prerequisites not yet satisfied
    Unlockable,   // prerequisites met; can spend to advance knowledge
    LockedOut,    // exclusive group: another member was chosen
}
```

---

## 3. Node Visual States

Five distinct display states. Every node on a page is in exactly one.

| State | `NodeKnowledge` | `NodeUnlockState` | Visual |
|---|---|---|---|
| **Gate-locked** | any | any | Tab dimmed; page not accessible |
| **Shadow** | `Shadow` | `Locked` | Hatched card, dashed border, category tag visible, name/glyph silhouette |
| **Shadow-Unlockable** | `Shadow` | `Unlockable` | Same as Shadow + amber border accent |
| **Partial** | `Partial` | `Locked` or `Unlockable` | Solid card, name visible, stats shown as ranges (`~7–17/s`), reveal button in inspector |
| **Revealed** | `Revealed` | `Locked` or `Unlockable` | Solid card, thicker border, glyph + name, full stats |
| **Locked-Out** | any | `LockedOut` | Dim card, strike-through glyph, faded category tag, no reveal action |

Research-line color stripe: every node card that has `knowledge >= Partial` displays a 4px left border in the category's color. Shadow cards have no color border.

Wishlist star: nodes in the player's wishlist render a `★` badge (top-right corner of card) at any knowledge level.

Milestone nodes (tier gate nodes): accent background (`var(--accent)`) when `knowledge >= Partial`. Dashed accent when `Shadow`.

---

## 4. Layout

### Tier tabs

Top strip. One tab per tier in the run's difficulty (Initiation: T1–T3; Standard: T1–T5; etc.).

Each tab:
- Label: `T{n} · {TIER_NAME}` (e.g. `T2 · ROOTS`)
- Subtext: `{revealed}/{total} nodes` (counts `knowledge == Revealed` vs. total nodes on page)
- Locked indicator: if the tier gate is not yet cleared, tab opacity 55% + 🔒 icon
- Active tab: bottom accent bar, solid background

Navigation: keyboard `←`/`→` between tabs; clicking a locked tab is allowed (shows the page in read-only mode — all nodes render as shadow or revealed, but the tier banner notes the gate condition).

### Canvas layout per page

BFS topological X placement:
- Source nodes (no in-page predecessors) → column 0
- Each node placed at `max(predecessor_depth) + 1`
- Nodes at same depth sorted by category tag order: extract → smelt → process → power → logistics → science → explore → fab

Y placement:
- Distributed evenly across available height by position within depth bucket
- Single-node depth: centred vertically

Gate cards (bridge nodes):
- **Entry gate card** (left gutter): the milestone node that gates THIS tier. Absent for T1.
- **Exit gate card** (right gutter): the milestone node that gates the NEXT tier. Absent for the run's terminal tier.
- Both cards render the same `TechTreeNode` entity — same click, same inspector.
- "↔ also on T{N}" label below each gate card.

Cross-tier port stubs (non-gate cross-tier edges):
- **Incoming** (left margin): nodes on this page that have prerequisites on a previous page. Dashed colored line from margin dot to destination node. Label: `← T{n} {source_name}`.
- **Outgoing** (right margin): nodes on this page that are prerequisites for nodes on a future page. Dashed colored line from source node to margin dot. Label: `T{n} → {dest_name}`.
- Source name subject to fog: if `knowledge == Shadow`, name renders as silhouette.
- Clicking a port stub label navigates to the source/destination tier page and selects that node.

Edges:
- Same research-line: solid line in category color, stroke-width 2.5
- Cross-line dependency: dashed line in `var(--ink-soft)`, stroke-width 1.5
- Edge visibility: if either endpoint is `Shadow` and `NodeUnlockState::Locked`, edge hidden by default (toggle via filter panel)
- Edge style encodes knowledge: both endpoints `>= Partial` → fully opaque; one endpoint Shadow → 55% opacity

---

## 5. Systems

### `tech_tree_open`

Trigger: player presses `Y` keybind or clicks Tech Tree button in HUD.

```
1. Set TechTreePanelState.open = true
2. If active_page not set, default to lowest tier with any Unlockable node; else T1
3. Pause game input (camera, build placement) — tech tree is a modal overlay
4. Run tech_tree_surface_pending_choices (see §7)
```

### `tech_tree_close`

Trigger: `Escape`, or `Y` keybind while open.

```
1. If RevealOverlayState::Open → close overlay first (one Escape per layer)
2. If choice_overlay is Some → close choice modal first
3. Set open = false; resume game input
```

### `tech_tree_change_page`

Trigger: tab click, `←`/`→` keys.

```
1. Set active_page to target tier
2. Clear selected_node
3. Close reveal overlay if open
4. Do NOT close choice overlay (persists across page changes)
```

### `tech_tree_node_click`

Trigger: player clicks a node card on the canvas or a gate card.

```
1. If node is LockedOut → no-op (no selection, no overlay)
2. Set selected_node = clicked entity
3. If RevealOverlayState::Open(other) where other != clicked → close overlay first
4. Do NOT auto-open reveal overlay (inspector shows reveal button; overlay opens from button)
```

### `tech_tree_open_reveal_overlay`

Trigger: player clicks "REVEAL → T{n}" button in inspector.

```
1. If node.unlock_state != Unlockable → show blocked reason in inspector (no overlay)
2. If node is in an exclusive group AND choice not yet committed for that group:
   → open choice overlay instead (see §7); do not open reveal overlay
3. Set reveal_overlay = RevealOverlayState::Open(entity)
```

### `tech_tree_confirm_reveal`

Trigger: player clicks "REVEAL → T{n} · {cost} R" button in reveal overlay.

```
1. Validate: node.unlock_state == Unlockable AND player has sufficient research balance
2. Emit RevealRequested { node: Entity, to_knowledge: NodeKnowledge }
3. Close reveal overlay
4. Inspector updates to reflect new knowledge (event-driven via research system)
```

### `tech_tree_wishlist_toggle`

Trigger: player clicks "★ add to wishlist" / "★ wishlisted" in inspector or reveal overlay.

```
1. Toggle node's wishlist membership
2. Update card badge immediately (reactive)
```

---

## 6. Inspector (Right Rail)

Always shows the currently `selected_node`. If none selected, shows placeholder text.

**Header:** `T{tier} · knowledge T{1|2|3}` label; node name with FogText treatment; category chip; milestone badge if applicable.

**Tag + rarity:** category chip (colored, filled); rarity tag (`Common` / `Uncommon` / `Rare` / `Unique` — visible at all knowledge levels since rarity is part of the shadow).

**Inputs:**
- `Revealed`: slot cards with item icon + quantity
- `Partial`: slot outlines with `?` icon (count known, items hidden)
- `Shadow`: hatched slot outlines, count hidden

**Output rate:**
- `Revealed`: exact rate + item name (`12.0/s · control chip`)
- `Partial`: range + material class (`~7–17/s · plate-class`)
- `Shadow`: redacted bar

**Machine class:**
- `Revealed`: exact machine type + tier (`assembly bench T3`)
- `Partial`: machine class (`bench-class T2`)
- `Shadow`: hidden

**Unlock cost:** (visible when `knowledge >= Partial`)
- Research cost by type: `{n} MS · {n} FR · {n} ENG · {n} DISC` (zero types omitted)
- Current balance shown beside each type for comparison

**Non-research criteria:** (visible only when `knowledge == Revealed`; omitted if none)
- Production milestones: `produce {n} {item_class}`
- Exploration triggers: `reach {zone_type}`
- Alien science gates: `analyze {sample_type}`

**Flavour text:** visible at `Partial` and `Revealed` (different text per tier — partial shows "scattered references…"; revealed shows the full flavour line).

**Blocked reason:** if `NodeUnlockState::Locked`, show why. Insufficient research is not a blocked reason — communicated through the reveal button state only.

| Reason | Display |
|---|---|
| Prerequisite not unlocked | `prereq: {node_name} not yet revealed` (clickable — navigates to that node) |
| Prerequisite is unresolved exclusive group | `exclusive choice pending — {group_name}` (clickable — opens choice modal) |
| Tier gate locked | `tier {n} gate not cleared` |

**Reveal button:** visible when `knowledge < Revealed`.
- Enabled: `Unlockable` AND not in unresolved exclusive group AND balance ≥ cost
- Disabled (greyed): `Locked`; OR `Unlockable` but balance < cost; OR in unresolved exclusive group
- If `Locked`: shows blocked reason text (above) instead of cost
- If `Unlockable` in unresolved exclusive group: label reads `choose between {n} options first` (opens choice modal)
- Label (normal): `REVEAL → T{next_tier} · {cost} R`
- Shows remaining balance after spend: `{balance - cost} R remaining` (shown even when negative)

**Wishlist button:** always visible; toggles membership.

**Cross-tier dependencies** (below inspector divider):
- Incoming: list of nodes from previous tiers that this node depends on (with FogText treatment)
- Outgoing: list of nodes on future tiers that depend on this node

---

## 7. Exclusive Group Choice Surface (resolves issue #9)

**Decision: modal overlay, dismissible to "decide later."**

Rationale: exclusive groups are architectural forks — the choice is irreversible and high-stakes. A tree highlight or sidebar panel risks the player missing the mutual exclusion. A modal forces acknowledgment without forcing an immediate decision (players can dismiss to explore options first, then return).

### Surfacing trigger

`tech_tree_surface_pending_choices` runs when:
1. Tech tree panel opens
2. A node's `NodeUnlockState` transitions to `Unlockable`

For each exclusive group where any member is `Unlockable` AND the group choice has not been committed:
- Set `TechTreePanelState.choice_overlay = Some(group_id)`
- Surface immediately (takes precedence over the reveal overlay)

Only one group surfaced at a time. If multiple groups are pending, surface the first; the next surfaces when the current is dismissed.

### Choice modal layout

Full-bleed backdrop (dims tree to 35% opacity). Centered panel.

**Header:** `EXCLUSIVE CHOICE · {group member count} options · choosing one locks out the rest`

**Member columns:** one column per group member (2–3 members). Each column:
- Node card (full-size version of the canvas card at its current knowledge tier)
- Category + rarity chips
- Effect summary: what unlocking grants (always shown at full detail — choice surface bypasses fog so the decision is informed)
- **Unlock conditions** (members may have different requirements; show all that apply):
  - Research cost: `{n} MS · {n} FR · {n} ENG · {n} DISC` by type
  - Non-research criteria if present: production milestones, exploration triggers, alien science gates
- `SELECT` button

**Footer:** `DECIDE LATER` button — dismisses modal and queues a reminder badge on all group member tabs. Players can explore the tree and return. The choice modal re-surfaces the next time any group member's reveal button is pressed, or the next time the tech tree opens while a pending group exists.

**Commit action:**

```
tech_tree_commit_choice(group_id, chosen_entity):
  1. Emit ChoiceCommitted { group: ExclusiveGroupId, chosen: Entity }
  2. Research system sets LockedOut on all other members
  3. Set TechTreePanelState.choice_overlay = None
  4. If chosen entity is Unlockable, auto-open reveal overlay for it
```

**Key invariant:** a player cannot spend research on any exclusive group member until the group choice has been committed. The reveal button in the inspector shows "choose between N options first" if the node is in an uncommitted group.

---

## 8. Top Bar

```
[TECH TREE]  research: {mat_sci} MS · {field} FR · {eng} ENG · {disc} DISC  frontier · {tier_name}    [search] [wishlist ({n})] [filter] [reveal queue]
```

Research display: all four research types shown as compact labeled values. Zero-balance types dimmed. (Matches HUD research pool widget layout — same resource, different density.)

"Frontier" label: name of the current terminal tier for this run's difficulty (e.g. "Contact" for Initiation, "Salvage" for Standard). Functions as the escape progress anchor.

**Wishlist button:** opens a right-sliding drawer listing all wishlisted nodes across all pages, grouped by tier. Each entry shows current knowledge + estimated cost to next tier. Clicking a wishlist entry navigates to that page and selects the node.

**Reveal queue:** lists nodes with pending `RevealRequested` events not yet processed (edge case: if the research system is mid-computation). Normally empty; surfaces errors if a reveal failed.

**Search / filter:** inline text search + category filter chips. Search matches: node name (if `>= Partial`), category tag, effect type ("reveals:plate", "machine:bench"). Shadow nodes with matching category still appear in results (as shadow cards). Nodes not matching current filters are dimmed on canvas; layout does not reflow.

---

## 9. Events

| Event | Producer | Consumer | Fields |
|---|---|---|---|
| `TechTreeOpened` | `tech_tree_open` | UI, analytics | — |
| `TechTreeClosed` | `tech_tree_close` | UI | — |
| `NodeSelected` | `tech_tree_node_click` | inspector | `node: Entity` |
| `RevealOverlayOpened` | `tech_tree_open_reveal_overlay` | UI | `node: Entity` |
| `RevealRequested` | `tech_tree_confirm_reveal` | research system | `node: Entity, to_knowledge: NodeKnowledge` |
| `RevealCompleted` | research system | UI (inspector refresh) | `node: Entity, new_knowledge: NodeKnowledge` |
| `ChoiceGroupSurfaced` | `tech_tree_surface_pending_choices` | UI | `group: ExclusiveGroupId` |
| `ChoiceCommitted` | `tech_tree_commit_choice` | research system | `group: ExclusiveGroupId, chosen: Entity` |
| `WishlistToggled` | `tech_tree_wishlist_toggle` | UI | `node: Entity, in_wishlist: bool` |
| `TierPageChanged` | `tech_tree_change_page` | UI | `tier: u8` |

---

## 10. Edge Cases

**Tier gate node appears on two pages.** The gate card is the same entity rendered in two different positions (left gutter of tier N+1, right gutter of tier N). Both click targets select the same entity. Inspector shows the same data from either page.

**All group members are Shadow.** The choice modal still shows full effects (the modal bypasses normal fog — the player needs full information to make an informed fork decision). This is an exception to the standard fog rules, scoped to the choice modal only.

**Player has 0 research.** All reveal buttons show cost and remaining balance (negative). Buttons are disabled. No blocked reason shown in the inspector body — insufficient research is communicated through the button state and cost display only.

**Node's primary prereq is in an exclusive group (any member unlocked satisfies it).** Inspector prereq display: shows "requires any of: {member A}, {member B}" with current knowledge of each. Once any member is unlocked, prereq is satisfied — this is reflected immediately.

**Alt prerequisites activated by run seed.** Inspector shows all active prerequisites for this run (primary + any activated alts). Players see the same prereq list every session for a given run seed.

**Port stub label for Shadow node.** Stub renders with category color dot + silhouette name treatment (no name text — just the category + tier label: `← T2 extract`).

**Locked tier tab clicked.** Navigates to the page. All nodes render in their knowledge state (some may be Shadow-visible, none are Unlockable since the gate is not cleared). A banner at the top of the canvas reads: `TIER {n} LOCKED · gate: {gate_condition_text}`. Reveal actions are blocked.

**RevealCompleted fires while reveal overlay is open on a different node.** Do not close or update the open overlay — the completed event updates the canvas card for the completed node reactively, but the inspector stays on the currently open overlay's node.

---

## 11. Integration Test Invariants

1. Opening the tech tree panel pauses game input (camera movement, build placement).
2. Closing while reveal overlay is open requires two Escape presses: first closes overlay, second closes panel.
3. A `LockedOut` node cannot be selected (click is a no-op).
4. A node in an exclusive group with no committed choice: reveal button shows "choose between N options first"; clicking opens the choice modal, not the reveal overlay.
5. `ChoiceCommitted` transitions all non-chosen group members to `LockedOut`; chosen member's `NodeUnlockState` is unchanged by the event.
6. `RevealRequested` is not emitted unless `NodeUnlockState == Unlockable` AND player balance >= cost.
7. Gate card entity is identical on both adjacent tier pages (same `Entity` ID, not a copy).
8. Clicking a cross-tier port stub label navigates to the source/destination page and selects the referenced node.
9. Search/filter does not reflow canvas layout — non-matching nodes are dimmed in place.
10. The choice modal shows full node effects regardless of the node's current `NodeKnowledge` level.
11. Tab node count (`revealed/total`) counts only `NodeKnowledge::Revealed` in numerator; `optional` nodes are included in both numerator and denominator.
12. A locked tier tab is navigable; all reveal actions on that page are blocked; banner shows gate condition.
13. Wishlist state persists across panel open/close within a run session.
14. `tech_tree_surface_pending_choices` fires on panel open; at most one choice group is surfaced at a time.

---

## 12. VS vs MVP Scope

**VS (vertical slice):**
- All node visual states (Shadow, Partial, Revealed, Unlockable, Locked-Out)
- Tier-paged layout, tabs T1–T3 (Initiation difficulty)
- Inspector with reveal action and wishlist toggle
- Reveal overlay with tier ladder, before/after diff, prereq chain
- Exclusive group choice modal (required: T1 has no exclusive groups, but system must be present for T2+)
- Top bar with research pool display and wishlist drawer
- Search by node name (partial-revealed nodes only) and category filter chips

**MVP additions:**
- Tiers T4–T10 tabs
- Port stub labels with fog-aware source names
- Reveal queue in top bar (error surface)
- Alternative prerequisite display in inspector (VS shows primary prereq only)
- "Also unlocked when revealed" side-effects section in reveal overlay

VS scope matches vertical slice signal §3.3 (Minimal Tech Tree): T1 node set fully functional, reveal mechanic, tier gate for T2 visible.

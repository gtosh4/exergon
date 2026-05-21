# Input & Keybindings

Canonical action token registry plus the input plugin design. Every player-facing input in the rest of the technical docs refers to an action token from §3 — never to a raw key or mouse button. The default binding for each token lives in this doc and is mutable from an in-game keybinding settings screen.

Read this before referencing any input in another technical doc, or before adding a new input-driven system.

---

## Table of Contents

1. [Crate Choice](#1-crate-choice)
2. [Concepts](#2-concepts)
3. [Action Token Registry](#3-action-token-registry)
4. [Contexts](#4-contexts)
5. [Persistence & Rebinding](#5-persistence--rebinding)
6. [Cross-Doc Usage Conventions](#6-cross-doc-usage-conventions)
7. [Vertical Slice Scope](#7-vertical-slice-scope)
8. [Edge Cases](#8-edge-cases)

---

## 1. Crate Choice

**[`bevy_enhanced_input`](https://docs.rs/bevy_enhanced_input/)** — modeled after Unreal Engine's Enhanced Input. Chosen over `leafwing-input-manager` because Exergon's input is **modal**: distinct context layers (Local body, drone Remote, Planner overlay, Tech Tree overlay, Machine UI panel, Landing screen) each want different bindings active simultaneously with shared raw keys (e.g. `R` rotates a placeable in Local but is unbound in Remote). `bevy_enhanced_input` stacks contexts and resolves consumption per-frame, which maps 1:1 to `PlayMode` states + open-modal markers.

Companion crate: **[`bevy-input-prompts`](https://crates.io/crates/bevy-input-prompts)** (or equivalent) — resolves an action token to a rebindable display glyph for in-world prompts ("press `{kbd:interact}` to open"). All HUD/tooltip strings that name an input go through this resolver — never hardcode "press E".

---

## 2. Concepts

| Term | Meaning |
|---|---|
| **Action** | A named gameplay verb. Rust type implementing `InputAction`. Examples: `PrimaryAction`, `Interact`, `RotateCw`. Output type is `bool`, `Vec2`, `f32`, or `Vec3` depending on the action. |
| **Token** | The doc-level reference to an action. Written as `{kbd:action_name}` in prose so docs decouple from the eventual Rust identifier. Rendered in the keybinding settings UI as a localized action label. |
| **Context** | A `InputContext` Rust type bundling a set of actions and their bindings. Pushed onto a body/drone/UI-root entity when its mode activates; popped on exit. Higher-priority contexts (UI modals) consume an input before lower-priority ones (gameplay) see it. |
| **Binding** | The raw input that fires an action: a `KeyCode`, `MouseButton`, gamepad button, or a chord with modifiers. Stored in a serializable `KeyBindings` resource — never hardcoded in systems. |
| **Trigger** | The lifecycle event for an action firing: `Started`, `Fired`, `Completed`, `Canceled`. Systems observe `Trigger<Fired<MyAction>>` or read `Events<Fired<MyAction>>`. |

---

## 3. Action Token Registry

Tokens are the canonical names referenced from every other doc. Default bindings shown are the out-of-box configuration; players rebind via settings.

### 3.1 Global / Menu

| Token | Default | Action | Context |
|---|---|---|---|
| `{kbd:menu_terminal}` | `T` | Open/close Terminal (item network) | `GlobalUiContext` |
| `{kbd:menu_index}` | `I` | Open/close Index (recipe browser) | `GlobalUiContext` |
| `{kbd:menu_planner}` | `Tab` | Open/close Planner | `GlobalUiContext` |
| `{kbd:menu_tech_tree}` | `Y` | Open/close Tech Tree | `GlobalUiContext` |
| `{kbd:cancel}` | `Escape` | Close active modal / cancel in-progress placement | `GlobalUiContext` (high priority) |
| `{kbd:toggle_topology}` | `N` | Toggle 3D network topology overlay | `GlobalUiContext` |

### 3.2 Local Body (in `PlayMode::Exploring`)

| Token | Default | Action | Output |
|---|---|---|---|
| `{kbd:movement}` | `WASD` | Body planar movement | `Vec2` |
| `{kbd:look}` | Mouse delta | Camera yaw/pitch | `Vec2` |
| `{kbd:interact}` | `E` | Interact with machine / Field Computer in reach | `bool` (Started) |
| `{kbd:primary_action}` | `MouseButton::Left` | Place selected hotbar item / commit placement stage | `bool` (Started) |
| `{kbd:secondary_action}` | `MouseButton::Right` | Cancel placement / context action | `bool` (Started) |
| `{kbd:remote_mode}` | `F` | Enter drone Remote mode (active or nearest drone) | `bool` (Started) |
| `{kbd:place_extend_modifier}` | `Shift` (held) | Multi-stage placement (rect / line / area) | `bool` (Pressed) |
| `{kbd:rotate_cw}` | `R` | Rotate active placeable CW by `BUILD_ROT_STEP` | `bool` (Started) |
| `{kbd:rotate_ccw}` | `Shift+R` | Rotate active placeable CCW / reset Free orientation | `bool` (Started) |
| `{kbd:rotate_fine}` | Scroll wheel | Continuous rotation while holding placeable | `f32` (axis) |
| `{kbd:rotate_free_drag}` | `R` held + mouse drag | Free-axis rotation around surface normal | `Vec2` (delta) |
| `{kbd:hotbar_slot_N}` | `1`–`9` | Select hotbar slot `N` (N ∈ 1–9) | `bool` (Started) |
| `{kbd:hotbar_bank_switch}` | `Shift+Scroll` | Cycle hotbar bank A/B/C | `f32` (axis) |

### 3.3 Drone Remote (in `PlayMode::DronePilot`)

| Token | Default | Action | Output |
|---|---|---|---|
| `{kbd:movement}` | `WASD` | Drone planar movement (rebound in this context) | `Vec2` |
| `{kbd:look}` | Mouse delta | Drone heading | `Vec2` |
| `{kbd:remote_mode}` | `F` | Exit Remote → Local | `bool` (Started) |
| `{kbd:primary_action}` | `MouseButton::Left` | Use selected drone tool (mining drill sample, scanner pulse) | `bool` (Started) |
| `{kbd:secondary_action}` | `MouseButton::Right` | Manual mine (`drone_mine_system`) | `bool` (Started) |
| `{kbd:hotbar_slot_N}` | `1`–`9` | Select drone hotbar slot `N` | `bool` (Started) |
| `{kbd:drone_quick_switch_N}` | `Alt+1`–`Alt+9` | Switch to drone bound to slot `N` (MVP+; see `drone.md §7`) | `bool` (Started) |

### 3.4 Body / Drone Quick-Switch (MVP+; see `aegis.md §7`)

| Token | Default | Action |
|---|---|---|
| `{kbd:body_quick_switch_N}` | `Alt+1`–`Alt+0` | Switch `ActiveBody` to body bound to slot `N` (1–9, 0) |

`{kbd:body_quick_switch_N}` and `{kbd:drone_quick_switch_N}` share the `Alt+digit` chord. The quick-switch slot can hold either a body or a drone; whichever is bound determines the effective behavior. See `aegis.md §7` for the unified slot model.

### 3.5 Terminal / Planner / Tech Tree (modal contexts)

| Token | Default | Action |
|---|---|---|
| `{kbd:ui_rename}` | Double-click (`MouseButton::Left` × 2 within rename threshold) | Begin inline rename on tab / machine name |
| `{kbd:ui_hotbar_assign}` | Drag-and-drop from item table → hotbar slot | Assign slot via `HotbarSlotSet` |
| `{kbd:ui_clear_slot}` | `MouseButton::Right` on slot | Clear hotbar slot |
| `{kbd:ui_context_menu}` | `MouseButton::Right` on row / node | Open contextual menu (Pin as goal, Watch prerequisites, Reset to default, etc.) |
| `{kbd:ui_page_left}` / `{kbd:ui_page_right}` | `←` / `→` | Page navigation in Tech Tree |

UI primitives (left-click activate, double-click rename, right-click context) reuse Bevy UI's default mouse handling. The keybinding settings screen exposes these as remappable in case a player wants e.g. a different rename gesture, but the default mapping matches platform convention.

---

## 4. Contexts

Contexts are stacked Rust types. Highest priority on top consumes input first.

```
┌──────────────────────────────────────────────┐
│ ModalContext     (Planner / Tech Tree /      │  priority 100
│                   Machine UI / Recipe Picker)│
├──────────────────────────────────────────────┤
│ HudContext       (hotbar, menu shortcuts,    │  priority 50
│                   topology toggle)           │
├──────────────────────────────────────────────┤
│ PlayContext      (one of:                    │  priority 10
│   - LocalBodyContext                         │
│   - DronePilotContext                        │
│   - LandingContext)                          │
├──────────────────────────────────────────────┤
│ GlobalContext    (Escape, screenshot,        │  priority 0
│                   pause)                     │
└──────────────────────────────────────────────┘
```

Activation rules:
- Exactly one `PlayContext` variant is active at any time, mirroring `PlayMode`.
- `ModalContext` is pushed when any panel opens (Planner, Tech Tree, Machine UI, Recipe Picker) and popped on close. While pushed, `PlayContext` actions do not fire — the body stands still while the player edits a plan.
- `HudContext` is always active in `GameState::Playing`. Menu shortcuts (`{kbd:menu_terminal}`, `{kbd:menu_tech_tree}`, etc.) live here so they work from any `PlayMode` and from inside one panel to swap to another. `{kbd:cancel}` priority is overridden in `ModalContext` to close the topmost panel first.

Switching `PlayMode` (e.g. `Exploring → DronePilot` on `{kbd:remote_mode}`) swaps the `PlayContext` variant: removes `LocalBodyContext`, inserts `DronePilotContext`. `{kbd:movement}` and `{kbd:look}` are defined in both contexts with different output sinks — the action token is the same, but `LocalBodyContext` routes it to the body controller and `DronePilotContext` routes it to the active drone.

---

## 5. Persistence & Rebinding

```rust
#[derive(Resource, Serialize, Deserialize, Default)]
pub struct KeyBindings {
    pub bindings: HashMap<ActionId, Vec<InputBinding>>,
}

pub enum InputBinding {
    Key(KeyCode),
    MouseButton(MouseButton),
    Chord { modifiers: ModifierMask, primary: PrimaryBinding },
    Axis(AxisBinding),       // mouse scroll, mouse delta, gamepad stick
}
```

`KeyBindings` is loaded at startup from `meta_save/keybindings.ron`. Out-of-box defaults are baked into `default_keybindings()` and used when the file is missing or a token is unbound.

Multiple bindings per action are allowed (e.g. `{kbd:menu_planner}` defaults to `Tab`; a player can additionally bind `P`). Conflicts (same chord bound to two actions in the same context) are surfaced in the settings UI but not auto-resolved — players choose which to keep.

Action tokens are stable identifiers. Renaming an action requires a migration step in the keybinding loader; raw bindings are migrated to the new token name. Defaults table in §3 is the source of truth — code regenerates `meta_save/keybindings.ron` if a new token is added that isn't in the saved file.

---

## 6. Cross-Doc Usage Conventions

Other technical docs follow these rules when referring to an input:

- **Prose:** write `{kbd:interact}`, never "press E". The token resolves to a current display glyph via the input-prompts helper.
- **System trigger descriptions:** write the action token in the trigger column — e.g. "Trigger: `{kbd:primary_action}` Started, in `LocalBodyContext`." Avoid `just_pressed(MouseButton::Left)` style references; the action layer is the contract, the raw binding is settings.
- **Code samples in docs:** when illustrating a system, use the action type directly: `fn on_primary(trigger: Trigger<Fired<PrimaryAction>>) { ... }`. Do not include `KeyCode`/`MouseButton` in code samples that describe gameplay — those belong in `input.md` only.
- **New input:** add the token to §3 before referencing it elsewhere. Reviewers reject PRs that introduce inputs without an entry here.

Example rewrite:

> **Trigger:** `{kbd:primary_action}` (Started) in `DronePilotContext` and active hotbar slot holds `mining_drill`.
>
> System reads `Fired<PrimaryAction>` events, queries `ActiveDrone`, raycasts forward `MINE_REACH` meters, …

vs. the old form ("Left-click (`MouseButton::Left`) while…"). The action token form survives rebinding; the raw form does not.

---

## 7. Vertical Slice Scope

| Feature | VS | MVP |
|---|---|---|
| Action tokens + `bevy_enhanced_input` plugin | ✓ | ✓ |
| Contexts: `GlobalContext`, `HudContext`, `LocalBodyContext`, `DronePilotContext`, `ModalContext` | ✓ | ✓ |
| `LandingContext` | ✓ | ✓ |
| Default `KeyBindings` baked in | ✓ | ✓ |
| Persistence to `meta_save/keybindings.ron` | — | ✓ |
| In-game keybinding settings screen | — | ✓ |
| Multiple bindings per action | — | ✓ |
| Chord support (`Alt+N`, `Shift+R`, `Shift+Scroll`) | ✓ | ✓ |
| Gamepad support | — | — (post-MVP) |
| Conflict surfacing in settings UI | — | ✓ |
| Input-prompt glyph resolver | ✓ (text only) | ✓ (icons) |

VS uses default bindings only — no rebinding UI. The token system still applies so MVP rebinding can land without touching gameplay docs.

---

## 8. Edge Cases

| Case | Behavior |
|---|---|
| Player rebinds `{kbd:rotate_cw}` to `MouseButton::Middle` | Settings UI accepts; `LocalBodyContext` binding swapped at runtime; no game restart needed. Existing `Shift+R` for `{kbd:rotate_ccw}` is untouched. |
| Player binds two actions to the same chord in the same context | Settings UI flags the conflict; both bindings persist; resolution order is action declaration order in the context type. Player explicitly chooses to clear one. |
| Modal opens while `{kbd:primary_action}` is held (e.g. mid-drag) | `ModalContext` pushes; on next frame the gameplay action receives a `Canceled` event and stops; the modal sees a fresh `Started` only if the player re-presses. |
| Two contexts both declare `{kbd:cancel}` | Highest-priority context consumes it. Modal closes before `{kbd:cancel}` reaches `LocalBodyContext`'s placement cancel. |
| `{kbd:hotbar_slot_3}` and `{kbd:drone_quick_switch_3}` share `3` digit | `Alt+3` vs. `3` is a different chord — both bindings coexist. Without `Alt`, `LocalBodyContext` claims `3` for hotbar. With `Alt`, the higher-priority quick-switch action fires. |
| Saved `keybindings.ron` references a removed action token | Loader logs a warning, discards the orphan binding, and writes a clean file. Missing tokens get the default binding. |
| Player rebinds `{kbd:cancel}` to something other than Escape | Allowed; modals close on whatever the player chose. The settings screen warns that some platform UI (Steam overlay, etc.) still uses Escape. |
| Mouse scroll bound to `{kbd:hotbar_bank_switch}` and `{kbd:rotate_fine}` in the same context | Both fire; `{kbd:hotbar_bank_switch}` is chord-gated on `Shift+Scroll`, `{kbd:rotate_fine}` on bare `Scroll` — distinct chords, no conflict. |
| New action added in a future patch that wasn't in the player's saved keybindings | Token resolves to its default on first load; loader writes the augmented file. Player sees the new binding in settings. |
| Two `Started<PrimaryAction>` events in one frame (engine quirk) | Systems consuming `Fired<PrimaryAction>` get one logical event per frame thanks to `bevy_enhanced_input`'s per-frame consumption. Same-frame duplicate emissions are coalesced upstream. |

---
name: bevy
description: >
  Bevy 0.18 reference and implementation guide for Exergon. Use when writing any Bevy
  code, designing ECS components, implementing systems, or answering questions about Bevy
  patterns. Triggers on: /bevy, "how do I do X in Bevy", "Bevy plugin", "Bevy system",
  "Bevy ECS", "spawn entity", "Bevy state", "Bevy asset", "Bevy camera", implementing
  any game system in this project. Apply proactively whenever writing Rust code that
  touches Bevy — do not wait to be asked.
---

# Bevy 0.18 Reference for Exergon

Version: **0.18.1**. Exergon is a **3D game** — use `Camera3d`, `Mesh3d`, `StandardMaterial`, PBR lighting.

## Resources

Load the relevant resource file(s) before writing code:

| Resource | Contents |
|----------|----------|
| `.claude/skills/bevy/ecs.md` | Component, Resource, Bundle, Query, Single, Commands, Hierarchy, Reflection, Component Hooks, Relationships |
| `.claude/skills/bevy/systems-state.md` | Schedules, Ordering, SystemSets, RunConditions, Fallible systems, SystemParam, States, SubStates, DespawnOnExit |
| `.claude/skills/bevy/rendering.md` | Camera3d, Mesh3d, lights, MeshPickingPlugin, Pointer events, Scene save/load |
| `.claude/skills/bevy/assets.md` | Asset loading, custom AssetLoader, RON data files |
| `.claude/skills/bevy/events.md` | Event, EntityEvent, Observer (global + entity-scoped), lifecycle observers |
| `.claude/skills/bevy/patterns.md` | Plugin pattern, Exergon-specific conventions (factory ECS, power graph, recipes, state machine) |


## 0.18 Breaking Changes (check before writing any Bevy code)

| Wrong | Correct |
|-------|---------|
| `Camera3dBundle { .. }` | `Camera3d` component directly |
| `PbrBundle { .. }` | `(Mesh3d(..), MeshMaterial3d(..))` tuple |
| `next_state.set(S)` to guard no-op | `next_state.set_if_neq(S)` |
| `DespawnOnExit<GameState::Building>` | `DespawnOnExit(GameState::Building)` |
| `BorderRadius` as separate component | `BorderRadius` inside `Node` struct |
| `DefaultPickingPlugins` | `MeshPickingPlugin` |
| `#[derive(Asset)]` without `TypePath` | also derive `TypePath` |
| `trigger.target()` in observers | `trigger.entity()` |
| `EntityDoesNotExistError` | removed — handle via `get_entity()` Result |

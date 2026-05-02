---
name: bevy
description: >
  Bevy reference and guide. Use when writing any Bevy code, designing ECS components, implementing systems,
  or answering questions about Bevy patterns.
  Triggers on: /bevy, "how do I do X in Bevy", "Bevy plugin", "Bevy system",
  "Bevy ECS", "spawn entity", "Bevy state", "Bevy asset", "Bevy camera", implementing
  any game feature in this project. Apply proactively whenever writing Rust code that
  touches Bevy — do not wait to be asked.
user-invocable: false
---

# Bevy 0.18 Reference for Exergon

Version: **0.18.1**. Exergon is a **3D game** — use `Camera3d`, `Mesh3d`, `StandardMaterial`, PBR lighting.

## Resources

Load the relevant resource file(s) before writing code:

| Resource | Description |
|----------|----------|
| [`ecs.md`](./ecs.md) | Fundamentals, almost always load this, especially when adding any business logic |
| [`schedules-state.md`](./schedules-state.md) | Schedules, Ordering, RunConditions, Fallible systems, SystemParam, States, SubStates, DespawnOnExit |
| [`rendering.md`](./system-state.md) | Camera3d, Mesh3d, lights, MeshPickingPlugin, Pointer events, Scene save/load |
| [`assets.md`](./assets.md) | Asset loading, custom AssetLoader, RON data files |

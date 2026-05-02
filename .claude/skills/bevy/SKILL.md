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

## Rules:
1. **Data lives in components and resources. Logic lives in systems and observers.** A method on a component is fine if it's a pure projection of its own fields (`Health::is_alive`, `Vec3::length`). Anything that touches another entity, spawns, despawns, or reads a resource belongs in a system or observer. The advice "components are just data" has limits — small impl blocks for invariant-preserving setters and convenient accessors are good — but anything that walks the world goes in a system.
2. **One plugin per domain.** Each feature gets a `XPlugin` struct that registers its messages, resources, observers, and systems. Plugins are composable, and breaking work into plugins is the canonical way to keep a Bevy project navigable as it grows. Drop plugins into `App` from a small `main.rs` (or a binary crate that depends on a library crate); resist the urge to put everything in one file.
3. **Centralize ordering with a `SystemSet` enum.** Define one enum with variants for each ordered phase of your game (`InputGather`, `AiBrain`, `Locomotion`, `CameraFollow`, `UpdateUi`, etc.), `chain()` them once in `app.rs`, and have plugins drop systems *into* those sets via `.in_set(...)`. Don't sprinkle `configure_sets` calls across plugins — that splits the source of truth and ordering becomes nondeterministic in practice.


## Resources

Load the relevant resource file(s) before writing code:

| Resource | Description |
|----------|----------|
| [`ecs.md`](./ecs.md) | Fundamentals, almost always load this, especially when adding any business logic |
| [`schedules-state.md`](./schedules-state.md) | Schedules, Ordering, RunConditions, Fallible systems, SystemParam, States, SubStates, DespawnOnExit |
| [`rendering.md`](./system-state.md) | Camera3d, Mesh3d, lights, MeshPickingPlugin, Pointer events, Scene save/load |
| [`assets.md`](./assets.md) | Asset loading, custom AssetLoader, RON data files |

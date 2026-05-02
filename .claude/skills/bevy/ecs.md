# Bevy 0.18 — ECS Core, Reflection, Hooks, Relationships
## Entities
Fundamental units, like game objects.

## Components:
Data holders (structs) attached to entities. No logic; pure data (x, y for Position).
- `Relationship`s: special component type (pair, with `RelationshipTarget`) to denote entity relationships
- Markers: empty structs used as a tag for queries
- `Bundle`s: use as convenience and logical grouping for spawning

✓ Single Responsibility: One component per data type (e.g., Position, not mixed with Render)

## Systems
Logic blocks processing entity groups sharing components. Parallel execution, efficient.
✓ System Efficiency: Process only relevant entities using queries. Leverage change detection, events, and messages
✓ `par_iter_mut` for parallel iteration when the body is independent across entities. Combine with `ParallelCommands::command_scope` to issue commands from parallel work.

testing:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_system() {
      // minimal app setup & registrations
      let mut app = App::new();
      app.add_systems(Update, my_system)

      // minimal world setup:
      let entity_id = app
          .world_mut()
          .spawn(/* ... */)
          .id();

      // Run systems
      app.update();

      // Run asserts
      // check world state with `app.world_mut().query::<>().iter(app.world())`
      assert_eq!(/* ... */)
    }
}
```

## Resources
Global, mutable data (Time) accessed by systems. Shared state. No world data.

## Queries
Select entities based on component criteria. Efficient selection.

Lifecycle / Change Detection:
- Add: Triggered when a component is added to an entity that did not already have it.
- Insert: Triggered when a component is added to an entity, regardless of whether it already had it.
- Replace: Triggered when a component is removed from an entity, regardless if it is then replaced with a new value.
- Remove: Triggered when a component is removed from an entity and not replaced, before the component is removed.
- Despawn: Triggered for each component on an entity when it is despawned.

## Messages
Pub (`MessageWriter`) / Sub (`MessageReader`) communication between systems.
✓ Use instead of polling for better performance

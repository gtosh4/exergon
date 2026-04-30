# Bevy 0.18 — Events & Observers

## Events (broadcast, consumed next frame)

```rust
#[derive(Event)]
pub struct RecipeDiscovered { pub recipe_id: RecipeId }

fn discover_recipe(mut writer: EventWriter<RecipeDiscovered>) {
    writer.write(RecipeDiscovered { recipe_id });
}

fn on_recipe_discovered(mut reader: EventReader<RecipeDiscovered>) {
    for event in reader.read() { /* ... */ }
}
```

## EntityEvent (targeted, can bubble through hierarchy)

```rust
#[derive(EntityEvent)]
pub struct MachineBroken { pub entity: Entity }

// Trigger on specific entity
commands.trigger(MachineBroken { entity: machine_id });

// Observers receive targeted entity
fn on_broken(trigger: On<MachineBroken>, mut commands: Commands) {
    commands.entity(trigger.entity()).insert(Broken);
}
```

## Observers

```rust
// Global observer — fires for all matching events
app.add_observer(on_broken);

// Entity observer — fires only for that entity
commands.spawn((Machine { recipe_id }, Transform::default(), Mesh3d(..), MeshMaterial3d(..)))
    .observe(|trigger: On<Pointer<Click>>, mut commands: Commands| {
        commands.entity(trigger.entity()).insert(Selected);
    });

// Stop event from bubbling
fn on_click(trigger: On<Pointer<Click>>) {
    trigger.propagate(false);
}
```

## Lifecycle Observers (react to component add/remove)

```rust
// Runs whenever Machine added to any entity
app.add_observer(|trigger: On<Add, Machine>, mut index: ResMut<MachineIndex>| {
    index.register(trigger.entity());
});

// Runs whenever Machine removed
app.add_observer(|trigger: On<Remove, Machine>, mut index: ResMut<MachineIndex>| {
    index.unregister(trigger.entity());
});
```

## Rule of Thumb

| Pattern | Use for |
|---------|---------|
| `Event` + `EventWriter/Reader` | Game logic signals (recipe found, tier unlocked) |
| `EntityEvent` | Targeted events that may need to bubble (damage, selection) |
| `Observer` entity-scoped | UI/input on specific entities (click machine, hover node) |
| `add_observer` global | Index maintenance, cross-cutting reactions |

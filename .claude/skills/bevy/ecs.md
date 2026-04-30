# Bevy 0.18 — ECS Core, Reflection, Hooks, Relationships

## Components, Resources, Bundles

```rust
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Machine { pub recipe_id: RecipeId }

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct RunSeed(pub u64);

#[derive(Bundle)]
pub struct MachineBundle {
    machine: Machine,
    transform: Transform,
    mesh: Mesh3d,
    material: MeshMaterial3d<StandardMaterial>,
}
```

## Queries

```rust
fn tick(mut q: Query<(&Machine, &mut ProductionState)>) {
    for (machine, mut state) in &mut q { /* ... */ }
}

// Filters
Query<&Transform, (With<Machine>, Without<Belt>)>
Query<&mut ProductionState, Changed<Inventory>>
Query<&mut ProductionState, Added<Machine>>
```

**`Single<T>`** — panics if 0 or 2+ matches:
```rust
fn follow_camera(camera: Single<&mut Transform, With<Camera3d>>) {
    camera.into_inner().translation.y += 0.1;
}

// Zero-or-one
fn maybe_boss(boss: Option<Single<&Boss>>) {
    if let Some(boss) = boss { /* ... */ }
}
```

## Commands (deferred — apply at ApplyDeferred)

```rust
commands.spawn((
    Machine { recipe_id },
    Transform::from_xyz(x, y, z),
    Mesh3d(mesh_handle),
    MeshMaterial3d(material_handle),
));

if let Ok(mut entity) = commands.get_entity(id) {
    entity.insert(NewComponent);
}
```

## Hierarchy

```rust
commands.spawn((Parent, Transform::default()))
    .with_children(|p| {
        p.spawn((Child, Transform::from_xyz(1.0, 0.0, 0.0)));
    });

// children! macro
commands.spawn((
    Parent,
    children![
        (Child, Transform::from_xyz(1.0, 0.0, 0.0)),
        (OtherChild, Transform::from_xyz(-1.0, 0.0, 0.0)),
    ],
));

fn sys(parents: Query<(&Parent, &Children)>, transforms: Query<&Transform>) {
    for (parent, children) in &parents {
        for child in children {
            let _ = transforms.get(*child);
        }
    }
}
```

## Reflection

Required for scene save/load and editor tooling:

```rust
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TechNode { pub id: NodeId, pub tier: u8, pub revealed: bool }

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct WorldReactivity(pub f32);

// Register in plugin
app.register_type::<TechNode>()
   .register_type::<WorldReactivity>();
```

## Component Hooks

Run synchronously on component add/remove. Use for invariant enforcement and indexes. Prefer observers for game logic.

```rust
#[derive(Component)]
#[component(on_add = on_add_machine, on_remove = on_remove_machine)]
pub struct Machine { pub recipe_id: RecipeId }

fn on_add_machine(mut world: DeferredWorld, ctx: HookContext) {
    let recipe_id = world.get::<Machine>(ctx.entity).unwrap().recipe_id;
    world.resource_mut::<MachineIndex>().insert(recipe_id, ctx.entity);
}

fn on_remove_machine(mut world: DeferredWorld, ctx: HookContext) {
    let recipe_id = world.get::<Machine>(ctx.entity).unwrap().recipe_id;
    world.resource_mut::<MachineIndex>().remove(&recipe_id);
}
```

Four lifecycle events: `on_add` (first insert), `on_insert` (every insert including replace),
`on_replace` (before overwrite), `on_remove` (before removal).

## Relationships (new in 0.18)

```rust
#[derive(Component)]
#[relationship(relationship_target = ConnectedTo)]
pub struct PoweredBy(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = PoweredBy)]
pub struct ConnectedTo(Vec<Entity>);

// Use — ConnectedTo on power_source_id updated automatically
commands.entity(machine_id).insert(PoweredBy(power_source_id));

// Spawn with relationship
commands.spawn((PowerSource, Transform::default()))
    .with_related_entities::<PoweredBy>(|spawner| {
        spawner.spawn(Machine { recipe_id });
    });

// Query
fn read_grid(sources: Query<(&PowerSource, &ConnectedTo)>, machines: Query<&Machine>) {
    for (src, connected) in &sources {
        for machine_entity in connected.iter() {
            let _ = machines.get(machine_entity);
        }
    }
}
```

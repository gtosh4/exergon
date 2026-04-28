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

Current version: **0.18.1**. All patterns below are verified for 0.18. Do not use
patterns from older tutorials without checking the migration guides.

**Exergon is a 3D game.** Use 3D rendering, Camera3d, Mesh3d, StandardMaterial, PBR lighting.

---

## Cargo.toml

```toml
[dependencies]
bevy = { version = "0.18", features = ["3d"] }

# Critical for usable iteration speed in dev
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

Use high-level feature profiles (`2d`, `3d`, `ui`) instead of individual feature flags.
`DefaultPlugins` for full setup; `MinimalPlugins` for headless/tests.

---

## ECS Core

```rust
// Component — plain struct
#[derive(Component, Reflect)]
pub struct Machine { pub recipe_id: RecipeId }

// Resource — singleton global state
#[derive(Resource, Reflect)]
pub struct RunSeed(pub u64);

// Bundle — group components for spawning
#[derive(Bundle)]
pub struct MachineBundle {
    machine: Machine,
    transform: Transform,
    mesh: Mesh3d,
    material: MeshMaterial3d<StandardMaterial>,
}

// System — plain function, params injected
fn tick_machines(
    mut query: Query<(&Machine, &mut ProductionState)>,
    recipes: Res<Assets<RecipeData>>,
) { ... }
```

**Query filters**: `With<T>`, `Without<T>`, `Changed<T>`, `Added<T>`
```rust
Query<&Transform, (With<Machine>, Without<Belt>)>
Query<&mut ProductionState, Changed<Inventory>>
```

**Commands are deferred** — spawn/insert/despawn apply at `ApplyDeferred`, not immediately:
```rust
// Spawn with tuple (preferred over bundles for ad-hoc)
commands.spawn((
    Machine { .. },
    Transform::from_xyz(x, y, z),
    Mesh3d(mesh_handle),
    MeshMaterial3d(material_handle),
));

// Modify existing — use get_entity() for fallible access
if let Ok(mut entity) = commands.get_entity(id) {
    entity.insert(NewComponent);
}
```

---

## Schedules & Ordering

Main schedules: `Startup`, `PreUpdate`, `Update`, `PostUpdate`, `FixedUpdate`

```rust
app.add_systems(Startup, setup_world)
   .add_systems(Update, (
       scan_inputs,
       process_machines.after(scan_inputs),
       update_belts,
   ).run_if(in_state(GameState::Playing)));
```

**System sets** for coarser ordering:
```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystems { Input, Simulation, Rendering }

app.configure_sets(Update, (
    GameSystems::Input,
    GameSystems::Simulation.after(GameSystems::Input),
    GameSystems::Rendering.after(GameSystems::Simulation),
));
```

`remove_systems_in_set()` fully removes systems (0.18) — prefer over run conditions when
a system should never run again.

---

## State Management

```rust
#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Exploring,
    Building,
    Researching,
}

// Register in plugin
app.init_state::<GameState>();

// Transition
fn check_loaded(mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::Exploring);        // always triggers (0.18 behavior)
    // next.set_if_neq(GameState::Exploring); // only triggers if value changes
}
```

> **0.18 BREAKING**: `set()` always fires transition events now. Use `set_if_neq()` for
> the previous guard behavior.

**Lifecycle schedules**:
```rust
app.add_systems(OnEnter(GameState::Building), spawn_build_ui)
   .add_systems(OnExit(GameState::Building), despawn_build_ui);
```

**Auto-despawn entities on exit**: add `DespawnOnExit<GameState::Building>` component.

**SubStates** for dependent states (e.g., `BuildMode` only exists inside `Building`):
```rust
#[derive(SubStates, Default, Clone, Eq, PartialEq, Debug, Hash)]
#[source(GameState = GameState::Building)]
pub enum BuildMode { #[default] Select, Place, Delete }
```

---

## 3D Rendering

```rust
// Camera — spawn component directly, NOT Camera3dBundle (deprecated)
commands.spawn((
    Camera3d::default(),
    Transform::from_xyz(0.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
));

// Dev free-fly camera (0.18 new built-in)
commands.spawn((FreeCamera::default(), Transform::from_xyz(0.0, 10.0, 10.0)));

// Mesh entity
commands.spawn((
    Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
    MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.4, 0.1),
        ..default()
    })),
    Transform::from_xyz(x, y, z),
));

// Load a glTF model
commands.spawn((
    SceneRoot(asset_server.load("models/machine.glb#Scene0")),
    Transform::from_xyz(x, y, z),
));

// Directional light (sun)
commands.spawn((
    DirectionalLight {
        illuminance: 10_000.0,
        shadows_enabled: true,
        ..default()
    },
    Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
));

// Point light
commands.spawn((
    PointLight { intensity: 1_500.0, ..default() },
    Transform::from_xyz(x, y, z),
));

// Ambient light (resource)
commands.insert_resource(AmbientLight { color: Color::WHITE, brightness: 200.0 });
```

**ScatteringMedium** (0.18 new): customize atmospheric haze/fog for alien world feel.

---

## Assets

```rust
// Load (async, returns Handle immediately)
let mesh: Handle<Mesh> = asset_server.load("models/machine.glb#Mesh0");
let scene: Handle<Scene> = asset_server.load("models/machine.glb#Scene0");

// Check loaded
asset_server.is_loaded_with_dependencies(&handle)

// Access data
fn use_recipe(recipes: Res<Assets<RecipeData>>, handle: Res<CurrentRecipe>) {
    if let Some(recipe) = recipes.get(&handle.0) { ... }
}
```

**Custom asset** (for recipes, tech nodes, planet modifiers — the data-driven content):
```rust
#[derive(Asset, Reflect, serde::Deserialize)]
pub struct RecipeData {
    pub inputs: Vec<(ItemId, f32)>,
    pub outputs: Vec<(ItemId, f32)>,
    pub energy_cost: f32,
}

#[derive(Default)]
pub struct RecipeLoader;

impl AssetLoader for RecipeLoader {
    type Asset = RecipeData;
    type Settings = ();
    type Error = anyhow::Error;
    async fn load(reader: &mut dyn Reader, ..) -> Result<RecipeData, ..> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron::de::from_bytes(&bytes)?)
    }
    fn extensions(&self) -> &[&str] { &["recipe.ron"] }
}

// Register in plugin
app.register_asset_loader(RecipeLoader)
   .init_asset::<RecipeData>()
   .register_type::<RecipeData>();
```

Hot reload: add `file_watcher` feature in dev profile.

---

## Reflection

Required for scene save/load and editor tooling. Derive on all game data types:

```rust
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TechNode {
    pub id: NodeId,
    pub tier: u8,
    pub revealed: bool,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct WorldReactivity(pub f32);

// Register in plugin
app.register_type::<TechNode>()
   .register_type::<WorldReactivity>();
```

---

## Events & Observers

**Events** — broadcast signals, processed next frame:
```rust
#[derive(Event)]
pub struct RecipeDiscovered { pub recipe_id: RecipeId }

// Send
fn discover_recipe(mut writer: EventWriter<RecipeDiscovered>) {
    writer.write(RecipeDiscovered { recipe_id });
}

// Read
fn on_recipe_discovered(mut reader: EventReader<RecipeDiscovered>) {
    for event in reader.read() { ... }
}
```

**Observers** — immediate, entity-scoped, reactive:
```rust
commands.spawn((Machine { .. }, Transform::default(), Mesh3d(..), MeshMaterial3d(..)))
    .observe(|trigger: On<Pointer<Click>>, mut commands: Commands| {
        commands.entity(trigger.target()).insert(Selected);
    });

// Stop bubble:
trigger.propagate(false);
```

Use **events** for game logic signals (recipe found, tier unlocked).
Use **observers** for UI/input reactions on specific entities (click machine, hover node).

---

## Picking (Click/Hover on 3D Entities)

```rust
// Add to App
app.add_plugins(DefaultPickingPlugins);
// MeshPickingPlugin handles 3D mesh ray-cast hit detection
app.add_plugins(MeshPickingPlugin);

// Make entity interactive
commands.spawn((
    Mesh3d(mesh_handle),
    MeshMaterial3d(mat_handle),
    Pickable::default(),
))
.observe(|trigger: On<Pointer<Click>>| { /* handle */ })
.observe(|trigger: On<Pointer<Over>>| { /* hover */ });
```

---

## Scene / Save-Load

```rust
// Save
fn save_run(world: &World, type_registry: Res<AppTypeRegistry>) {
    let scene = DynamicSceneBuilder::from_world(world)
        .extract_entities(entity_iter)
        .extract_resources()
        .build();
    let ron = scene.serialize(&type_registry).unwrap();
    std::fs::write("save/run.scn.ron", ron).unwrap();
}

// Load
fn load_run(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<DynamicScene> = asset_server.load("save/run.scn.ron");
    commands.spawn(DynamicSceneRoot(handle));
}
```

Requires `#[derive(Reflect)]` + `#[reflect(Component/Resource)]` on everything saved.

---

## Plugin Pattern

Each game system gets its own plugin:

```rust
pub struct ProductionPlugin;

impl Plugin for ProductionPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<Machine>()
            .register_type::<ProductionState>()
            .init_asset::<RecipeData>()
            .register_asset_loader(RecipeLoader)
            .add_event::<RecipeDiscovered>()
            .add_systems(Startup, load_recipes)
            .add_systems(Update, (
                tick_machines,
                update_belt_flow,
            ).in_set(GameSystems::Simulation)
             .run_if(in_state(GameState::Playing)));
    }
}
```

Top-level `main.rs`:
```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            WorldPlugin,
            ProductionPlugin,
            ResearchPlugin,
            TechTreePlugin,
            UiPlugin,
        ))
        .run();
}
```

---

## 0.18 Breaking Changes (avoid these patterns)

| Wrong | Correct |
|-------|---------|
| `Camera3dBundle { .. }` | `Camera3d` component directly |
| `PbrBundle { .. }` | `(Mesh3d(..), MeshMaterial3d(..))` tuple |
| `next_state.set(S)` to guard no-op | `next_state.set_if_neq(S)` |
| `BorderRadius` as separate component | `BorderRadius` inside `Node` struct |
| `RenderTarget` field on `Camera` | `RenderTarget` as separate required component |
| `EntityDoesNotExistError` | removed — handle via `get_entity()` Result |
| Non-archetypal query `QueryData` bound | `ArchetypeQueryData` bound |

---

## Exergon-Specific Patterns

**Recipe graph as asset**: `.recipe.ron` files, custom `RecipeLoader`. Runtime graph built
by resolving `Handle<RecipeData>` references. Terminal node = escape artifact.

**Tech tree nodes**: `.node.ron` files, custom asset type with `tier_range`, `rarity`,
`unlock_vectors` fields. Node pool for run selected at seed time.

**World seed**: `#[derive(Resource, Reflect)]` so it's included in save files.

**Factory entities**: Flat ECS — many small components, not large structs.
Marker components: `Machine`, `Belt`, `Storage`, `PowerSource`.
Shared data components: `WorldPosition`, `Inventory`, `PowerDraw`.

**Run state machine**: `GameState` at top level; `BuildMode`, `ResearchMode` as SubStates.

**Alien atmosphere**: Use `ScatteringMedium` (0.18) for per-world atmospheric effects tied
to planet physical modifiers (density, color shift for different star distances).

**glTF for machine models**: Load via `SceneRoot(asset_server.load("models/x.glb#Scene0"))`.
Use `GltfExtensionHandler` (0.18) to embed collider/picking metadata in glTF files.

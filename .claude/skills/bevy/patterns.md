# Bevy 0.18 — Plugin Pattern & Exergon Conventions

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
            .add_observer(on_add_machine)
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

## Exergon-Specific Patterns

**Factory entities**: Flat ECS — many small components, not large structs.
- Marker components: `Machine`, `Belt`, `Storage`, `PowerSource`
- Shared data: `WorldPosition`, `Inventory`, `PowerDraw`

**Machine → power source**: Use `PoweredBy`/`ConnectedTo` relationship pattern. Network topology in relationship components, not a separate graph resource.

**Recipe graph as asset**: `.recipe.ron` files, custom `RecipeLoader`. Runtime graph built by resolving `Handle<RecipeData>` references. Terminal node = escape artifact.

**Tech tree nodes**: `.node.ron` files with `tier_range`, `rarity`, `unlock_vectors`. Node pool selected at seed time.

**World seed**: `#[derive(Resource, Reflect)]` + `#[reflect(Resource)]` so it's included in save files.

**Run state machine**: `GameState` at top level; `BuildMode`, `PlayMode` as SubStates.

**glTF for machine models**: `SceneRoot(asset_server.load("models/x.glb#Scene0"))`.

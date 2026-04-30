# Bevy 0.18 — Schedules, Systems, State

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

**Run conditions** — custom conditions are systems returning `bool`:
```rust
fn has_power(grid: Res<PowerGrid>) -> bool { grid.available() > 0.0 }

app.add_systems(Update, tick_machines.run_if(has_power));

// Combinators
.run_if(resource_exists::<GridState>.and(has_power))
.run_if(in_state(PlayMode::Building).or(in_state(PlayMode::Exploring)))
```

**Fallible systems** — return `Result`, use `?`, Bevy handles errors:
```rust
fn load_config(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) -> Result {
    let handle = asset_server.load_or_err("data/config.ron")?;
    commands.insert_resource(ConfigHandle(handle));
    Ok(())
}

// Set global error handler (default panics)
app.set_error_handler(bevy::ecs::error::warn);
```

**Custom `SystemParam`** — bundle multiple params:
```rust
#[derive(SystemParam)]
struct MachineContext<'w, 's> {
    machines: Query<'w, 's, (&'static Machine, &'static mut ProductionState)>,
    recipes: Res<'w, Assets<RecipeData>>,
    power: Res<'w, PowerGrid>,
}

fn tick(mut ctx: MachineContext) { /* use ctx.machines, ctx.recipes, ctx.power */ }
```

## State Management

```rust
#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState { #[default] Loading, Exploring, Building, Researching }

app.init_state::<GameState>();

fn check_loaded(mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::Exploring);          // always fires transition events (0.18)
    // next.set_if_neq(GameState::Exploring); // only fires if value changed
}
```

> **0.18 BREAKING**: `set()` always fires transition events. Use `set_if_neq()` for prior guard behavior.

**Lifecycle schedules**:
```rust
app.add_systems(OnEnter(GameState::Building), spawn_build_ui)
   .add_systems(OnExit(GameState::Building), despawn_build_ui);
```

**Auto-despawn** — takes state *value* not type param:
```rust
commands.spawn((BuildCursor, DespawnOnExit(GameState::Building)));
commands.spawn((LeftMessage, DespawnOnEnter(GameState::Building)));

// Propagates to children automatically
commands.spawn((
    BuildUiRoot,
    DespawnOnExit(GameState::Building),
    children![(Button, Label::new("Place"))],
));
```

**SubStates** — only exist inside a parent state:
```rust
#[derive(SubStates, Default, Clone, Eq, PartialEq, Debug, Hash)]
#[source(GameState = GameState::Building)]
pub enum BuildMode { #[default] Select, Place, Delete }

app.add_sub_state::<BuildMode>();
```

**Exergon state machine**: `GameState` at top level; `BuildMode`, `PlayMode` as SubStates.

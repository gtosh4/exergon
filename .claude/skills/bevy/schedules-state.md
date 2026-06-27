# Bevy 0.19 — Schedules, State

## Ordering Systems
- `after`/`before`: explicit ordering
- `chain`: sequential
- `SystemSet`+`in_set`: Grouping

## Run Conditions
- State: `in_state`, `state_changed`, `state_exists`
- Resource: `resource_changed`
- Other: `on_message`, `and`, `or`, `not`

## State Machine

### Read & Transition
- `Res<State<T>>`: Read current state
- `ResMut<NextState<T>>`: Write next state

### Transition Schedules
Order: `OnExit` → `OnTransition` → `OnEnter`

### State Change Events
- `StateTransitionEvent<T>`: For systems

## SubStates
Active only when parent state matches. Freely mutable via `NextState`.
`State<GamePhase>` resource only exists while `AppState == InGame`.


## Entity Lifecycle Helpers
`DespawnOnExit`, `DespawnOnEnter` components for state change
- **0.19:** these can now fire during *same-state* transitions — review if you re-enter a state.

## Fallible Systems & Executor (0.19 renames)
- Last-resort error handler: `DefaultErrorHandler` → **`FallbackErrorHandler`** (`insert_resource(FallbackErrorHandler(h))`).
- Executor: `ExecutorKind` / `set_executor_kind` removed → `schedule.set_executor(MultiThreadedExecutor::new())`.
- `System::type_id` → `System::system_type`.

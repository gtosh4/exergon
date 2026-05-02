# Bevy 0.18 — Schedules, State

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

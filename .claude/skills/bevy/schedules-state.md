# Bevy 0.18 — Schedules, State
Finite-state machine for game state

## State
Manage states in systems with `State`/`NextState` in queries.

## Schedule & Conditions
Run systems
- continuously in states using condition: `.run_if(in_state(..))`
- on state transitions: `OnEnter`, `OnTransition`, `OnExit`

Order systems when necessary using `.after` (or convenience `.chain`).

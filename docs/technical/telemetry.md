# Telemetry System

ECS components, resource structure, event schema, derived metrics, system logic, and log format for development-build telemetry. Supports VS §6 (Instrumentation) and playtest protocol §7. **VS: JSONL log file per run, debug builds only, no network. MVP: opt-in online reporting (§9).**

Read `vertical_slice.md §6` for the full list of required events and metrics. Read `technical-design.md §1` for seed and run identity.

---

## 1. Overview

Telemetry is a passive observer: it reads existing game events and state, never drives gameplay. All telemetry code is gated behind `#[cfg(debug_assertions)]`.

**Log file:** `{app_data_dir}/telemetry/{run_id}.jsonl`  
**Format:** one JSON object per line (`serde_json`)  
**Flush:** entire buffer written on `OnExit(GameState::Playing)` (run end or quit)

`run_id` = `"{seed_string}-{unix_timestamp_secs}"` — unique per run launch.

### 1.1 Storage Format: JSONL vs SQLite

| | JSONL | SQLite |
|---|---|---|
| Complexity | Zero dep, append-only | Requires `rusqlite`/`sqlx` |
| Readability | `jq`, `cat`, any text editor | Needs sqlite3 CLI or viewer |
| Multi-run queries | Manual concatenate + parse | Native SQL across runs |
| Schema changes | Add fields freely | Migration required |
| Atomic writes | Partial write on crash | WAL mode handles this |
| Single-run analysis | Good | Good |

**VS decision:** JSONL. Simpler, no schema to maintain, easy to inspect during playtests.

---

## 2. Resource: `TelemetryLog`

Inserted on `OnEnter(GameState::Playing)`. Removed on `OnExit(GameState::Playing)` after flush.

```rust
#[derive(Resource)]
pub struct TelemetryLog {
    pub run_id: String,
    pub run_start: std::time::Instant,
    pub records: Vec<serde_json::Value>,

    // production stall tracking (§3.3)
    pub stall_start: Option<f32>,           // elapsed_secs when stall began
    pub blocked_machines: std::collections::HashSet<Entity>, // machines with active RecipeBlocked*

    // Remote mode tracking
    pub remote_entry_t: Option<f32>,   // elapsed_secs when DronePilot mode entered
    pub remote_trips: u32,
    pub pending_re_engage: bool,        // true between RemoteModeExit and first Running machine
}
```

---

## 3. Event Schema

Each record is a JSON object `{"t": <elapsed_secs>, "event": "<Name>", ...fields}`.

`t` is seconds since `TelemetryLog.run_start` (f32, two decimal places).

### 3.1 Run lifecycle

| Event | Additional fields | Source |
|---|---|---|
| `RunStarted` | `seed`, `profile` (run type string), `unix_ts` | `OnEnter(GameState::Playing)` |
| `EscapeCompleted` | `escape_time_secs` (from `EscapeEvent`) | `EscapeEvent` |
| `RunAbandoned` | *(none)* | `OnExit(GameState::Playing)` without `EscapeCompleted` |

### 3.2 Repeated events

| Event | Additional fields | Source |
|---|---|---|
| `MachinePlaced` | `machine_type`, `grid_pos` | `WorldObjectEvent` machine-placed action |
| `MachineRemoved` | `machine_type`, `grid_pos` | `WorldObjectEvent` machine-removed action |
| `RecipeStarted` | `machine_id`, `recipe_id` | `JobStarted` |
| `RecipeFinished` | `machine_id`, `recipe_id`, `duration_secs`, `output_count` | `JobComplete` |
| `PropertyViewed` | `property`, `context` | `PlanetPropertyViewed` |
| `TechRevealed` | `node_id` | `TechTreeProgress.unlocked_nodes` change (each new entry) |
| `ResearchSpent` | `type_id`, `amount` | `UnlockNodeRequest` with `ResearchSpend` vector |
| `Discovery` | `key` | `DiscoveryEvent` |
| `EscapeItemProduced` | `item_id` | `JobComplete` with output on machine with `EscapeObjective` |
| `ProductionStalledStart` | `blocked_machine_count` | `RecipeBlocked*` fires AND no machine `Running` (§5.9) |
| `ProductionStalledEnd` | `duration_secs` | Any machine transitions to `Running` while stall active |
| `PowerNetworkFailure` | `reason` (`"voltage"` \| `"amps"`), `affected_count` | `RecipeBlockedVoltage` / `RecipeBlockedAmps` |
| `PowerNetworkRestored` | `downtime_secs` | Machine un-blocks after `PowerNetworkFailure` |
| `LogisticStall` | `route_key`, `reason` | `RecipeBlockedInputs` / `RecipeBlockedOutputs` |
| `RemoteModeEntry` | `trip_n` (1-indexed) | `OnEnter(PlayMode::DronePilot)` |
| `RemoteModeExit` | `trip_n`, `duration_secs` | `OnExit(PlayMode::DronePilot)` |

**Production stall definition:** Stall begins when ≥1 machine has an active `RecipeBlocked*` event and zero machines are `Running`. Stall ends when any machine enters `Running`. Pure idle (no recipe configured) does NOT trigger a stall — machines must have a recipe assigned to count.

---

## 4. Systems

All systems are registered only when `#[cfg(debug_assertions)]`. They run in `GameSystems::Simulation` order unless noted.

### 4.1 `telemetry_run_start` — `OnEnter(GameState::Playing)`

1. Compute `run_id` from `SeedResource` and current Unix timestamp.
2. Insert `TelemetryLog` resource.
3. Append `RunStarted { seed, profile, unix_ts }`.

`profile` from run configuration (e.g., `"Initiation"`, `"StandardProbe"`); use `"Unknown"` if unavailable VS.

### 4.2 `telemetry_observe_property` — `Update` in `GameState::Playing`

Reads `EventReader<PlanetPropertyViewed>`. Append `PropertyViewed { property, context }` for each event.

### 4.3 `telemetry_observe_research` — `Update` in `GameState::Playing`

Reads `EventReader<UnlockNodeRequest>` and monitors `TechTreeProgress.unlocked_nodes` via `Changed<TechTreeProgress>`.

- On `UnlockNodeRequest` with `ResearchSpend` vector: append `ResearchSpent { type_id, amount }`.
- On `Changed<TechTreeProgress>`: for each newly added entry in `unlocked_nodes`, append `TechRevealed { node_id }`. Track previous snapshot to diff against new entries.

### 4.4 `telemetry_observe_machines` — `Update` in `GameState::Playing`

Reads `EventReader<WorldObjectEvent>` and queries `Query<&MachineState, Changed<MachineState>>`.

- On machine-placed `WorldObjectEvent`: append `MachinePlaced { machine_type, grid_pos }`.
- On machine-removed `WorldObjectEvent`: append `MachineRemoved { machine_type, grid_pos }`.
- On `MachineState` change to `Running`: if `pending_re_engage`, clear flag (re-engagement happened; `RemoteModeEntry` timestamp in log is sufficient for post-processing).

### 4.5 `telemetry_observe_jobs` — `Update` in `GameState::Playing`

Reads `EventReader<JobStarted>`, `EventReader<JobComplete>`, `Query<(), With<EscapeObjective>>`.

- On `JobStarted`: append `RecipeStarted { machine_id, recipe_id }`.
- On `JobComplete`: append `RecipeFinished { machine_id, recipe_id, duration_secs, output_count }`. If machine has `EscapeObjective`: also append `EscapeItemProduced { item_id }`.

`JobStarted` must be confirmed or added in the crafting system.

### 4.6 `telemetry_observe_power_and_logistics` — `Update` in `GameState::Playing`

Reads `EventReader<RecipeBlockedVoltage>`, `EventReader<RecipeBlockedAmps>`, `EventReader<RecipeBlockedInputs>`, `EventReader<RecipeBlockedOutputs>`.

**Power failures:**
- On voltage/amps block: add machine to `blocked_machines`. Append `PowerNetworkFailure { reason, affected_count: blocked_machines.len() }`.
- Track `power_failure_active: bool`; when machine un-blocks after failure, append `PowerNetworkRestored { downtime_secs }`.

**Logistics stalls:**
- On `RecipeBlockedInputs` / `RecipeBlockedOutputs`: add machine to `blocked_machines`. Append `LogisticStall { route_key: "<machine_id>/<slot>", reason }` (deduplicate same route within frame).

### 4.7 `telemetry_observe_discovery` — `Update` in `GameState::Playing`

Reads `EventReader<DiscoveryEvent>`. Append `Discovery { key }` for each event.

### 4.8 `telemetry_observe_remote_mode`

- **`OnEnter(PlayMode::DronePilot)`:** Increment `remote_trips`. Set `remote_entry_t = elapsed`. Append `RemoteModeEntry { trip_n: remote_trips }`.
- **`OnExit(PlayMode::DronePilot)`:** Append `RemoteModeExit { trip_n, duration_secs: elapsed - remote_entry_t }`. Set `pending_re_engage = true`.

### 4.9 `telemetry_production_stall` — `Update` in `GameState::Playing`

Runs **after** §4.6. Queries `Query<&MachineState>`, reads `blocked_machines`.

**Stall begins:** `blocked_machines` non-empty AND zero machines `Running` AND `stall_start.is_none()`.
- Set `stall_start = Some(elapsed)`. Append `ProductionStalledStart { blocked_machine_count }`.

**Stall ends:** `stall_start.is_some()` AND any machine transitions to `Running`.
- Append `ProductionStalledEnd { duration_secs: elapsed - stall_start }`. Clear `stall_start`.

Each frame: remove from `blocked_machines` any entity now `Running` or despawned.

### 4.10 `telemetry_observe_escape` — `Update` in `GameState::Playing`

Reads `EventReader<EscapeEvent>`. Append `EscapeCompleted { escape_time_secs }`. Set `TelemetryLog.escaped = true`.

### 4.11 `telemetry_run_end` — `OnExit(GameState::Playing)`

1. If `stall_start.is_some()`: append `ProductionStalledEnd { duration_secs }`.
2. If not `escaped`: append `RunAbandoned`.
3. Create `telemetry/` directory if absent. Write `records` as JSONL to `{run_id}.jsonl`.
4. Remove `TelemetryLog` resource.

---

## 5. Execution Order

```
OnEnter(GameState::Playing)
└── telemetry_run_start

Update (GameState::Playing, GameSystems::Simulation)
├── telemetry_observe_property
├── telemetry_observe_research
├── telemetry_observe_machines
├── telemetry_observe_jobs
├── telemetry_observe_power_and_logistics  // populates blocked_machines
├── telemetry_observe_discovery
├── telemetry_observe_escape
└── telemetry_production_stall             // reads blocked_machines, must run last

OnEnter(PlayMode::DronePilot)
└── telemetry_observe_remote_mode (entry half)

OnExit(PlayMode::DronePilot)
└── telemetry_observe_remote_mode (exit half)

OnExit(GameState::Playing)
└── telemetry_run_end
```

---

## 6. Edge Cases

**No blocked machines when stall check runs:** `telemetry_production_stall` skips if `blocked_machines` is empty. Stall only meaningful once at least one machine has a recipe and fires `RecipeBlocked*`.

**Player quits via OS kill:** `OnExit` does not fire. Log is lost. Acceptable for VS — use in-game quit only during playtests.

**Remote re-engage: player enters Remote mode again before any machine runs:** `pending_re_engage` stays `true` from prior trip. New `RemoteModeEntry` overwrites `remote_entry_t`. In post, correlate `RemoteModeExit` → next `MachineState Running` transition time from `RecipeStarted` timestamps.

**Multiple `JobComplete` in same frame:** all processed and logged — no deduplication needed.

**`TechRevealed` diff across frames:** system must snapshot `unlocked_nodes` each frame and diff against previous to detect new entries, since the set is not an event stream.

---

## 7. Integration Tests

**Test 1 — RunStarted written on PlayState entry**  
Trigger `OnEnter(GameState::Playing)`. Assert `TelemetryLog.records[0].event == "RunStarted"`.

**Test 2 — MachinePlaced repeated**  
Send two machine-placed `WorldObjectEvent` messages. Assert exactly two `MachinePlaced` records in `TelemetryLog.records`.

**Test 3 — Production stall interval**  
Spawn two machines with recipes. Send `RecipeBlockedInputs` for both (populates `blocked_machines`). Run `telemetry_production_stall`. Assert `ProductionStalledStart` emitted and `stall_start.is_some()`. Set one machine to `Running` (clears it from `blocked_machines`). Run again. Assert `ProductionStalledEnd` emitted, `stalled_intervals.len() == 1`.

**Test 4 — Remote trip count**  
Enter and exit `PlayMode::DronePilot` three times. Assert `remote_trips == 3`, three `RemoteModeEntry` records, three `RemoteModeExit` records.

**Test 5 — RunAbandoned last record on exit without escape**  
Trigger `OnExit(GameState::Playing)` without prior `EscapeCompleted`. Assert last record `event == "RunAbandoned"`.

---

## 8. VS vs MVP Scope

**VS:** All events and derived metrics above. JSONL file output. `#[cfg(debug_assertions)]` gate. No UI for viewing telemetry in-game.

**MVP additions:**
- Opt-in telemetry for external playtesters (feature flag, not `debug_assertions`).
- Disclosure / opt-out flow for external builds.
- Multi-run aggregate report: import JSONL → shared `telemetry.sqlite`; query with SQL.
- Session replay markers (structured timestamps for video sync with playtest recordings).
- **Online reporting** — options to evaluate:

  | Option | Cost | Notes |
  |---|---|---|
  | **GameAnalytics** | Free tier | Game-specific REST API, designed for indie. Rust: HTTP POST with `reqwest`. No SDK. |
  | **PostHog** | Free up to 1M events/mo, self-hostable | Open source, strong privacy story. REST capture API straightforward. |
  | **Custom endpoint** | Server cost only | Full control, simplest schema. Needs a machine to receive + store. |
  | **Amplitude / Mixpanel** | Freemium | More marketing-oriented; less suited to low-level game telemetry. |

  **Recommendation:** PostHog self-hosted (single Docker container) or GameAnalytics free tier. Both accept HTTP POST event batches with no custom SDK. Implement as async fire-and-forget on `RunEnd` using `reqwest`. Gate behind opt-in feature flag. No blocking, no retry — VS playtests are controlled environments.

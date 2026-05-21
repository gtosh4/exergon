# Save Architecture

Single canonical run save (continuous, atomic-write), sparse milestone checkpoints, opt-in hardcore mode. RON format via `moonshine-save`. Covers: save model rationale, Run entity (marker requires `Save`, lifetime, components), saveable entity inventory (machines, cables, networks, tech tree), run save (file path, header, checkpoints), meta save (file path, triggers, contents), save/load flow, cloud saves placeholder, VS scope.

**Read before:** implementing `src/save/`, touching `moonshine_save::Save`/`Unload` tagging, adding new run-scoped global state, or implementing the main menu run-select flow.

---

## Table of Contents

1. [Save Model](#1-save-model)
2. [Run Entity](#2-run-entity)
3. [ECS Save Tags](#3-ecs-save-tags)
4. [Run Save](#4-run-save)
5. [Meta Save](#5-meta-save)
6. [Save / Load Flow](#6-save--load-flow)
7. [Cloud Saves](#7-cloud-saves)
8. [Systems](#8-systems)
9. [Edge Cases](#9-edge-cases)
10. [Integration Test Invariants](#10-integration-test-invariants)
11. [VS / Post-VS Scope](#11-vs--post-vs-scope)

---

## 1. Save Model

**Hybrid: continuous primary save + sparse milestone checkpoints. Hardcore mode opt-in (post-VS).**

### Why hybrid

- **Roguelite tension preserved.** Checkpoints occur at tier boundaries only, never inside the science discovery loop. Players cannot reload to re-roll research reveals, recipe parameters, or scouting finds. The seed and its derived state are committed once and stick.
- **Factory-game safety expected.** Standard runs are 10–15h and Pinnacle runs 30–50h+ ([`gdd.md §4`](../gdd.md)). Players require a recovery path from crash, corruption, or catastrophic in-factory mistake. A single-timeline model with no escape hatch is hostile at these run lengths.
- **Matches permadeath stance.** [`gdd.md §16`](../gdd.md) makes permadeath opt-in and post-MVP. The default model must support recovery; hardcore mode strips it.
- **Aligns with point-buy meta-progression.** Hardcore mode becomes a [`gdd.md §14`](../gdd.md) challenge modifier — disables checkpoints, awards challenge points.

### What the model offers

| Mechanism | When | Player control |
|---|---|---|
| Primary save | Continuous; written on triggers and on every checkpoint write | Always on |
| Milestone checkpoints | Auto-written on tier unlock and on "escape construction begins" | Read-only; never overwritten until run ends |
| Manual checkpoint | Player-named, single slot per run, overwritable | One slot only; no save-spam |
| Rolling backups | Hidden ring buffer of last N primary save writes | Exposed only on corruption-recovery flow |
| Hardcore mode (post-VS) | Disables checkpoints + manual checkpoint; primary save only | Opt-in at run start; refunds challenge points |

### Why not pure Minecraft-style

- 20-hour run with no checkpoint = save-corruption risk is unacceptable
- Factory-genre audience expects safety net
- "Decisions stick" without recovery = anxiety, not tension

### Why not full Factorio-style

- Arbitrary save slots enable save-scumming research RNG and unlock-vector reveals — undermines [`gdd.md §6`](../gdd.md) discovery loop and [`gdd.md §5`](../gdd.md) seed integrity
- Slot management UI is wasted surface for a single-run-at-a-time game
- Cloud sync N-way conflict resolution adds disproportionate complexity

### Checkpoint trigger events

| Event | Source |
|---|---|
| Tier unlocked | `TechTreeProgress` advances to a tier boundary |
| Escape construction starts | First escape-artifact prerequisite item produced |
| Player triggers manual checkpoint | Pause menu → Checkpoint |

Auto-checkpoint files are read-only from the run-select UI: the player can load them but not overwrite them. The primary save continues independently.

---

## 2. Run Entity

A single entity spawned at run start (new game or load). Carries all run-scoped global components. ECS anchor for data that belongs to the run as a whole rather than to any world object (machines, deposits, player position).

```rust
/// Marker component. Query `With<Run>` to reach run-scoped globals.
///
/// `#[require(Save)]` follows `moonshine-save`'s pattern for grouping
/// marker components: any entity with `Run` is automatically tagged for
/// serialization. Same pattern applies to `Planet`, `TechTree`,
/// `LogisticsNetwork`, `PowerNetwork`, and other marker components that
/// identify save-game entity groups. Data components do not need this
/// (see `moonshine-save` README).
#[derive(Component, Reflect)]
#[require(Save)]
pub struct Run;
```

**Lifetime:**
- **Spawned:** when the player starts a new run or loads an existing one. Spawned before any generation systems run.
- **Despawned:** when the run ends (escape completed) or the player returns to the main menu. On return-to-menu, the run remains `InProgress` on disk and is offered for resume next launch. Despawn flushes all in-memory run-scoped state.

### Components on the Run entity

| Component | Source doc | Purpose |
|---|---|---|
| `Run` | this doc | Marker for `With<Run>` queries; `#[require(Save)]` |
| `RunSaveHeader` | this doc | run_id, seed string, difficulty, status, timestamps |
| `RunSeed` | `seed.md` | Master seed (text + u64 hash) |
| `DomainSeeds` | `seed.md` | Per-domain sub-seeds |
| `TechTreeProgress` | `research.md` | Unlocked nodes, recipes, machines |
| `ResearchPool` | `research.md` | Accumulated research points by type |
| `HardcoreMode` | this doc (post-VS) | Marker; disables checkpoint systems when present |

All components are `Reflect`-registered and derive serde traits for RON serialization. All persist in the run save.

> **TechTreeProgress and ResearchPool placement note:** these are run-scoped globals, not player-physical state. Stay on the Run entity. If multiplayer is added post-VS, they can migrate to per-Player or per-Team entities with the same component-based pattern — no resource-to-component refactor required.

> **Supersedes `research.md §2`:** that section places `TechTreeProgress` and `ResearchPool` on the player entity. They belong on the Run entity. `research.md §2` should be updated to match.

### RunSaveHeader

```rust
#[derive(Component, Reflect, Clone)]
pub struct RunSaveHeader {
    /// Opaque base58-encoded id; see `run_id derivation` below.
    pub run_id: String,
    /// Display seed — same as RunSeed.text.
    pub seed_text: String,
    pub difficulty: DifficultyTier,
    pub status: RunStatus,
    pub start_time_secs: u64,
    pub end_time_secs: Option<u64>,
    /// Accumulated play time in seconds; updated on each save write.
    pub total_playtime_secs: f64,
    /// Auto and manual checkpoints written for this run.
    pub checkpoints: Vec<CheckpointHeader>,
}

#[derive(Reflect, Clone)]
pub struct CheckpointHeader {
    pub kind: CheckpointKind,
    pub created_at_secs: u64,
    pub label: String,
    pub file_name: String,
}

#[derive(Reflect, Clone, PartialEq, Eq)]
pub enum CheckpointKind {
    TierUnlock(u8),
    EscapeConstructionStart,
    Manual,
}

#[derive(Reflect, Clone, PartialEq, Eq, Default)]
pub enum RunStatus {
    #[default]
    InProgress,
    /// Player completed the escape sequence. Save retained read-only;
    /// meta-progression payout (difficulty ladder unlock, narrative,
    /// challenge points) granted exactly once at the moment of completion.
    Completed,
}

#[derive(Reflect, Clone, PartialEq, Eq, Default)]
pub enum DifficultyTier {
    #[default]
    Standard,
    // post-VS: additional tiers
}
```

### run_id derivation

VS: `base58(blake3(start_time_secs).truncate(8 bytes))`. Opaque, readable, unique per run.

> **Post-VS:** the inputs are arbitrary — `run_id` is treated as a pure identifier with no semantic content. If we want stronger semantic collision resistance later (e.g. distinguishing replays of identical seed + planet identity + difficulty), the input set can be expanded — e.g. `blake3(seed_text || planet_name || difficulty || start_time_secs)` — without breaking on-disk save compatibility, since `run_id` is only used as the file name and is opaque to load.

`run_id` is set once at spawn and never changes. Primary save file: `saves/runs/{run_id}.ron`.

---

## 3. ECS Save Tags

`moonshine-save` serializes entities tagged `Save` and despawns entities tagged `Unload` before loading. Marker components for save-game entity groups should use `#[require(Save)]` so any entity carrying the marker is automatically tagged.

### Tagged `Save` (persisted to run save)

| Entity type | Marker(s) using `#[require(Save)]` | What's saved |
|---|---|---|
| **Run entity** | `Run` | All components (§2) |
| **Planet entity** | `Planet` | `PlanetProperties`, `PlanetPropertyVisibility` (see `planet-identity.md`) |
| **TechTree entity** | `TechTree` | The run's realized tech tree: node set, per-node unlock vectors, tier shadows. Saved because node selection and unlock vectors are seeded per run; protects against content-pool drift between game versions. Separate from `TechTreeProgress` (player progress, on Run entity). |
| **Logistics Network entities** | `LogisticsNetwork` | `LogisticsNetworkMembers` (member entity list) |
| **Power Network entities** | `PowerNetwork` | `PowerNetworkMembers` (member entity list, voltage tier, amp capacity) |
| Machine entities | `Machine` (post-VS placeable marker) | `Machine`, `MachineTier`, `MachineJobPolicy`, `RecipeProcessor`, installed module components, port bindings, `LogisticsPortOf`/`EnergyPortOf` references |
| Logistics cable entities | `LogisticsCableSegment` | `LogisticsCableSegment` (from, to, path), `LogisticsNetworkMember(Entity)` |
| Power cable entities | `PowerCableSegment` | `PowerCableSegment`, `PowerNetworkMember(Entity)` |
| Storage unit entities | (covered by `Machine`) | `StorageUnit { items }` |
| Generator entities | (covered by `Machine`) | `GeneratorUnit { pos, voltage_tier, watts, buffer_joules, max_buffer_joules }` |
| Deposit entities | `MinedDeposit` | Only deposits **modified** by mining or miner attachment. Unmodified deposits regenerate from seed on chunk load; not saved. |
| Outpost structure entities | `Outpost` | Beacon position, aegis field state, all components |
| Drone entities | `Drone` | Position, tier, mode, inventory |
| Player entity | `Player` | `Transform` (position only); physics and rendering components excluded via component filter |

Network membership components (`LogisticsNetworkMember(Entity)`, `PowerNetworkMember(Entity)`) on cable, port, and generator entities **are saved**. After load, network entity ids are remapped by `moonshine-save`; membership references are restored intact. No topology rebuild required on load — saved membership is authoritative.

### Tagged `Unload` (despawned before load)

- Chunk terrain mesh entities
- Particle effects
- Camera rig
- All UI entities
- Transient visual entities (cable overlays, placement ghosts)

`Unload` entities are despawned before the load pass. After load, chunk streaming re-spawns terrain around the restored player `Transform`.

### Resources (not serialized)

Re-initialized on load; never part of any save file. Either content-derived (regenerable from seed + asset pool) or transient.

| Resource | Why not saved |
|---|---|
| `RecipeGraph` | Generated deterministically from seed + content asset pool. Regenerated on load. |
| Content asset handles (`TechTreeContentPool`, recipe templates, item defs) | Loaded from disk asset at startup; not run-state. |
| All UI state resources | Transient view state. |
| Chunk streaming caches | Rebuilt from player position after load. |

> **Note on `RecipeGraph`:** like `TechTree`, the realized graph is seeded per run. Unlike `TechTree`, the graph is a pure projection from `(seed, RunSeed, content asset)` with no player-affected state; regeneration on load is deterministic and lossless. If content assets change between save and load, the graph regeneration may differ from the original — same risk as `TechTree`, addressed there by save-on-disk. If the same risk becomes load-bearing for `RecipeGraph`, promote to a `RecipeGraph` entity with `#[require(Save)]`.

---

## 4. Run Save

### File layout

```
saves/
  runs/
    {run_id}/
      run.ron                         ← primary save (continuous)
      checkpoints/
        tier_{N}.ron                  ← auto checkpoint, tier unlock
        escape_start.ron              ← auto checkpoint, escape construction
        manual.ron                    ← single manual checkpoint slot
      backups/
        run.ron.bak.0                 ← rolling backups of primary (N=3)
        run.ron.bak.1
        run.ron.bak.2
```

The directory is created at first save if absent. Runs are **never automatically deleted**; the player manages them from the run-select screen.

### Format

RON via `moonshine-save` (v0.6.1, Bevy 0.18 compatible). All `Save`-tagged entities and their `Reflect`-registered components are written. No SQLite, no custom binary format.

### Primary save triggers

| Trigger | When |
|---|---|
| **Manual** | Player opens pause menu → Save |
| **Periodic auto** | Every N seconds of in-game time during `InProgress` (default 60s; tunable) |
| **Checkpoint** | Any checkpoint-write event also flushes the primary save |
| **Exit save** | Player opens pause → Quit; save completes before process exits |

Periodic auto + manual + checkpoint-piggyback collectively cover the "continuous timeline" contract. No multiple save slots per run.

### Checkpoint writes

| Trigger | File | Behavior |
|---|---|---|
| `TechTreeProgress` advances to tier boundary | `checkpoints/tier_{N}.ron` | Written once per tier; never overwritten. Read-only thereafter. |
| First escape-artifact prerequisite item produced | `checkpoints/escape_start.ron` | Written once per run. Read-only thereafter. |
| Player pause menu → Checkpoint | `checkpoints/manual.ron` | Single slot; overwrites previous manual checkpoint for this run. Always overwritable while `InProgress`. |

Checkpoint files contain a full snapshot — same content as primary save at that instant. Loading a checkpoint replaces all `Save`-tagged entities with the checkpoint state and continues from there; the primary save is then overwritten on next save trigger.

### Rolling backups

On every primary save write: rename `run.ron` → `run.ron.bak.0`, shifting `.bak.0` → `.bak.1` → `.bak.2` → discard. Then write the new `run.ron` atomically (temp file + rename).

Backups are hidden from the run-select UI. Surfaced only when the primary save fails to deserialize — then the player is offered a recovery flow with timestamps and a "use backup N" choice.

### Header-only reads

`RunSaveHeader` is the first component serialized on the Run entity. The run-select screen reads only the header (not the full save) to populate the run list and its checkpoints. File path is deterministic from `run_id`, so the header can be deserialized without loading the rest.

### Completed runs

Runs with `RunStatus::Completed` remain on disk. Loading a completed run is read-only — `trigger_save_system` and all checkpoint systems skip writes when status is not `InProgress`. Players can inspect completed runs without altering save state.

### Player wants out of a run

Per [`gdd.md §16`](../gdd.md), there is no forced failure and every run is completable. Players who decide they are done with an `InProgress` run have two paths:

- **Quit to menu.** Run stays `InProgress`; appears under "Resume run" next launch. Player may come back later.
- **Delete run.** Run-select UI offers per-run delete, which removes the run directory (primary save, checkpoints, backups). Player must confirm. There is no soft-archive: the run is gone.

There is no `Abandoned` status. Difficulty-ladder unlock, narrative, and challenge points are gated on `Completed` per [`gdd.md §14`](../gdd.md); a deleted or abandoned-in-the-colloquial-sense run grants none of these. Codex entries earned mid-run remain in `meta.ron` (they were written when earned), and player-saved blueprints persist independently of run state.

---

## 5. Meta Save

### File location

```
saves/
  meta.ron   ← single file; persists across all runs
```

### Contents

| Component | Description |
|---|---|
| `Codex` | Discovered entries (biomes, machines, materials, planet modifiers, alien organisms). Accumulated across all runs. |
| `UnlockedContent` | Meta-progression grants: run modifiers, narrative fragments, blueprint slots, starting boons pool, challenge-point balance. |
| `Blueprints` | Layout templates (machine positions, tiers, orientations, logistics connections). No recipe data. Slot count expandable via meta-progression. |

### Update triggers

Meta save is written:
- On run completion (escape triggered)
- On mid-run milestone triggers (first codex entry, defined discovery events)

Mid-run milestone writes are immediate. Codex entries and blueprints earned mid-run persist regardless of whether the run is later completed, quit-and-resumed, or deleted. Each milestone trigger is idempotent on its target entry in `Codex` / `Blueprints`: re-firing the same trigger on a resumed run is a no-op. This decouples meta-progression from run lifecycle and avoids double-grant bugs.

Meta save is not written on manual saves, periodic auto-saves, exit saves, or checkpoint writes during an in-progress run (only run save is written then).

If `meta.ron` is absent on startup, `Codex`, `UnlockedContent`, and `Blueprints` are initialized to defaults (all empty). This is not an error.

---

## 6. Save / Load Flow

### New run

1. Player submits seed and difficulty on new-run screen.
2. Spawn Run entity with: `Run` (auto-tagged `Save`), `RunSaveHeader` (new `run_id`, `status: InProgress`, `start_time_secs: now`, empty `checkpoints`), `RunSeed` (from input), `DomainSeeds::from_master(run_seed.hash)`, `TechTreeProgress::default()`, `ResearchPool::default()`. If hardcore was selected, also insert `HardcoreMode`.
3. Spawn Planet entity (`Planet` + `PlanetProperties` + `PlanetPropertyVisibility`).
4. Spawn TechTree entity (`TechTree` + realized node set and unlock vectors).
5. Spawn Player entity, spawn initial terrain chunks.
6. Trigger primary save immediately (valid save exists if player quits on first frame).
7. Transition to `GameState::Playing`.

### Save (primary)

1. `update_playtime_system`: accumulate elapsed time into `RunSaveHeader.total_playtime_secs`.
2. `trigger_save_system`: if status is `InProgress`, rotate `backups/run.ron.bak.{0..N}`, then write all `Save`-tagged entities to `saves/runs/{run_id}/run.ron` (atomic temp-rename).
3. If run just ended: `meta_save_system` writes `saves/meta.ron`.

### Checkpoint write

1. Detect trigger (tier unlock, escape construction start, or manual button).
2. If `HardcoreMode` is present on Run entity: skip silently.
3. Write all `Save`-tagged entities to the appropriate checkpoint file path.
4. Append a `CheckpointHeader` to `RunSaveHeader.checkpoints`.
5. Trigger a primary save write (carries the updated header).

### Load

1. Player selects a run on run-select screen. The screen has already read each run's `RunSaveHeader` for display.
2. Player picks a file: primary save or one of the checkpoint files.
3. Despawn all `Unload`-tagged entities (terrain, visuals, UI).
4. Clear transient resources (UI state, chunk caches).
5. Regenerate `RecipeGraph` from `(RunSeed, content asset)`.
6. `moonshine_save::load`: restore all `Save` entities from the chosen RON file.
7. `post_load_init_system`: re-init camera position from player `Transform`, reset chunk streaming bounds, validate network membership references resolved to valid entities.
8. Transition to `GameState::Playing`.

Loading a checkpoint sets the in-memory state to that snapshot. The primary save (`run.ron`) is **not** overwritten until the next save trigger fires; if the player immediately quits after loading a checkpoint without playing, the primary save retains its prior state. If they play and a save trigger fires, the primary save is overwritten with the checkpoint-derived state.

### Run end

`RunEndEvent` fires only on escape-sequence completion. Quit-to-menu and run-delete are not run-end events; they do not run this flow.

1. `set_run_status_system`: set `RunSaveHeader.status = Completed`, set `end_time_secs`.
2. Trigger primary save (writes final run state and updated header).
3. `meta_save_system`: write `saves/meta.ron` (difficulty-ladder unlock, narrative, challenge-point grant).
4. Despawn Run entity.
5. Transition to main menu.

### Quit to menu

1. Trigger primary save (status remains `InProgress`).
2. Despawn Run entity (in-memory only; save file persists).
3. Transition to main menu.

### Delete run

Triggered from run-select UI with confirmation. Removes `saves/runs/{run_id}/` recursively (primary save, checkpoints, backups). `meta.ron` is untouched. No grant or rollback.

---

## 7. Cloud Saves

**VS:** Not implemented. Save files are local only under the OS-standard game data directory.

**Post-VS (Steam):** On each local save write (primary or checkpoint), sync the run save directory and `meta.ron` to Steam Remote Storage via the `steamworks` crate. On first launch, compare local and cloud timestamps; if cloud is newer, prompt the player to choose (auto-prefer cloud is the default). Conflict resolution is a player choice — no automatic merge. Save file paths and format are unchanged by cloud support; sync is a post-write layer.

---

## 8. Systems

| System | When | Description |
|---|---|---|
| `spawn_run_system` | On `NewRunEvent` or load | Spawn Run entity with all §2 components |
| `update_playtime_system` | `SaveSystems::PreSave` | Add frame delta to `total_playtime_secs` |
| `trigger_save_system` | `SaveSystems::PreSave` | Detect triggers (manual, periodic, exit, checkpoint-piggyback); rotate backups; write primary save if `InProgress` |
| `tier_unlock_checkpoint_system` | After `TechTreeProgress` mutation | If new tier crossed and not `HardcoreMode`: write `tier_{N}.ron`; piggyback primary save |
| `escape_start_checkpoint_system` | After `EscapeConstructionEvent` | If first escape prerequisite item produced and not `HardcoreMode`: write `escape_start.ron`; piggyback primary save |
| `manual_checkpoint_system` | On pause menu `CheckpointButtonPressed` | If not `HardcoreMode`: overwrite `manual.ron`; piggyback primary save |
| `meta_save_system` | On `RunEndEvent` | Write meta save |
| `load_run_system` | On `LoadRunEvent` | Execute load sequence; argument carries chosen file (primary or checkpoint) |
| `post_load_init_system` | After `moonshine_save::load` | Re-init camera; validate network membership refs; trigger chunk re-stream |
| `despawn_run_system` | On `RunEndEvent` | Set status, save, despawn Run entity |

### Execution order

```
SaveSystems::PreSave
  → update_playtime_system
  → tier_unlock_checkpoint_system
  → escape_start_checkpoint_system
  → manual_checkpoint_system
  → trigger_save_system

On RunEndEvent (ordered):
  → despawn_run_system   (sets status, fires save)
  → meta_save_system
```

Checkpoint systems run before `trigger_save_system` so the appended `CheckpointHeader` is included in the primary save written this frame.

---

## 9. Edge Cases

1. **Crash mid-save:** primary save uses atomic temp-rename. The previous `run.ron` and the rolling `run.ron.bak.*` files remain intact if the process crashes during write. `moonshine-save` flush behavior: verify it provides atomic semantics; if not, the temp-rename wrapper around its writer is the load-bearing guarantee.

2. **Corrupted primary save on load:** load fails → player offered backup-recovery flow with `run.ron.bak.0..N` timestamps. Player picks one; that file is loaded and renamed back to `run.ron`. Surfaced UI is intentionally minimal — this is a rare path.

3. **Save on first frame:** the initial primary save contains a valid Run entity with default progress. World has no machines or mined deposits. Load of this save regenerates terrain from seed (chunk streaming) and finds no Machine/MinedDeposit entities to restore.

4. **Multiple runs open on disk:** each run directory is independent. Loading run B replaces all `Save` entities; run A's directory is untouched. Meta save is shared across all runs.

5. **Visiting a completed run:** all checkpoint systems and `trigger_save_system` check `RunSaveHeader.status != InProgress` and skip all writes. The player can explore freely without changing any file.

6. **Player quits immediately:** exit-save fires before process exit. If the game receives SIGKILL, the most recent primary save (or backup) is offered on next launch. Runs with `InProgress` status are listed under "Resume run" on the main menu.

7. **Crashed/orphaned run on next launch:** any `InProgress` run on disk is offered for resume. Player who never wants to resume a given run deletes it from the run-select UI.

8. **`saves/` directory missing:** created at first save write. `meta.ron` missing → defaults. Neither is an error.

9. **Hardcore mode:** `HardcoreMode` component on Run entity gates all checkpoint systems off. Only primary save + rolling backups remain. Backups can be disabled too (post-VS configurable; default keeps them — backups are crash protection, not save-scumming).

10. **Loading a checkpoint after the primary save advanced further:** allowed. Player is rewinding to an earlier state. Next save write overwrites `run.ron` with checkpoint-derived state. No warning needed — checkpoint files clearly show their trigger and timestamp in the load UI.

11. **Save schema mismatch (post-VS):** pre-release: no versioning, load may behave incorrectly if generation code changed. Handle case-by-case. Post-release: add schema version to `RunSaveHeader`, warn player on load if version mismatches, allow continue at own risk.

---

## 10. Integration Test Invariants

1. New run: Run entity exists with `Run`, `RunSaveHeader`, `RunSeed`, `DomainSeeds`, `TechTreeProgress`, `ResearchPool`. Planet and TechTree entities exist.
2. `RunSaveHeader.status == InProgress` immediately after new run spawn.
3. `RunSaveHeader.run_id` is non-empty, base58, and matches the save directory name.
4. Primary save → load round-trip: `RunSeed.text` and `RunSeed.hash` are identical.
5. Primary save → load round-trip: `TechTreeProgress.unlocked_nodes` is identical.
6. Primary save → load round-trip: Machine entities are restored at same world positions.
7. Primary save → load round-trip: Network entity membership is preserved (every cable's `LogisticsNetworkMember(Entity)` resolves to a Network entity whose `LogisticsNetworkMembers` includes the cable).
8. After load: no `Unload`-tagged entities exist in the world.
9. `trigger_save_system` does not write when `RunStatus == Completed`.
10. `meta_save_system` is called exactly once on run end; not called during mid-run manual saves, periodic saves, or checkpoint writes.
11. Loading from a missing file returns a handled error, not a panic.
12. Two runs with different seeds produce two independent save directories.
13. `total_playtime_secs` strictly increases on each primary save write during an `InProgress` run.
14. Tier-unlock checkpoint: crossing a tier boundary writes `tier_{N}.ron` exactly once; subsequent tier-unlock events for the same tier do not overwrite it.
15. Manual checkpoint: pressing the button N times produces exactly one `manual.ron` (the latest); `RunSaveHeader.checkpoints` reflects the latest manual entry only after consolidation.
16. `HardcoreMode` (post-VS test): with `HardcoreMode` on the Run entity, no checkpoint files are written for any trigger.
17. Rolling backups: after N+1 primary save writes, exactly N `run.ron.bak.*` files exist and timestamps are monotonically older.
18. Loading `manual.ron` then triggering a primary save overwrites `run.ron`; loading `tier_2.ron` does not modify `tier_2.ron`.
19. Run-delete UI: deleting an `InProgress` run from the run-select screen removes its directory; no orphaned files remain; `meta.ron` is untouched (mid-run codex entries persist).

---

## 11. VS / Post-VS Scope

### VS

- Run entity with all components in §2 (no `HardcoreMode`)
- Planet entity and TechTree entity tagged `Save`
- Primary run save (local RON, atomic temp-rename, one per run)
- Auto checkpoints: tier unlock, escape construction start
- Manual checkpoint slot
- Rolling backups (hidden, surfaced on corruption)
- Meta save (local RON, codex and blueprints)
- Save triggers: manual, periodic auto, checkpoint-piggyback, exit-save
- Load from run-select screen, including from any checkpoint
- New run flow
- Completed read-only enforcement
- Run-select UI delete-run flow (per-run directory removal with confirmation)
- Crashed-run resume detection

### Post-VS

- Hardcore mode (`HardcoreMode` component, point-buy challenge integration per [`gdd.md §14`](../gdd.md))
- Cloud saves (Steam Remote Storage sync)
- Save schema versioning (warn on mismatch, migrate case-by-case)
- Additional difficulty tiers
- Save file migration tooling
- Full UI for completed-run inspection
- Configurable backup count / disabled backups
- If multiplayer added: migrate `TechTreeProgress` and `ResearchPool` to per-Player or per-Team entities

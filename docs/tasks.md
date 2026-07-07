# Vertical Slice Task Breakdown

> Work plan derived from [`vertical_slice.md`](vertical_slice.md) and [`milestones.md`](milestones.md). Designs in [`technical/`](technical/) are complete; this file lists code work only. Order roughly top-down: foundation before features, blockers before polish.

Legend: `[ ]` not started · `[~]` in progress / partial · `[x]` done

- Update task status as completed
- Ask questions on ambiguity, options, opinions
- Commit between sub-phases

---

## Phase 0 — Foundation

Required before most VS features land. Other phases may proceed in parallel where independent.

### 0.1 Save / Load — `src/save/`
Design: [`save.md`](technical/save.md). Do-not-stub (blocks Standard Probe gate).

- [x] Add `moonshine_save` crate
- [x] `Run` marker component + `RunSaveHeader` (seed, profile, elapsed, timestamp)
- [~] Tag saveable entities with `Save` / `Unload` — Run entity tagged; gameplay entities (Machine, cables, networks, Drone, MinedDeposit, Outpost, Player) deferred pending `Reflect` + `MapEntities` work
- [x] Run save: serialize to local RON, one file per run
- [x] Header-only read path for menu list
- [x] Load flow: deserialize → restore world → resume `GameState::Playing`
- [x] New-run flow: tear down current `Run`, spawn fresh
- [x] Meta save stub (codex/blueprints empty for VS)
- [~] Integration test: save mid-run, load, verify world + factory + research + drone state match — covers Run entity + seed components + `TechTreeProgress`/`ResearchPool` via `include_resource`; factory/drone deferred with entity tagging

Deviations from design:
- `TechTreeProgress` and `ResearchPool` remain Resources, saved via `SaveWorld::include_resource` instead of being migrated to Run-entity components.
- Checkpoints, rolling backups, and HardcoreMode remain post-VS.

### 0.2 Telemetry — `src/telemetry/`
Design: [`telemetry.md`](technical/telemetry.md). VS §6 gate requirement.

- [x] `TelemetryLog` resource, JSONL writer (`#[cfg(debug_assertions)]` gated)
- [x] `RunStarted` event (seed, profile, timestamp)
- [~] First-occurrence events: `tech_node_revealed`, `machine_placed`, `discovery_event`, `remote_mode_entry`/`exit` wired; `planet_property_viewed`, `research_spent`, `power_failure`/`resolved`, `drone_deployed`, `escape_item_produced`, `escape_completed`, `stable_production` deferred until source events exist
- [~] Repeated events: `blocked_state_enter`/`exit` wired (`telemetry_observe_blocked` on aggregate `SlotBlocked` set → `BlockedStateEnter`/`BlockedStateExit`); `tutorial_trigger` deferred (no source event)
- [~] Derived metrics calculator: `compute_metrics` → `DerivedMetrics`, appended as `RunSummary` at run end. Sourced: time-to-first-insight, time-to-first-research-unlock, time-to-first-discovery, blocked-state count + duration, remote trips, total run time. Deferred (no source event): time-to-stable-production, re-engage time
- [~] Event integration points wired across systems — wired for existing emitters; gated behind future event additions for the rest

### 0.3 Seed System — `planet` domain
Design: [`seed.md`](technical/seed.md), [`planet-identity.md`](technical/planet-identity.md).

- [x] Add `planet` field to `DomainSeeds`
- [x] Pcg64 RNG factory for planet domain (`DomainSeeds::planet_rng`)
- [x] Unit test: same `RunSeed` → identical planet domain stream

---

## Phase 1 — Planet Identity (VS §3.1, §3.2)

Foundation for first-hour insight signal and planet-dependent power choice.

### 1.1 Data model — `src/planet/`
Design: [`planet-identity.md`](technical/planet-identity.md).

- [x] `PlanetProperties` component: 6 float axes (solar, thermal, wind, geological, atmospheric, pressure) + hazard type enum
- [x] `PlanetPropertyVisibility` component: per-axis `Hidden`/`Qualitative`/`Revealed`
- [x] `PlanetArchetype` asset (RON): axis means/variances, hazard, descriptive text
- [x] Curate 3 VS archetypes in `assets/planet/archetypes/` (frozen_geothermal, desert_radiant, humid_jungle)
- [x] Archetype-based property generation system (planet domain RNG)
- [x] Property reveal triggers — `property_reveal_system` emits `PlanetPropertyRevealed`: drone scan (fog reveal in DronePilot) → geological_activity Hidden→Qualitative; first research spend (proxy for atmospheric sample analysis) → atmospheric_oxygen + atmospheric_pressure Hidden→Revealed

### 1.2 Property-to-gameplay bindings
- [x] Solar efficiency modifier on solar generator output — applied to existing `generator` via `solar_modifier` at placement
- [~] Combustion efficiency modifier on combustion generator output — `combustion_modifier` helper ready; awaits Phase 4 generator type
- [~] Geothermal modifier on geothermal generator — `geothermal_modifier` helper ready; awaits Phase 4
- [~] Wind modifier on wind generator — `wind_modifier` helper ready; awaits Phase 4
- [ ] Hazard effect hook (thermal/pressure → recipe/machine modifier) — hazard variant cosmetic in VS per design

### 1.3 Landing panel — `PlayMode::Landing`
- [x] Add `Landing` to `PlayMode` substate (default)
- [x] Landing panel UI: archetype name, visible properties, descriptive text, "Begin" button
- [x] Transition Landing → Exploring on confirm

### 1.4 In-run Terminal Planet page
- [x] Terminal screen tab: planet properties (Network / Planet tabs in inventory panel)
- [~] Per-property "how this affects play" tooltip — landing panel rows carry hints; terminal-tab tooltips deferred

### 1.5 Insight beat feedback
- [x] `PropertyDecisionValidated` event (fires when player action matches planet hint)
- [x] Field-computer message on validation — `fire_insight_validation` emits "Well-suited — {label} {property} supports {machine} here." on `PropertyDecisionValidated`
- [x] Telemetry: emit insight-candidate event — `telemetry_observe_insight` logs `InsightCandidate` on `PropertyDecisionValidated`

---

## Phase 2 — Escape Loop Closure (VS §3.9)

Completes the loop. Gateway entity + key recipe already exist.

### 2.1 Escape activation
Design: [`escape-condition.md`](technical/escape-condition.md).

- [x] `RunState` resource: `InProgress` / `Completed`
- [x] Gateway interact: prompt when key in hand, charge progress over power input — player cables gateway machine to network; `activate_gateway` recipe starts when `gateway_key` present
- [x] `EscapeEvent` — fired by `escape_objective_system` on `JobComplete` for `EscapeObjective` machine
- [x] Catalyst recipe input wiring (gateway consumes key) — `activate_gateway` recipe takes `gateway_key` as input

### 2.2 Escape UI
- [x] Escape progress display (HUD widget) — bottom-right progress bar while gateway running
- [~] Site interaction prompt — gateway spawned at ruins; cable connection provides implicit prompt; dedicated proximity UI deferred
- [~] Missing-requirement display — HUD hidden until gateway running; explicit checklist deferred
- [x] Completion screen: seed, elapsed time, archetype, "Main Menu" button
- [x] Visible in-world completion moment (simple VFX placeholder OK) — `spawn_escape_vfx` emissive burst + PointLight at gateway; `escape_sequence_system` delays results screen `ESCAPE_FLASH_SECS` (1.5s) while burst expands/fades

### 2.3 Telemetry hooks
- [x] Emit `escape_item_produced` when key crafted (`forge_gateway_key` job complete)
- [x] Emit `escape_completed` on gateway activation

---

## Phase 3 — Planning UI (VS §3.5)

### 3.1 Recipe browser
Design: [`planning-ui.md`](technical/planning-ui.md).

- [x] `RecipePicker` overlay panel
- [x] List known/revealed recipes with filter (unlocked-only toggle, search)
- [x] Recipe detail view: inputs, outputs, machine, compare panel

### 3.2 Escape dependency graph
- [x] Sankey production graph component (BFS from escape item, column layout)
- [x] Per-node Inspector rail: ratios, machine count, under-planned alert, lock count

### 3.3 Multi-plan support
- [x] `PlanState` component on plan entity
- [x] Named plans per run (`PlanName`), `PlanList` resource, single active plan (VS scope)
- [~] Save plans via `Save` tag — entity exists; `Save`/`Reflect` deferred with entity tagging work

### 3.4 Alerts panel
- [x] Under-planned alerts shown inline in Inspector (ratio math); dedicated aggregated panel of live blocked machines — `src/ui/hud/alerts.rs` top-left HUD, one row per `SlotBlocked` machine
- [x] Per-alert: machine, blocked-reason, jump-to action — row shows `{machine} LV{tier} — {reason}`; click sets `MachineStatusPanel.entity` to open that machine's UI

---

## Phase 4 — Power Expansion (VS §3.7)

Currently 1 generator type. Need 2 with planet-dependent viability + diagnostics.

### 4.1 Second generator
Design: [`power.md`](technical/power.md).

- [x] Define solar + combustion generators (assets/machines/)
- [x] `GeneratorDef` per type with env-port hookup
- [x] `EnvFactorRegistry`: Solar, Combustion (atmospheric oxygen)
- [x] Recipe output energy for combustion (`energy_output` field on `ConcreteRecipe`, deposited into buffer on completion)
- [x] Variance application via `EnvFactorRegistry` initialized from planet properties at run start

### 4.2 Power diagnostics
- [x] Per-machine power-blocked reason via `SlotBlocked(SlotBlockReason::NoPower)`
- [x] HUD: supply/demand totals, deficit warning (red text when demand > supply)
- [x] Generator buffer display (current kJ / max kJ)

---

## Phase 5 — Research UI (VS §3.4)

Logic exists; surface missing.

Design: [`research.md`](technical/research.md).

- [x] Research balance HUD widget (current points) — `src/ui/hud/research.rs`, stacked above power HUD
- [x] Reveal cost surface on tech-tree node hover — cost shown on node card (`{cost} RP`), green if affordable
- [x] Blocked-reason display for unaffordable reveals — "Need N more RP" (WARN) or "↑ Prereqs not met" (ERR) in detail panel
- [x] Research-source display: which machines produce this currency — SOURCE section in detail for ResearchSpend nodes, derived from RecipeGraph producers
- [x] VS uses single currency; defer second type — `ResearchPool { points: f32 }`, single-currency throughout

---

## Phase 6 — Drone & Aegis (VS §3.8)

Drone partial. Aegis absent.

### 6.1 Aegis — `src/aegis/`
Design: [`aegis.md`](technical/aegis.md).

- [x] `AegisEmitter` + `AegisRadius` + `AegisActive` components (`src/aegis/mod.rs`)
- [x] Boundary check system — `aegis_boundary_check_system`, `InAegis` marker on player
- [x] Local-mode constraint enforcement — `aegis_movement_constraint_system` clamps velocity
- [x] Atmospheric exposure outside aegis — `AtmosphericExposure` + timer + body destruction + `RunFailed`
- [~] Outpost beacon: power-gated aegis extender — deferred to MVP per design doc §11

### 6.2 Drone improvements
Design: [`drone.md`](technical/drone.md).

- [x] Fog-of-war map data (per-chunk reveal state) — `FogOfWar` resource, 4m cells, u16 bitmask per chunk
- [x] Drone scan action: reveals fog in radius — `fog_reveal_system` + `character_fog_reveal_system`
- [~] Map markers for discovered sites — discovery events fire; visual map overlay deferred to post-VS
- [x] Drone cargo/sample HUD — inventory panel shown in DronePilot mode (`src/ui/hud/drone.rs`)
- [x] Return-and-deposit prompt — "E — deposit samples" when drone near aegis emitter; `drone_deposit_system`
- [x] Mode indicator widget: Local vs Remote — `◈ LOCAL` / `◈ REMOTE` HUD top-right
- [x] At least 2 scouted destinations in starter chunk — `mineral_deposit` + `alien_artifact` scout sites
- [x] Drone damage/delay risk model — `DroneHealth` stub; no hazard source in VS per design §11

---

## Phase 7 — 3D Readability Overlays (VS §3.6)

Do-not-stub: 3D topology must be legible.

Design: [`planning-ui.md`](technical/planning-ui.md) §topology.

- [x] Network topology overlay (toggle key — `N`)
- [x] Per-network filter (logistics / power)
- [x] Cable highlighting per selected network
- [x] Machine-state color overlay (running / brownout / idle) — BlockReason/PowerPaused states deferred to Phase 4
- [x] Bottleneck pulse indicator (pulsing ring on brownout machines)

---

## Phase 8 — Tech Tree Polish (VS §3.3)

Code present; verify against design.

- [x] Audit node visual states: Shadow, Partial, Revealed, Unlockable, Locked-Out
- [x] Locked-reason display on hover — footer line shows hovered node's locked reason (`prereq: {name} not yet unlocked` / `locked out — {peer} chosen`); insufficient RP excluded per design §blocked-reason
- [x] Exclusive-group choice modal (per design issue #9)
- [x] Expand node pool to design minimum: smelting, extraction, 2 power options, logistics, analysis, drone, alien material, escape synthesis
- [x] Cross-tier port stubs in questbook layout — node cards show `←T{n}`/`T{n}→` for cross-tier prereq/dependent edges; detail-panel REQUIRES & LEADS-TO rows click to jump tier page

---

## Phase 9 — Field Computer Surface

Do-not-stub: delivery surface required, persona deferred.

- [x] `FieldComputerMessage` event
- [x] HUD widget: bottom-corner message log
- [x] Placeholder text for: arrival, first property reveal, first research spend, first drone deploy, first discovery, escape unlock
- [x] Dismiss + history pane

---

## Phase 10 — Curated Seeds

VS §5 + milestone gate.

- [x] Seed-template file (`assets/seeds/curated.ron`): 5 entries
- [x] Each varies: power viability, resource geography, discovery-site location, alien material chain, research pressure
- [x] Main menu: "Curated seed" picker alongside text input
- [x] Validate each seed plays through Insight Run — `tests/curated_seeds.rs` sweeps all 5 curated seeds through landing→mine→first-research-unlock→atmospheric-reveal on simulated time (same `hash_text`→`DomainSeeds` path as the game)

---

## Phase 11 — Playtest Protocol Execution

After Phases 0–10 complete.

- [ ] First-time player test (1 Insight Run, 90–120 min)
- [ ] Repeat-run player test (same player, 3 runs, different seeds)
- [ ] Standard Probe test (1 run, 3–5 h, save/resume mid-session)
- [ ] Written observations against §7 questions for each session
- [ ] Compare results to §9 success/failure criteria

---

## Dependency Graph

```
Phase 0 (Save, Telemetry, Seed) ──┬──► Phase 1 (Planet) ──┬──► Phase 2 (Escape) ──► Phase 11
                                  │                       │
                                  ├──► Phase 3 (Planner)  ├──► Phase 4 (Power)
                                  │                       │
                                  ├──► Phase 5 (Research) ├──► Phase 6 (Drone+Aegis)
                                  │                       │
                                  ├──► Phase 7 (Overlay)  ├──► Phase 8 (Tech tree)
                                  │                       │
                                  └──► Phase 9 (Field comp)└──► Phase 10 (Seeds)
```

Phases 3–10 mostly independent once Phase 0 ships. Pick by team capacity. Phase 11 gates VS completion.

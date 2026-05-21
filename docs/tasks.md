# Vertical Slice Task Breakdown

> Work plan derived from [`vertical_slice.md`](vertical_slice.md) and [`milestones.md`](milestones.md). Designs in [`technical/`](technical/) are complete; this file lists code work only. Order roughly top-down: foundation before features, blockers before polish.

Legend: `[ ]` not started · `[~]` in progress · `[x]` done

---

## Phase 0 — Foundation

Required before most VS features land. Other phases may proceed in parallel where independent.

### 0.1 Save / Load — `src/save/`
Design: [`save.md`](technical/save.md). Do-not-stub (blocks Standard Probe gate).

- [ ] Add `moonshine_save` crate
- [ ] `Run` marker component + `RunSaveHeader` (seed, profile, elapsed, timestamp)
- [ ] Tag saveable entities with `Save` / `Unload`
- [ ] Run save: serialize to local RON, one file per run
- [ ] Header-only read path for menu list
- [ ] Load flow: deserialize → restore world → resume `GameState::Playing`
- [ ] New-run flow: tear down current `Run`, spawn fresh
- [ ] Meta save stub (codex/blueprints empty for VS)
- [ ] Integration test: save mid-run, load, verify world + factory + research + drone state match

### 0.2 Telemetry — `src/telemetry/`
Design: [`telemetry.md`](technical/telemetry.md). VS §6 gate requirement.

- [ ] `TelemetryLog` resource, JSONL writer (`#[cfg(debug_assertions)]` gated)
- [ ] `RunStarted` event (seed, profile, timestamp)
- [ ] First-occurrence events: planet_property_viewed, tech_node_revealed, research_spent, machine_placed, stable_production, power_failure, power_failure_resolved, drone_deployed, remote_mode_entry/exit, discovery_event, escape_item_produced, escape_completed
- [ ] Repeated events: blocked_state_enter/exit, tutorial_trigger
- [ ] Derived metrics calculator: time-to-first-insight, time-to-first-research-unlock, time-to-stable-production, time-to-first-discovery, blocked-state count + duration, remote trips, re-engage time, total run time
- [ ] Event integration points wired across systems (one PR per source system to keep diffs small)

### 0.3 Seed System — `planet` domain
Design: [`seed.md`](technical/seed.md), [`planet-identity.md`](technical/planet-identity.md).

- [ ] Add `planet` field to `DomainSeeds`
- [ ] Pcg64 RNG factory for planet domain
- [ ] Unit test: same `RunSeed` → identical planet domain stream

---

## Phase 1 — Planet Identity (VS §3.1, §3.2)

Foundation for first-hour insight signal and planet-dependent power choice.

### 1.1 Data model — `src/planet/`
Design: [`planet-identity.md`](technical/planet-identity.md).

- [ ] `PlanetProperties` component: 6 float axes (solar, thermal, wind, geological, atmospheric, pressure) + hazard type enum
- [ ] `PlanetPropertyVisibility` component: per-axis `Hidden`/`Qualitative`/`Revealed`
- [ ] `PlanetArchetype` asset (RON): axis means/variances, hazard, descriptive text
- [ ] Curate 3 VS archetypes in `assets/planet/archetypes/` (low-solar/oxygen-rich, high-geothermal, windy/cold)
- [ ] Archetype-based property generation system (planet domain RNG)
- [ ] Property reveal triggers (research spend, drone scan, time-on-planet — per design)

### 1.2 Property-to-gameplay bindings
- [ ] Solar efficiency modifier on solar generator output
- [ ] Combustion efficiency modifier on combustion generator output
- [ ] Geothermal modifier on geothermal generator
- [ ] Wind modifier on wind generator
- [ ] Hazard effect hook (thermal/pressure → recipe/machine modifier)

### 1.3 Landing panel — `PlayMode::Landing`
- [ ] Add `Landing` to `PlayMode` substate
- [ ] Landing panel UI: archetype name, visible properties, descriptive text, "Begin" button
- [ ] Transition Landing → Exploring on confirm

### 1.4 In-run Terminal Planet page
- [ ] Terminal screen tab: planet properties (visible only)
- [ ] Per-property "how this affects play" tooltip

### 1.5 Insight beat feedback
- [ ] `PropertyDecisionValidated` event (fires when player action matches planet hint)
- [ ] Field-computer message on validation
- [ ] Telemetry: emit insight-candidate event

---

## Phase 2 — Escape Loop Closure (VS §3.9)

Completes the loop. Gateway entity + key recipe already exist.

### 2.1 Escape activation
Design: [`escape-condition.md`](technical/escape-condition.md).

- [ ] `RunState` resource: `InProgress` / `Completed`
- [ ] Gateway interact: prompt when key in hand, charge progress over power input
- [ ] `EscapeEvent` (started, charging, completed)
- [ ] Catalyst recipe input wiring (gateway consumes key)

### 2.2 Escape UI
- [ ] Escape progress display (HUD widget)
- [ ] Site interaction prompt (proximity-triggered)
- [ ] Missing-requirement display (key? power? location?)
- [ ] Completion screen: seed, elapsed time, archetype, "Start new run" button
- [ ] Visible in-world completion moment (simple VFX placeholder OK)

### 2.3 Telemetry hooks
- [ ] Emit `escape_item_produced` when key crafted
- [ ] Emit `escape_completed` on gateway activation

---

## Phase 3 — Planning UI (VS §3.5)

No planner panel exists. Bulk of UI work.

### 3.1 Recipe browser
Design: [`planning-ui.md`](technical/planning-ui.md).

- [ ] `RecipePicker` overlay panel
- [ ] List known/revealed recipes with filter
- [ ] Recipe detail view: inputs, outputs, machine, time, energy

### 3.2 Escape dependency graph
- [ ] Sankey production graph component (from escape item backward)
- [ ] Per-node Inspector rail: ratios, machine count estimate, time-to-N

### 3.3 Multi-plan support
- [ ] `PlanState` component on plan entity
- [ ] Named plans per run, list + create/delete
- [ ] Save plans via `Save` tag

### 3.4 Alerts panel
- [ ] Aggregated machine-blocked alerts list
- [ ] Per-alert: machine, blocked-reason, jump-to action

---

## Phase 4 — Power Expansion (VS §3.7)

Currently 1 generator type. Need 2 with planet-dependent viability + diagnostics.

### 4.1 Second generator
Design: [`power.md`](technical/power.md).

- [ ] Define solar + combustion generators (assets/machines/)
- [ ] `GeneratorDef` per type with env-port hookup
- [ ] `EnvFactorRegistry`: Solar, Combustion (atmospheric oxygen)
- [ ] Recipe output `RecipeOutput::Energy` for each
- [ ] Variance application to energy output

### 4.2 Power diagnostics
- [ ] Per-machine power-blocked reason exposed via `SlotBlockReason`
- [ ] HUD: supply/demand totals, deficit warning
- [ ] Generator output display (current watts / max)

---

## Phase 5 — Research UI (VS §3.4)

Logic exists; surface missing.

Design: [`research.md`](technical/research.md).

- [ ] Research balance HUD widget (current points)
- [ ] Reveal cost surface on tech-tree node hover
- [ ] Blocked-reason display for unaffordable reveals
- [ ] Research-source display: which machines produce this currency
- [ ] VS uses single currency; defer second type

---

## Phase 6 — Drone & Aegis (VS §3.8)

Drone partial. Aegis absent.

### 6.1 Aegis — `src/aegis/`
Design: [`aegis.md`](technical/aegis.md).

- [ ] `AegisField` component (radius, source entity)
- [ ] Boundary check system
- [ ] Local-mode constraint enforcement (player limited to aegis radius)
- [ ] Atmospheric exposure outside aegis (hazard placeholder OK)
- [ ] Outpost beacon: power-gated aegis extender

### 6.2 Drone improvements
Design: [`drone.md`](technical/drone.md).

- [ ] Fog-of-war map data (per-chunk reveal state)
- [ ] Drone scan action: reveals fog in radius
- [ ] Map markers for discovered sites
- [ ] Drone cargo/sample HUD
- [ ] Return-and-deposit prompt
- [ ] Mode indicator widget: Local vs Remote
- [ ] At least 2 scouted destinations in starter chunk with different value/risk
- [ ] Drone damage/delay risk model (no permanent loss for VS)

---

## Phase 7 — 3D Readability Overlays (VS §3.6)

Do-not-stub: 3D topology must be legible.

Design: [`planning-ui.md`](technical/planning-ui.md) §topology.

- [ ] Network topology overlay (toggle key)
- [ ] Per-network filter (logistics / power)
- [ ] Cable highlighting per selected network
- [ ] Machine-state color overlay (running / blocked / unpowered)
- [ ] Bottleneck pulse indicator

---

## Phase 8 — Tech Tree Polish (VS §3.3)

Code present; verify against design.

- [ ] Audit node visual states: Shadow, Partial, Revealed, Unlockable, Locked-Out
- [ ] Locked-reason display on hover
- [ ] Exclusive-group choice modal (per design issue #9)
- [ ] Expand node pool to design minimum: smelting, extraction, 2 power options, logistics, analysis, drone, alien material, escape synthesis
- [ ] Cross-tier port stubs in questbook layout

---

## Phase 9 — Field Computer Surface

Do-not-stub: delivery surface required, persona deferred.

- [ ] `FieldComputerMessage` event
- [ ] HUD widget: bottom-corner message log
- [ ] Placeholder text for: arrival, first property reveal, first research spend, first drone deploy, first discovery, escape unlock
- [ ] Dismiss + history pane

---

## Phase 10 — Curated Seeds

VS §5 + milestone gate.

- [ ] Seed-template file (`assets/seeds/curated.ron`): 5 entries
- [ ] Each varies: power viability, resource geography, discovery-site location, alien material chain, research pressure
- [ ] Main menu: "Curated seed" picker alongside text input
- [ ] Validate each seed plays through Insight Run

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

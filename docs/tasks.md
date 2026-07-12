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
- [x] At least 2 scouted destinations in starter chunk — `mineral_deposit` + `precursor_artifact` scout sites
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
- [x] Expand node pool to design minimum: smelting, extraction, 2 power options, logistics, analysis, drone, exotic material, escape synthesis
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
- [x] Each varies: power viability, resource geography, discovery-site location, exotic material chain, research pressure
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

---

# Standard Run Content — Full Standard

> **Milestone placement: NOT Vertical Slice.** This block builds toward the **Demo (MVP)** gate "Standard difficulty playable end-to-end" (`milestones.md`). It is a **single fixed, hand-authored Standard run** (tiers 1–5, ~71 nodes, ~13h design target, successor-launch escape) — the Standard analog of the VS curated seed. The **seeded ~215-node pool + per-run selection and the procedural graph validator stay deferred to Alpha** (`milestones.md` Alpha gate); they are *not* in this push. Do not fold these phases into VS phases 0–11.
>
> Source of truth: [`standard-run-design.md`](standard-run-design.md) (esp. §5 tiers, §8 escape, §9 flags). Node tables live there and in [`tech-tree-design.md`](tech-tree-design.md); escape ECS in [`technical/escape-condition.md`](technical/escape-condition.md); research themes in [`technical/research.md`](technical/research.md). Numbers throughout are representative and **unvalidated** — playtest-tuned after e2e completability (§9 #4).

Phases are dependency-ordered (§10). Owner tags: `engine` · `content-designer` · `docs-curator` · `playtest-verifier`.

## Phase A — Engine: `ProductionMilestone` unlock vector `[engine]`

The one true blocker. The vector is a stub (`tech_tree/mod.rs` enum variant + UI render only); the auto-unlock loop (`research/mod.rs`) handles `ExplorationDiscovery` + `PrerequisiteChain` but not this. Required by the T1 skeleton (Basic Miner ← 50 stone, Land Drone Mk1 ← 20 iron) and every production-milestone tier gate. Gates Phase B onward. (§1, §9 #1)

- [ ] Per-material running produced-count tracking
- [ ] Auto-unlock a node when its produced-count threshold is met
- [ ] Hinted progress readout on the node card (e.g. "50/100 refined units")
- [ ] Tests: count accrues on job completion; node auto-unlocks at threshold; progress hint renders

## Phase B — Content: materials + forming ladder `[content-designer]` (dep: A)

The base material spine (§2.1) and the staggered forming ladder (§3.1). Interlock via byproducts is the planning content (§1 principle 1). (§9 #2)

- [x] 10 base materials incl. bronze (Cu+Sn) and steel (Fe+C) alloys, with form groups (`metal`: ore/crushed/dust/ingot/plate/wire; `alloy`: ingot/plate/wire). Silicon = Unique item (refined from crushed stone); gears = per-alloy Composite (bronze_gear) rather than a universal form — see Phase B notes below
- [x] Forming machines: crusher, washer, plate_roller, wire_drawer (alloy-smelt reuses `smelter`) + ladder recipes `crush_*`+gravel, `smelt_*_crushed` (2→3 yield), `wash_*`+slag, `wash_iron` (+trace copper), `smelt_*_dust`, `roll_iron_plate`+scrap, `draw_metal` (retargeted to wire_drawer), `alloy_bronze`, `alloy_steel`, `form_gear_bronze`
- [x] Byproducts + sinks: gravel (→ `gravel_filler` → construction_filler), slag (→ `scrub_slag` → recovered iron_dust), metal scrap (→ `recycle_scrap` → iron_dust); trace-copper cross-feed on `wash_iron`
- [x] T1 reconciliation: Ore Crusher authored as T2 node (`ore_crusher`, ProductionMilestone 100 iron_ingot); T1 skeleton in `tech-tree-design.md §6` reconciled to 7 fixed direct-smelt nodes. T2 forming/material nodes added: `tin_extraction`, `ore_crusher`, `bronze_alloying`, `silicon_refining`, `gravel_sink`

> **Phase B notes / flags** (`content-designer`): (1) The RON `RecipeTemplate` engine hardcodes `output qty 1.0` + `byproducts: []` — it cannot express the ladder's byproducts, non-unit yields, or cross-feed. The byproduct/yield ladder steps are therefore hand-authored **concrete** recipes (iron/copper/tin exemplars) rather than templates; a template-engine extension would DRY this to all metals (engine follow-up). (2) `draw_metal` kept at 1 ingot→1 wire (design §5 wanted 1→2 — blocked by the template output-qty limit; changed, flagged). (3) `ProductionMilestone` gating applied only to the new `ore_crusher` node; the design's Basic Miner←50 stone / Land Drone←20 iron would require switching `ore_extraction`/`drone_recon` to milestones, but the e2e test hard-codes `ore_extraction` as a player-requested ResearchSpend unlock — that conversion is an engine+test task, not RON-only. (4) Wash/plate/steel recipes are graph-coherent but their unlock nodes (Ore Washer, Slag Scrubber, Steel Alloying T3; Plate Roller T4) land in later phases. Known future-consumer danglers: `bronze_gear`, `silicon` (Phase C), `iron_plate`, `steel_ingot`, `*_wire` (T3–T5).

## Phase C — Content: research themes + packs `[content-designer + docs-curator]` (parallel with B)

The 4-theme research ladder (§3.2). **Reverses the VS single-currency stance** (VS §3.4 / Phase 5) — validate lockout mitigations hold (no theme strandable) in the Phase F sweep. (§9 #9)

- [x] Extend `research.md` with the 4-theme ladder (Material / Engineering / Discovery / Synthesis): generation source, spend gates, online tier, per-theme yield-recipe ladder, Synthesis↔void coupling — **§3/§4 rewritten** (Discovery/Synthesis documented but marked "content pending Phase D")
- [x] RON research recipes per theme per tier — **Material ladder**: T1 `basic_analysis` (4 stone→10, existing, `research_points` alias) / T2 `analyze_bronze_gear` (1 gear→15 `research.material`) / T3 `form_silicon_chip` + `analyze_silicon_chip` (1 chip→25). **Engineering generator**: `analyze_circuit` (1 circuit_board→20 `research.engineering`). New item `silicon_chip`
- [x] Encode the Material-vs-Engineering classification rule in `research.md §3` (single-material-form = Material, multi-material-assembly = Engineering; no item is both). Applied: bronze_gear/silicon_chip → Material generators; circuit_board → Engineering generator

> **Phase C notes / flags** (`content-designer`): (1) **Re-themed to `engineering`**: `advanced_processing` (300) + `resonite_engineering` (500) — both are T2 Processing nodes whose prereq chain passes through `basic_processing` (unlocks `make_circuit`→`analyze_circuit`), so engineering is earnable before either is reachable. Both sit on the escape path; reachability confirmed. (2) **Left `material`, flagged**: `logistics_basics` (T1, no prereqs — bootstrap node; engineering isn't online at T1, per §3.2 note) and `gravel_sink` (T2 optional; reachable via `ore_crusher` *without* `basic_processing`, so engineering isn't guaranteed earnable — re-theming would risk stranding the optional node). (3) **Deferred to Phase D**: `drone_recon` (Discovery theme) and `escape_synthesis` (Synthesis theme) stay `material` — their currencies need drone/exotic sources not yet online (re-theming now would soft-lock). (4) `research.material`/`research.engineering` outputs show as producer-only in `item_uses` — expected; they are currency routed by `research_theme_of`, not logistics items.

## Phase D — Content: exotic chains + T2–T5 nodes `[content-designer]` (dep: B, C)

The ~63 tier-2–5 nodes (T2 12 / T3 16 / T4 20 / T5 15), exotic lines, and the successor systems. Each successor system pulls a different exotic line so the launch needs the whole graph (§2.2, §5, §8.1). (§9 #3 RON renames land here)

- [x] Exotic materials + deposits: Fluxite (`fluxite_relic_cache`), Vitreite (synthesized composite item), Cryophase (`cryophase_deposit`, remote second-site); `derelict_ship` for the discount; Resonite/Xalite chain folded in (xalite deposit now emits minable `_shard` forms)
- [x] Exotic processing recipes + `coolant_runoff` byproduct + `reclaim_coolant` sink (loop closes to `exotic_solvent` → fuel; soft two-recipe fuel path keeps venting non-blocking — §4/§11)
- [x] Successor systems (core/chassis/drive/sensor) + provisioning module (kit chain) + `launch_successor` cascade recipe
- [~] Tech nodes T3–T5: **launch spine authored** (26 new nodes) with unlock vectors + §7 hints; **optional/yield nodes deferred** (design-only, see `tech-tree-design.md §6bis`). T2 nodes already shipped Phase B/C.
- [~] **Fluxite generator** (higher-output V2 stand-in) + **advanced assembler** authored. **No engine voltage tier** exists (`min_voltage_tier`/`RecipeBlockedVoltage` absent) — power transition modeled *softly* via high exotic `energy_cost` + the Fluxite generator's higher output, **not** a hard block. Voltage-2 network N/A. **Flagged.**
- [x] Port the T3–T5 node definitions to `tech-tree-design.md` (§6bis)

> **Phase D notes / flags** (`content-designer`): The chain **loads and resolves** (`tech_path launch_successor`) and `cargo test` stays green (283 unit + curated_seeds + landing e2e). Three **engine gaps** surfaced — none block the reachability proof or tests (e2e stops at drone scan), but all block an actual in-game Standard win and are engine follow-ups:
> 1. **Deposit discovery is hardcoded to xalite.** `drone::deposit_discovery_system` only fires `DiscoveryEvent("xalite_deposit")` when a deposit ore id `== "xalite"`. The new `ExplorationDiscovery` keys (`fluxite_relic_cache`, `cryophase_deposit`, `derelict_ship`) — and now the xalite one, since the deposit gained `_shard` forms — need a general deposit→discovery-key mechanism. (The xalite deposit keeps a bare `"xalite"` ore entry so its existing discovery still fires; that entry is a pre-existing dangling mined item.)
> 2. **`EscapeObjective` is never attached to a player-built `launch_site`.** Only the run-gen gateway gets it (`world::ruins`). See `escape-condition.md §7.1`.
> 3. **No voltage-tier system.** `min_voltage_tier`/`RecipeBlockedVoltage` do not exist in the engine (contrary to the brief); the T4 power transition is modeled softly (high `energy_cost` + higher-output Fluxite generator), never a hard block.
>
> **Deviations:** optional/yield nodes deferred to design-only (spine-only RON); base-metal "Extraction" gates folded (the `smelt_metal` template already yields all ingots); `launch_site_assembly` ProductionMilestone("4 systems") → ResearchSpend+prereqs+recipe-inputs (single-material limit of the vector). Numbers representative/unvalidated (§9 #4). Byproduct loop (`coolant_runoff`→`exotic_solvent`) kept closable and non-blocking via the `refine_exotic_fuel__raw` fallback.

## Phase E — Escape spec + wiring `[content-designer + docs-curator]` (dep: D)

Write the Standard escape that `escape-condition.md §7` currently defers. No new engine — launch site is an `EscapeObjective` running one recipe (§8). (§9 #3)

- [x] Write §7 successor-launch spec into `technical/escape-condition.md` (§7.1)
- [~] RON: launch-site machine + `launch_successor` recipe authored. **Engine hook flagged:** a player-built `launch_site` is never given the `EscapeObjective` marker (only the run-gen gateway is, via `world::ruins`); the win won't fire until placement/run-gen attaches it (§7.1 flag).
- [x] RON: `make_successor_chassis__salvaged` derelict-discount variant consuming `salvaged_hull` (fixed run = derelict present, §8.3)

## Phase F — Reachability + tests `[playtest-verifier]` (dep: all)

Hand-verify reachability (procedural validator stays Alpha) + extend the e2e path to victory (§1, §9 #8). Per CLAUDE.md, add a matching stage to the e2e test for each new stage on the landing→victory path.

- [x] `tech_path launch_successor` (MCP) — resolves; full chain reachable in tier order, every recipe input has a producer (hand-traced via `item_uses`). `escape_synthesis` gateway chain intact.
- [x] Extend `tests/standard_full_run.rs` with stages T3 → victory — full landing→launch on earned research (zero injected node/recipe unlocks); measured ~2979s (0.83h) automated-optimal virtual time
- [ ] Curated Standard run config (the fixed Standard "seed" equivalent)

## Carried-forward open flags (§9)

Design questions, not tasks — track as risks; do not lock into content yet.

- [ ] **Non-terminal tier exit gates are TBD** (§9 #6) — the §5 tier-exit anchors are provisional; full tier-gate design is a separate pass (`tech-tree-design.md §7 Q#1–2). Don't lock gate quantities in content.
- [ ] **Beneficial coolant-runoff → terraform-product is post-MVP stretch** (§9 #5, GDD §11) — ship harmful/neutral streams now; coolant-as-terraform + the provisioning discount (Terraform Router, Terraform Provisioning nodes) are optional stretch.
- [ ] **Frontier (no-precursor) variant is a later second config** (§9 #7) — this run seeds the derelict (discount path); the scratch-build chassis alt is a separate curated Standard config, deferred.
- [ ] **`wire_drawer` machine buildability audit** — the e2e completability check found `basic_processing` unlocked a dead recipe id `draw_copper` (fixed → `draw_metal__copper`). Separately, `wire_drawer` has no `make_wire_drawer` recipe and no `UnlockMachine` effect on any node (as do crusher/washer/refinery) — confirm machine placement really is a separate placeables path, not a gap that blocks a real player from building the machine.

Ambiguities flagged for the design owner (not decided here):
- Phase D lumps the Resonite/Xalite chain into the T2–T5 node work; the source task list named only Fluxite/Vitreite/Cryophase. Resonite is a T2/T3 exotic (§2.2) and part of the same node tables — confirm it belongs in D rather than a separate item.
- Node totals: 7 (T1) + 12 + 16 + 20 + 15 = 70; design states ~71. Representative and unvalidated (§9 #4) — reconcile the exact count at content authoring.

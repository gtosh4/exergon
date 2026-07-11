# Technical design docs
These docs describe the deep technical (Bevy, ECS, events, etc) implementation designs. They should be detailed enough to write integration tests without guessing so the tests can validate the implementation.

These should always be up to date or ahead of the code. When implementing new features or changes, always update the designs first then the code to match.

## Designs

### [`networks.md`](networks.md) — Network System Design
Generic cable network system and both concrete kinds (logistics and power). Covers ECS structure, topology systems (place/remove/split/merge), routing, and how power gating interacts with recipe start. **Read before touching `src/network/`, `src/logistics/`, or `src/power/`.**

### [`power.md`](power.md) — Power System Design
Generator kinds (Active fuel-fed, Passive env-fed, Burst event-driven), `GeneratorDef` asset schema, environmental ports + virtual items (`sunlight_tick`, `heat_tick`, `energy_pulse`), `LocalVirtualStock`, recipe-driven energy production (`RecipeOutput::Energy`), throttle modes (`OnBufferFull` default, `NeverThrottle` opt-in), batteries (`BatteryUnit` with charge/discharge rate caps), `EnvFactorRegistry` (Solar/Thermal/Lightning/Wind), weather + day/night hooks, lightning-rod burst targeting, run variance layers, 16 edge cases. Supersedes `networks.md §3` for the generator-fill model — that section's `generator_tick_system` constant-watts behavior is replaced by recipe-completion writes. **Read before touching `src/power/` for generator/battery logic, adding a generator def, or wiring an env source.**

### [`research.md`](research.md) — Science & Research System
ECS components, system step-by-step logic, events/messages, and edge cases for research stations, research pool, knowledge visibility, and the player-initiated unlock flow. VS and MVP differences noted inline. **Read before touching `src/research/` or adding research station recipes.**

### [`escape-condition.md`](escape-condition.md) — Escape Condition Design
ECS components, system logic (gateway charge, interact, status UI), events, edge cases, and integration test descriptions for the Initiation escape (precursor gateway activation). Includes recipe system extension for catalyst inputs. MVP escape types outlined. **Read before implementing gateway activation, EscapeEvent, or RunState.**

### [`mining.md`](mining.md) — Mining & Deposit System
Ore extraction from surface deposits. Covers ECS components (`OreDeposit`, `MinerMachine`), miner placement and deposit linking, depletion curve, weighted ore sampling, manual mining, and deposit discovery. **Read before touching `src/logistics/miner.rs`, `src/drone/`, or deposit-related code in `src/world/generation.rs`.**

### [`drone.md`](drone.md) — Drone System Design
ECS components, system logic (Local↔Remote mode transition, fog-of-war reveal, sample collection, range scanning, multiple drone switching), events, edge cases, and execution order. VS and MVP scope noted inline. **Read before touching `src/drone/` or anything involving `PlayMode::DronePilot`.**

### [`aegis.md`](aegis.md) — Aegis System Design
ECS components, system logic (boundary check, Local mode constraint, atmospheric exposure, outpost beacon power, body switching), events, edge cases, and execution order. VS and MVP scope noted inline. **Read before implementing aegis fields, body switching, or outpost beacons.**

### [`crafting.md`](crafting.md) — Crafting System Design
ECS components, system logic (recipe execution, job dispatch, catalyst reservation, module effects, plan resolution), events, edge cases, and execution order. Resolves: Recipe Graph Runtime Integration, Catalyst Inputs, and Auto-crafting Job Dispatch todos. VS and MVP scope noted inline. **Read before implementing recipe execution, crafting jobs, or auto-crafting dispatch.**

### [`inventory.md`](inventory.md) — Inventory System Design
ECS components, system logic (hotbar, drone inventory, storage units, Terminal screen runtime data, goal tracker), events, edge cases, and execution order. Covers the no-personal-inventory model, hotbar-as-network-view, drone deposit flow, NetworkFlowLedger (Δ/min), and pin-based goal tracker. VS and MVP scope noted inline. **Read before implementing the hotbar, Terminal screen, drone deposit, or storage capacity.**

### [`machine-ui.md`](machine-ui.md) — Machine UI Technical Design
ECS components, system logic (open/close, identity, progress, power status, module slots, port binding editor, recipe table C/P flag editing), events, edge cases. Also defines the **revised `MachineJobPolicy`** (supersedes `crafting.md §4`) with per-recipe `RecipePolicy` carrying independent C/P flags and machine-level `CraftingJobMode`/`passive` defaults. VS and MVP scope noted inline. **Read before implementing the machine panel, `MachineJobPolicy`, `PortPolicy` editing, or `SlotBlockReason`.**

### [`planning-ui.md`](planning-ui.md) — Planning UI Technical Design
ECS components, system logic (Sankey production graph, per-node Inspector rail, Recipe Picker overlay, 3D network topology overlay with per-network filter), events, edge cases, and execution order. Plans future factory additions using ratio math — does not read live machine state. Multiple named plans per run; each is a saved `PlanState` component on a plan entity. Wireframe: `ui_mock/planner-wireframes.html`. VS and MVP scope noted inline. **Read before implementing the planner panel, Sankey graph, Inspector, Recipe Picker, or topology overlay.**

### [`telemetry.md`](telemetry.md) — Telemetry System
ECS resource structure, event schema (run lifecycle, first-occurrence, repeated), derived metrics, system logic, JSONL log format, and edge cases for development-build telemetry. Covers all VS §6 required events and derived metrics. `#[cfg(debug_assertions)]` gated — no analytics pipeline, no network. **Read before implementing the telemetry plugin, `TelemetryLog` resource, or adding new observable events.**

### [`generation.md`](generation.md) — World & Chunk Generation
Coordinate system, chunk streaming (spawn/despawn distances, hysteresis), heightmap generation (`HybridMulti<Perlin>`, ±50 m range), underground resource-domain query system (cell grid, biome bands, ellipsoidal veins), surface deposit placement (one per 64×64 m cell, 33% probability), discovery site placement (seeded XZ, drone proximity trigger), chunk boundary conditions (seam-free by construction), seed→geography mapping (all generation domains keyed independently from `DomainSeeds.world`), and 25 integration test invariants. VS vs. MVP scope (biomes, world bounds, core zone guarantee) noted inline. **Read before touching `src/world/generation.rs`, `src/world/ruins.rs`, `src/content/mod.rs` (resource domains/biomes/veins), or anything that places world objects.**

### [`tech-tree-ui.md`](tech-tree-ui.md) — Tech Tree UI Technical Design
ECS components, node visual states (Shadow/Partial/Revealed/Unlockable/Locked-Out), tier-paged questbook layout (BFS topological X, category Y, gate bridge cards, cross-tier port stubs), inspector rail, reveal overlay (tier ladder, before/after diff, prereq chain), exclusive-group choice modal (resolves `tech-tree-design.md` issue #9 — modal approach), top bar, events, edge cases, and 14 integration test invariants. VS scope: T1–T3, full reveal mechanic, tier gate display. **Read before implementing the tech tree panel, `TechTreePanelState`, reveal overlay, or exclusive-group surfacing.**

### [`planet-identity.md`](planet-identity.md) — Planet Identity & Seed System
ECS components, archetype-based property generation algorithm (3 VS archetypes; 6 float axes + hazard type), property-to-gameplay effect bindings (solar/combustion/geothermal/wind/thermodynamic/pressure modifiers with exact formulas), property visibility model (Hidden → Qualitative → Revealed with reveal triggers), landing panel UI (`PlayMode::Landing`), in-run Terminal Planet page, and insight beat feedback system (`PropertyDecisionValidated`). Also requires adding `planet` domain to `DomainSeeds`. **Read before implementing planet property generation, the landing panel, or the VS §3.1/§3.2 insight beat.**

### [`seed.md`](seed.md) — Seed System
`RunSeed` (text → u64 via xxh64), `DomainSeeds` (keyed sub-seed derivation), per-site derivation pattern, chunk streaming determinism guarantee, random phrase generation for empty input, RNG algorithm choice (Pcg64 not SmallRng), tech tree validity concept (deferred), versioning policy (none pre-release). **Read before modifying `src/seed/mod.rs`, adding a generation domain, or writing code that draws RNG values from a seed.**

### [`save.md`](save.md) — Save Architecture
Run entity (`Run` marker, `RunSaveHeader`, lifetime), saveable entity inventory (`moonshine_save::Save`/`Unload` tagging), run save (local RON, one file per run, header-only reads, never deleted), meta save (codex, blueprints, milestone triggers), save/load/new-run flow, cloud saves placeholder (post-VS). **Read before implementing `src/save/`, tagging entities with `Save`/`Unload`, or adding run-scoped global state.**

### [`recipe-graph.md`](recipe-graph.md) — Recipe Graph
Definition and data model: materials (base vs. exotic, form-group membership), form groups (ordered form sets), item kinds (Derived/Composite/Unique, exactly-one terminal), recipe templates and expansion, `ConcreteRecipe` fields (unified `outputs`, `RecipeInput` with `consumed` flag, independent `min_voltage_tier`), `RecipeGraph` resource and lookup indexes (`by_output`/`by_input`/`by_machine`), graph structure (tier products, cross-tier reuse, single critical path via tech-tree node selection), byproduct routing convention, 14 validity invariants, seed integration (VS curated, post-VS procedural). **Read before modifying `src/recipe_graph/`, adding a material/form-group/template/recipe asset, or touching `RecipeGraph` consumers.**

### [`machines.md`](machines.md) — Machine System
`MachineDef`/`MachineTierDef` asset schema (per-tier `item_id`, scene path, IO port offsets/labels/kinds, reserved `module_slots` field; port positions are dev placeholders, real models source from GLTF child entities), `MachineRegistry` with `tier_def_by_item` lookup, `Machine` ECS component (`item_id` + `orientation`), port entity spawn via `OnAdd<Machine>` observer, removal flow (in-flight input/catalyst/amp return, cable cleanup, type-specific observer hook), tier upgrade (recipe-graph model where tier N+1 consumes tier N machine; upgrade-kit path with nearest matching-kind port rebind; deconstruct-and-replace path with manual reconfig), IO port routing delegated to `machine-ui.md` + `networks.md`, orientation contract (4 rotations, no mirror in VS), save tagging (`#[require(Save)]`, mid-recipe state fully persisted), and deferred-modules stub. Generic placement/removal flow lives in [`building.md`](building.md). **Read before touching `src/machine/`, adding a machine asset, implementing tier upgrade, or modifying placement/removal flows.**

### [`input.md`](input.md) — Input & Keybindings
Canonical action token registry, default bindings, context stack (Local body / Drone / Modal / HUD / Global), and `bevy_enhanced_input` plugin design. Every input referenced in another technical doc resolves to a token defined here. **Read before adding a new input-driven system or referencing a keybind in prose.**

### [`building.md`](building.md) — Building System
`PlaceableDef` RON asset schema (`InteractionShape::{Single,TwoEndpoint,AreaRect}`, `SurfaceRule`, `SnapRule`, `OrientationSupport`, `GhostHint`), `PlaceableRegistry`, `PlaceableColliderCache` (scene-mesh AABB or `footprint_override`), single-point flow (machines, generator, storage_crate, decorations), two-endpoint flow (logistics_cable, power_cable — port-snap on both clicks), area-rect flow (platform — 2-corner with air projection), `BuildOrientation` resource (sticky across hotbar swap, `{kbd:rotate_cw}` / `{kbd:rotate_ccw}` cycles — see `input.md`), centralized validator with `PlacementReason` enum, resolved removal (raycast → Machine/Platform/cable/port→owner), extended `WorldObjectEvent` with `orientation` field, `PlacementRejected` event, `BuildingSet → MachineScanSet → NetworkSystems` same-frame chain. **Read before touching `src/world/interaction.rs` placement/ghost logic, adding a placeable item, or modifying the `WorldObjectEvent`/`CableConnectionEvent` contracts.**

### [`testing.md`](testing.md) — Testing & Dev Tooling
Test layers (per-system `mod tests`, per-recipe integration tests, the one landing→victory e2e test), how the e2e test fast-forwards simulated time with `TimeUpdateStrategy::ManualDuration` + the `advance_until` primitive, the step-by-step recipe for adding a new stage to the e2e test as each game stage is implemented, and the `assets` RON query CLI (`cargo run --bin assets recipe|tech|path|uses …`). **Read before adding a gameplay stage to the landing→victory path, extending `tests/standard_full_run.rs`, or reaching for a `.ron` file by hand.**

---

## TODO
These are systems/designs known to be needed **for the vertical slice** but without a doc yet. Write the spec before writing the code.
DO NOT consider current implementation/code when making designs - do the best design in absolute.
Ask questions if there is any ambiguity or decisions to be made in the design.

(empty — all VS designs complete)

---

## Post-VS Designs

Systems explicitly deferred from the vertical slice (`vertical_slice.md §8`). Write specs before implementing, but not before VS is complete.

### World Reactivity

`world-reactivity.md` — per-region tracking (reactivity score 0–1, spread to adjacent regions, seeded rate multiplier per region), reactivity sources (machine pollution, extraction, experimentation, power output — all continuous or pulse), hybrid continuous+threshold effect model (efficiency degradation formula, 4 threshold events at 0.25/0.50/0.75/0.90, fire-once-per-region), recovery (faster than buildup, thresholds don't reverse), per-source breakdown (MVP: expose level only; post-MVP: full cause breakdown).

### Codex & Meta-Progression

`codex.md` — codex entry types (biome, node type, planet modifier, machine type, exotic material), creation triggers (first-encounter via drone/presence), type-level vs. run-specific data distinction, accumulated observations across runs. Meta-progression: unlock triggers (run completion by difficulty tier, in-run milestones), grant types (biome, run modifier, narrative, blueprint slot, starting boon). Blueprints: layout-only templates (machines, tiers, positions, orientations, logistics connections), not recipe solutions, finite slots expandable via meta-progression. Note: save file format covered in Save Architecture above.

### Run Modifier System

`run-modifiers.md` — `RunModifierDef` asset schema (id, display name, cost, description, `#[cfg(debug_assertions)]` guard flag, mutually-exclusive group), `ModifierRegistry` resource, `WizardDraft` modifier selection (multi-select, group exclusion enforcement), `NewRunEvent` modifier payload (replaces ad-hoc `test_mode: bool`), `ActiveModifiers` run-scoped resource, system hook protocol (each modifier registers an `OnAdd<ActiveModifiers>` observer or `run_if` condition), cost ledger (sum of selected costs, shown in wizard summary step). **Write before replacing the current `test_mode` field with a general modifier list, adding any new modifier, or changing how the wizard surfaces modifier selection.**

### Recipe Graph Generation Algorithm

Once curated seeds are replaced by procedural generation: backwards-from-terminal generation ordering, template expansion, parameter variance bounds (inputs 50–200%, yield 60–150%, time 50–300%, energy 50–250%), byproduct routing, validity invariants, and the standalone generator validator required before shipping procedural seeds (`vertical_slice.md §10.6`).

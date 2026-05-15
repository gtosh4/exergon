# Planet Identity & Seed System

ECS components, generation algorithm, property-to-gameplay bindings, visibility model, landing panel UI, and insight beat feedback for planet properties. Read `gdd.md §5` for design intent. Seed derivation and RNG are documented in `technical-design.md §1` and not repeated here.

**Requires:** add `planet: u64` to `DomainSeeds` in `src/seed/mod.rs` and add the corresponding `derive(master, "planet")` call in `DomainSeeds::from_master`.

---

## Table of Contents

1. [Overview](#1-overview)
2. [ECS Structure](#2-ecs-structure)
3. [Planet Archetype & Property Generation](#3-planet-archetype--property-generation)
4. [Property-to-Gameplay Effect Bindings](#4-property-to-gameplay-effect-bindings)
5. [Property Visibility Model](#5-property-visibility-model)
6. [Landing Panel UI](#6-landing-panel-ui)
7. [In-Run Property Display](#7-in-run-property-display)
8. [Insight Beat Feedback](#8-insight-beat-feedback)
9. [Events](#9-events)
10. [Execution Order](#10-execution-order)
11. [Vertical Slice Scope](#11-vertical-slice-scope)
12. [Edge Cases](#12-edge-cases)
13. [Integration Test Descriptions](#13-integration-test-descriptions)

---

## 1. Overview

Each run's planet has a set of physical properties generated from the run seed. Properties apply passive modifiers to gameplay systems (power generation efficiency, machine cooling, fluid processing), bias resource abundance and mixture during world generation, and give each run a legible identity that experienced players can read at landing and immediately form a rough strategy from.

Two systems close the design loop:
- **Visibility model** — properties are partially revealed at landing and fully revealed through scouting, rewarding players who engage with exploration.
- **Insight beat feedback** — a lightweight validation system emits a notification when the player's first power choice correlates with the planet's strongest-signal property, confirming their inference. VS implements this for power first; post-VS beats should also validate resource-scouting and processing decisions.

---

## 2. ECS Structure

### Planet entity

Spawned at run start by `generate_planet_properties`. Fixed for the entire run.

```
Planet entity
├── PlanetProperties          ← physical properties; save-game state
└── PlanetPropertyVisibility  ← per-property reveal state; save-game state
```

### `PlanetProperties` component

```
PlanetProperties (Component)
├── archetype: String                 ← archetype asset name; drives name generation
├── stellar_distance: f32             ← 0.0 (very close) → 1.0 (very far)
├── atmospheric_oxygen: f32           ← 0.0 (none) → 1.0 (super-oxygenated)
├── geological_activity: f32          ← 0.0 (inert) → 1.0 (hyperactive)
├── temperature: f32                  ← 0.0 (extreme cold) → 1.0 (extreme heat)
├── atmospheric_pressure: f32         ← 0.0 (near vacuum) → 1.0 (extreme pressure)
├── wind_intensity: f32               ← 0.0 (none) → 1.0 (gale-force)
├── hazard_type: EnvironmentalHazard  ← cosmetic only; fixed per run
└── name: PlanetName                  ← catalog designation + epithet
```

```rust
pub enum EnvironmentalHazard {
    EmInterference,
    CorrosiveParticulates,
    ExoticRadiation,
}

pub struct PlanetName {
    pub catalog: String,  // e.g. "KXR-4729"
    pub epithet: String,  // e.g. "Frozen Geothermal"
}
```

### `PlanetPropertyVisibility` component

Tracks which properties are visible to the player. Updated by scouting and sample analysis. Transitions are one-way (Hidden → Qualitative → Revealed).

```
PlanetPropertyVisibility (Component)
├── stellar_distance: PropertyVisibility
├── atmospheric_oxygen: PropertyVisibility
├── geological_activity: PropertyVisibility
├── temperature: PropertyVisibility
├── atmospheric_pressure: PropertyVisibility
├── wind_intensity: PropertyVisibility
└── hazard_type: PropertyVisibility     ← always Revealed; never changes
```

```rust
pub enum PropertyVisibility {
    Hidden,       // Not shown; player knows more data exists (locked row in UI)
    Qualitative,  // Category label shown (e.g. "Dim"); no exact value or modifier
    Revealed,     // Exact value + effect modifier shown
}
```

### Player entity additions

`generate_planet_properties` inserts two components onto the existing Player entity:

```
Player entity (additions)
├── PlanetPropertyViewLog  ← which properties the player has viewed; save-game state
└── InsightBeatFired       ← marker; present after insight beat check runs once
```

### `PlanetPropertyViewLog` component

```
PlanetPropertyViewLog (Component on Player)
└── viewed: HashSet<PlanetPropertyKey>
```

```rust
pub enum PlanetPropertyKey {
    StellarDistance,
    AtmosphericOxygen,
    GeologicalActivity,
    Temperature,
    AtmosphericPressure,
    WindIntensity,
    HazardType,
}
```

### `InsightBeatFired` component

Marker component inserted onto the Player entity after the insight beat check runs for the first time (whether or not it emits `PropertyDecisionValidated`). Guards against re-firing.

---

## 3. Planet Archetype & Property Generation

Generation runs once at run start using the `planet` domain sub-seed. All outputs are deterministic for a given seed.

### Archetype assets

Archetypes are Bevy assets loaded from `assets/planet/archetypes/*.ron`. Each file defines one archetype's property sampling ranges. The VS ships 3 files; add more files post-MVP to expand the pool.

Asset type:

```rust
#[derive(Asset, Reflect, Deserialize)]
pub struct PlanetArchetypeAsset {
    pub name: String,
    pub temperature: (f32, f32),
    pub geological_activity: (f32, f32),
    pub stellar_distance: (f32, f32),
    pub atmospheric_oxygen: (f32, f32),
    pub atmospheric_pressure: (f32, f32),
    pub wind_intensity: (f32, f32),
}
```

A `PlanetArchetypeHandles` resource (not save-game state) holds the `Vec<Handle<PlanetArchetypeAsset>>` loaded during `GameState::Loading`.

Example — `assets/planet/archetypes/frozen_geothermal.ron`:

```ron
(
    name: "FrozenGeothermal",
    temperature: (0.00, 0.35),
    geological_activity: (0.60, 1.00),
    stellar_distance: (0.55, 1.00),
    atmospheric_oxygen: (0.20, 0.60),
    atmospheric_pressure: (0.40, 0.80),
    wind_intensity: (0.10, 0.50),
)
```

`assets/planet/archetypes/desert_radiant.ron`:

```ron
(
    name: "DesertRadiant",
    temperature: (0.65, 1.00),
    geological_activity: (0.00, 0.35),
    stellar_distance: (0.00, 0.45),
    atmospheric_oxygen: (0.00, 0.35),
    atmospheric_pressure: (0.00, 0.45),
    wind_intensity: (0.00, 0.30),
)
```

`assets/planet/archetypes/humid_jungle.ron`:

```ron
(
    name: "HumidJungle",
    temperature: (0.45, 0.70),
    geological_activity: (0.30, 0.65),
    stellar_distance: (0.30, 0.65),
    atmospheric_oxygen: (0.60, 1.00),
    atmospheric_pressure: (0.55, 1.00),
    wind_intensity: (0.20, 0.60),
)
```

### Step 1 — Select archetype

Draw uniformly from `PlanetArchetypeHandles` using the planet sub-seed. All loaded archetypes have equal weight.

### Step 2 — Sample properties

For each property axis (in the order listed in `PlanetArchetypeAsset`), sample uniformly within the selected archetype's `(min, max)` range. Each property is an independent draw from the seeded RNG stream.

### Step 3 — Select hazard type

Sample `EnvironmentalHazard` uniformly from the three options, independent of archetype. Hazard type has no gameplay effect — it determines lore framing and warning text flavor only.

### Step 4 — Generate planet name

**Catalog designation:** `{3-letter code}-{4-digit number}`
- 3-letter code: seed-derived uppercase consonant sequence with a pseudo-vowel at position 2 (chosen from R, L, N) to be pronounceable. Example: "KRX", "VNB", "GLR".
- 4-digit number: `(planet_seed % 9000) + 1000` → range [1000, 9999].

**Epithet:** Two-word label from the two property axes with the highest normalized deviation from neutral:

For each axis p, deviation = `|p - 0.5| * 2` (0.0 = neutral, 1.0 = extreme end of range).

Deviation-to-label mapping:

| Axis | Low label (p < 0.5) | High label (p > 0.5) |
|---|---|---|
| temperature | Frozen | Scorching |
| geological_activity | Inert | Geothermal |
| stellar_distance | Radiant (close star = high irradiance) | Dim |
| atmospheric_oxygen | Anoxic | Oxygen-Rich |
| atmospheric_pressure | Thin-Air | Dense |
| wind_intensity | Calm | Windswept |

Epithet = `"{primary_label} {secondary_label}"` using the top two axes by deviation. Axes with deviation < 0.2 (near-neutral) are skipped; if fewer than two qualify, use only those that do. Primary label is listed first.

Example: temperature=0.05 (deviation=0.9), geological_activity=0.85 (deviation=0.7) → `"Frozen Geothermal"`.

Full display name on landing panel: `{catalog} "{epithet}"` — e.g., `KRX-4729 "Frozen Geothermal"`.

---

## 4. Property-to-Gameplay Effect Bindings

All modifiers are multipliers applied to the affected system's base value. 1.0 is neutral.

### Solar efficiency

```
solar_modifier = lerp(1.6, 0.4, stellar_distance)
```

| stellar_distance | solar_modifier | Irradiance label |
|---|---|---|
| 0.0 | 1.6× | Blazing |
| 0.25 | 1.3× | Strong |
| 0.5 | 1.0× | Moderate |
| 0.75 | 0.7× | Dim |
| 1.0 | 0.4× | Faint |

Applied to: base output of all solar generator machines.

### Combustion efficiency

```
combustion_modifier = lerp(0.3, 1.8, atmospheric_oxygen)
```

| atmospheric_oxygen | combustion_modifier | O₂ label |
|---|---|---|
| 0.0 | 0.3× | Trace |
| 0.25 | 0.675× | Scarce |
| 0.5 | 1.05× | Standard |
| 0.75 | 1.425× | Rich |
| 1.0 | 1.8× | Super-Oxygenated |

Applied to: fuel efficiency (output per unit fuel) of all combustion generator machines.

### Geothermal availability and output

Threshold: `geological_activity >= 0.3` → geothermal tech tree nodes are **eligible** for seeding this run. Below this threshold, geothermal nodes are excluded from the pool during tech tree generation.

When eligible:
```
geothermal_modifier = lerp(0.5, 2.0, (geological_activity - 0.3) / 0.7)
```

Applied to: base output of all geothermal generator machines.

> **Tech tree generation note:** The tech tree seeding step queries the Planet entity for `geological_activity` from `PlanetProperties` before generating the node pool. Geothermal and wind are the only VS properties that gate tech tree node **eligibility**. Other properties may bias resource worldgen and tune machine output, but should not remove required progression paths unless a validated fallback exists.

### Thermodynamic cycle efficiency

```
thermo_modifier = lerp(0.6, 1.4, temperature)
```

Applied to: base efficiency of thermodynamic cycle generators (heat engines, steam turbines).

Machine cooling cost:
```
cooling_modifier = lerp(0.5, 2.0, temperature)
```

Applied to: power draw for machine cooling components. Hot planets increase cooling costs; cold planets reduce them.

### Fluid dynamics

```
pressure_modifier = lerp(0.6, 1.4, atmospheric_pressure)
```

Applied to: throughput rate of fluid processing machines and pump efficiency.

### Wind power availability and output

Threshold: `wind_intensity >= 0.4` → wind power tech tree nodes are eligible for seeding. Below threshold, excluded.

When eligible:
```
wind_modifier = lerp(0.0, 2.0, wind_intensity)
```

Applied to: base output of wind generator machines.

### Resource abundance and mixture

World generation consumes `PlanetProperties` before placing resource deposits. These bindings do not replace resource placement rules from the world generator; they provide seeded, legible bias values that tune resource weights, deposit richness, and deposit form within hard solvability constraints.

Derived bindings:

```
deep_metal_abundance = lerp(0.7, 1.6, geological_activity)
surface_metal_exposure = lerp(1.3, 0.7, geological_activity)
fluid_pocket_abundance = lerp(0.6, 1.5, atmospheric_pressure)
volatile_ice_abundance = lerp(1.6, 0.4, temperature)
oxidized_material_bias = lerp(0.4, 1.6, atmospheric_oxygen)
reduced_material_bias = lerp(1.6, 0.4, atmospheric_oxygen)
erosion_exposure_bias = lerp(0.8, 1.4, wind_intensity)
```

Applied to:
- **Ore deposit richness:** multiplier on target deposit richness for matching resource classes, not a guarantee that every patch is rich.
- **Resource mixture:** relative weights between oxidized and reduced variants of material families when both are allowed by content data.
- **Deposit form:** probability that resources appear as exposed surface nodes, buried patches, fluid pockets, ice/volatile deposits, or unique geothermal sites.
- **Biome/domain affinity:** weighting within the valid domain+biome set for each resource. A resource with no valid biome or domain on the generated world must be substituted by a guaranteed fallback path during world validation.

Examples:
- High `geological_activity` increases deep metallic richness and geothermal-site resource weights, while reducing easy surface metal exposure.
- Low `temperature` increases ice and cryogenic volatile availability; high `temperature` suppresses those forms unless a biome explicitly overrides it.
- High `atmospheric_oxygen` favors oxidized material variants; low oxygen favors reduced variants.
- High `atmospheric_pressure` increases fluid-pocket frequency and pump-friendly resource clusters.

> **World generation note:** Resource placement queries the Planet entity for `PlanetProperties` before selecting final resource weights. These biases are soft unless a content asset marks a resource as hard-restricted to a property threshold. World validation must still ensure critical resources have at least one reachable acquisition path.

### Hazard type

No gameplay modifier. Determines:
- Aegis boundary warning text flavor (e.g., "EM field boundary" vs. "Corrosion boundary")
- Lore framing in codex entries for the aegis system
- Visual particle variant on the aegis field boundary

---

## 5. Property Visibility Model

### Initial state at run start

| Property | Display name | Initial visibility | Reveal trigger |
|---|---|---|---|
| temperature | Surface Temperature | Qualitative | Environmental sensors on escape pod |
| stellar_distance | Solar Irradiance | Qualitative | Star visible in sky at landing |
| wind_intensity | Wind Intensity | Qualitative | Visible from environment (particles, audio) |
| hazard_type | Environmental Hazard | Revealed | Always fully visible; safety-critical |
| geological_activity | Geological Activity | Hidden | First surface scan completes |
| atmospheric_oxygen | Atmospheric O₂ | Hidden | Atmospheric sample analysis |
| atmospheric_pressure | Atmospheric Pressure | Hidden | Atmospheric sample analysis |

Note: `stellar_distance` is displayed as **Solar Irradiance** throughout the UI. The label scale is inverted (lower distance = higher irradiance = higher label tier) for player legibility.

### Qualitative label tiers (5 tiers, linear across [0.0, 1.0])

| Property | [0.0, 0.2) | [0.2, 0.4) | [0.4, 0.6) | [0.6, 0.8) | [0.8, 1.0] |
|---|---|---|---|---|---|
| temperature | Frigid | Cold | Temperate | Warm | Scorching |
| solar irradiance (1−stellar_distance) | Faint | Dim | Moderate | Strong | Blazing |
| wind_intensity | Calm | Gentle | Moderate | Strong | Gale |
| geological_activity | Inert | Quiet | Active | Volatile | Hyperactive |
| atmospheric_oxygen | Trace | Scarce | Standard | Rich | Super-Oxygenated |
| atmospheric_pressure | Vacuum | Thin | Standard | Dense | Crushing |

### Reveal triggers

- **First surface scan:** Player initiates scan action in `PlayMode::DronePilot` → `geological_activity` transitions Hidden → Qualitative.
- **Atmospheric sample analysis:** Player completes atmospheric sample analysis in the research station → `atmospheric_oxygen` transitions Hidden → Revealed; `atmospheric_pressure` transitions Hidden → Revealed. Both fire as separate `PlanetPropertyRevealed` events.

Each transition fires `PlanetPropertyRevealed { property, new_visibility }`.

---

## 6. Landing Panel UI

Shown immediately on entering `GameState::Playing`, before any player input is processed. Implemented as a new `PlayMode::Landing` sub-state: the default `PlayMode` changes from `Exploring` to `Landing`. On panel dismiss, transitions to `PlayMode::Exploring`.

### ECS structure

```
PlanetLandingPanel entity
├── PlanetLandingPanel (marker component)
└── Node (full-screen modal UI root)
    └── [child UI nodes]
```

### Layout

```
┌─────────────────────────────────────────────────────┐
│  KRX-4729  "Frozen Geothermal"                      │
│  ══════════════════════════════════════             │
│  [Color strip: temperature-tinted gradient]          │
│                                                      │
│  ● Solar Irradiance       Faint                      │
│    Solar generators produce less power here          │
│                                                      │
│  ● Surface Temperature    Frigid                     │
│    Cooling costs reduced; ice deposits more likely   │
│                                                      │
│  ● Wind Intensity         Gentle                     │
│    Wind generation may be viable                     │
│                                                      │
│  ● Environmental Hazard   EM Interference            │
│    Operations outside aegis field disrupt AI         │
│                                                      │
│  ○ Geological Activity    [Scan required]            │
│    Deep metals and geothermal sites unknown          │
│  ○ Atmospheric O₂         [Sample required]          │
│    Combustion and material chemistry unknown         │
│  ○ Atmospheric Pressure   [Sample required]          │
│    Fluid pocket conditions unknown                   │
│                                                      │
│                              [ Begin Run → ]         │
└─────────────────────────────────────────────────────┘
```

- **Qualitative/Revealed rows (●):** icon + display name + qualitative label + one-line effect summary.
- **Hidden rows (○):** icon + display name + italicized hint text (`[Scan required]` or `[Sample required]`). Presence of locked rows communicates that more data exists.
- **Color strip:** background gradient derived from `temperature` (blue at 0.0 → orange-red at 1.0). VS: solid gradient only; no particle effects.

### Systems

**`spawn_landing_panel`**
- Schedule: `OnEnter(PlayMode::Landing)`
- Queries Planet entity for `PlanetProperties`, `PlanetPropertyVisibility`
- Spawns the `PlanetLandingPanel` entity and its full child UI tree

**`despawn_landing_panel`**
- Schedule: `OnExit(PlayMode::Landing)`
- Despawns `PlanetLandingPanel` entity and all children via `DespawnRecursive`

**`landing_panel_button`**
- Schedule: `Update` in `PlayMode::Landing`
- Watches "Begin Run" button interaction
- On press: transitions `PlayMode` → `Exploring`; emits `LandingPanelDismissed`

**`landing_panel_view_tracker`**
- Schedule: `Update` in `PlayMode::Landing`
- Watches `Interaction` component on each property row entity
- On `Interaction::Hovered` or `Interaction::Pressed`: emits `PlanetPropertyViewed { property, context: ViewContext::LandingPanel }`; inserts property key into `PlanetPropertyViewLog.viewed` on Player entity

---

## 7. In-Run Property Display

Planet properties are accessible via the Terminal's Planet page (follows the Terminal screen model in `inventory.md`). Accessible from any `PlayMode`.

### Display per property

- **Revealed:** display name + qualitative label + exact value (2 decimal places) + primary effect modifier(s). Example: `"Solar Irradiance: Faint [0.91] → 0.43× solar output"`. Properties with resource effects also list those derived biases, e.g. `"Atmospheric Pressure: Dense [0.78] → 1.22× fluid throughput, 1.30× fluid pocket abundance"`.
- **Qualitative:** display name + qualitative label + `"Exact data not yet gathered"`
- **Hidden:** display name + `"[Scan required]"` or `"[Sample required]"` + one-line hint on how to reveal

### View tracking

Viewing any property row in the Terminal emits `PlanetPropertyViewed { property, context: ViewContext::Terminal }` and inserts the key into `PlanetPropertyViewLog`. Uses the same hover/press interaction model as the landing panel.

### Live updates

When `PlanetPropertyRevealed` fires, the Terminal's Planet page updates the affected row immediately if currently open.

---

## 8. Insight Beat Feedback

Implements VS §3.2: the player infers a planet property, acts on it, receives concrete confirmation.

### `insight_beat_check` system

Schedule: `Update` in `PlayMode::Exploring | PlayMode::Building`

Does not run if Player entity has `InsightBeatFired` component.

On receiving the first `MachineBuilt` event where `machine_class == PowerProducer`:

1. Look up the relevant planet property and its modifier for the placed machine type (queries Planet entity for `PlanetProperties`):

| Machine type | Relevant property | Modifier | Threshold |
|---|---|---|---|
| SolarGenerator | stellar_distance | solar_modifier | >= 1.2× (stellar_distance <= 0.33) |
| CombustionGenerator | atmospheric_oxygen | combustion_modifier | >= 1.2× (atmospheric_oxygen >= 0.40) |
| GeothermalGenerator | geological_activity | geothermal_modifier | >= 1.2× (geological_activity >= 0.62) |
| WindGenerator | wind_intensity | wind_modifier | >= 1.2× (wind_intensity >= 0.64) |

2. If the modifier exceeds the threshold **AND** the relevant property key is in `PlanetPropertyViewLog.viewed` (on Player entity) → emit `PropertyDecisionValidated { property, machine_type, modifier }`.

3. Insert `InsightBeatFired` component onto Player entity regardless of whether the event fired.

### Validation notification

On `PropertyDecisionValidated`: spawn a non-blocking notification entity that despawns after 5 seconds or on player dismiss input.

Text pattern: `"Well-suited — {qualitative_label} {display_name} supports {machine_display_name} here."`

Examples:
- `"Well-suited — Faint Solar Irradiance supports Combustion Generator here."`
- `"Well-suited — Hyperactive Geological Activity supports Geothermal Generator here."`

---

## 9. Events

```rust
// Property transitions to higher visibility
pub struct PlanetPropertyRevealed {
    pub property: PlanetPropertyKey,
    pub new_visibility: PropertyVisibility,
}

// Player focuses a property row in the landing panel or terminal
pub struct PlanetPropertyViewed {
    pub property: PlanetPropertyKey,
    pub context: ViewContext,
}

pub enum ViewContext {
    LandingPanel,
    Terminal,
}

// First power machine placed and correlates with a viewed planet property
pub struct PropertyDecisionValidated {
    pub property: PlanetPropertyKey,
    pub machine_type: MachineType,
    pub modifier: f32,
}

// Player dismissed the landing panel
pub struct LandingPanelDismissed;
```

---

## 10. Execution Order

Within `GameSystems::Simulation`:

1. `generate_planet_properties` — `OnEnter(GameState::Playing)`; spawns Planet entity with `PlanetProperties` + `PlanetPropertyVisibility`; inserts `PlanetPropertyViewLog` onto Player entity
2. `spawn_landing_panel` — `OnEnter(PlayMode::Landing)`
3. `landing_panel_view_tracker` — `Update` in `PlayMode::Landing`
4. `landing_panel_button` — `Update` in `PlayMode::Landing`; transitions `PlayMode`
5. `despawn_landing_panel` — `OnExit(PlayMode::Landing)`
6. `property_reveal_system` — `Update`; reads scan/analysis completion events; mutates Planet entity's `PlanetPropertyVisibility`; emits `PlanetPropertyRevealed`
7. `terminal_planet_view_tracker` — `Update` while Terminal Planet page is open; emits `PlanetPropertyViewed`
8. `insight_beat_check` — `Update` in `PlayMode::Exploring | PlayMode::Building`; one-shot; guarded by `InsightBeatFired`

---

## 11. Vertical Slice Scope

VS delivers the full pipeline. Limitations:

- **3 archetypes** (FrozenGeothermal, DesertRadiant, HumidJungle). Post-MVP: larger pool from content data files.
- **3 hazard types**. Post-MVP: additional types with distinct visual variants.
- **Geological activity and wind intensity** are the only VS properties that gate tech tree node eligibility. Other properties can bias resource abundance, resource mixture, and processing economics, but cannot remove required progression paths without a validated fallback.
- **Insight beat** tracks only power-producing machine placements. Post-MVP: extend to other decision types (exploration routing, logistics architecture).
- **Color strip** on landing panel is a solid temperature-tinted gradient. Post-MVP: hazard-type particle overlay layer.
- **Full survey reveal** (bulk reveal of all Hidden properties) is not implemented in VS. Properties reveal only via their specific triggers.

---

## 12. Edge Cases

1. **Landing panel dismissed without viewing any properties:** `PlanetPropertyViewLog` is empty; `insight_beat_check` runs but `PropertyDecisionValidated` does not emit. Telemetry records the miss (VS bad signal if frequent).

2. **geological_activity exactly at threshold (0.3):** Geothermal nodes are eligible (inclusive). `geothermal_modifier = 0.5×`.

3. **wind_intensity exactly at threshold (0.4):** Wind nodes eligible. `wind_modifier = 0.8×` — below the 1.2× insight threshold, so `PropertyDecisionValidated` does not emit for wind even if the property was viewed.

4. **Balanced planet (all modifiers near 1.0×):** No `PropertyDecisionValidated` fires. Landing panel renders correctly with "Moderate" / "Standard" labels. Runs with no clear power signal are weak on VS signal §3.1; should not occur frequently — archetype sampling ranges are designed to produce clear reads.

5. **Player builds first power machine without viewing the relevant property:** `PropertyDecisionValidated` does not emit even if the modifier is above threshold. Player succeeded without reading the planet. Telemetry records this separately from the validation event.

6. **Player views property via Terminal after dismissing the landing panel, before building first power machine:** `PlanetPropertyViewLog` records Terminal views; `insight_beat_check` checks the full log and will emit `PropertyDecisionValidated` if conditions are met.

7. **Wind node seeded despite wind_intensity < 0.4:** Cannot occur — geothermal and wind eligibility is evaluated by the tech tree generator from `PlanetProperties` before any node is drawn. If no wind node was seeded despite meeting the threshold (node pool exhaustion or RNG did not select it), `wind_modifier` exists but no wind machine can be built; `insight_beat_check` receives no `MachineBuilt` event for WindGenerator.

---

## 13. Integration Test Descriptions

1. **Determinism:** Two independent calls to `generate_planet_properties` with the same seed string produce identical `PlanetProperties` components on the spawned Planet entity (all 7 float fields equal within f32 epsilon; enum fields exactly equal).

2. **Domain isolation:** `PlanetProperties` for seed S is unchanged when the world generation code path is modified. Verified by asserting `DomainSeeds::planet` is derived solely from the master seed (not from `DomainSeeds::world` or any other domain).

3. **Archetype bounds:** For 1000 seeded runs with archetype forced to each value, all 6 float property values fall within the archetype's defined range.

4. **Name uniqueness:** For 1000 distinct seeds, no two runs produce the same catalog designation string.

5. **Modifier formula spot-checks:**
   - `stellar_distance=0.0` → `solar_modifier=1.6`
   - `stellar_distance=0.5` → `solar_modifier=1.0`
   - `stellar_distance=1.0` → `solar_modifier=0.4`
   - `atmospheric_oxygen=0.0` → `combustion_modifier=0.3`
   - `atmospheric_oxygen=1.0` → `combustion_modifier=1.8`
   - `geological_activity=0.3` → `geothermal_modifier=0.5`
   - `geological_activity=1.0` → `geothermal_modifier=2.0`

6. **Geothermal gate:** `geological_activity=0.25` → geothermal nodes absent from tech tree pool. `geological_activity=0.35` → geothermal nodes present in pool.

7. **Initial visibility:** Immediately after `generate_planet_properties`: Planet entity's `PlanetPropertyVisibility` has `temperature`, `stellar_distance`, `wind_intensity` == Qualitative; `hazard_type` == Revealed; `atmospheric_oxygen`, `atmospheric_pressure`, `geological_activity` == Hidden.

8. **Reveal event:** Simulate atmospheric sample analysis completion → `PlanetPropertyRevealed { property: AtmosphericOxygen, new_visibility: Revealed }` emitted; Planet entity's `PlanetPropertyVisibility.atmospheric_oxygen` == Revealed afterwards.

9. **Landing panel lifecycle:** `PlanetLandingPanel` entity present during `PlayMode::Landing`; absent after transition to `PlayMode::Exploring`.

10. **View log — landing panel:** Player hovers the solar irradiance row → `PlanetPropertyViewed { property: StellarDistance, context: LandingPanel }` emitted; Player entity's `PlanetPropertyViewLog.viewed` contains `StellarDistance`.

11. **Insight beat fires:** Given `stellar_distance=0.1` (solar_modifier=1.48); `StellarDistance` in view log; first `MachineBuilt { machine_class: PowerProducer, machine_type: SolarGenerator }` → `PropertyDecisionValidated { property: StellarDistance, modifier: 1.48 }` emitted.

12. **Insight beat suppressed — property not viewed:** Same conditions, view log empty → `PropertyDecisionValidated` not emitted.

13. **Insight beat suppressed — modifier below threshold:** `stellar_distance=0.4` (solar_modifier=1.04); `StellarDistance` in view log; `MachineBuilt` for SolarGenerator → `PropertyDecisionValidated` not emitted (1.04 < 1.2 threshold).

14. **Insight beat fires once:** Two power machines placed sequentially in the same run → `PropertyDecisionValidated` emitted at most once; Player entity has `InsightBeatFired` component after the first machine.

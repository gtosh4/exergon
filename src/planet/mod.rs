//! Planet identity: per-run physical properties, visibility model, landing
//! panel, and insight beat feedback. Design: `docs/technical/planet-identity.md`.

use bevy::prelude::*;
use moonshine_save::prelude::Save;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::content::load_ron_dir;
use crate::drone::FogCellRevealedEvent;
use crate::machine::{MachineBuilt, MachineClass, PowerProducerKind};
use crate::power::EnvFactorRegistry;
use crate::research::TechNodeUnlocked;
use crate::seed::DomainSeeds;
use crate::world::Player;
use crate::{GameState, PlayMode};

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Planet>()
            .register_type::<PlanetProperties>()
            .register_type::<PlanetPropertyVisibility>()
            .register_type::<PropertyVisibility>()
            .register_type::<EnvironmentalHazard>()
            .register_type::<PlanetName>()
            .register_type::<PlanetPropertyViewLog>()
            .register_type::<InsightBeatFired>()
            .init_resource::<PlanetArchetypes>()
            .add_message::<PlanetPropertyRevealed>()
            .add_message::<PlanetPropertyViewed>()
            .add_message::<PropertyDecisionValidated>()
            .add_systems(Startup, load_archetypes)
            .add_systems(
                OnTransition {
                    exited: GameState::Loading,
                    entered: GameState::Playing,
                },
                (generate_planet_properties, init_env_factor_registry).chain(),
            )
            .add_systems(
                Update,
                insight_beat_check
                    .run_if(in_state(PlayMode::Exploring).or_else(in_state(PlayMode::Building))),
            )
            .add_systems(
                Update,
                property_reveal_system.run_if(in_state(GameState::Playing)),
            );
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
#[require(Save)]
pub struct Planet;

#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component)]
pub struct PlanetProperties {
    pub archetype: String,
    pub stellar_distance: f32,
    pub atmospheric_oxygen: f32,
    pub geological_activity: f32,
    pub temperature: f32,
    pub atmospheric_pressure: f32,
    pub wind_intensity: f32,
    pub hazard_type: EnvironmentalHazard,
    pub name: PlanetName,
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct PlanetPropertyVisibility {
    pub stellar_distance: PropertyVisibility,
    pub atmospheric_oxygen: PropertyVisibility,
    pub geological_activity: PropertyVisibility,
    pub temperature: PropertyVisibility,
    pub atmospheric_pressure: PropertyVisibility,
    pub wind_intensity: PropertyVisibility,
    pub hazard_type: PropertyVisibility,
}

impl Default for PlanetPropertyVisibility {
    fn default() -> Self {
        Self {
            stellar_distance: PropertyVisibility::Qualitative,
            atmospheric_oxygen: PropertyVisibility::Hidden,
            geological_activity: PropertyVisibility::Hidden,
            temperature: PropertyVisibility::Qualitative,
            atmospheric_pressure: PropertyVisibility::Hidden,
            wind_intensity: PropertyVisibility::Qualitative,
            hazard_type: PropertyVisibility::Revealed,
        }
    }
}

impl PlanetProperties {
    /// Numeric value for a property axis. `HazardType` has no numeric axis and
    /// returns 0.0.
    pub fn axis(&self, key: PlanetPropertyKey) -> f32 {
        match key {
            PlanetPropertyKey::StellarDistance => self.stellar_distance,
            PlanetPropertyKey::AtmosphericOxygen => self.atmospheric_oxygen,
            PlanetPropertyKey::GeologicalActivity => self.geological_activity,
            PlanetPropertyKey::Temperature => self.temperature,
            PlanetPropertyKey::AtmosphericPressure => self.atmospheric_pressure,
            PlanetPropertyKey::WindIntensity => self.wind_intensity,
            PlanetPropertyKey::HazardType => 0.0,
        }
    }
}

impl PlanetPropertyVisibility {
    pub fn get(&self, key: PlanetPropertyKey) -> PropertyVisibility {
        match key {
            PlanetPropertyKey::StellarDistance => self.stellar_distance,
            PlanetPropertyKey::AtmosphericOxygen => self.atmospheric_oxygen,
            PlanetPropertyKey::GeologicalActivity => self.geological_activity,
            PlanetPropertyKey::Temperature => self.temperature,
            PlanetPropertyKey::AtmosphericPressure => self.atmospheric_pressure,
            PlanetPropertyKey::WindIntensity => self.wind_intensity,
            PlanetPropertyKey::HazardType => self.hazard_type,
        }
    }

    pub fn set(&mut self, key: PlanetPropertyKey, value: PropertyVisibility) {
        let slot = match key {
            PlanetPropertyKey::StellarDistance => &mut self.stellar_distance,
            PlanetPropertyKey::AtmosphericOxygen => &mut self.atmospheric_oxygen,
            PlanetPropertyKey::GeologicalActivity => &mut self.geological_activity,
            PlanetPropertyKey::Temperature => &mut self.temperature,
            PlanetPropertyKey::AtmosphericPressure => &mut self.atmospheric_pressure,
            PlanetPropertyKey::WindIntensity => &mut self.wind_intensity,
            PlanetPropertyKey::HazardType => &mut self.hazard_type,
        };
        *slot = value;
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PropertyVisibility {
    #[default]
    Hidden,
    Qualitative,
    Revealed,
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlanetPropertyKey {
    StellarDistance,
    AtmosphericOxygen,
    GeologicalActivity,
    Temperature,
    AtmosphericPressure,
    WindIntensity,
    HazardType,
}

impl PlanetPropertyKey {
    pub const ALL: [PlanetPropertyKey; 7] = [
        PlanetPropertyKey::StellarDistance,
        PlanetPropertyKey::Temperature,
        PlanetPropertyKey::WindIntensity,
        PlanetPropertyKey::HazardType,
        PlanetPropertyKey::GeologicalActivity,
        PlanetPropertyKey::AtmosphericOxygen,
        PlanetPropertyKey::AtmosphericPressure,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            PlanetPropertyKey::StellarDistance => "Solar Irradiance",
            PlanetPropertyKey::Temperature => "Surface Temperature",
            PlanetPropertyKey::WindIntensity => "Wind Intensity",
            PlanetPropertyKey::HazardType => "Environmental Hazard",
            PlanetPropertyKey::GeologicalActivity => "Geological Activity",
            PlanetPropertyKey::AtmosphericOxygen => "Atmospheric O\u{2082}",
            PlanetPropertyKey::AtmosphericPressure => "Atmospheric Pressure",
        }
    }

    pub fn hidden_hint(self) -> &'static str {
        match self {
            PlanetPropertyKey::GeologicalActivity => "[Scan required]",
            PlanetPropertyKey::AtmosphericOxygen | PlanetPropertyKey::AtmosphericPressure => {
                "[Sample required]"
            }
            _ => "",
        }
    }

    pub fn power_hint(self) -> &'static str {
        match self {
            PlanetPropertyKey::StellarDistance => "Solar power output",
            PlanetPropertyKey::AtmosphericOxygen => "Combustion power output",
            PlanetPropertyKey::GeologicalActivity => "Geothermal power viability",
            PlanetPropertyKey::WindIntensity => "Wind power output",
            PlanetPropertyKey::Temperature => "Thermal process efficiency",
            PlanetPropertyKey::AtmosphericPressure => "Pressure-dependent processes",
            PlanetPropertyKey::HazardType => "Drone exposure risk",
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EnvironmentalHazard {
    #[default]
    EmInterference,
    CorrosiveParticulates,
    ExoticRadiation,
}

impl EnvironmentalHazard {
    pub fn display(self) -> &'static str {
        match self {
            EnvironmentalHazard::EmInterference => "EM Interference",
            EnvironmentalHazard::CorrosiveParticulates => "Corrosive Particulates",
            EnvironmentalHazard::ExoticRadiation => "Exotic Radiation",
        }
    }
}

#[derive(Reflect, Clone, Debug, Default)]
pub struct PlanetName {
    pub catalog: String,
    pub epithet: String,
}

#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component)]
pub struct PlanetPropertyViewLog {
    pub viewed: Vec<PlanetPropertyKey>,
}

impl PlanetPropertyViewLog {
    pub fn record(&mut self, key: PlanetPropertyKey) {
        if !self.viewed.contains(&key) {
            self.viewed.push(key);
        }
    }
    pub fn contains(&self, key: PlanetPropertyKey) -> bool {
        self.viewed.contains(&key)
    }
}

/// Marker on the Player entity; present after `insight_beat_check` ran once.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct InsightBeatFired;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct PlanetPropertyRevealed {
    pub property: PlanetPropertyKey,
    pub new_visibility: PropertyVisibility,
}

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct PlanetPropertyViewed {
    pub property: PlanetPropertyKey,
    pub context: ViewContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewContext {
    LandingPanel,
    Terminal,
}

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct PropertyDecisionValidated {
    pub property: PlanetPropertyKey,
    pub kind: PowerProducerKind,
    pub modifier: f32,
}

// ---------------------------------------------------------------------------
// Archetype asset
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub struct PlanetArchetypeDef {
    pub name: String,
    pub temperature: (f32, f32),
    pub geological_activity: (f32, f32),
    pub stellar_distance: (f32, f32),
    pub atmospheric_oxygen: (f32, f32),
    pub atmospheric_pressure: (f32, f32),
    pub wind_intensity: (f32, f32),
}

#[derive(Resource, Default)]
pub struct PlanetArchetypes {
    pub defs: Vec<PlanetArchetypeDef>,
}

fn load_archetypes(mut commands: Commands) {
    let defs = load_ron_dir::<PlanetArchetypeDef>("assets/planet/archetypes", "planet_archetype");
    info!("Loaded {} planet archetypes", defs.len());
    commands.insert_resource(PlanetArchetypes { defs });
}

// ---------------------------------------------------------------------------
// Modifier helpers (gameplay bindings). Phase 4 will wire combustion / wind /
// geothermal once those generator types exist.
// ---------------------------------------------------------------------------

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub fn solar_modifier(stellar_distance: f32) -> f32 {
    lerp(1.6, 0.4, stellar_distance)
}
pub fn combustion_modifier(atmospheric_oxygen: f32) -> f32 {
    lerp(0.3, 1.8, atmospheric_oxygen)
}
pub fn geothermal_modifier(geological_activity: f32) -> f32 {
    if geological_activity < 0.3 {
        0.0
    } else {
        lerp(0.5, 2.0, (geological_activity - 0.3) / 0.7)
    }
}
pub fn wind_modifier(wind_intensity: f32) -> f32 {
    if wind_intensity < 0.4 {
        0.0
    } else {
        lerp(0.0, 2.0, wind_intensity)
    }
}
pub fn thermo_modifier(temperature: f32) -> f32 {
    lerp(0.6, 1.4, temperature)
}
pub fn cooling_modifier(temperature: f32) -> f32 {
    lerp(0.5, 2.0, temperature)
}
pub fn pressure_modifier(atmospheric_pressure: f32) -> f32 {
    lerp(0.6, 1.4, atmospheric_pressure)
}

#[allow(clippy::indexing_slicing)]
pub fn qualitative_label(key: PlanetPropertyKey, value: f32) -> &'static str {
    let bucket = ((value.clamp(0.0, 1.0) * 5.0).floor() as usize).min(4);
    match key {
        PlanetPropertyKey::Temperature => {
            ["Frigid", "Cold", "Temperate", "Warm", "Scorching"][bucket]
        }
        PlanetPropertyKey::StellarDistance => {
            // Inverted: lower distance = higher irradiance.
            ["Blazing", "Strong", "Moderate", "Dim", "Faint"][bucket]
        }
        PlanetPropertyKey::WindIntensity => {
            ["Calm", "Gentle", "Moderate", "Strong", "Gale"][bucket]
        }
        PlanetPropertyKey::GeologicalActivity => {
            ["Inert", "Quiet", "Active", "Volatile", "Hyperactive"][bucket]
        }
        PlanetPropertyKey::AtmosphericOxygen => {
            ["Trace", "Scarce", "Standard", "Rich", "Super-Oxygenated"][bucket]
        }
        PlanetPropertyKey::AtmosphericPressure => {
            ["Vacuum", "Thin", "Standard", "Dense", "Crushing"][bucket]
        }
        PlanetPropertyKey::HazardType => "",
    }
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

fn generate_planet_properties(
    mut commands: Commands,
    archetypes: Res<PlanetArchetypes>,
    seeds_q: Query<&DomainSeeds>,
    existing_q: Query<Entity, With<Planet>>,
    player_q: Query<Entity, With<Player>>,
) {
    if !existing_q.is_empty() {
        info!("generate_planet_properties: planet already exists, skipping");
        return;
    }
    let seeds = match seeds_q.single() {
        Ok(s) => s,
        Err(err) => {
            warn!("generate_planet_properties: DomainSeeds query failed: {err:?}");
            return;
        }
    };
    if archetypes.defs.is_empty() {
        warn!("No planet archetypes loaded; skipping planet generation");
        return;
    }

    let mut rng = seeds.planet_rng();
    let props = generate_properties_from(&mut rng, &archetypes.defs, seeds.planet);
    let name_str = format!(
        "{} \"{}\"",
        props.name.catalog,
        if props.name.epithet.is_empty() {
            "Unnamed"
        } else {
            props.name.epithet.as_str()
        }
    );
    info!("Generated planet: {name_str} ({})", props.archetype);
    commands.spawn((Planet, props, PlanetPropertyVisibility::default()));

    if let Ok(player) = player_q.single() {
        commands
            .entity(player)
            .insert(PlanetPropertyViewLog::default());
    }
}

/// Pure generation used by tests; mutates the seeded RNG.
pub fn generate_properties_from<R: Rng>(
    rng: &mut R,
    defs: &[PlanetArchetypeDef],
    planet_seed: u64,
) -> PlanetProperties {
    debug_assert!(!defs.is_empty(), "archetype pool must not be empty");
    #[allow(clippy::indexing_slicing)]
    let archetype = &defs[rng.gen_range(0..defs.len())];

    let sample = |r: &mut R, (lo, hi): (f32, f32)| -> f32 {
        if hi <= lo { lo } else { r.gen_range(lo..hi) }
    };

    let temperature = sample(rng, archetype.temperature);
    let geological_activity = sample(rng, archetype.geological_activity);
    let stellar_distance = sample(rng, archetype.stellar_distance);
    let atmospheric_oxygen = sample(rng, archetype.atmospheric_oxygen);
    let atmospheric_pressure = sample(rng, archetype.atmospheric_pressure);
    let wind_intensity = sample(rng, archetype.wind_intensity);

    let hazard_type = match rng.gen_range(0..3) {
        0 => EnvironmentalHazard::EmInterference,
        1 => EnvironmentalHazard::CorrosiveParticulates,
        _ => EnvironmentalHazard::ExoticRadiation,
    };

    let name = build_name(
        planet_seed,
        temperature,
        geological_activity,
        stellar_distance,
        atmospheric_oxygen,
        atmospheric_pressure,
        wind_intensity,
    );

    PlanetProperties {
        archetype: archetype.name.clone(),
        stellar_distance,
        atmospheric_oxygen,
        geological_activity,
        temperature,
        atmospheric_pressure,
        wind_intensity,
        hazard_type,
        name,
    }
}

fn init_env_factor_registry(
    mut registry: ResMut<EnvFactorRegistry>,
    planet_q: Query<&PlanetProperties, With<Planet>>,
) {
    if let Ok(props) = planet_q.single() {
        registry.solar = solar_modifier(props.stellar_distance);
        registry.combustion = combustion_modifier(props.atmospheric_oxygen);
    }
}

#[allow(clippy::indexing_slicing)]
fn build_name(
    planet_seed: u64,
    temperature: f32,
    geological_activity: f32,
    stellar_distance: f32,
    atmospheric_oxygen: f32,
    atmospheric_pressure: f32,
    wind_intensity: f32,
) -> PlanetName {
    const CONSONANTS: &[u8] = b"BCDFGHJKMPQSTVWXZ";
    const VOWELS: &[u8] = b"RLN";
    let c1 = CONSONANTS[((planet_seed >> 4) as usize) % CONSONANTS.len()];
    let v = VOWELS[((planet_seed >> 12) as usize) % VOWELS.len()];
    let c3 = CONSONANTS[((planet_seed >> 20) as usize) % CONSONANTS.len()];
    let num = (planet_seed % 9000) + 1000;
    let catalog = format!("{}{}{}-{:04}", c1 as char, v as char, c3 as char, num);

    let axes: [(PlanetPropertyKey, f32); 6] = [
        (PlanetPropertyKey::Temperature, temperature),
        (PlanetPropertyKey::GeologicalActivity, geological_activity),
        (PlanetPropertyKey::StellarDistance, stellar_distance),
        (PlanetPropertyKey::AtmosphericOxygen, atmospheric_oxygen),
        (PlanetPropertyKey::AtmosphericPressure, atmospheric_pressure),
        (PlanetPropertyKey::WindIntensity, wind_intensity),
    ];

    let label_for = |key: PlanetPropertyKey, value: f32| -> &'static str {
        let low = value < 0.5;
        match key {
            PlanetPropertyKey::Temperature => {
                if low {
                    "Frozen"
                } else {
                    "Scorching"
                }
            }
            PlanetPropertyKey::GeologicalActivity => {
                if low {
                    "Inert"
                } else {
                    "Geothermal"
                }
            }
            PlanetPropertyKey::StellarDistance => {
                if low {
                    "Radiant"
                } else {
                    "Dim"
                }
            }
            PlanetPropertyKey::AtmosphericOxygen => {
                if low {
                    "Anoxic"
                } else {
                    "Oxygen-Rich"
                }
            }
            PlanetPropertyKey::AtmosphericPressure => {
                if low {
                    "Thin-Air"
                } else {
                    "Dense"
                }
            }
            PlanetPropertyKey::WindIntensity => {
                if low {
                    "Calm"
                } else {
                    "Windswept"
                }
            }
            _ => "",
        }
    };

    let mut deviations: Vec<(PlanetPropertyKey, f32, f32)> = axes
        .iter()
        .map(|&(k, v)| (k, v, (v - 0.5).abs() * 2.0))
        .filter(|(_, _, d)| *d >= 0.2)
        .collect();
    deviations.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let epithet = match deviations.as_slice() {
        [] => String::new(),
        [(k, v, _)] => label_for(*k, *v).to_string(),
        [(k1, v1, _), (k2, v2, _), ..] => {
            format!("{} {}", label_for(*k1, *v1), label_for(*k2, *v2))
        }
    };

    PlanetName { catalog, epithet }
}

// ---------------------------------------------------------------------------
// Property reveal (design §5)
// ---------------------------------------------------------------------------

/// Advances property visibility as the player scouts and researches, emitting a
/// `PlanetPropertyRevealed` per transition (design §5):
/// - First drone scan (fog reveal while in `DronePilot`) → `geological_activity`
///   Hidden → Qualitative.
/// - First research spend (tech node unlocked via research; atmospheric sample
///   analysis proxy) → `atmospheric_oxygen` and `atmospheric_pressure`
///   Hidden → Revealed.
fn property_reveal_system(
    mode: Res<State<PlayMode>>,
    mut fog: bevy::ecs::message::MessageReader<FogCellRevealedEvent>,
    mut unlocked: bevy::ecs::message::MessageReader<TechNodeUnlocked>,
    mut vis_q: Query<&mut PlanetPropertyVisibility>,
    mut revealed: bevy::ecs::message::MessageWriter<PlanetPropertyRevealed>,
) {
    // Drain both readers unconditionally so transitions fire exactly once.
    let scanned = fog.read().count() > 0 && *mode.get() == PlayMode::DronePilot;
    let researched = unlocked.read().filter(|n| n.via_research).count() > 0;
    let Ok(mut vis) = vis_q.single_mut() else {
        return;
    };

    if scanned && vis.geological_activity == PropertyVisibility::Hidden {
        vis.geological_activity = PropertyVisibility::Qualitative;
        revealed.write(PlanetPropertyRevealed {
            property: PlanetPropertyKey::GeologicalActivity,
            new_visibility: PropertyVisibility::Qualitative,
        });
    }

    if researched {
        for key in [
            PlanetPropertyKey::AtmosphericOxygen,
            PlanetPropertyKey::AtmosphericPressure,
        ] {
            if vis.get(key) != PropertyVisibility::Revealed {
                vis.set(key, PropertyVisibility::Revealed);
                revealed.write(PlanetPropertyRevealed {
                    property: key,
                    new_visibility: PropertyVisibility::Revealed,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Insight beat
// ---------------------------------------------------------------------------

fn insight_beat_check(
    mut commands: Commands,
    mut built: bevy::ecs::message::MessageReader<MachineBuilt>,
    mut validated: bevy::ecs::message::MessageWriter<PropertyDecisionValidated>,
    planet_q: Query<&PlanetProperties>,
    player_q: Query<(Entity, &PlanetPropertyViewLog, Has<InsightBeatFired>), With<Player>>,
) {
    let Ok((player, log, already)) = player_q.single() else {
        return;
    };
    if already {
        built.clear();
        return;
    }
    let Ok(planet) = planet_q.single() else {
        return;
    };

    let mut any_power = false;
    let mut emitted = None;
    for ev in built.read() {
        let MachineClass::PowerProducer(kind) = ev.class else {
            continue;
        };
        any_power = true;
        let (property, modifier) = match kind {
            PowerProducerKind::Solar => (
                PlanetPropertyKey::StellarDistance,
                solar_modifier(planet.stellar_distance),
            ),
            PowerProducerKind::Combustion => (
                PlanetPropertyKey::AtmosphericOxygen,
                combustion_modifier(planet.atmospheric_oxygen),
            ),
            PowerProducerKind::Geothermal => (
                PlanetPropertyKey::GeologicalActivity,
                geothermal_modifier(planet.geological_activity),
            ),
            PowerProducerKind::Wind => (
                PlanetPropertyKey::WindIntensity,
                wind_modifier(planet.wind_intensity),
            ),
        };
        if modifier >= 1.2 && log.contains(property) {
            emitted = Some(PropertyDecisionValidated {
                property,
                kind,
                modifier,
            });
        }
        break;
    }
    if !any_power {
        return;
    }
    if let Some(ev) = emitted {
        validated.write(ev);
    }
    commands.entity(player).insert(InsightBeatFired);
}

// ---------------------------------------------------------------------------
// Tests (minimal — covers Phase 1 spec items)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_pcg::Pcg64;

    fn frozen_geothermal() -> PlanetArchetypeDef {
        PlanetArchetypeDef {
            name: "FrozenGeothermal".into(),
            temperature: (0.00, 0.35),
            geological_activity: (0.60, 1.00),
            stellar_distance: (0.55, 1.00),
            atmospheric_oxygen: (0.20, 0.60),
            atmospheric_pressure: (0.40, 0.80),
            wind_intensity: (0.10, 0.50),
        }
    }

    fn defs() -> Vec<PlanetArchetypeDef> {
        vec![frozen_geothermal()]
    }

    #[test]
    fn determinism_same_seed_same_properties() {
        let seeds = DomainSeeds::from_master(12345);
        let mut r1 = seeds.planet_rng();
        let mut r2 = seeds.planet_rng();
        let a = generate_properties_from(&mut r1, &defs(), seeds.planet);
        let b = generate_properties_from(&mut r2, &defs(), seeds.planet);
        assert_eq!(a.archetype, b.archetype);
        assert!((a.stellar_distance - b.stellar_distance).abs() < f32::EPSILON);
        assert!((a.geological_activity - b.geological_activity).abs() < f32::EPSILON);
        assert!((a.temperature - b.temperature).abs() < f32::EPSILON);
        assert!((a.atmospheric_oxygen - b.atmospheric_oxygen).abs() < f32::EPSILON);
        assert!((a.atmospheric_pressure - b.atmospheric_pressure).abs() < f32::EPSILON);
        assert!((a.wind_intensity - b.wind_intensity).abs() < f32::EPSILON);
        assert_eq!(a.hazard_type, b.hazard_type);
        assert_eq!(a.name.catalog, b.name.catalog);
    }

    #[test]
    fn archetype_bounds_respected() {
        let arc = frozen_geothermal();
        let pool = vec![arc.clone()];
        for seed in [1u64, 7, 42, 99, 1000, 99999] {
            let mut rng = Pcg64::seed_from_u64(seed);
            let p = generate_properties_from(&mut rng, &pool, seed);
            assert!(p.temperature >= arc.temperature.0 && p.temperature < arc.temperature.1);
            assert!(
                p.geological_activity >= arc.geological_activity.0
                    && p.geological_activity < arc.geological_activity.1
            );
            assert!(
                p.stellar_distance >= arc.stellar_distance.0
                    && p.stellar_distance < arc.stellar_distance.1
            );
        }
    }

    #[test]
    fn modifier_formula_spot_checks() {
        assert!((solar_modifier(0.0) - 1.6).abs() < 1e-5);
        assert!((solar_modifier(0.5) - 1.0).abs() < 1e-5);
        assert!((solar_modifier(1.0) - 0.4).abs() < 1e-5);
        assert!((combustion_modifier(0.0) - 0.3).abs() < 1e-5);
        assert!((combustion_modifier(1.0) - 1.8).abs() < 1e-5);
        assert!((geothermal_modifier(0.25) - 0.0).abs() < 1e-5);
        assert!((geothermal_modifier(0.3) - 0.5).abs() < 1e-5);
        assert!((geothermal_modifier(1.0) - 2.0).abs() < 1e-5);
        assert!((wind_modifier(0.39) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn initial_visibility_matches_spec() {
        let v = PlanetPropertyVisibility::default();
        assert_eq!(v.temperature, PropertyVisibility::Qualitative);
        assert_eq!(v.stellar_distance, PropertyVisibility::Qualitative);
        assert_eq!(v.wind_intensity, PropertyVisibility::Qualitative);
        assert_eq!(v.hazard_type, PropertyVisibility::Revealed);
        assert_eq!(v.geological_activity, PropertyVisibility::Hidden);
        assert_eq!(v.atmospheric_oxygen, PropertyVisibility::Hidden);
        assert_eq!(v.atmospheric_pressure, PropertyVisibility::Hidden);
    }

    #[test]
    fn property_visibility_set_and_get() {
        let mut v = PlanetPropertyVisibility::default();
        v.set(
            PlanetPropertyKey::AtmosphericOxygen,
            PropertyVisibility::Revealed,
        );
        assert_eq!(
            v.get(PlanetPropertyKey::AtmosphericOxygen),
            PropertyVisibility::Revealed
        );
    }

    fn reveal_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
            .add_message::<FogCellRevealedEvent>()
            .add_message::<TechNodeUnlocked>()
            .add_message::<PlanetPropertyRevealed>()
            .add_systems(Update, property_reveal_system);
        app
    }

    fn set_mode(app: &mut App, mode: PlayMode) {
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<PlayMode>>()
            .set(mode);
        app.update();
    }

    fn revealed_events(app: &App) -> Vec<PlanetPropertyRevealed> {
        let msgs = app
            .world()
            .resource::<bevy::ecs::message::Messages<PlanetPropertyRevealed>>();
        let mut cursor = msgs.get_cursor();
        cursor.read(msgs).cloned().collect()
    }

    #[test]
    fn drone_scan_reveals_geological_activity() {
        let mut app = reveal_app();
        let planet = app
            .world_mut()
            .spawn((
                Planet,
                PlanetProperties::default(),
                PlanetPropertyVisibility::default(),
            ))
            .id();
        set_mode(&mut app, PlayMode::DronePilot);

        app.world_mut()
            .write_message(FogCellRevealedEvent { cell: IVec2::ZERO });
        app.update();

        let vis = app.world().get::<PlanetPropertyVisibility>(planet).unwrap();
        assert_eq!(vis.geological_activity, PropertyVisibility::Qualitative);
        let evs = revealed_events(&app);
        assert!(
            evs.iter()
                .any(|e| e.property == PlanetPropertyKey::GeologicalActivity
                    && e.new_visibility == PropertyVisibility::Qualitative)
        );
    }

    #[test]
    fn fog_reveal_outside_drone_mode_does_not_reveal() {
        let mut app = reveal_app();
        let planet = app
            .world_mut()
            .spawn((
                Planet,
                PlanetProperties::default(),
                PlanetPropertyVisibility::default(),
            ))
            .id();
        set_mode(&mut app, PlayMode::Exploring);

        app.world_mut()
            .write_message(FogCellRevealedEvent { cell: IVec2::ZERO });
        app.update();

        let vis = app.world().get::<PlanetPropertyVisibility>(planet).unwrap();
        assert_eq!(vis.geological_activity, PropertyVisibility::Hidden);
    }

    #[test]
    fn research_spend_reveals_atmospheric_properties() {
        let mut app = reveal_app();
        let planet = app
            .world_mut()
            .spawn((
                Planet,
                PlanetProperties::default(),
                PlanetPropertyVisibility::default(),
            ))
            .id();
        set_mode(&mut app, PlayMode::Exploring);

        app.world_mut().write_message(TechNodeUnlocked {
            node_id: "smelting".into(),
            via_research: true,
        });
        app.update();

        let vis = app.world().get::<PlanetPropertyVisibility>(planet).unwrap();
        assert_eq!(vis.atmospheric_oxygen, PropertyVisibility::Revealed);
        assert_eq!(vis.atmospheric_pressure, PropertyVisibility::Revealed);
        let evs = revealed_events(&app);
        assert_eq!(
            evs.iter()
                .filter(|e| e.new_visibility == PropertyVisibility::Revealed)
                .count(),
            2
        );
    }

    fn insight_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
            .add_message::<MachineBuilt>()
            .add_message::<PropertyDecisionValidated>()
            .add_systems(Update, insight_beat_check);
        app
    }

    fn spawn_planet_and_player(
        app: &mut App,
        planet: PlanetProperties,
        viewed: &[PlanetPropertyKey],
    ) -> Entity {
        app.world_mut().spawn((Planet, planet));
        let mut log = PlanetPropertyViewLog::default();
        for k in viewed {
            log.record(*k);
        }
        app.world_mut().spawn((Player, log)).id()
    }

    fn solar_planet(stellar_distance: f32) -> PlanetProperties {
        PlanetProperties {
            stellar_distance,
            ..Default::default()
        }
    }

    #[test]
    fn insight_beat_fires_when_modifier_high_and_property_viewed() {
        let mut app = insight_app();
        let player = spawn_planet_and_player(
            &mut app,
            solar_planet(0.1),
            &[PlanetPropertyKey::StellarDistance],
        );
        app.world_mut().write_message(MachineBuilt {
            entity: Entity::PLACEHOLDER,
            machine_type: "solar_generator".into(),
            class: MachineClass::PowerProducer(PowerProducerKind::Solar),
            pos: Vec3::ZERO,
        });
        app.update();
        let events = app
            .world()
            .resource::<bevy::ecs::message::Messages<PropertyDecisionValidated>>();
        let mut reader = events.get_cursor();
        let got: Vec<_> = reader.read(events).cloned().collect();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].property, PlanetPropertyKey::StellarDistance);
        assert!(app.world().get::<InsightBeatFired>(player).is_some());
    }

    #[test]
    fn insight_beat_suppressed_when_property_not_viewed() {
        let mut app = insight_app();
        spawn_planet_and_player(&mut app, solar_planet(0.1), &[]);
        app.world_mut().write_message(MachineBuilt {
            entity: Entity::PLACEHOLDER,
            machine_type: "solar_generator".into(),
            class: MachineClass::PowerProducer(PowerProducerKind::Solar),
            pos: Vec3::ZERO,
        });
        app.update();
        let events = app
            .world()
            .resource::<bevy::ecs::message::Messages<PropertyDecisionValidated>>();
        let mut reader = events.get_cursor();
        assert_eq!(reader.read(events).count(), 0);
    }

    #[test]
    fn insight_beat_suppressed_when_modifier_below_threshold() {
        let mut app = insight_app();
        // stellar_distance = 0.4 → solar_modifier = 1.6 - 1.2*0.4 = 1.12 < 1.2
        spawn_planet_and_player(
            &mut app,
            solar_planet(0.4),
            &[PlanetPropertyKey::StellarDistance],
        );
        app.world_mut().write_message(MachineBuilt {
            entity: Entity::PLACEHOLDER,
            machine_type: "solar_generator".into(),
            class: MachineClass::PowerProducer(PowerProducerKind::Solar),
            pos: Vec3::ZERO,
        });
        app.update();
        let events = app
            .world()
            .resource::<bevy::ecs::message::Messages<PropertyDecisionValidated>>();
        let mut reader = events.get_cursor();
        assert_eq!(reader.read(events).count(), 0);
    }
}

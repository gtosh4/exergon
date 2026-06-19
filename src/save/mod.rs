//! Run save / load. See `docs/technical/save.md`.
//!
//! VS scope (this module):
//! - Run entity with `Run` marker, `RunSaveHeader`, `RunSeed`, `DomainSeeds`
//! - Primary run save (single RON per run, header-only read path)
//! - New-run + load flows, meta save stub
//!
//! Deviations from design (tracked):
//! - `TechTreeProgress` and `ResearchPool` remain Resources, saved via
//!   `SaveWorld::include_resource`. Migration to components is a separate task.
//! - Cables and networks are now persisted (LogisticsCableSegment, PowerCableSegment,
//!   their network entities, and LogisticsNetworkMember/PowerNetworkMember with MapEntities).
//!   Port entities (LogisticsPortOf, EnergyPortOf) are still ephemeral — recreated by
//!   on_machine_added and re-joined via port_placed_system on first Playing Update.
//! - Remaining gameplay entities not yet saved: Drone, MinedDeposit, Outpost, Player.
//! - Checkpoints and rolling backups are post-VS.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use base58::ToBase58;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use moonshine_save::prelude::*;
use serde::de::DeserializeSeed;

use crate::GameState;
use crate::logistics::{
    LogisticsCableSegment, LogisticsNetwork, LogisticsNetworkMember, StorageUnit,
};
use crate::machine::{Machine, MachineState, Platform};
use crate::planet::{Planet, PlanetProperties};
use crate::pod::PodNetwork;
use crate::power::{GeneratorUnit, PodPowered};
use crate::power::{PowerCableSegment, PowerNetwork, PowerNetworkMember};
use crate::research::{ResearchPool, TechTreeProgress};
use crate::seed::{DomainSeeds, RunSeed, hash_text};

/// Marker for the run-scoped entity. Any entity carrying `Run` is auto-tagged
/// for serialization via `#[require(Save)]`.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
#[require(Save)]
pub struct Run;

#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct RunSaveHeader {
    pub run_id: String,
    pub seed_text: String,
    pub difficulty: DifficultyTier,
    pub status: RunStatus,
    pub start_time_secs: u64,
    pub end_time_secs: Option<u64>,
    pub total_playtime_secs: f64,
    pub checkpoints: Vec<CheckpointHeader>,
}

#[derive(Reflect, Clone, Default, Debug, PartialEq, Eq)]
pub struct CheckpointHeader {
    pub kind: CheckpointKind,
    pub created_at_secs: u64,
    pub label: String,
    pub file_name: String,
}

#[derive(Reflect, Clone, Default, Debug, PartialEq, Eq)]
pub enum CheckpointKind {
    #[default]
    Manual,
    TierUnlock(u8),
    EscapeConstructionStart,
}

#[derive(Reflect, Clone, Default, Debug, PartialEq, Eq)]
pub enum RunStatus {
    #[default]
    InProgress,
    Completed,
}

#[derive(Reflect, Clone, Default, Debug, PartialEq, Eq)]
pub enum DifficultyTier {
    #[default]
    Initiation,
    Standard,
    Advanced,
    Pinnacle,
}

/// Marker resource: current run started with the dev test loadout modifier.
#[derive(Resource, Default)]
pub struct DevTestMode;

/// Player submitted a new run from the menu.
#[derive(Message)]
pub struct NewRunEvent {
    pub seed_text: String,
    pub test_mode: bool,
}

/// Player chose to load a run from the run-select screen.
#[derive(Message)]
pub struct LoadRunEvent {
    pub run_id: String,
}

/// Run completed (escape sequence finished).
#[derive(Message, Default)]
pub struct RunEndEvent;

pub const AUTO_SAVE_SECS: f32 = 60.0;

#[derive(Resource, Default)]
pub struct AutoSaveTimer(pub f32);

/// Resource holding the root save directory. Override in tests.
#[derive(Resource, Clone)]
pub struct SaveRoot(pub PathBuf);

impl Default for SaveRoot {
    fn default() -> Self {
        Self(PathBuf::from("saves"))
    }
}

impl SaveRoot {
    pub fn runs_dir(&self) -> PathBuf {
        self.0.join("runs")
    }
    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(run_id)
    }
    pub fn run_save_path(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("run.ron")
    }
    pub fn meta_path(&self) -> PathBuf {
        self.0.join("meta.ron")
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveRoot>()
            .init_resource::<AutoSaveTimer>()
            .register_type::<Run>()
            .register_type::<RunSaveHeader>()
            .register_type::<RunStatus>()
            .register_type::<DifficultyTier>()
            .register_type::<CheckpointKind>()
            .register_type::<CheckpointHeader>()
            .register_type::<Vec<CheckpointHeader>>()
            .register_type::<Option<u64>>()
            .add_message::<NewRunEvent>()
            .add_message::<LoadRunEvent>()
            .add_message::<RunEndEvent>()
            .add_observer(save_on_default_event)
            .add_observer(load_on_default_event)
            .add_systems(
                Update,
                (
                    spawn_run_on_new_event,
                    load_run_on_event,
                    update_playtime,
                    auto_save_tick,
                    save_on_run_end,
                ),
            )
            .add_systems(OnExit(GameState::Playing), exit_save);
    }
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `base58(blake3(start_time_secs).truncate(8 bytes))`.
pub fn derive_run_id(start_time_secs: u64) -> String {
    let hash = blake3::hash(&start_time_secs.to_le_bytes());
    let bytes = hash.as_bytes();
    bytes.get(..8).unwrap_or(bytes).to_base58()
}

fn spawn_run_on_new_event(
    mut commands: Commands,
    mut events: MessageReader<NewRunEvent>,
    save_root: Res<SaveRoot>,
    mut next_state: ResMut<NextState<GameState>>,
    pod_machines_q: Query<Entity, With<PodPowered>>,
) {
    let Some(event) = events.read().last() else {
        return;
    };
    // Clear pod machines left from any previous session (they are Save-marked and persist
    // when exiting Playing, so a new run must clean them up before spawning fresh ones).
    for e in &pod_machines_q {
        commands.entity(e).despawn();
    }
    if event.test_mode {
        commands.insert_resource(DevTestMode);
    } else {
        commands.remove_resource::<DevTestMode>();
    }
    let start = now_unix_secs();
    let run_id = derive_run_id(start);
    let hash = hash_text(&event.seed_text);
    commands.spawn((
        Run,
        RunSaveHeader {
            run_id: run_id.clone(),
            seed_text: event.seed_text.clone(),
            difficulty: DifficultyTier::Initiation,
            status: RunStatus::InProgress,
            start_time_secs: start,
            end_time_secs: None,
            total_playtime_secs: 0.0,
            checkpoints: Vec::new(),
        },
        RunSeed {
            text: event.seed_text.clone(),
            hash,
        },
        DomainSeeds::from_master(hash),
    ));
    // Make sure dir exists so first periodic save lands somewhere.
    let _ = std::fs::create_dir_all(save_root.run_dir(&run_id));
    next_state.set(GameState::Loading);
}

fn load_run_on_event(
    mut commands: Commands,
    mut events: MessageReader<LoadRunEvent>,
    save_root: Res<SaveRoot>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(event) = events.read().last() else {
        return;
    };
    let path = save_root.run_save_path(&event.run_id);
    commands.trigger_load(LoadWorld::default_from_file(path));
    next_state.set(GameState::Loading);
}

fn update_playtime(time: Res<Time>, mut header_q: Query<&mut RunSaveHeader>) {
    let Ok(mut header) = header_q.single_mut() else {
        return;
    };
    if header.status == RunStatus::InProgress {
        header.total_playtime_secs += time.delta_secs_f64();
    }
}

fn auto_save_tick(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<AutoSaveTimer>,
    save_root: Res<SaveRoot>,
    header_q: Query<&RunSaveHeader>,
) {
    let Ok(header) = header_q.single() else {
        timer.0 = 0.0;
        return;
    };
    if header.status != RunStatus::InProgress {
        return;
    }
    timer.0 += time.delta_secs();
    if timer.0 >= AUTO_SAVE_SECS {
        timer.0 = 0.0;
        trigger_run_save(&mut commands, &save_root, &header.run_id);
    }
}

fn exit_save(mut commands: Commands, save_root: Res<SaveRoot>, header_q: Query<&RunSaveHeader>) {
    let Ok(header) = header_q.single() else {
        return;
    };
    if header.status == RunStatus::InProgress {
        trigger_run_save(&mut commands, &save_root, &header.run_id);
    }
}

fn save_on_run_end(
    mut commands: Commands,
    mut events: MessageReader<RunEndEvent>,
    mut header_q: Query<&mut RunSaveHeader>,
    save_root: Res<SaveRoot>,
) {
    if events.read().next().is_none() {
        return;
    }
    let Ok(mut header) = header_q.single_mut() else {
        return;
    };
    header.status = RunStatus::Completed;
    header.end_time_secs = Some(now_unix_secs());
    trigger_run_save(&mut commands, &save_root, &header.run_id);
    write_meta_save(&save_root);
}

/// Trigger a primary save write for the given run.
pub fn trigger_run_save(commands: &mut Commands, save_root: &SaveRoot, run_id: &str) {
    let path = save_root.run_save_path(run_id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut save = SaveWorld::default_into_file(path)
        .include_resource::<TechTreeProgress>()
        .include_resource::<ResearchPool>();
    save.components = SceneFilter::deny_all()
        .allow::<Run>()
        .allow::<RunSaveHeader>()
        .allow::<RunSeed>()
        .allow::<DomainSeeds>()
        .allow::<Machine>()
        .allow::<MachineState>()
        .allow::<Transform>()
        .allow::<StorageUnit>()
        .allow::<GeneratorUnit>()
        .allow::<PodPowered>()
        .allow::<PodNetwork>()
        .allow::<Platform>()
        .allow::<Planet>()
        .allow::<PlanetProperties>()
        .allow::<LogisticsNetwork>()
        .allow::<LogisticsCableSegment>()
        .allow::<LogisticsNetworkMember>()
        .allow::<PowerNetwork>()
        .allow::<PowerCableSegment>()
        .allow::<PowerNetworkMember>();
    commands.trigger_save(save);
}

/// Empty meta save stub for VS — Codex/Blueprints land here post-VS.
pub fn write_meta_save(save_root: &SaveRoot) {
    let path = save_root.meta_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, "// meta save stub — populated post-VS\n()\n");
}

/// Header-only deserialization for the run-select screen. Returns `None` if
/// the file is missing or the header could not be parsed.
pub fn read_run_header(
    save_root: &SaveRoot,
    run_id: &str,
    registry: &TypeRegistry,
) -> Option<RunSaveHeader> {
    let path = save_root.run_save_path(run_id);
    let bytes = std::fs::read(&path).ok()?;
    let mut de = ron::Deserializer::from_bytes(&bytes).ok()?;
    let scene_de = bevy::scene::serde::SceneDeserializer {
        type_registry: registry,
    };
    let scene = scene_de.deserialize(&mut de).ok()?;
    for entity in &scene.entities {
        for component in &entity.components {
            let info = component.get_represented_type_info()?;
            if info.type_path() == RunSaveHeader::type_path() {
                return <RunSaveHeader as bevy::reflect::FromReflect>::from_reflect(
                    component.as_partial_reflect(),
                );
            }
        }
    }
    None
}

/// List run ids that have a `run.ron` on disk.
pub fn list_run_ids(save_root: &SaveRoot) -> Vec<String> {
    let dir = save_root.runs_dir();
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(|s| s.to_owned()) else {
            continue;
        };
        if save_root.run_save_path(&name).is_file() {
            out.push(name);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_is_base58_and_deterministic() {
        let a = derive_run_id(1_700_000_000);
        let b = derive_run_id(1_700_000_000);
        assert_eq!(a, b);
        assert!(!a.is_empty());
        assert!(a.chars().all(|c| !"0OIl".contains(c)));
    }

    #[test]
    fn run_id_changes_with_input() {
        assert_ne!(derive_run_id(1), derive_run_id(2));
    }

    #[test]
    fn save_root_paths_compose() {
        let root = SaveRoot(PathBuf::from("/tmp/exergon_test"));
        let p = root.run_save_path("abc");
        assert!(p.ends_with("runs/abc/run.ron"));
        assert!(root.meta_path().ends_with("meta.ron"));
    }
}

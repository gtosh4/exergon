//! Development-build telemetry. See `docs/technical/telemetry.md`.
//!
//! VS scope (this module):
//! - `TelemetryLog` resource collecting JSON records, flushed on
//!   `OnExit(GameState::Playing)` as JSONL.
//! - `#[cfg(debug_assertions)]` gate — plugin is a no-op in release builds.
//! - Events wired here are limited to those currently emitted by the game:
//!   `RunStarted`, `MachinePlaced`/`MachineRemoved` (from `WorldObjectEvent`),
//!   `Discovery` (from `DiscoveryEvent`), `TechRevealed` (diff of
//!   `TechTreeProgress.unlocked_nodes`), `RemoteModeEntry`/`Exit`,
//!   `RunAbandoned`/`EscapeCompleted`.
//!
//! Deferred until the corresponding game systems land (per
//! `docs/technical/telemetry.md §3`): `RecipeStarted`/`RecipeFinished`,
//! `RecipeBlocked*` family, `PropertyViewed`, `EscapeItemProduced`,
//! `ProductionStalled*`, `PowerNetworkFailure/Restored`, `LogisticStall`.

#![cfg(debug_assertions)]

use std::collections::HashSet;
use std::io::Write as _;
use std::path::PathBuf;

use bevy::prelude::*;
use serde_json::json;

use crate::escape::EscapeEvent;
use crate::logistics::JobComplete;
use crate::research::TechTreeProgress;
use crate::save::{Run, RunSaveHeader, SaveRoot};
use crate::world::{WorldObjectEvent, WorldObjectKind};
use crate::{GameState, PlayMode};

#[derive(Resource)]
pub struct TelemetryLog {
    pub run_id: String,
    pub elapsed_secs: f32,
    pub records: Vec<serde_json::Value>,
    pub prev_unlocked: HashSet<String>,
    pub escaped: bool,
    pub remote_trips: u32,
    pub remote_entry_t: Option<f32>,
}

impl TelemetryLog {
    fn append(&mut self, event: &str, fields: serde_json::Value) {
        let mut record = json!({
            "t": (self.elapsed_secs * 100.0).round() / 100.0,
            "event": event,
        });
        if let serde_json::Value::Object(extra) = fields
            && let serde_json::Value::Object(record_map) = &mut record
        {
            for (k, v) in extra {
                record_map.insert(k, v);
            }
        }
        self.records.push(record);
    }
}

/// Override for the directory where JSONL telemetry files are written. When
/// absent, the writer falls back to `<save_root>/telemetry/`. Tests insert
/// this resource to point at a temp dir.
#[derive(Resource, Clone, Default)]
pub struct TelemetryRoot(pub Option<PathBuf>);

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TelemetryRoot>()
            .add_message::<JobComplete>()
            .add_message::<EscapeEvent>()
            .add_message::<crate::planet::PropertyDecisionValidated>()
            .add_systems(
                OnEnter(GameState::Playing),
                telemetry_run_start.run_if(not(resource_exists::<TelemetryLog>)),
            )
            .add_systems(OnEnter(GameState::MainMenu), telemetry_run_end)
            .add_systems(OnEnter(GameState::Escaped), telemetry_run_end)
            .add_systems(OnEnter(PlayMode::DronePilot), telemetry_remote_enter)
            .add_systems(OnExit(PlayMode::DronePilot), telemetry_remote_exit)
            .add_systems(
                Update,
                (
                    telemetry_tick_elapsed,
                    telemetry_observe_machines,
                    telemetry_observe_discovery,
                    telemetry_observe_research,
                    telemetry_observe_escape_item,
                    telemetry_observe_escape_complete,
                    telemetry_observe_insight,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn telemetry_path(telemetry_root: &TelemetryRoot, save_root: &SaveRoot, run_id: &str) -> PathBuf {
    telemetry_root
        .0
        .clone()
        .unwrap_or_else(|| save_root.0.join("telemetry"))
        .join(format!("{run_id}.jsonl"))
}

fn telemetry_run_start(
    mut commands: Commands,
    header_q: Query<&RunSaveHeader, With<Run>>,
    telemetry_root: Res<TelemetryRoot>,
    save_root: Res<SaveRoot>,
) {
    let (run_id, unix_ts) = match header_q.single() {
        Ok(h) => (h.run_id.clone(), h.start_time_secs),
        Err(_) => ("unknown".to_owned(), 0),
    };
    let path = telemetry_path(&telemetry_root, &save_root, &run_id);
    let session_n = if path.exists() {
        std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| l.contains("\"RunStarted\"") || l.contains("\"RunResumed\""))
            .count() as u32
            + 1
    } else {
        1
    };
    let mut log = TelemetryLog {
        run_id: run_id.clone(),
        elapsed_secs: 0.0,
        records: Vec::new(),
        prev_unlocked: HashSet::new(),
        escaped: false,
        remote_trips: 0,
        remote_entry_t: None,
    };
    if session_n == 1 {
        log.append("RunStarted", json!({ "seed": run_id, "unix_ts": unix_ts }));
    } else {
        log.append(
            "RunResumed",
            json!({ "seed": run_id, "session_n": session_n, "unix_ts": unix_ts }),
        );
    }
    commands.insert_resource(log);
}

fn telemetry_tick_elapsed(time: Res<Time>, log: Option<ResMut<TelemetryLog>>) {
    if let Some(mut log) = log {
        log.elapsed_secs += time.delta_secs();
    }
}

fn telemetry_observe_machines(
    mut events: MessageReader<WorldObjectEvent>,
    log: Option<ResMut<TelemetryLog>>,
) {
    let Some(mut log) = log else { return };
    for e in events.read() {
        let event_name = match e.kind {
            WorldObjectKind::Placed => "MachinePlaced",
            WorldObjectKind::Removed => "MachineRemoved",
        };
        log.append(
            event_name,
            json!({
                "machine_type": e.item_id,
                "grid_pos": [e.transform.translation.x, e.transform.translation.y, e.transform.translation.z],
            }),
        );
    }
}

fn telemetry_observe_discovery(
    mut events: MessageReader<crate::research::DiscoveryEvent>,
    log: Option<ResMut<TelemetryLog>>,
) {
    let Some(mut log) = log else { return };
    for e in events.read() {
        let key = e.0.clone();
        log.append("Discovery", json!({ "key": key }));
    }
}

fn telemetry_observe_research(
    progress: Option<Res<TechTreeProgress>>,
    log: Option<ResMut<TelemetryLog>>,
) {
    let (Some(progress), Some(mut log)) = (progress, log) else {
        return;
    };
    let current: HashSet<String> = progress.unlocked_nodes.iter().cloned().collect();
    let new_nodes: Vec<String> = current.difference(&log.prev_unlocked).cloned().collect();
    for node_id in new_nodes {
        log.append("TechRevealed", json!({ "node_id": node_id }));
    }
    log.prev_unlocked = current;
}

fn telemetry_remote_enter(log: Option<ResMut<TelemetryLog>>) {
    let Some(mut log) = log else { return };
    log.remote_trips += 1;
    log.remote_entry_t = Some(log.elapsed_secs);
    let trip = log.remote_trips;
    log.append("RemoteModeEntry", json!({ "trip_n": trip }));
}

fn telemetry_remote_exit(log: Option<ResMut<TelemetryLog>>) {
    let Some(mut log) = log else { return };
    let trip = log.remote_trips;
    let entry = log.remote_entry_t.take().unwrap_or(log.elapsed_secs);
    let duration = log.elapsed_secs - entry;
    log.append(
        "RemoteModeExit",
        json!({ "trip_n": trip, "duration_secs": duration }),
    );
}

fn telemetry_observe_escape_item(
    mut job_complete: MessageReader<JobComplete>,
    log: Option<ResMut<TelemetryLog>>,
) {
    let Some(mut log) = log else { return };
    for job in job_complete.read() {
        if job.recipe_id == "forge_gateway_key" {
            log.append("EscapeItemProduced", json!({ "recipe_id": job.recipe_id }));
        }
    }
}

fn telemetry_observe_escape_complete(
    mut escape_events: MessageReader<EscapeEvent>,
    log: Option<ResMut<TelemetryLog>>,
) {
    let Some(mut log) = log else { return };
    for ev in escape_events.read() {
        log.escaped = true;
        log.append(
            "EscapeCompleted",
            json!({ "escape_time_secs": ev.escape_time_secs }),
        );
    }
}

fn telemetry_observe_insight(
    mut validated: MessageReader<crate::planet::PropertyDecisionValidated>,
    log: Option<ResMut<TelemetryLog>>,
) {
    let Some(mut log) = log else { return };
    for ev in validated.read() {
        log.append(
            "InsightCandidate",
            json!({ "property": format!("{:?}", ev.property), "modifier": ev.modifier }),
        );
    }
}

fn telemetry_run_end(
    mut commands: Commands,
    log: Option<ResMut<TelemetryLog>>,
    telemetry_root: Res<TelemetryRoot>,
    save_root: Res<SaveRoot>,
) {
    let Some(mut log) = log else { return };
    if !log.escaped {
        log.append("RunAbandoned", json!({}));
    }
    let path = telemetry_path(&telemetry_root, &save_root, &log.run_id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let lines: Vec<String> = log
        .records
        .iter()
        .filter_map(|r| serde_json::to_string(r).ok())
        .collect();
    let body = lines.join("\n") + "\n";
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = f.write_all(body.as_bytes());
    }
    commands.remove_resource::<TelemetryLog>();
}

#[cfg(test)]
mod tests {
    use bevy::scene::ScenePlugin;
    use bevy::state::app::StatesPlugin;

    use super::*;
    use crate::research::{DiscoveryEvent, ResearchPlugin};
    use crate::save::{NewRunEvent, SavePlugin};
    use crate::seed::SeedPlugin;

    fn tmp_root(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("exergon_tel_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn make_app(save_dir: PathBuf, telemetry_dir: PathBuf) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            StatesPlugin,
            ScenePlugin,
        ))
        .insert_resource(SaveRoot(save_dir))
        .insert_resource(TelemetryRoot(Some(telemetry_dir)))
        .add_plugins((SeedPlugin, SavePlugin, ResearchPlugin, TelemetryPlugin))
        .init_state::<GameState>()
        .add_sub_state::<PlayMode>()
        .add_message::<WorldObjectEvent>();
        app
    }

    fn enter_playing(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();
    }

    #[test]
    fn run_started_appended_on_play_entry() {
        let mut app = make_app(tmp_root("rs_save"), tmp_root("rs_tel"));
        // Spawn run entity first so the header is queryable.
        app.world_mut()
            .resource_mut::<Messages<NewRunEvent>>()
            .write(NewRunEvent {
                seed_text: "t-rs".into(),
                test_mode: false,
            });
        app.update();
        enter_playing(&mut app);

        let log = app.world().resource::<TelemetryLog>();
        assert_eq!(log.records.len(), 1);
        assert_eq!(log.records[0]["event"], "RunStarted");
    }

    #[test]
    fn machine_placed_event_emits_record() {
        let mut app = make_app(tmp_root("mp_save"), tmp_root("mp_tel"));
        app.world_mut()
            .resource_mut::<Messages<NewRunEvent>>()
            .write(NewRunEvent {
                seed_text: "t-mp".into(),
                test_mode: false,
            });
        app.update();
        enter_playing(&mut app);

        app.world_mut()
            .resource_mut::<Messages<WorldObjectEvent>>()
            .write(WorldObjectEvent {
                transform: Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
                item_id: "smelter".into(),
                kind: WorldObjectKind::Placed,
            });
        app.update();

        let log = app.world().resource::<TelemetryLog>();
        let evts: Vec<&str> = log
            .records
            .iter()
            .filter_map(|r| r["event"].as_str())
            .collect();
        assert!(evts.contains(&"MachinePlaced"));
    }

    #[test]
    fn discovery_event_emits_record() {
        let mut app = make_app(tmp_root("dis_save"), tmp_root("dis_tel"));
        app.world_mut()
            .resource_mut::<Messages<NewRunEvent>>()
            .write(NewRunEvent {
                seed_text: "t-dis".into(),
                test_mode: false,
            });
        app.update();
        enter_playing(&mut app);

        app.world_mut()
            .resource_mut::<Messages<DiscoveryEvent>>()
            .write(DiscoveryEvent("xalite_deposit".into()));
        app.update();

        let log = app.world().resource::<TelemetryLog>();
        let evt = log
            .records
            .iter()
            .find(|r| r["event"] == "Discovery")
            .expect("Discovery record should exist");
        assert_eq!(evt["key"], "xalite_deposit");
    }

    #[test]
    fn insight_validation_emits_record() {
        let mut app = make_app(tmp_root("ins_save"), tmp_root("ins_tel"));
        app.world_mut()
            .resource_mut::<Messages<NewRunEvent>>()
            .write(NewRunEvent {
                seed_text: "t-ins".into(),
                test_mode: false,
            });
        app.update();
        enter_playing(&mut app);

        app.world_mut()
            .resource_mut::<Messages<crate::planet::PropertyDecisionValidated>>()
            .write(crate::planet::PropertyDecisionValidated {
                property: crate::planet::PlanetPropertyKey::StellarDistance,
                kind: crate::machine::PowerProducerKind::Solar,
                modifier: 1.48,
            });
        app.update();

        let log = app.world().resource::<TelemetryLog>();
        let evt = log
            .records
            .iter()
            .find(|r| r["event"] == "InsightCandidate")
            .expect("InsightCandidate record should exist");
        assert_eq!(evt["property"], "StellarDistance");
    }

    #[test]
    fn tech_revealed_diff_emits_per_new_node() {
        let mut app = make_app(tmp_root("tr_save"), tmp_root("tr_tel"));
        app.world_mut()
            .resource_mut::<Messages<NewRunEvent>>()
            .write(NewRunEvent {
                seed_text: "t-tr".into(),
                test_mode: false,
            });
        app.update();
        enter_playing(&mut app);

        {
            let mut progress = app.world_mut().resource_mut::<TechTreeProgress>();
            progress.unlocked_nodes.insert("alpha".into());
        }
        app.update();
        {
            let mut progress = app.world_mut().resource_mut::<TechTreeProgress>();
            progress.unlocked_nodes.insert("beta".into());
        }
        app.update();

        let log = app.world().resource::<TelemetryLog>();
        let nodes: Vec<String> = log
            .records
            .iter()
            .filter(|r| r["event"] == "TechRevealed")
            .filter_map(|r| r["node_id"].as_str().map(str::to_owned))
            .collect();
        assert!(nodes.contains(&"alpha".to_owned()));
        assert!(nodes.contains(&"beta".to_owned()));
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn run_end_writes_jsonl_and_appends_abandoned() {
        let save_dir = tmp_root("re_save");
        let tel_dir = tmp_root("re_tel");
        let mut app = make_app(save_dir.clone(), tel_dir.clone());
        app.world_mut()
            .resource_mut::<Messages<NewRunEvent>>()
            .write(NewRunEvent {
                seed_text: "t-re".into(),
                test_mode: false,
            });
        app.update();
        let run_id = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<&RunSaveHeader, With<Run>>();
            q.single(world).unwrap().run_id.clone()
        };
        enter_playing(&mut app);

        // Exit Playing → run_end fires.
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::MainMenu);
        app.update();

        let path = tel_dir.join(format!("{run_id}.jsonl"));
        assert!(path.is_file(), "jsonl must be written to {path:?}");
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("\"event\":\"RunStarted\""));
        assert!(body.contains("\"event\":\"RunAbandoned\""));
        assert!(!app.world().contains_resource::<TelemetryLog>());
    }

    #[test]
    fn multi_session_appends_and_emits_run_resumed() {
        let save_dir = tmp_root("ms_save");
        let tel_dir = tmp_root("ms_tel");
        let mut app = make_app(save_dir.clone(), tel_dir.clone());
        app.world_mut()
            .resource_mut::<Messages<NewRunEvent>>()
            .write(NewRunEvent {
                seed_text: "t-ms".into(),
                test_mode: false,
            });
        app.update();
        let run_id = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<&RunSaveHeader, With<Run>>();
            q.single(world).unwrap().run_id.clone()
        };

        // Session 1: play then quit to main menu
        enter_playing(&mut app);
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::MainMenu);
        app.update();

        let path = tel_dir.join(format!("{run_id}.jsonl"));
        let s1 = std::fs::read_to_string(&path).unwrap();
        assert!(s1.contains("\"RunStarted\""));

        // Session 2: load same run (re-enter Playing; Run entity still exists)
        enter_playing(&mut app);
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::MainMenu);
        app.update();

        let s12 = std::fs::read_to_string(&path).unwrap();
        let events: Vec<&str> = s12
            .lines()
            .filter_map(|l| {
                serde_json::from_str::<serde_json::Value>(l)
                    .ok()
                    .and_then(|v| v["event"].as_str().map(str::to_owned))
                    .map(|s| Box::leak(s.into_boxed_str()) as &str)
            })
            .collect();
        assert_eq!(events.iter().filter(|&&e| e == "RunStarted").count(), 1);
        assert_eq!(events.iter().filter(|&&e| e == "RunResumed").count(), 1);
        // session 2 RunResumed has session_n = 2
        let resumed = s12.lines().find(|l| l.contains("\"RunResumed\"")).unwrap();
        assert!(resumed.contains("\"session_n\":2"));
    }

    #[test]
    fn pause_and_resume_does_not_restart_run() {
        let mut app = make_app(tmp_root("pr_save"), tmp_root("pr_tel"));
        app.world_mut()
            .resource_mut::<Messages<NewRunEvent>>()
            .write(NewRunEvent {
                seed_text: "t-pr".into(),
                test_mode: false,
            });
        app.update();
        enter_playing(&mut app);

        let elapsed_before = app.world().resource::<TelemetryLog>().elapsed_secs;

        // Pause
        app.world_mut()
            .resource_mut::<NextState<PlayMode>>()
            .set(PlayMode::Paused);
        app.update();

        // TelemetryLog must still exist (run not ended)
        assert!(app.world().contains_resource::<TelemetryLog>());

        // Resume
        app.world_mut()
            .resource_mut::<NextState<PlayMode>>()
            .set(PlayMode::Exploring);
        app.update();

        let log = app.world().resource::<TelemetryLog>();
        // Still same run — no extra RunStarted
        let run_started_count = log
            .records
            .iter()
            .filter(|r| r["event"] == "RunStarted")
            .count();
        assert_eq!(run_started_count, 1, "RunStarted must fire exactly once");
        // elapsed not reset to 0
        assert!(
            log.elapsed_secs >= elapsed_before,
            "elapsed must not reset on unpause"
        );
    }
}

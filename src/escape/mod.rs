use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::GameState;
use crate::logistics::JobComplete;
use crate::machine::Machine;
use crate::planet::PlanetProperties;
use crate::research::{DiscoveryEvent, TechTreeProgress};
use crate::save::{Run, RunEndEvent, RunSaveHeader, RunStatus};

/// Marker: recipe completion on this machine triggers run escape.
#[derive(Component)]
pub struct EscapeObjective;

/// Fired when the escape recipe completes on an `EscapeObjective` machine.
#[derive(Clone, Message)]
pub struct EscapeEvent {
    pub escape_time_secs: f32,
}

/// Runtime escape state — set to `Completed` when `EscapeEvent` fires.
#[derive(Resource, Default, Clone, PartialEq, Eq, Debug)]
pub enum RunState {
    #[default]
    InProgress,
    Completed,
}

/// Stats captured at escape time; read by the completion screen.
#[derive(Resource, Default)]
pub struct EscapeStats {
    pub archetype: String,
    pub seed_text: String,
    pub playtime_secs: f64,
}

/// Duration of the in-world completion burst before the results screen shows.
const ESCAPE_FLASH_SECS: f32 = 1.5;

/// Gates the delayed transition to `Escaped` so the in-world flash is visible.
#[derive(Resource)]
struct EscapeSequence {
    timer: Timer,
}

/// In-world completion VFX at the gateway: an expanding emissive burst.
#[derive(Component)]
struct EscapeFlash {
    timer: Timer,
    base_intensity: f32,
}

pub struct EscapePlugin;

impl Plugin for EscapePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunState>()
            .init_resource::<EscapeStats>()
            .add_message::<EscapeEvent>()
            .add_systems(
                Update,
                (
                    tag_escape_machines_system,
                    escape_objective_system,
                    on_escape_system,
                    spawn_escape_vfx.run_if(resource_added::<EscapeSequence>),
                    escape_sequence_system,
                    unlock_gateway_on_discovery,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn escape_objective_system(
    mut job_complete: MessageReader<JobComplete>,
    escape_q: Query<(), With<EscapeObjective>>,
    time: Res<Time>,
    mut escape_events: MessageWriter<EscapeEvent>,
) {
    for job in job_complete.read() {
        if escape_q.get(job.machine).is_ok() {
            escape_events.write(EscapeEvent {
                escape_time_secs: time.elapsed_secs(),
            });
        }
    }
}

/// Content-defined machine types that are escape objectives. A player-placed escape
/// structure gets the `EscapeObjective` marker here; the run-generated gateway gets it
/// at spawn (`world::ruins`). Keeps escape completion real for the launch site (VS §8).
const ESCAPE_MACHINE_TYPES: &[&str] = &["launch_site"];

/// Tag a newly-placed escape-structure machine with `EscapeObjective` so its recipe
/// completion fires the win, exactly like the run-generated gateway.
fn tag_escape_machines_system(
    mut commands: Commands,
    q: Query<(Entity, &Machine), (Added<Machine>, Without<EscapeObjective>)>,
) {
    for (entity, machine) in &q {
        if ESCAPE_MACHINE_TYPES.contains(&machine.machine_type.as_str()) {
            commands.entity(entity).insert(EscapeObjective);
        }
    }
}

fn on_escape_system(
    mut escape_events: MessageReader<EscapeEvent>,
    mut run_state: ResMut<RunState>,
    mut escape_stats: ResMut<EscapeStats>,
    mut header_q: Query<&mut RunSaveHeader, With<Run>>,
    planet_q: Query<&PlanetProperties>,
    mut run_end: MessageWriter<RunEndEvent>,
    mut commands: Commands,
) {
    if escape_events.read().next().is_none() {
        return;
    }
    *run_state = RunState::Completed;

    if let Ok(mut header) = header_q.single_mut() {
        header.status = RunStatus::Completed;
        escape_stats.seed_text = header.seed_text.clone();
        escape_stats.playtime_secs = header.total_playtime_secs;
    }
    if let Ok(planet) = planet_q.single() {
        escape_stats.archetype = planet.archetype.clone();
    }

    run_end.write(RunEndEvent);
    // Delay the results screen so the in-world burst (`spawn_escape_vfx`) is seen.
    commands.insert_resource(EscapeSequence {
        timer: Timer::from_seconds(ESCAPE_FLASH_SECS, TimerMode::Once),
    });
}

/// Spawn the emissive burst at the gateway when the escape sequence begins.
fn spawn_escape_vfx(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    gateway_q: Query<&Transform, With<EscapeObjective>>,
) {
    let pos = gateway_q
        .single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    let base_intensity = 5_000_000.0;
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.9, 1.0),
            emissive: LinearRgba::rgb(4.0, 8.0, 12.0),
            unlit: true,
            ..default()
        })),
        PointLight {
            color: Color::srgb(0.6, 0.9, 1.0),
            intensity: base_intensity,
            range: 60.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(pos + Vec3::Y * 2.0),
        EscapeFlash {
            timer: Timer::from_seconds(ESCAPE_FLASH_SECS, TimerMode::Once),
            base_intensity,
        },
        DespawnOnExit(GameState::Playing),
    ));
}

/// Tick the delay, animate the burst (expand + fade), and show results when done.
fn escape_sequence_system(
    time: Res<Time>,
    seq: Option<ResMut<EscapeSequence>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut flash_q: Query<(&mut Transform, &mut PointLight, &mut EscapeFlash)>,
) {
    let Some(mut seq) = seq else {
        return;
    };
    seq.timer.tick(time.delta());

    for (mut transform, mut light, mut flash) in &mut flash_q {
        flash.timer.tick(time.delta());
        let f = flash.timer.fraction();
        transform.scale = Vec3::splat(1.0 + f * 7.0);
        light.intensity = flash.base_intensity * (1.0 - f);
    }

    if seq.timer.is_finished() {
        next_state.set(GameState::Escaped);
        commands.remove_resource::<EscapeSequence>();
    }
}

/// Unlock `activate_gateway` recipe when gateway ruins are discovered.
fn unlock_gateway_on_discovery(
    mut discovery_events: MessageReader<DiscoveryEvent>,
    mut progress: ResMut<TechTreeProgress>,
) {
    for event in discovery_events.read() {
        if event.0 == "gateway_ruins" {
            progress
                .unlocked_recipes
                .insert("activate_gateway".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;
    use std::time::Duration;

    #[derive(Resource, Default)]
    struct EscapeCount(usize);

    fn count_escape_events(mut reader: MessageReader<EscapeEvent>, mut count: ResMut<EscapeCount>) {
        for _ in reader.read() {
            count.0 += 1;
        }
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<JobComplete>()
            .add_message::<EscapeEvent>()
            .init_resource::<RunState>()
            .init_resource::<EscapeStats>()
            .init_resource::<EscapeCount>()
            .add_systems(
                Update,
                (escape_objective_system, count_escape_events).chain(),
            );
        app
    }

    fn escape_machine(machine_type: &str) -> Machine {
        use crate::machine::{Mirror, Orientation, Rotation};
        Machine {
            machine_type: machine_type.to_string(),
            tier: 2,
            orientation: Orientation {
                rotation: Rotation::North,
                mirror: Mirror::Normal,
            },
            energy_ports: vec![],
            logistics_ports: vec![],
        }
    }

    #[test]
    fn launch_site_machine_gets_escape_objective() {
        let mut app = App::new();
        app.add_systems(Update, tag_escape_machines_system);
        let e = app.world_mut().spawn(escape_machine("launch_site")).id();
        app.update();
        assert!(
            app.world().get::<EscapeObjective>(e).is_some(),
            "placed launch_site must become an escape objective"
        );
    }

    #[test]
    fn ordinary_machine_gets_no_escape_objective() {
        let mut app = App::new();
        app.add_systems(Update, tag_escape_machines_system);
        let e = app.world_mut().spawn(escape_machine("smelter")).id();
        app.update();
        assert!(app.world().get::<EscapeObjective>(e).is_none());
    }

    #[test]
    fn escape_objective_fires_escape_event() {
        let mut app = make_app();
        let machine = app.world_mut().spawn(EscapeObjective).id();
        app.world_mut().write_message(JobComplete {
            machine,
            recipe_id: "activate_gateway".to_string(),
        });
        app.update();

        assert_eq!(
            app.world().resource::<EscapeCount>().0,
            1,
            "EscapeEvent should fire for EscapeObjective machine"
        );
    }

    #[test]
    fn non_escape_machine_does_not_fire_escape_event() {
        let mut app = make_app();
        let machine = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(JobComplete {
            machine,
            recipe_id: "basic_smelting".to_string(),
        });
        app.update();

        assert_eq!(
            app.world().resource::<EscapeCount>().0,
            0,
            "Non-escape machine must not fire EscapeEvent"
        );
    }

    fn sequence_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_systems(
                Update,
                escape_sequence_system.run_if(in_state(GameState::Playing)),
            );
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();
        app
    }

    #[test]
    fn sequence_delays_transition_while_running() {
        let mut app = sequence_app();
        app.insert_resource(EscapeSequence {
            timer: Timer::from_seconds(ESCAPE_FLASH_SECS, TimerMode::Once),
        });
        app.update();

        assert_eq!(
            *app.world().resource::<State<GameState>>().get(),
            GameState::Playing,
            "must stay in Playing until the burst finishes"
        );
        assert!(
            app.world().get_resource::<EscapeSequence>().is_some(),
            "sequence still in progress"
        );
    }

    #[test]
    fn sequence_transitions_to_escaped_when_finished() {
        let mut app = sequence_app();
        let mut timer = Timer::from_seconds(ESCAPE_FLASH_SECS, TimerMode::Once);
        timer.tick(Duration::from_secs_f32(ESCAPE_FLASH_SECS + 1.0));
        app.insert_resource(EscapeSequence { timer });
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<GameState>>().get(),
            GameState::Escaped,
            "finished burst transitions to results screen"
        );
        assert!(
            app.world().get_resource::<EscapeSequence>().is_none(),
            "sequence removed after transition"
        );
    }
}

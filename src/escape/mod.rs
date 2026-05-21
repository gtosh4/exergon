use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::GameState;
use crate::logistics::JobComplete;
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

pub struct EscapePlugin;

impl Plugin for EscapePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunState>()
            .init_resource::<EscapeStats>()
            .add_message::<EscapeEvent>()
            .add_systems(
                Update,
                (
                    escape_objective_system,
                    on_escape_system,
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

fn on_escape_system(
    mut escape_events: MessageReader<EscapeEvent>,
    mut run_state: ResMut<RunState>,
    mut escape_stats: ResMut<EscapeStats>,
    mut header_q: Query<&mut RunSaveHeader, With<Run>>,
    planet_q: Query<&PlanetProperties>,
    mut run_end: MessageWriter<RunEndEvent>,
    mut next_state: ResMut<NextState<GameState>>,
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
    next_state.set(GameState::Escaped);
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
}

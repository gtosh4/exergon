use avian3d::prelude::LinearVelocity;
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::world::Player;
use crate::{FixedGameSystems, GameState, PlayMode};

pub const AEGIS_RADIUS: f32 = 60.0;
const EXPOSURE_LETHAL_THRESHOLD_SECS: f32 = 30.0;

pub struct AegisPlugin;

impl Plugin for AegisPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EnteredAegis>()
            .add_message::<LeftAegis>()
            .add_message::<BodyDestroyed>()
            .add_message::<RunFailed>()
            .add_systems(
                OnTransition {
                    exited: GameState::Loading,
                    entered: GameState::Playing,
                },
                spawn_aegis_emitter,
            )
            .add_systems(
                Update,
                (
                    aegis_boundary_check_system,
                    atmospheric_exposure_system.after(aegis_boundary_check_system),
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not(in_state(PlayMode::Paused))),
            )
            .add_systems(
                FixedUpdate,
                aegis_movement_constraint_system
                    .in_set(FixedGameSystems::Constraint)
                    .run_if(in_state(PlayMode::Exploring))
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct AegisEmitter;

#[derive(Component)]
pub struct AegisRadius(pub f32);

/// Present on an aegis emitter when it is actively projecting a field.
/// AegisEmitter entities always carry this. OutpostBeacon entities gain/lose it with power.
#[derive(Component)]
pub struct AegisActive;

/// Marker on the player body while inside at least one active aegis field.
#[derive(Component)]
pub struct InAegis;

#[derive(Component)]
pub struct AtmosphericExposure {
    pub elapsed_secs: f32,
}

/// Damage model stub — tracks health but no damage source in VS.
#[derive(Component)]
pub struct DroneHealth {
    pub current: f32,
    pub max: f32,
}

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct EnteredAegis {
    pub body: Entity,
}

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct LeftAegis {
    pub body: Entity,
}

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct BodyDestroyed {
    pub body: Entity,
}

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct RunFailed;

fn spawn_aegis_emitter(mut commands: Commands) {
    commands.spawn((
        AegisEmitter,
        AegisRadius(AEGIS_RADIUS),
        AegisActive,
        Transform::from_translation(Vec3::ZERO),
    ));
}

fn aegis_boundary_check_system(
    mut commands: Commands,
    player_q: Query<(Entity, &Transform, Option<&InAegis>), With<Player>>,
    aegis_q: Query<(&AegisRadius, &Transform), With<AegisActive>>,
    mut entered: MessageWriter<EnteredAegis>,
    mut left: MessageWriter<LeftAegis>,
) {
    let Ok((player_entity, player_transform, in_aegis)) = player_q.single() else {
        return;
    };
    let inside = aegis_q.iter().any(|(radius, aegis_transform)| {
        player_transform
            .translation
            .distance(aegis_transform.translation)
            <= radius.0
    });
    match (inside, in_aegis.is_some()) {
        (true, false) => {
            commands
                .entity(player_entity)
                .insert(InAegis)
                .remove::<AtmosphericExposure>();
            entered.write(EnteredAegis {
                body: player_entity,
            });
        }
        (false, true) => {
            commands
                .entity(player_entity)
                .remove::<InAegis>()
                .insert(AtmosphericExposure { elapsed_secs: 0.0 });
            left.write(LeftAegis {
                body: player_entity,
            });
        }
        _ => {}
    }
}

fn aegis_movement_constraint_system(
    player_q: Query<(&Transform, Option<&InAegis>), With<Player>>,
    aegis_q: Query<(&AegisRadius, &Transform), With<AegisActive>>,
    mut vel_q: Query<&mut LinearVelocity, With<Player>>,
    time: Res<Time>,
) {
    let Ok((player_transform, in_aegis)) = player_q.single() else {
        return;
    };
    if in_aegis.is_none() {
        return;
    }
    let Ok(mut vel) = vel_q.single_mut() else {
        return;
    };
    let Some((radius, aegis_center)) = aegis_q
        .iter()
        .min_by(|(_, ta), (_, tb)| {
            let da = player_transform.translation.distance(ta.translation);
            let db = player_transform.translation.distance(tb.translation);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(r, t)| (r.0, t.translation))
    else {
        return;
    };

    let dt = time.delta_secs();
    let next_pos = player_transform.translation + vel.0 * dt;
    if next_pos.distance(aegis_center) > radius {
        let outward = (player_transform.translation - aegis_center).normalize_or_zero();
        let outward_component = vel.0.dot(outward);
        if outward_component > 0.0 {
            vel.0 -= outward * outward_component;
        }
    }
}

fn atmospheric_exposure_system(
    mut commands: Commands,
    mut exposure_q: Query<(Entity, &mut AtmosphericExposure), With<Player>>,
    all_players: Query<Entity, With<Player>>,
    time: Res<Time>,
    mut destroyed: MessageWriter<BodyDestroyed>,
    mut failed: MessageWriter<RunFailed>,
) {
    let Ok((entity, mut exposure)) = exposure_q.single_mut() else {
        return;
    };
    exposure.elapsed_secs += time.delta_secs();
    if exposure.elapsed_secs >= EXPOSURE_LETHAL_THRESHOLD_SECS {
        commands.entity(entity).despawn();
        destroyed.write(BodyDestroyed { body: entity });
        if all_players.iter().count() <= 1 {
            failed.write(RunFailed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    use crate::{GameState, PlayMode};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .add_message::<EnteredAegis>()
            .add_message::<LeftAegis>()
            .add_message::<BodyDestroyed>()
            .add_message::<RunFailed>()
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
            .add_systems(
                Update,
                (
                    aegis_boundary_check_system,
                    atmospheric_exposure_system.after(aegis_boundary_check_system),
                ),
            );
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();
        app
    }

    #[test]
    fn boundary_check_inserts_in_aegis_when_inside() {
        let mut app = test_app();
        app.world_mut().spawn((
            AegisEmitter,
            AegisRadius(50.0),
            AegisActive,
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        let player = app
            .world_mut()
            .spawn((Player, Transform::from_xyz(10.0, 0.0, 0.0)))
            .id();

        app.update();

        assert!(
            app.world().get::<InAegis>(player).is_some(),
            "player inside aegis should have InAegis"
        );
    }

    #[test]
    fn boundary_check_no_in_aegis_when_outside() {
        let mut app = test_app();
        app.world_mut().spawn((
            AegisEmitter,
            AegisRadius(50.0),
            AegisActive,
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        let player = app
            .world_mut()
            .spawn((Player, Transform::from_xyz(100.0, 0.0, 0.0)))
            .id();

        app.update();

        assert!(
            app.world().get::<InAegis>(player).is_none(),
            "player outside aegis should not have InAegis"
        );
    }

    #[test]
    fn boundary_check_adds_exposure_when_exiting() {
        let mut app = test_app();
        app.world_mut().spawn((
            AegisEmitter,
            AegisRadius(50.0),
            AegisActive,
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        let player = app
            .world_mut()
            .spawn((Player, Transform::from_xyz(10.0, 0.0, 0.0), InAegis))
            .id();

        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = Vec3::new(100.0, 0.0, 0.0);
        app.update();

        assert!(app.world().get::<InAegis>(player).is_none());
        assert!(
            app.world().get::<AtmosphericExposure>(player).is_some(),
            "exiting aegis should add AtmosphericExposure"
        );
    }

    #[test]
    fn atmospheric_exposure_destroys_body_at_threshold() {
        let mut app = test_app();
        let player = app
            .world_mut()
            .spawn((
                Player,
                AtmosphericExposure {
                    elapsed_secs: EXPOSURE_LETHAL_THRESHOLD_SECS,
                },
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<Player>(player).is_none(),
            "player body should be despawned after lethal exposure"
        );
    }
}

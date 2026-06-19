use avian3d::prelude::*;
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::world::Player;
use crate::{GameState, PlayMode};

const AEGIS_COLOR: Color = Color::srgb(0.2, 0.8, 1.0);

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
                Update,
                (
                    aegis_position_constraint_system,
                    aegis_boundary_check_system,
                    atmospheric_exposure_system.after(aegis_boundary_check_system),
                    aegis_boundary_gizmos,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not(in_state(PlayMode::Paused))),
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

/// Inverted sphere trimesh: winding reversed so face normals point inward.
/// When the player tries to exit, the inward normal pushes them back.
/// The drone is in a different collision layer and passes through freely.
pub(crate) fn aegis_sphere_collider(radius: f32) -> Collider {
    const STACKS: u32 = 16;
    const SLICES: u32 = 24;
    let mut verts: Vec<Vec3> = Vec::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    for i in 0..=STACKS {
        let phi = std::f32::consts::PI * i as f32 / STACKS as f32;
        let (sin_phi, cos_phi) = phi.sin_cos();
        for j in 0..=SLICES {
            let theta = std::f32::consts::TAU * j as f32 / SLICES as f32;
            let (sin_theta, cos_theta) = theta.sin_cos();
            verts.push(Vec3::new(
                radius * sin_phi * cos_theta,
                radius * cos_phi,
                radius * sin_phi * sin_theta,
            ));
        }
    }
    for i in 0..STACKS {
        for j in 0..SLICES {
            let a = i * (SLICES + 1) + j;
            let b = a + SLICES + 1;
            // Reversed winding → normals face inward
            tris.push([a, b, a + 1]);
            tris.push([b, b + 1, a + 1]);
        }
    }
    Collider::trimesh(verts, tris)
}

fn aegis_position_constraint_system(
    mut player_q: Query<(&mut Position, &mut LinearVelocity), With<Player>>,
    aegis_q: Query<(&Transform, &AegisRadius), With<AegisActive>>,
) {
    let Ok((mut player_pos, mut velocity)) = player_q.single_mut() else {
        return;
    };
    let pos = player_pos.0;

    let inside = aegis_q
        .iter()
        .any(|(t, AegisRadius(r))| pos.distance(t.translation) <= *r);
    if inside {
        return;
    }

    // Outside all active aegis fields — clamp to nearest boundary
    if let Some((center, radius)) = aegis_q
        .iter()
        .map(|(t, AegisRadius(r))| (t.translation, *r))
        .min_by(|(a, ra), (b, rb)| {
            (pos.distance(*a) - ra)
                .partial_cmp(&(pos.distance(*b) - rb))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    {
        let dir = (pos - center).normalize_or_zero();
        player_pos.0 = center + dir * (radius * 0.99);
        let radial = velocity.0.dot(dir);
        if radial > 0.0 {
            velocity.0 -= dir * radial;
        }
    }
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

fn aegis_boundary_gizmos(
    mut gizmos: Gizmos,
    aegis_q: Query<(&Transform, &AegisRadius), With<AegisActive>>,
    time: Res<Time>,
) {
    for (transform, AegisRadius(radius)) in &aegis_q {
        let alpha = (time.elapsed_secs() * 1.5).sin() * 0.15 + 0.6;
        let base_color = Color::srgba(0.2, 0.8, 1.0, alpha);
        let dim_color = Color::srgba(0.2, 0.8, 1.0, alpha * 0.5);
        let center = transform.translation;
        let half_pi = std::f32::consts::FRAC_PI_2;

        // Base circle at ground level
        gizmos.circle(
            Isometry3d::new(center, Quat::from_rotation_x(half_pi)),
            *radius,
            base_color,
        );

        // Latitude rings at 1/3 and 2/3 dome height
        for &frac in &[0.35_f32, 0.70] {
            let h = radius * frac;
            let r_lat = (radius * radius - h * h).sqrt();
            gizmos.circle(
                Isometry3d::new(center + Vec3::Y * h, Quat::from_rotation_x(half_pi)),
                r_lat,
                dim_color,
            );
        }

        // 4 meridian arcs from base to apex
        for i in 0..4_u32 {
            let azimuth = i as f32 * half_pi;
            let (az_sin, az_cos) = azimuth.sin_cos();
            let pts: Vec<Vec3> = (0..=16)
                .map(|j| {
                    let el = j as f32 * half_pi / 16.0;
                    let (el_sin, el_cos) = el.sin_cos();
                    center + Vec3::new(el_cos * az_cos, el_sin, el_cos * az_sin) * *radius
                })
                .collect();
            gizmos.linestrip(pts, dim_color);
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

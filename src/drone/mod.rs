use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy_tnua::TnuaUserControlsSystems;
use bevy_tnua::builtins::{TnuaBuiltinWalk, TnuaBuiltinWalkConfig};
use bevy_tnua::prelude::*;
use bevy_tnua_avian3d::prelude::*;

use crate::world::MainCamera;
use crate::{GameState, PlayMode};

pub struct DronePlugin;

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
pub enum DroneScheme {}

#[derive(Component)]
pub struct Drone;

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            TnuaControllerPlugin::<DroneScheme>::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .add_systems(OnEnter(GameState::Playing), spawn_land_drone)
        .add_systems(
            Update,
            (
                toggle_drone_mode,
                drone_pilot_input
                    .in_set(TnuaUserControlsSystems)
                    .run_if(in_state(PlayMode::DronePilot)),
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn spawn_land_drone(
    mut commands: Commands,
    mut scheme_configs: ResMut<Assets<DroneSchemeConfig>>,
) {
    commands.spawn((
        Transform::from_xyz(0.0, 100.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.8),
        TnuaController::<DroneScheme>::default(),
        TnuaConfig::<DroneScheme>(scheme_configs.add(DroneSchemeConfig {
            basis: TnuaBuiltinWalkConfig {
                // capsule bottom is 0.8m below center; float above that
                float_height: 1.0,
                speed: 15.0,
                ..Default::default()
            },
        })),
        TnuaAvian3dSensorShape(Collider::cylinder(0.39, 0.0)),
        LockedAxes::ROTATION_LOCKED,
        Drone,
    ));
}

fn toggle_drone_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    mode: Res<State<PlayMode>>,
    mut next_mode: ResMut<NextState<PlayMode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyF) {
        match mode.get() {
            PlayMode::Exploring => next_mode.set(PlayMode::DronePilot),
            PlayMode::DronePilot => next_mode.set(PlayMode::Exploring),
            _ => {}
        }
    }
}

fn drone_pilot_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<AccumulatedMouseMotion>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    drone_q: Query<&Transform, (With<Drone>, Without<MainCamera>)>,
    mut controller_q: Query<&mut TnuaController<DroneScheme>, With<Drone>>,
) {
    let Ok(drone_transform) = drone_q.single() else {
        return;
    };
    let Ok(mut camera) = camera_q.single_mut() else {
        return;
    };
    let Ok(mut controller) = controller_q.single_mut() else {
        return;
    };

    // Mouse look
    let yaw = -mouse.delta.x * 0.003;
    let pitch = -mouse.delta.y * 0.003;
    if yaw != 0.0 || pitch != 0.0 {
        let (current_yaw, current_pitch, _) = camera.rotation.to_euler(EulerRot::YXZ);
        let new_pitch = (current_pitch + pitch).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        camera.rotation = Quat::from_euler(EulerRot::YXZ, current_yaw + yaw, new_pitch, 0.0);
    }

    // Camera follows drone at eye height
    camera.translation = drone_transform.translation + Vec3::Y * 0.5;

    // WASD direction in horizontal plane relative to camera facing
    let cam_fwd = *camera.forward();
    let forward = Vec3::new(cam_fwd.x, 0.0, cam_fwd.z).normalize_or_zero();
    let cam_right = *camera.right();
    let right = Vec3::new(cam_right.x, 0.0, cam_right.z).normalize_or_zero();

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += right;
    }

    controller.basis = TnuaBuiltinWalk {
        desired_motion: direction.normalize_or_zero(),
        ..Default::default()
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[test]
    fn f_key_enters_drone_pilot_mode() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                Update,
                toggle_drone_mode.run_if(in_state(GameState::Playing)),
            );

        // Enter Playing state
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();

        // F press → system runs → state applies next frame
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update(); // toggle_drone_mode runs, sets NextState
        app.update(); // StateTransition applies

        assert_eq!(
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::DronePilot
        );
    }

    #[test]
    fn f_key_exits_drone_pilot_mode() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                Update,
                toggle_drone_mode.run_if(in_state(GameState::Playing)),
            );

        // Enter Playing → DronePilot
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        app.update(); // apply transition
        assert_eq!(
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::DronePilot
        );

        // Release and press again to exit
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyF);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        app.update(); // apply transition

        assert_eq!(
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::Exploring
        );
    }
}

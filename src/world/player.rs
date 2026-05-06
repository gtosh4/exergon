use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::GameState;
use crate::inventory::InventoryOpen;
use crate::ui::{MachineStatusPanel, StorageStatusPanel, TechTreePanelOpen};

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct Player;

pub(super) fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 80.0, 0.0),
        MainCamera,
    ));
}

pub(super) fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(0.0, 100.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.8),
        GravityScale(0.0),
        LinearDamping(0.0),
        LockedAxes::ROTATION_LOCKED,
        Player,
    ));
}

pub(super) fn setup_world_once(
    mut commands: Commands,
    existing_lights: Query<(), With<DirectionalLight>>,
) {
    if existing_lights.is_empty() {
        commands.spawn((
            DirectionalLight {
                illuminance: 10_000.0,
                shadows_enabled: true,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.5, 0.0)),
        ));
        commands.insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.4, 0.45, 0.6),
            brightness: 200.0,
            ..default()
        });
    }
}

pub(super) fn lock_cursor(mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

pub(super) fn unlock_cursor(mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

pub(super) fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    inv_open: Option<Res<InventoryOpen>>,
) {
    let blocked = inv_open.is_some_and(|o| o.0);
    if keyboard.just_pressed(KeyCode::Escape) && !blocked {
        next_state.set(GameState::Paused);
    }
}

pub(super) fn resume_on_escape(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Playing);
    }
}

pub(super) fn toggle_inventory(
    keyboard: Res<ButtonInput<KeyCode>>,
    inv_open: Option<ResMut<InventoryOpen>>,
) {
    let Some(mut open) = inv_open else { return };
    let should_toggle =
        keyboard.just_pressed(KeyCode::Tab) || (keyboard.just_pressed(KeyCode::Escape) && open.0);
    if should_toggle {
        open.0 = !open.0;
    }
}

pub(super) fn any_ui_open(
    inv: Option<Res<InventoryOpen>>,
    machine: Option<Res<MachineStatusPanel>>,
    storage: Option<Res<StorageStatusPanel>>,
    tech: Option<Res<TechTreePanelOpen>>,
) -> bool {
    inv.is_some_and(|o| o.0)
        || machine.is_some_and(|m| m.entity.is_some())
        || storage.is_some_and(|s| s.0.is_some())
        || tech.is_some_and(|t| t.open)
}

pub(super) fn sync_cursor(
    inv: Option<Res<InventoryOpen>>,
    machine: Option<Res<MachineStatusPanel>>,
    storage: Option<Res<StorageStatusPanel>>,
    tech: Option<Res<TechTreePanelOpen>>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let ui_open = inv.is_some_and(|o| o.0)
        || machine.is_some_and(|m| m.entity.is_some())
        || storage.is_some_and(|s| s.0.is_some())
        || tech.is_some_and(|t| t.open);
    if let Ok(mut cursor) = cursor_q.single_mut() {
        if ui_open {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        } else {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

pub(super) fn camera_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    player_q: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut velocity_q: Query<&mut LinearVelocity, With<Player>>,
) {
    let Ok(mut camera) = camera_q.single_mut() else {
        return;
    };

    let yaw = -mouse_motion.delta.x * 0.003;
    let pitch = -mouse_motion.delta.y * 0.003;
    if yaw != 0.0 || pitch != 0.0 {
        let (current_yaw, current_pitch, _) = camera.rotation.to_euler(EulerRot::YXZ);
        let new_yaw = current_yaw + yaw;
        let new_pitch = (current_pitch + pitch).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        camera.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, new_pitch, 0.0);
    }

    let Ok(player_transform) = player_q.single() else {
        return;
    };
    let Ok(mut velocity) = velocity_q.single_mut() else {
        return;
    };

    camera.translation = player_transform.translation + Vec3::Y * 0.5;

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction += *camera.forward();
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= *camera.forward();
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= *camera.right();
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += *camera.right();
    }
    if keyboard.pressed(KeyCode::Space) {
        direction += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ControlLeft) {
        direction -= Vec3::Y;
    }

    velocity.0 = if direction != Vec3::ZERO {
        direction.normalize() * 15.0
    } else {
        Vec3::ZERO
    };
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;

    use super::*;
    use crate::GameState;
    use crate::inventory::InventoryOpen;

    #[test]
    fn toggle_pause_escape_transitions_to_paused() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, toggle_pause.run_if(in_state(GameState::Playing)));

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<GameState>>().get(),
            GameState::Paused
        );
    }

    #[test]
    fn toggle_pause_blocked_when_inventory_open() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(InventoryOpen(true))
            .add_systems(Update, toggle_pause.run_if(in_state(GameState::Playing)));

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<GameState>>().get(),
            GameState::Playing
        );
    }

    #[test]
    fn setup_world_once_spawns_light_when_none_exists() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup_world_once);
        app.update();

        let world = app.world_mut();
        let light_count = world.query::<&DirectionalLight>().iter(world).count();
        assert_eq!(light_count, 1);
        assert!(world.get_resource::<GlobalAmbientLight>().is_some());
    }

    #[test]
    fn setup_world_once_does_not_duplicate_light() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup_world_once);

        // Pre-spawn a light so the system skips
        app.world_mut().spawn(DirectionalLight::default());
        app.update();

        let world = app.world_mut();
        let light_count = world.query::<&DirectionalLight>().iter(world).count();
        assert_eq!(light_count, 1);
    }

    #[test]
    fn toggle_inventory_tab_opens_when_closed() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(InventoryOpen(false))
            .add_systems(Update, toggle_inventory);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();

        assert!(app.world().resource::<InventoryOpen>().0);
    }

    #[test]
    fn toggle_inventory_escape_closes_when_open() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(InventoryOpen(true))
            .add_systems(Update, toggle_inventory);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        assert!(!app.world().resource::<InventoryOpen>().0);
    }

    #[test]
    fn toggle_inventory_no_op_without_resource() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, toggle_inventory);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        // Just checking no panic when InventoryOpen resource absent
    }

    #[test]
    fn resume_on_escape_sets_playing() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, resume_on_escape.run_if(in_state(GameState::Paused)));

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Paused);
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<GameState>>().get(),
            GameState::Playing
        );
    }

    fn camera_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<AccumulatedMouseMotion>()
            .add_systems(Update, camera_input);
        app
    }

    #[test]
    fn camera_input_w_key_runs_without_panic() {
        let mut app = camera_app();
        app.world_mut().spawn((Transform::default(), MainCamera));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
    }

    #[test]
    fn camera_input_sad_ctrl_space_hit_branches() {
        let mut app = camera_app();
        app.world_mut().spawn((Transform::default(), MainCamera));
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::KeyS);
            kb.press(KeyCode::KeyA);
            kb.press(KeyCode::KeyD);
            kb.press(KeyCode::Space);
        }
        app.update();
    }

    #[test]
    fn camera_input_no_camera_does_not_panic() {
        let mut app = camera_app();
        // No MainCamera spawned — early return branch covered
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
    }

    #[test]
    fn camera_input_mouse_rotation_updates_camera() {
        use bevy::input::mouse::AccumulatedMouseMotion;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<AccumulatedMouseMotion>()
            .add_systems(Update, camera_input);

        let cam = app
            .world_mut()
            .spawn((Transform::default(), MainCamera))
            .id();

        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = bevy::math::Vec2::new(100.0, 50.0);
        app.update();

        let rot = app.world().get::<Transform>(cam).unwrap().rotation;
        assert_ne!(
            rot,
            bevy::math::Quat::IDENTITY,
            "mouse input should rotate camera"
        );
    }
}

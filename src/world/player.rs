use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_voxel_world::prelude::*;

use crate::GameState;
use crate::inventory::InventoryOpen;

use super::generation::WorldConfig;

#[derive(Component)]
pub struct MainCamera;

pub(super) fn is_blocked(voxel_world: &VoxelWorld<'_, WorldConfig>, center: Vec3, r: f32) -> bool {
    let offsets = [
        Vec3::new(-r, -r, -r),
        Vec3::new(-r, -r, r),
        Vec3::new(-r, r, -r),
        Vec3::new(-r, r, r),
        Vec3::new(r, -r, -r),
        Vec3::new(r, -r, r),
        Vec3::new(r, r, -r),
        Vec3::new(r, r, r),
    ];
    offsets.iter().any(|&o| {
        matches!(
            voxel_world.get_voxel((center + o).floor().as_ivec3()),
            WorldVoxel::Solid(_)
        )
    })
}

pub(super) fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 80.0, 0.0),
        MainCamera,
        VoxelWorldCamera::<WorldConfig>::default(),
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

pub(super) fn toggle_inventory(
    keyboard: Res<ButtonInput<KeyCode>>,
    inv_open: Option<ResMut<InventoryOpen>>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Some(mut open) = inv_open else { return };
    let should_toggle =
        keyboard.just_pressed(KeyCode::Tab) || (keyboard.just_pressed(KeyCode::Escape) && open.0);
    if !should_toggle {
        return;
    }
    open.0 = !open.0;
    if let Ok(mut cursor) = cursor_q.single_mut() {
        if open.0 {
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
    time: Res<Time>,
    voxel_world: VoxelWorld<WorldConfig>,
) {
    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    let yaw = -mouse_motion.delta.x * 0.003;
    let pitch = -mouse_motion.delta.y * 0.003;

    if yaw != 0.0 || pitch != 0.0 {
        let (current_yaw, current_pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let new_yaw = current_yaw + yaw;
        let new_pitch = (current_pitch + pitch).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, new_pitch, 0.0);
    }

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction += *transform.forward();
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= *transform.forward();
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= *transform.right();
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += *transform.right();
    }
    if keyboard.pressed(KeyCode::Space) {
        direction += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ControlLeft) {
        direction -= Vec3::Y;
    }

    if direction == Vec3::ZERO {
        return;
    }

    const R: f32 = 0.35;
    let delta = direction.normalize() * 50.0 * time.delta_secs();
    let current = transform.translation;

    if is_blocked(&voxel_world, current + delta, R) {
        let dx = Vec3::new(delta.x, 0.0, 0.0);
        let dy = Vec3::new(0.0, delta.y, 0.0);
        let dz = Vec3::new(0.0, 0.0, delta.z);
        if !is_blocked(&voxel_world, current + dx, R) {
            transform.translation.x += dx.x;
        }
        if !is_blocked(&voxel_world, current + dy, R) {
            transform.translation.y += dy.y;
        }
        if !is_blocked(&voxel_world, current + dz, R) {
            transform.translation.z += dz.z;
        }
    } else {
        transform.translation += delta;
    }
}

use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::drone::DroneCargoOpen;
use crate::inventory::InventoryOpen;
use crate::ui::panels::craft_modal::CraftModal;
use crate::ui::panels::planner::{PlannerOpen, RecipePickerState};
use crate::ui::{MachineStatusPanel, StorageStatusPanel, TechTreePanelOpen};
use crate::{GameLayer, PlayMode};

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct Player;

/// Built-in hand scanner on the player body — always present, not a tech unlock.
/// Fires HandScanComplete events via the interaction system.
#[derive(Component)]
pub struct HandScanner;

pub(super) fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 80.0, 0.0),
        MainCamera,
    ));
}

pub(super) fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Capsule3d::new(0.4, 0.8));
    let material = materials.add(StandardMaterial::from_color(Color::srgb(0.2, 0.8, 1.0)));
    commands.spawn((
        Transform::from_xyz(0.0, 20.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.8),
        GravityScale(0.0),
        LinearDamping(0.0),
        LockedAxes::ROTATION_LOCKED,
        Player,
        HandScanner,
        CollisionLayers::new(
            GameLayer::Player,
            [GameLayer::Default, GameLayer::AegisBoundary],
        ),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        DespawnOnExit(crate::GameState::Playing),
    ));
}

pub(super) fn zero_player_velocity(mut velocity_q: Query<&mut LinearVelocity, With<Player>>) {
    if let Ok(mut vel) = velocity_q.single_mut() {
        vel.0 = Vec3::ZERO;
    }
}

pub(super) fn setup_world_once(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.5, 0.0)),
        DespawnOnExit(crate::GameState::Playing),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.4, 0.45, 0.6),
        brightness: 200.0,
        ..default()
    });
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
    mut next_mode: ResMut<NextState<PlayMode>>,
    mut inv_open: Option<ResMut<InventoryOpen>>,
    mut drone_cargo_open: Option<ResMut<DroneCargoOpen>>,
    mut craft_modal: Option<ResMut<CraftModal>>,
    mut machine: Option<ResMut<MachineStatusPanel>>,
    mut storage: Option<ResMut<StorageStatusPanel>>,
    mut tech: Option<ResMut<TechTreePanelOpen>>,
    mut planner: Option<ResMut<PlannerOpen>>,
    mut picker: Option<ResMut<RecipePickerState>>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    if craft_modal.as_ref().is_some_and(|m| m.0.is_some()) {
        if let Some(ref mut m) = craft_modal {
            m.0 = None;
        }
        return;
    }
    if drone_cargo_open.as_ref().is_some_and(|o| o.0) {
        if let Some(ref mut o) = drone_cargo_open {
            o.0 = false;
        }
        return;
    }
    if inv_open.as_ref().is_some_and(|o| o.0) {
        if let Some(ref mut o) = inv_open {
            o.0 = false;
        }
        return;
    }
    if picker.as_ref().is_some_and(|p| p.open) {
        if let Some(ref mut p) = picker {
            p.open = false;
        }
        return;
    }
    if planner.as_ref().is_some_and(|p| p.open) {
        if let Some(ref mut p) = planner {
            p.open = false;
        }
        return;
    }
    if machine.as_ref().is_some_and(|m| m.entity.is_some())
        || storage.as_ref().is_some_and(|s| s.0.is_some())
        || tech.as_ref().is_some_and(|t| t.open)
    {
        if let Some(ref mut m) = machine {
            m.entity = None;
        }
        if let Some(ref mut s) = storage {
            s.0 = None;
        }
        if let Some(ref mut t) = tech {
            t.open = false;
        }
        return;
    }
    next_mode.set(PlayMode::Paused);
}

pub(super) fn toggle_planner(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut planner: Option<ResMut<PlannerOpen>>,
    mut picker: Option<ResMut<RecipePickerState>>,
) {
    let Some(ref mut planner) = planner else {
        return;
    };
    if keyboard.just_pressed(KeyCode::KeyP) {
        planner.open = !planner.open;
        if !planner.open
            && let Some(ref mut p) = picker
        {
            p.open = false;
        }
    }
}

pub(super) fn resume_on_escape(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_mode: ResMut<NextState<PlayMode>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_mode.set(PlayMode::Exploring);
    }
}

pub(super) fn toggle_inventory(
    keyboard: Res<ButtonInput<KeyCode>>,
    play_mode: Option<Res<State<PlayMode>>>,
    inv_open: Option<ResMut<InventoryOpen>>,
    drone_cargo_open: Option<ResMut<DroneCargoOpen>>,
) {
    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }
    let in_drone_mode = play_mode.is_some_and(|m| *m.get() == PlayMode::DronePilot);
    if in_drone_mode {
        if let Some(mut open) = drone_cargo_open {
            open.0 = !open.0;
        }
    } else if let Some(mut open) = inv_open {
        open.0 = !open.0;
    }
}

pub(super) fn any_ui_open(
    inv: Option<Res<InventoryOpen>>,
    drone_cargo: Option<Res<DroneCargoOpen>>,
    craft_modal: Option<Res<CraftModal>>,
    machine: Option<Res<MachineStatusPanel>>,
    storage: Option<Res<StorageStatusPanel>>,
    tech: Option<Res<TechTreePanelOpen>>,
    planner: Option<Res<PlannerOpen>>,
) -> bool {
    inv.is_some_and(|o| o.0)
        || drone_cargo.is_some_and(|o| o.0)
        || craft_modal.is_some_and(|m| m.0.is_some())
        || machine.is_some_and(|m| m.entity.is_some())
        || storage.is_some_and(|s| s.0.is_some())
        || tech.is_some_and(|t| t.open)
        || planner.is_some_and(|p| p.open)
}

pub(super) fn sync_cursor(
    inv: Option<Res<InventoryOpen>>,
    machine: Option<Res<MachineStatusPanel>>,
    storage: Option<Res<StorageStatusPanel>>,
    tech: Option<Res<TechTreePanelOpen>>,
    planner: Option<Res<PlannerOpen>>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let ui_open = inv.is_some_and(|o| o.0)
        || machine.is_some_and(|m| m.entity.is_some())
        || storage.is_some_and(|s| s.0.is_some())
        || tech.is_some_and(|t| t.open)
        || planner.is_some_and(|p| p.open);
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
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    player_q: Query<&Transform, (With<Player>, Without<MainCamera>)>,
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
    camera.translation = player_transform.translation + Vec3::Y * 0.5;
}

pub(super) fn player_velocity(
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    mut velocity_q: Query<&mut LinearVelocity, With<Player>>,
) {
    let Ok(camera) = camera_q.single() else {
        return;
    };
    let Ok(mut velocity) = velocity_q.single_mut() else {
        return;
    };

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
    use crate::inventory::InventoryOpen;
    use crate::{GameState, PlayMode};

    #[test]
    fn toggle_pause_escape_transitions_to_paused() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
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
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::Paused
        );
    }

    #[test]
    fn toggle_pause_blocked_when_inventory_open() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
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
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::Exploring
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
    fn toggle_pause_escape_closes_inventory_when_open() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
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

        assert!(!app.world().resource::<InventoryOpen>().0);
        assert_eq!(
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::Exploring
        );
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
    fn resume_on_escape_sets_exploring() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, resume_on_escape.run_if(in_state(PlayMode::Paused)));

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<PlayMode>>()
            .set(PlayMode::Paused);
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::Exploring
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

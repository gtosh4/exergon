use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy_tnua::TnuaUserControlsSystems;
use bevy_tnua::builtins::{TnuaBuiltinWalk, TnuaBuiltinWalkConfig};
use bevy_tnua::prelude::*;
use bevy_tnua_avian3d::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;

use crate::logistics::StorageUnit;
use crate::world::MainCamera;
use crate::world::generation::OreDeposit;
use crate::{GameState, PlayMode};

const MINE_REACH: f32 = 4.0;

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
                drone_mine_system.run_if(in_state(PlayMode::DronePilot)),
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn spawn_land_drone(mut commands: Commands, mut scheme_configs: ResMut<Assets<DroneSchemeConfig>>) {
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

/// Asymptotic yield factor: starts near 1.0, decays toward a floor > 0.
/// Floor and decay rate vary per deposit via `depletion_seed`.
pub(crate) fn yield_factor(total_extracted: f32, depletion_seed: u64) -> f32 {
    let floor = 0.1 + (depletion_seed % 100) as f32 * 0.001;
    let k = 0.02 + (depletion_seed % 50) as f32 * 0.001;
    floor + (1.0 - floor) * (-k * total_extracted).exp()
}

pub(crate) fn sample_ore<R: Rng>(ores: &[(String, f32)], rng: &mut R) -> Option<String> {
    let total: f32 = ores.iter().map(|(_, w)| w).sum();
    if total <= 0.0 {
        return None;
    }
    let roll = rng.gen_range(0.0f32..total);
    let mut acc = 0.0;
    for (id, w) in ores {
        acc += w;
        if roll < acc {
            return Some(id.clone());
        }
    }
    ores.last().map(|(id, _)| id.clone())
}

fn drone_mine_system(
    mouse: Res<ButtonInput<MouseButton>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    spatial_query: SpatialQuery,
    mut deposit_q: Query<&mut OreDeposit>,
    mut storage_q: Query<&mut StorageUnit>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(cam) = camera_q.single() else {
        return;
    };
    let dir = Dir3::new(*cam.forward()).unwrap_or(Dir3::NEG_Z);
    let Some(hit) =
        spatial_query.cast_ray(cam.translation, dir, MINE_REACH, true, &Default::default())
    else {
        return;
    };
    let Ok(mut deposit) = deposit_q.get_mut(hit.entity) else {
        return;
    };
    let rng_seed = deposit.depletion_seed ^ deposit.total_extracted.to_bits() as u64;
    let mut rng = Pcg64::seed_from_u64(rng_seed);
    if let Some(ore_id) = sample_ore(&deposit.ores, &mut rng) {
        if let Some(mut unit) = storage_q.iter_mut().next() {
            *unit.items.entry(ore_id).or_insert(0) += 1;
        }
        deposit.total_extracted += 1.0;
    }
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

    #[test]
    fn yield_factor_decreases_monotonically() {
        let seed = 12345u64;
        let y: Vec<f32> = (0..5)
            .map(|i| yield_factor(i as f32 * 20.0, seed))
            .collect();
        for w in y.windows(2) {
            assert!(w[0] > w[1], "yield should decrease: {} > {}", w[0], w[1]);
        }
        assert!(y[4] > 0.0, "yield floor must be above zero");
    }

    #[test]
    fn sample_ore_single_entry_always_returns_it() {
        let ores = vec![("iron_ore".to_string(), 1.0f32)];
        let mut rng = Pcg64::seed_from_u64(0);
        assert_eq!(sample_ore(&ores, &mut rng), Some("iron_ore".to_string()));
    }

    #[test]
    fn sample_ore_empty_returns_none() {
        let ores: Vec<(String, f32)> = vec![];
        let mut rng = Pcg64::seed_from_u64(0);
        assert_eq!(sample_ore(&ores, &mut rng), None);
    }

    #[test]
    fn mine_samples_ore_and_increments_extracted() {
        use crate::world::generation::OreDeposit;

        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("copper_ore".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 0,
        };

        let rng_seed = deposit.depletion_seed ^ deposit.total_extracted.to_bits() as u64;
        let mut rng = Pcg64::seed_from_u64(rng_seed);
        let ore = sample_ore(&deposit.ores, &mut rng);
        if ore.is_some() {
            deposit.total_extracted += 1.0;
        }

        assert_eq!(ore.as_deref(), Some("copper_ore"));
        assert_eq!(deposit.total_extracted, 1.0);
        assert!(!deposit.ores.is_empty(), "deposit must persist");
    }

    #[test]
    fn repeated_mining_degrades_yield() {
        use crate::world::generation::OreDeposit;

        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("iron_ore".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 42,
        };
        let y_before = yield_factor(deposit.total_extracted, deposit.depletion_seed);
        for _ in 0..10 {
            deposit.total_extracted += 1.0;
        }
        let y_after = yield_factor(deposit.total_extracted, deposit.depletion_seed);
        assert!(
            y_before > y_after,
            "yield must degrade: {} > {}",
            y_before,
            y_after
        );
        assert!(y_after > 0.0, "yield floor must remain above zero");
    }
}

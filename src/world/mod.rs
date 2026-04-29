use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_voxel_world::prelude::*;
use noise::{HybridMulti, NoiseFn, Perlin};

use crate::seed::DomainSeeds;
use crate::GameState;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VoxelWorldPlugin::with_config(WorldConfig::default()))
            .add_systems(Startup, spawn_camera)
            .add_systems(OnEnter(GameState::Loading), finish_loading)
            .add_systems(OnEnter(GameState::Playing), (setup_world_once, lock_cursor))
            .add_systems(OnEnter(GameState::Paused), unlock_cursor)
            .add_systems(
                Update,
                (
                    camera_input,
                    toggle_pause,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
struct MainCamera;

#[derive(Resource, Clone, Default)]
struct WorldConfig {
    world_seed: u64,
    active: bool,
}

impl VoxelWorldConfig for WorldConfig {
    type MaterialIndex = u8;
    type ChunkUserBundle = ();

    fn spawning_distance(&self) -> u32 {
        if self.active { 12 } else { 0 }
    }

    fn min_despawn_distance(&self) -> u32 {
        2
    }

    fn voxel_lookup_delegate(&self) -> VoxelLookupDelegate<Self::MaterialIndex> {
        let seed = self.world_seed;
        Box::new(move |_chunk_pos, _lod, _previous| make_voxel_fn(seed))
    }

    fn texture_index_mapper(&self) -> Arc<dyn Fn(Self::MaterialIndex) -> [u32; 3] + Send + Sync> {
        Arc::new(|mat| match mat {
            1 => [1, 1, 1],
            _ => [0, 0, 0],
        })
    }
}

fn make_voxel_fn(
    seed: u64,
) -> Box<dyn FnMut(IVec3, Option<WorldVoxel>) -> WorldVoxel + Send + Sync> {
    let mut noise = HybridMulti::<Perlin>::new((seed ^ (seed >> 32)) as u32);
    noise.octaves = 5;
    noise.frequency = 1.1;
    noise.lacunarity = 2.8;
    noise.persistence = 0.4;

    let mut cache = HashMap::<(i32, i32), f64>::new();

    Box::new(move |pos: IVec3, _previous| {
        if pos.y < 1 {
            return WorldVoxel::Solid(0);
        }
        let [x, y, z] = pos.as_dvec3().to_array();
        let surface = *cache
            .entry((pos.x, pos.z))
            .or_insert_with(|| noise.get([x / 1000.0, z / 1000.0]) * 50.0);
        if y >= surface {
            WorldVoxel::Air
        } else if y >= surface - 1.0 {
            WorldVoxel::Solid(1)
        } else {
            WorldVoxel::Solid(0)
        }
    })
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 80.0, 0.0),
        MainCamera,
        VoxelWorldCamera::<WorldConfig>::default(),
    ));
}

fn finish_loading(
    mut world_config: ResMut<WorldConfig>,
    domain_seeds: Option<Res<DomainSeeds>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(seeds) = domain_seeds {
        world_config.world_seed = seeds.world;
    }
    world_config.active = true;
    next_state.set(GameState::Playing);
}

fn setup_world_once(
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

fn lock_cursor(mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn unlock_cursor(mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Paused);
    }
}

fn camera_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
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

    let speed = 50.0;
    if direction != Vec3::ZERO {
        transform.translation += direction.normalize() * speed * time.delta_secs();
    }
}

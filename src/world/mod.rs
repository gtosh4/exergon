use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_voxel_world::prelude::*;
use noise::{HybridMulti, NoiseFn, Perlin};

use crate::content::VeinRegistry;
use crate::seed::DomainSeeds;
use crate::GameState;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VoxelWorldPlugin::with_config(WorldConfig::default()))
            .init_resource::<LookTarget>()
            .add_systems(Startup, spawn_camera)
            .add_systems(OnEnter(GameState::Loading), finish_loading)
            .add_systems(OnEnter(GameState::Playing), (setup_world_once, lock_cursor))
            .add_systems(OnEnter(GameState::Paused), unlock_cursor)
            .add_systems(
                Update,
                (
                    camera_input,
                    toggle_pause,
                    update_look_target.after(camera_input),
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
    vein_registry: Option<Arc<VeinRegistry>>,
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
        let registry = self.vein_registry.clone();
        Box::new(move |_chunk_pos, _lod, _previous| make_voxel_fn(seed, registry.clone()))
    }

    fn texture_index_mapper(&self) -> Arc<dyn Fn(Self::MaterialIndex) -> [u32; 3] + Send + Sync> {
        Arc::new(|mat| match mat {
            1 => [1, 1, 1],
            2 => [2, 2, 2],
            3 => [3, 3, 3],
            4 => [4, 4, 4],
            5 => [5, 5, 5],
            6 => [6, 6, 6],
            _ => [0, 0, 0],
        })
    }
}

fn make_voxel_fn(
    seed: u64,
    registry: Option<Arc<VeinRegistry>>,
) -> Box<dyn FnMut(IVec3, Option<WorldVoxel>) -> WorldVoxel + Send + Sync> {
    let mut noise = HybridMulti::<Perlin>::new((seed ^ (seed >> 32)) as u32);
    noise.octaves = 5;
    noise.frequency = 1.1;
    noise.lacunarity = 2.8;
    noise.persistence = 0.4;

    let mut surface_cache = std::collections::HashMap::<(i32, i32), f64>::new();

    Box::new(move |pos: IVec3, _previous| {
        if pos.y < 1 {
            return WorldVoxel::Solid(0);
        }
        let [x, y, z] = pos.as_dvec3().to_array();
        let surface = *surface_cache
            .entry((pos.x, pos.z))
            .or_insert_with(|| noise.get([x / 1000.0, z / 1000.0]) * 50.0);

        if y >= surface {
            WorldVoxel::Air
        } else if y >= surface - 1.0 {
            WorldVoxel::Solid(1)
        } else {
            let mat = registry
                .as_ref()
                .and_then(|r| r.ore_at(seed, pos.x, pos.y, pos.z))
                .unwrap_or(0);
            WorldVoxel::Solid(mat)
        }
    })
}

#[derive(Resource, Default, Clone)]
pub enum LookTarget {
    #[default]
    Nothing,
    Voxel {
        material: u8,
    },
}

fn is_blocked(voxel_world: &VoxelWorld<'_, WorldConfig>, center: Vec3, r: f32) -> bool {
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

fn update_look_target(
    camera_q: Query<&Transform, With<MainCamera>>,
    voxel_world: VoxelWorld<WorldConfig>,
    mut look_target: ResMut<LookTarget>,
) {
    let Ok(cam) = camera_q.single() else {
        *look_target = LookTarget::Nothing;
        return;
    };

    let origin = cam.translation;
    let forward = cam.forward();
    let max_dist = 8.0_f32;

    let ray = Ray3d::new(origin, forward);
    let hit = voxel_world
        .raycast(ray, &|(_pos, voxel)| matches!(voxel, WorldVoxel::Solid(_)))
        .filter(|hit| hit.position.distance(origin) <= max_dist);

    *look_target = match hit {
        None => LookTarget::Nothing,
        Some(hit) => match hit.voxel {
            WorldVoxel::Solid(mat) => LookTarget::Voxel { material: mat },
            _ => LookTarget::Nothing,
        },
    };
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
    registry: Option<Res<VeinRegistry>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(seeds) = domain_seeds {
        world_config.world_seed = seeds.world;
    }
    if let Some(reg) = registry {
        world_config.vein_registry = Some(Arc::new(reg.clone()));
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

    if !is_blocked(&voxel_world, current + delta, R) {
        transform.translation += delta;
    } else {
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
    }
}

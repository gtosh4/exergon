use std::collections::HashSet;
use std::sync::Arc;

use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use noise::{HybridMulti, NoiseFn, Perlin};
use xxhash_rust::xxh64::xxh64;

use crate::GameState;
use crate::content::{DepositRegistry, VeinRegistry};
use crate::machine::MachineVisualAssets;
use crate::seed::DomainSeeds;

use super::MainCamera;

pub(crate) const CHUNK_SIZE: i32 = 64;
const SPAWN_DIST: i32 = 4;
const DESPAWN_DIST: i32 = 6;

#[derive(Resource, Clone, Default)]
pub(crate) struct WorldConfig {
    pub world_seed: u64,
    pub active: bool,
    pub(crate) vein_registry: Option<Arc<VeinRegistry>>,
}

#[derive(Component)]
pub(crate) struct TerrainChunk {
    pub(crate) chunk_pos: IVec2,
}

#[derive(Resource, Default)]
pub(crate) struct SpawnedChunks(pub(crate) HashSet<IVec2>);

/// Reusable terrain height sampler — create once per operation, call `height_at` many times.
pub(crate) struct TerrainSampler {
    noise: HybridMulti<Perlin>,
}

impl TerrainSampler {
    pub(crate) fn new(seed: u64) -> Self {
        let mut noise = HybridMulti::<Perlin>::new((seed ^ (seed >> 32)) as u32);
        noise.octaves = 5;
        noise.frequency = 1.1;
        noise.lacunarity = 2.8;
        noise.persistence = 0.4;
        Self { noise }
    }

    pub(crate) fn height_at(&self, wx: f64, wz: f64) -> f32 {
        (self.noise.get([wx / 1000.0, wz / 1000.0]) * 50.0) as f32
    }
}

pub(super) fn generate_chunk_mesh(seed: u64, cx: i32, cz: i32) -> Mesh {
    let n = CHUNK_SIZE as usize;
    let verts = n + 1;

    let sampler = TerrainSampler::new(seed);
    let height = |lx: usize, lz: usize| -> f32 {
        let wx = (cx * CHUNK_SIZE + lx as i32) as f64;
        let wz = (cz * CHUNK_SIZE + lz as i32) as f64;
        sampler.height_at(wx, wz)
    };

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(verts * verts);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(verts * verts);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(verts * verts);

    for lz in 0..=n {
        for lx in 0..=n {
            let h = height(lx, lz);
            positions.push([lx as f32, h, lz as f32]);

            // Compute normal from neighboring heights.
            let h_px = height(lx + 1, lz);
            let h_nx = if lx > 0 { height(lx - 1, lz) } else { h };
            let h_pz = height(lx, lz + 1);
            let h_nz = if lz > 0 { height(lx, lz - 1) } else { h };
            let dx = h_px - h_nx;
            let dz = h_pz - h_nz;
            let n_vec = Vec3::new(-dx, 2.0, -dz).normalize();
            normals.push(n_vec.to_array());

            uvs.push([lx as f32 / n as f32, lz as f32 / n as f32]);
        }
    }

    let mut indices: Vec<u32> = Vec::with_capacity(n * n * 6);
    for lz in 0..n {
        for lx in 0..n {
            let i = (lz * verts + lx) as u32;
            let stride = verts as u32;
            indices.push(i);
            indices.push(i + stride);
            indices.push(i + 1);
            indices.push(i + 1);
            indices.push(i + stride);
            indices.push(i + stride + 1);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub(super) fn spawn_chunks(
    mut commands: Commands,
    world_config: Res<WorldConfig>,
    mut spawned: ResMut<SpawnedChunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_q: Query<&Transform, With<MainCamera>>,
) {
    if !world_config.active {
        return;
    }
    let Ok(cam) = camera_q.single() else {
        return;
    };

    let cam_chunk = IVec2::new(
        (cam.translation.x / CHUNK_SIZE as f32).floor() as i32,
        (cam.translation.z / CHUNK_SIZE as f32).floor() as i32,
    );

    for cz in (cam_chunk.y - SPAWN_DIST)..=(cam_chunk.y + SPAWN_DIST) {
        for cx in (cam_chunk.x - SPAWN_DIST)..=(cam_chunk.x + SPAWN_DIST) {
            let chunk_pos = IVec2::new(cx, cz);
            if spawned.0.contains(&chunk_pos) {
                continue;
            }

            let mesh = generate_chunk_mesh(world_config.world_seed, cx, cz);
            let world_pos = Vec3::new(
                cx as f32 * CHUNK_SIZE as f32,
                0.0,
                cz as f32 * CHUNK_SIZE as f32,
            );

            commands.spawn((
                TerrainChunk { chunk_pos },
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.3, 0.5, 0.2),
                    perceptual_roughness: 0.9,
                    ..default()
                })),
                Transform::from_translation(world_pos),
                RigidBody::Static,
            ));
            spawned.0.insert(chunk_pos);
        }
    }
}

pub(super) fn despawn_chunks(
    mut commands: Commands,
    world_config: Res<WorldConfig>,
    mut spawned: ResMut<SpawnedChunks>,
    chunk_q: Query<(Entity, &TerrainChunk)>,
    camera_q: Query<&Transform, With<MainCamera>>,
) {
    if !world_config.active {
        return;
    }
    let Ok(cam) = camera_q.single() else {
        return;
    };

    let cam_chunk = IVec2::new(
        (cam.translation.x / CHUNK_SIZE as f32).floor() as i32,
        (cam.translation.z / CHUNK_SIZE as f32).floor() as i32,
    );

    for (entity, chunk) in &chunk_q {
        let diff = chunk.chunk_pos - cam_chunk;
        let dist = diff.x.abs().max(diff.y.abs());
        if dist > DESPAWN_DIST {
            commands.entity(entity).despawn();
            spawned.0.remove(&chunk.chunk_pos);
        }
    }
}

// Avian can't read Mesh3d when TrimeshFromMesh is inserted (assign_material runs later).
// Build the collider directly here once Mesh3d is present.
pub(super) fn add_chunk_colliders(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    query: Query<(Entity, &Mesh3d), (Changed<Mesh3d>, With<RigidBody>)>,
) {
    for (entity, mesh3d) in query.iter() {
        let Some(mesh) = meshes.get(&mesh3d.0) else {
            continue;
        };
        if let Some(collider) = Collider::trimesh_from_mesh(mesh) {
            commands.entity(entity).insert(collider);
        }
    }
}

/// Surface ore deposit marker spawned at terrain height when a chunk loads.
#[derive(Component)]
pub(crate) struct OreDeposit {
    pub(crate) chunk_pos: IVec2,
    pub(crate) ores: Vec<(String, f32)>,
    pub(crate) total_extracted: f32,
    pub(crate) depletion_seed: u64,
}

/// Returns the ore list for the deposit cell that covers `chunk_pos`, or None.
///
/// Each 64×64 chunk maps to exactly one deposit cell; this checks that cell.
pub(crate) fn chunk_deposit(
    world_seed: u64,
    chunk_pos: IVec2,
    registry: &DepositRegistry,
) -> Option<Vec<(String, f32)>> {
    let wx = (chunk_pos.x * CHUNK_SIZE) as f32 + CHUNK_SIZE as f32 * 0.5;
    let wz = (chunk_pos.y * CHUNK_SIZE) as f32 + CHUNK_SIZE as f32 * 0.5;
    registry.ore_at(world_seed, wx, wz)
}

pub(super) fn spawn_deposit_markers(
    mut commands: Commands,
    registry: Res<DepositRegistry>,
    world_config: Res<WorldConfig>,
    new_chunks: Query<(Entity, &TerrainChunk), Added<TerrainChunk>>,
    visuals: Option<Res<MachineVisualAssets>>,
) {
    if !world_config.active {
        return;
    }
    let sampler = TerrainSampler::new(world_config.world_seed);
    for (_chunk_entity, chunk) in &new_chunks {
        let Some(ores) = chunk_deposit(world_config.world_seed, chunk.chunk_pos, &registry) else {
            continue;
        };
        let wx = (chunk.chunk_pos.x * CHUNK_SIZE) as f32 + CHUNK_SIZE as f32 * 0.5;
        let wz = (chunk.chunk_pos.y * CHUNK_SIZE) as f32 + CHUNK_SIZE as f32 * 0.5;
        let surface_y = sampler.height_at(wx as f64, wz as f64);

        let depletion_seed = {
            let mut key = world_config.world_seed.to_le_bytes().to_vec();
            key.extend_from_slice(b"depl");
            key.extend_from_slice(&chunk.chunk_pos.x.to_le_bytes());
            key.extend_from_slice(&chunk.chunk_pos.y.to_le_bytes());
            xxh64(&key, 0)
        };

        let mut entity_cmd = commands.spawn((
            OreDeposit {
                chunk_pos: chunk.chunk_pos,
                ores,
                total_extracted: 0.0,
                depletion_seed,
            },
            Transform::from_xyz(wx, surface_y + 0.75, wz),
        ));
        if let Some(ref v) = visuals {
            entity_cmd.insert(SceneRoot(v.deposit_scene.clone()));
        }
    }
}

pub(super) fn despawn_deposit_markers(
    mut commands: Commands,
    deposit_q: Query<(Entity, &OreDeposit)>,
    spawned: Res<SpawnedChunks>,
) {
    for (entity, deposit) in &deposit_q {
        if !spawned.0.contains(&deposit.chunk_pos) {
            commands.entity(entity).despawn();
        }
    }
}

pub(super) fn setup_world_config(
    mut world_config: ResMut<WorldConfig>,
    domain_seeds: Option<Res<DomainSeeds>>,
    registry: Option<Res<VeinRegistry>>,
) {
    if let Some(seeds) = domain_seeds {
        world_config.world_seed = seeds.world;
    }
    if let Some(reg) = registry {
        world_config.vein_registry = Some(Arc::new(reg.clone()));
    }
    world_config.active = true;
}

pub(super) fn poll_assets_loaded(
    asset_server: Res<AssetServer>,
    visuals: Option<Res<crate::machine::MachineVisualAssets>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(visuals) = visuals else { return };

    let all_loaded = visuals
        .gltf_handles
        .values()
        .all(|h| asset_server.is_loaded_with_dependencies(h))
        && asset_server.is_loaded_with_dependencies(&visuals.platform_scene)
        && asset_server.is_loaded_with_dependencies(&visuals.deposit_scene);

    if all_loaded {
        next_state.set(GameState::Playing);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;

    use super::*;
    use crate::GameState;
    use crate::content::{DepositDef, DepositRegistry};

    #[test]
    fn terrain_sampler_new_does_not_panic() {
        let _ = TerrainSampler::new(0);
        let _ = TerrainSampler::new(u64::MAX);
    }

    #[test]
    fn terrain_sampler_height_at_is_deterministic() {
        let s = TerrainSampler::new(42);
        assert_eq!(s.height_at(100.0, 200.0), s.height_at(100.0, 200.0));
    }

    #[test]
    fn terrain_sampler_different_seeds_differ() {
        let a = TerrainSampler::new(1).height_at(137.0, 251.0);
        let b = TerrainSampler::new(2).height_at(137.0, 251.0);
        assert_ne!(a, b, "different seeds should produce different heights");
    }

    #[test]
    fn generate_chunk_mesh_vertex_count() {
        let mesh = generate_chunk_mesh(0, 0, 0);
        let n = (CHUNK_SIZE + 1) as usize;
        let expected_verts = n * n;
        let pos = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("mesh has positions");
        assert_eq!(
            pos.len(),
            expected_verts,
            "mesh should have {expected_verts} vertices"
        );
    }

    #[test]
    fn generate_chunk_mesh_index_count() {
        let mesh = generate_chunk_mesh(0, 0, 0);
        let n = CHUNK_SIZE as usize;
        let expected_indices = n * n * 6;
        let indices = mesh.indices().expect("mesh has indices");
        assert_eq!(
            indices.len(),
            expected_indices,
            "mesh should have {expected_indices} indices"
        );
    }

    #[test]
    fn generate_chunk_mesh_deterministic() {
        let m1 = generate_chunk_mesh(42, 3, -2);
        let m2 = generate_chunk_mesh(42, 3, -2);
        let pos1 = m1.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let pos2 = m2.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        assert_eq!(
            format!("{pos1:?}"),
            format!("{pos2:?}"),
            "same seed+coords must produce identical mesh"
        );
    }

    fn iron_deposit_registry() -> DepositRegistry {
        DepositRegistry::new(vec![DepositDef {
            id: "iron".into(),
            ores: vec![("iron".into(), 1.0)],
        }])
    }

    #[test]
    fn chunk_deposit_empty_registry_returns_none() {
        let reg = DepositRegistry::new(vec![]);
        assert!(chunk_deposit(0, IVec2::ZERO, &reg).is_none());
    }

    #[test]
    fn chunk_deposit_is_deterministic() {
        let reg = iron_deposit_registry();
        let a = chunk_deposit(42, IVec2::new(1, 2), &reg);
        let b = chunk_deposit(42, IVec2::new(1, 2), &reg);
        assert_eq!(a, b);
    }

    #[test]
    fn chunk_deposit_different_chunks_can_differ() {
        let reg = iron_deposit_registry();
        // With 33% probability per cell, two adjacent chunks are unlikely both Some or both None.
        // Check that not every chunk returns the same result (statistical).
        let results: Vec<bool> = (0..20)
            .map(|i| chunk_deposit(99, IVec2::new(i, 0), &reg).is_some())
            .collect();
        let all_same = results.windows(2).all(|w| w[0] == w[1]);
        assert!(!all_same, "expected variation across chunk deposit checks");
    }

    #[test]
    fn despawn_deposit_markers_removes_marker_for_unspawned_chunk() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SpawnedChunks::default())
            .add_systems(Update, despawn_deposit_markers);

        let chunk_pos = IVec2::new(3, 5);
        let marker = app
            .world_mut()
            .spawn(OreDeposit {
                chunk_pos,
                ores: vec![],
                total_extracted: 0.0,
                depletion_seed: 0,
            })
            .id();

        app.update();

        assert!(
            app.world().get::<OreDeposit>(marker).is_none(),
            "deposit not in SpawnedChunks should be despawned"
        );
    }

    #[test]
    fn despawn_deposit_markers_keeps_marker_for_spawned_chunk() {
        let mut app = App::new();
        let chunk_pos = IVec2::new(1, 2);
        let mut spawned = SpawnedChunks::default();
        spawned.0.insert(chunk_pos);
        app.add_plugins(MinimalPlugins)
            .insert_resource(spawned)
            .add_systems(Update, despawn_deposit_markers);

        let marker = app
            .world_mut()
            .spawn(OreDeposit {
                chunk_pos,
                ores: vec![],
                total_extracted: 0.0,
                depletion_seed: 0,
            })
            .id();

        app.update();

        assert!(
            app.world().get::<OreDeposit>(marker).is_some(),
            "deposit in SpawnedChunks should be kept"
        );
    }

    #[test]
    fn despawn_chunks_removes_chunk_beyond_despawn_dist() {
        use crate::world::MainCamera;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(WorldConfig {
                world_seed: 0,
                active: true,
                vein_registry: None,
            })
            .insert_resource(SpawnedChunks::default())
            .add_systems(Update, despawn_chunks);

        app.world_mut().spawn((Transform::default(), MainCamera));

        let far_pos = IVec2::new(DESPAWN_DIST + 2, 0);
        let chunk = app
            .world_mut()
            .spawn(TerrainChunk { chunk_pos: far_pos })
            .id();
        app.world_mut()
            .resource_mut::<SpawnedChunks>()
            .0
            .insert(far_pos);

        app.update();

        assert!(
            app.world().get::<TerrainChunk>(chunk).is_none(),
            "chunk beyond despawn dist should be removed"
        );
        assert!(!app.world().resource::<SpawnedChunks>().0.contains(&far_pos));
    }

    #[test]
    fn despawn_chunks_keeps_chunk_within_despawn_dist() {
        use crate::world::MainCamera;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(WorldConfig {
                world_seed: 0,
                active: true,
                vein_registry: None,
            })
            .insert_resource(SpawnedChunks::default())
            .add_systems(Update, despawn_chunks);

        app.world_mut().spawn((Transform::default(), MainCamera));

        let near_pos = IVec2::new(1, 0);
        let chunk = app
            .world_mut()
            .spawn(TerrainChunk {
                chunk_pos: near_pos,
            })
            .id();
        app.world_mut()
            .resource_mut::<SpawnedChunks>()
            .0
            .insert(near_pos);

        app.update();

        assert!(
            app.world().get::<TerrainChunk>(chunk).is_some(),
            "chunk within despawn dist should be kept"
        );
    }

    #[test]
    fn setup_world_config_activates_world_config() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .insert_resource(WorldConfig::default())
            .add_systems(OnEnter(GameState::Loading), setup_world_config);

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Loading);
        app.update();

        assert!(app.world().resource::<WorldConfig>().active);
    }
}

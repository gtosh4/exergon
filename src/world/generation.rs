use std::collections::HashSet;
use std::sync::Arc;

use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use noise::{HybridMulti, NoiseFn, Perlin};

use crate::GameState;
use crate::content::VeinRegistry;
use crate::seed::DomainSeeds;

use super::MainCamera;

pub(crate) const CHUNK_SIZE: i32 = 64;
const SPAWN_DIST: i32 = 4;
const DESPAWN_DIST: i32 = 6;

#[derive(Resource, Clone, Default)]
pub(crate) struct WorldConfig {
    pub(crate) world_seed: u64,
    pub(crate) active: bool,
    pub(crate) vein_registry: Option<Arc<VeinRegistry>>,
}

#[derive(Component)]
pub(crate) struct TerrainChunk {
    pub(crate) chunk_pos: IVec2,
}

#[derive(Resource, Default)]
pub(crate) struct SpawnedChunks(pub(crate) HashSet<IVec2>);

pub(super) fn generate_chunk_mesh(seed: u64, cx: i32, cz: i32) -> Mesh {
    let n = CHUNK_SIZE as usize;
    let verts = n + 1;

    let mut noise = HybridMulti::<Perlin>::new((seed ^ (seed >> 32)) as u32);
    noise.octaves = 5;
    noise.frequency = 1.1;
    noise.lacunarity = 2.8;
    noise.persistence = 0.4;

    let height = |lx: usize, lz: usize| -> f32 {
        let wx = (cx * CHUNK_SIZE + lx as i32) as f64;
        let wz = (cz * CHUNK_SIZE + lz as i32) as f64;
        (noise.get([wx / 1000.0, wz / 1000.0]) * 50.0) as f32
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

pub(super) fn finish_loading(
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

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;

    use super::*;
    use crate::GameState;

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

    #[test]
    fn finish_loading_activates_world_config() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .insert_resource(WorldConfig::default())
            .add_systems(OnEnter(GameState::Loading), finish_loading);

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Loading);
        app.update();

        assert!(app.world().resource::<WorldConfig>().active);
    }
}

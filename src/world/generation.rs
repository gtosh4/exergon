use std::sync::Arc;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use noise::{HybridMulti, NoiseFn, Perlin};

use crate::GameState;
use crate::content::VeinRegistry;
use crate::seed::DomainSeeds;

#[derive(Resource, Clone, Default)]
pub(crate) struct WorldConfig {
    pub(crate) world_seed: u64,
    pub(crate) active: bool,
    pub(crate) vein_registry: Option<Arc<VeinRegistry>>,
    pub(crate) texture_layers: u32,
}

impl VoxelWorldConfig for WorldConfig {
    type MaterialIndex = u8;
    type ChunkUserBundle = RigidBody;

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

    fn voxel_texture(&self) -> Option<(String, u32)> {
        if self.texture_layers > 0 {
            Some(("textures/blocks.png".into(), self.texture_layers))
        } else {
            None
        }
    }

    fn texture_index_mapper(&self) -> Arc<dyn Fn(Self::MaterialIndex) -> [u32; 3] + Send + Sync> {
        Arc::new(|mat| match mat {
            1 => [1, 2, 3],
            2 => [4, 4, 4],
            3 => [5, 5, 5],
            4 => [6, 6, 6],
            5 => [7, 7, 7],
            6 => [8, 8, 8],
            7 => [9, 9, 9],     // smelter_core
            8 => [10, 10, 10],  // machine_casing
            9 => [11, 11, 11],  // assembler_core
            10 => [12, 12, 12], // refinery_core
            11 => [13, 13, 13], // gateway_core
            12 => [14, 14, 14], // logistics_cable
            13 => [15, 15, 15], // power_cable
            14 => [16, 16, 16], // storage_crate
            15 => [17, 17, 17], // generator
            16 => [18, 18, 18], // energy_io
            17 => [19, 19, 19], // logistics_io
            _ => [0, 0, 0],
        })
    }

    fn chunk_meshing_delegate(
        &self,
    ) -> ChunkMeshingDelegate<Self::MaterialIndex, Self::ChunkUserBundle> {
        Some(Box::new(
            |pos, lod, data_shape, mesh_shape, previous_data| {
                let mut inner = default_chunk_meshing_delegate::<u8, RigidBody>(
                    pos,
                    lod,
                    data_shape,
                    mesh_shape,
                    previous_data,
                );
                Box::new(move |voxels, ds, ms, mapper| {
                    let (mesh, _) = inner(voxels, ds, ms, mapper);
                    (mesh, Some(RigidBody::Static))
                })
            },
        ))
    }
}

pub(super) fn make_voxel_fn(
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
    use bevy_voxel_world::prelude::*;

    use super::*;
    use crate::GameState;

    #[test]
    fn spawning_distance_active() {
        let config = WorldConfig {
            active: true,
            ..Default::default()
        };
        assert_eq!(config.spawning_distance(), 12);
    }

    #[test]
    fn spawning_distance_inactive() {
        let config = WorldConfig {
            active: false,
            ..Default::default()
        };
        assert_eq!(config.spawning_distance(), 0);
    }

    #[test]
    fn min_despawn_distance_is_two() {
        assert_eq!(WorldConfig::default().min_despawn_distance(), 2);
    }

    #[test]
    fn voxel_texture_returns_path_when_layers_set() {
        let config = WorldConfig {
            texture_layers: 5,
            ..Default::default()
        };
        assert_eq!(
            config.voxel_texture(),
            Some(("textures/blocks.png".to_string(), 5))
        );
    }

    #[test]
    fn voxel_texture_returns_none_when_no_layers() {
        let config = WorldConfig {
            texture_layers: 0,
            ..Default::default()
        };
        assert_eq!(config.voxel_texture(), None);
    }

    #[test]
    fn texture_index_mapper_maps_known_materials() {
        let mapper = WorldConfig::default().texture_index_mapper();
        assert_eq!(mapper(0u8), [0, 0, 0]); // unknown → default
        assert_eq!(mapper(1u8), [1, 2, 3]); // surface material (different top/side/bottom)
        assert_eq!(mapper(7u8), [9, 9, 9]); // smelter_core
        assert_eq!(mapper(17u8), [19, 19, 19]); // logistics_io
        assert_eq!(mapper(18u8), [0, 0, 0]); // out of defined range
    }

    #[test]
    fn texture_index_mapper_defined_materials_have_nonzero_index() {
        let mapper = WorldConfig::default().texture_index_mapper();
        for mat in 1u8..=17 {
            let [a, _, _] = mapper(mat);
            assert_ne!(a, 0, "mat {mat} should have a non-zero texture index");
        }
    }

    #[test]
    fn make_voxel_fn_bedrock_below_y1() {
        let mut f = make_voxel_fn(0, None);
        assert_eq!(f(IVec3::new(0, 0, 0), None), WorldVoxel::Solid(0));
        assert_eq!(f(IVec3::new(5, -3, 5), None), WorldVoxel::Solid(0));
    }

    #[test]
    fn make_voxel_fn_high_altitude_is_air() {
        let mut f = make_voxel_fn(0, None);
        // noise * 50 < 100; y=1000 is always above surface
        assert_eq!(f(IVec3::new(0, 1000, 0), None), WorldVoxel::Air);
    }

    #[test]
    fn make_voxel_fn_underground_with_registry_uses_ore_lookup() {
        use crate::content::{BiomeDef, LayerDef, OreSpec, VeinDef, VeinRegistry};
        use std::sync::Arc;

        let vein = VeinDef {
            id: "iron_vein".to_string(),
            density: 1.0,
            primary: OreSpec {
                name: "Iron".to_string(),
                material: 3,
                weight: 100,
            },
            secondary: OreSpec {
                name: "Stone".to_string(),
                material: 0,
                weight: 0,
            },
            sporadic: None,
        };
        let layer = LayerDef {
            id: "deep".to_string(),
            name: "Deep".to_string(),
            y_cell_range: (-100, 100),
        };
        let biome = BiomeDef {
            id: "deep_biome".to_string(),
            layer: "deep".to_string(),
            vein_pool: vec![("iron_vein".to_string(), 1)],
        };
        let reg = Arc::new(VeinRegistry::new(vec![vein], vec![layer], vec![biome]));
        let mut f = make_voxel_fn(0, Some(reg));

        // Scan many positions at y=2 (y >= 1, not bedrock).
        // When surface > 3 the position is deep underground → the registry
        // `and_then(|r| r.ore_at(...))` closure executes.
        // Solid(m) with m != 1 proves we hit that branch (Solid(1) is surface layer).
        let hit = (0i32..1000).any(
            |x| matches!(f(IVec3::new(x * 7, 2, x * 3), None), WorldVoxel::Solid(m) if m != 1),
        );
        assert!(hit, "no deep-underground solid found in 1000 positions");
    }

    #[test]
    fn voxel_lookup_delegate_returns_callable_fn() {
        let config = WorldConfig {
            world_seed: 42,
            ..Default::default()
        };
        let delegate = config.voxel_lookup_delegate();
        let mut voxel_fn = delegate(IVec3::ZERO, 0, None);
        assert_eq!(voxel_fn(IVec3::new(0, 0, 0), None), WorldVoxel::Solid(0));
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

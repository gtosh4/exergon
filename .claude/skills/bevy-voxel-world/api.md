# bevy_voxel_world 0.15 — API Reference

## Contents
- [VoxelWorldPlugin](#voxelworldplugin)
- [VoxelWorldConfig trait](#voxelworldconfig-trait)
- [VoxelWorld system param](#voxelworld-system-param)
- [WorldVoxel enum](#worldvoxel-enum)
- [VoxelRaycastResult](#voxelraycastresult)
- [VoxelWorldCamera](#voxelworldcamera)
- [Chunk events](#chunk-events)
- [Constants and helpers](#constants-and-helpers)

---

## VoxelWorldPlugin

```rust
VoxelWorldPlugin::with_config(config: C) -> VoxelWorldPlugin<C>
VoxelWorldPlugin::<C>::minimal()            // no mesh spawning (for tests)
VoxelWorldPlugin::default()                 // DefaultWorld config
plugin.with_material(mat: M) -> VoxelWorldPlugin<C, M>  // custom material
```

Registers all internal systems in `PreUpdate`. Inserts `config` as a `Resource`.

---

## VoxelWorldConfig trait

Must implement for your config type: `Resource + Default + Clone`.

### Required associated types

```rust
type MaterialIndex: Copy + Hash + PartialEq + Eq + Default + Send + Sync;
// Usually u8. Stored per voxel — keep small.

type ChunkUserBundle: Bundle + Clone;
// Extra components added to chunk entities during meshing.
// Use () if not needed.
```

### Key methods (all have defaults)

```rust
fn spawning_distance(&self) -> u32  // chunks from camera to spawn; default 10
                                    // return 0 to disable world gen entirely

fn min_despawn_distance(&self) -> u32  // always-loaded radius; default 1

fn chunk_despawn_strategy(&self) -> ChunkDespawnStrategy
// ChunkDespawnStrategy::FarAwayOrOutOfView (default) | FarAway

fn chunk_spawn_strategy(&self) -> ChunkSpawnStrategy
// ChunkSpawnStrategy::CloseAndInView (default) | Close

fn max_spawn_per_frame(&self) -> usize  // default 10000
fn spawning_rays(&self) -> usize        // default 100
fn spawning_ray_margin(&self) -> u32    // default 25
```

### Texture methods

```rust
fn voxel_texture(&self) -> Option<(String, u32)>
// ("textures/blocks.png", layer_count)
// Path is relative to assets/. None = built-in 4-color texture.
// File must exist at startup (loaded synchronously by asset server).

fn texture_index_mapper(&self) -> TextureIndexMapperFn<Self::MaterialIndex>
// Arc<dyn Fn(MaterialIndex) -> [u32; 3]>
// Returns [top_layer, sides_layer, bottom_layer] indices into the array texture.
// Called per-voxel during meshing.
```

Example:
```rust
fn texture_index_mapper(&self) -> Arc<dyn Fn(u8) -> [u32; 3] + Send + Sync> {
    Arc::new(|mat| match mat {
        1 => [1, 2, 3],   // top=1, sides=2, bottom=3
        2 => [4, 4, 4],
        _ => [0, 0, 0],   // fallback
    })
}
```

### World generation

```rust
fn voxel_lookup_delegate(&self) -> VoxelLookupDelegate<Self::MaterialIndex>
// Box<dyn Fn(IVec3, LodLevel, Option<ChunkData<I>>) -> VoxelLookupFn<I>>
// Called per-chunk on a background thread.
// Returns a per-voxel function: FnMut(IVec3, Option<WorldVoxel<I>>) -> WorldVoxel<I>
// The Option<WorldVoxel> arg is the previously generated voxel (for incremental updates).
```

### Material init control

```rust
fn init_custom_materials(&self) -> bool  // default true
// Return false to defer until texture is ready; manually insert VoxelWorldMaterialHandle.
```

### LOD (optional)

```rust
fn chunk_lod(&self, chunk_position: IVec3, previous_lod: Option<LodLevel>, camera_position: Vec3) -> LodLevel
// Default 0 for all chunks.

fn chunk_data_shape(&self, lod_level: LodLevel) -> UVec3   // default 34³ (32+2 padding)
fn chunk_meshing_shape(&self, lod_level: LodLevel) -> UVec3
fn chunk_regenerate_strategy(&self) -> ChunkRegenerateStrategy
// ChunkRegenerateStrategy::Reuse (default) | Repopulate
```

### Other

```rust
fn attach_chunks_to_root(&self) -> bool  // default true; false = better perf for large worlds
fn max_active_chunk_threads(&self) -> usize  // default usize::MAX
fn debug_draw_chunks(&self) -> bool  // default false
fn init_root(&self, commands: Commands, root: Entity)  // customize world root entity
```

---

## VoxelWorld system param

```rust
// In system signature:
voxel_world: VoxelWorld<WorldConfig>       // read-only
mut voxel_world: VoxelWorld<WorldConfig>   // needed for set_voxel
// Or as non-system fn arg: &VoxelWorld<'_, WorldConfig>
```

### Methods

```rust
// Read a single voxel. Returns WorldVoxel::Unset if chunk not loaded.
fn get_voxel(&self, position: IVec3) -> WorldVoxel<C::MaterialIndex>

// Write a voxel. Buffered — applied in PreUpdate next frame.
// Creates chunk if none exists at that position.
fn set_voxel(&mut self, position: IVec3, voxel: WorldVoxel<C::MaterialIndex>)

// Sendable closure for use in async tasks / parallel queries.
fn get_voxel_fn(&self) -> Arc<dyn Fn(IVec3) -> WorldVoxel<C::MaterialIndex> + Send + Sync>

// Chunk data (if loaded). chunk_pos in chunk units (floor(voxel_pos / 32)).
fn get_chunk_data(&self, chunk_pos: IVec3) -> Option<ChunkData<C::MaterialIndex>>
fn get_chunk_data_fn(&self) -> Arc<dyn Fn(IVec3) -> Option<ChunkData<C::MaterialIndex>> + Send + Sync>

// Raycast — see VoxelRaycastResult below.
fn raycast(
    &self,
    ray: Ray3d,
    filter: &impl Fn((Vec3, WorldVoxel<C::MaterialIndex>)) -> bool,
) -> Option<VoxelRaycastResult<C::MaterialIndex>>

fn raycast_fn(&self) -> Arc<RaycastFn<C::MaterialIndex>>
// Sendable raycast closure.
```

---

## WorldVoxel enum

```rust
pub enum WorldVoxel<I = u8> {
    Unset,      // Chunk not generated yet — treat as opaque for collision, transparent for render
    Air,        // Explicitly empty
    Solid(I),   // Solid block; I = material index
}

// Methods:
fn is_unset(&self) -> bool
fn is_air(&self) -> bool
fn is_solid(&self) -> bool
```

**Never set a voxel to `Unset`** — use `Air` to clear a block.

---

## VoxelRaycastResult

```rust
pub struct VoxelRaycastResult<I = u8> {
    pub position: Vec3,           // world-space float position of hit voxel
    pub normal: Option<Vec3>,     // face normal (None for VoxelFace::None)
    pub voxel: WorldVoxel<I>,
}

fn voxel_pos(&self) -> IVec3        // position.floor().as_ivec3()
fn voxel_normal(&self) -> Option<IVec3>  // normal.floor().as_ivec3()
```

Typical use:
```rust
let hit = voxel_world.raycast(ray, &|(_pos, voxel)| matches!(voxel, WorldVoxel::Solid(_)));
if let Some(hit) = hit {
    let pos: IVec3 = hit.voxel_pos();
    let normal: IVec3 = hit.voxel_normal().unwrap_or(IVec3::Y);
    // place at pos + normal, break at pos
}
```

---

## VoxelWorldCamera

```rust
#[derive(Component)]
pub struct VoxelWorldCamera<C> { .. }
impl<C> Default for VoxelWorldCamera<C> { .. }
```

Mark exactly one camera entity. Controls which chunks spawn/despawn.

```rust
commands.spawn((
    Camera3d::default(),
    VoxelWorldCamera::<WorldConfig>::default(),
));
```

---

## Chunk events

All are `Message` (Bevy 0.18 messaging). Read with `MessageReader`.

```rust
ChunkWillSpawn<C>    // chunk about to be added
ChunkWillDespawn<C>  // chunk about to be removed
ChunkWillRemesh<C>   // chunk about to regenerate mesh
ChunkWillUpdate<C>   // voxel in chunk was set via set_voxel
ChunkWillChangeLod<C>

// All have fields:
pub chunk_key: IVec3   // chunk position in chunk units
pub entity: Entity
```

System example:
```rust
fn on_chunk_spawn(mut evr: MessageReader<ChunkWillSpawn<WorldConfig>>) {
    for ev in evr.read() {
        let chunk_key: IVec3 = ev.chunk_key;
    }
}
```

---

## Constants and helpers

```rust
VOXEL_SIZE: f32 = 1.0

// From bevy_voxel_world::custom_meshing (not prelude):
CHUNK_SIZE_U: u32 = 32
CHUNK_SIZE_I: i32 = 32
CHUNK_SIZE_F: f32 = 32.0

// Helper (in prelude):
fn get_chunk_voxel_position(position: IVec3) -> (IVec3, UVec3)
// Returns (chunk_pos, local_voxel_pos_with_padding)
// chunk_pos = floor(voxel_pos / 32) in chunk units
```

## VoxelFace enum

```rust
pub enum VoxelFace { None, Bottom, Top, Left, Right, Back, Forward }
// Implements TryFrom<VoxelFace> for Vec3
// None -> Err(()), others -> unit Vec3 (e.g. Top -> Vec3::Y)
```

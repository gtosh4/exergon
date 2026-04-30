# bevy_voxel_world 0.15 — Patterns

## Contents
- [Plugin setup](#plugin-setup)
- [Config struct](#config-struct)
- [World generation](#world-generation)
- [Texture atlas setup](#texture-atlas-setup)
- [Late texture init (deferred material)](#late-texture-init)
- [Raycast to hit block](#raycast-to-hit-block)
- [Get and set voxels](#get-and-set-voxels)
- [Collision / blocked check](#collision--blocked-check)
- [Disabling world gen until ready](#disabling-world-gen-until-ready)
- [Chunk events](#chunk-events)

---

## Plugin setup

Call in `Plugin::build()`. Config is a `Resource` inserted by the plugin, so read other resources first if needed.

```rust
impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // Read pre-inserted resources before building config
        let texture_layers = app
            .world()
            .get_resource::<BlockAtlasLayers>()
            .map(|r| r.0)
            .unwrap_or(0);

        app.add_plugins(VoxelWorldPlugin::with_config(WorldConfig {
            texture_layers,
            ..Default::default()
        }));
    }
}
```

---

## Config struct

```rust
#[derive(Resource, Clone, Default)]
pub struct WorldConfig {
    pub world_seed: u64,
    pub active: bool,        // used to gate spawning_distance
    pub texture_layers: u32,
}

impl VoxelWorldConfig for WorldConfig {
    type MaterialIndex = u8;
    type ChunkUserBundle = ();

    fn spawning_distance(&self) -> u32 {
        if self.active { 12 } else { 0 }
    }

    fn voxel_texture(&self) -> Option<(String, u32)> {
        if self.texture_layers > 0 {
            Some(("textures/blocks.png".into(), self.texture_layers))
        } else {
            None
        }
    }

    fn texture_index_mapper(&self) -> Arc<dyn Fn(u8) -> [u32; 3] + Send + Sync> {
        Arc::new(|mat| match mat {
            1 => [1, 2, 3],   // top=1, sides=2, bottom=3
            2 => [4, 4, 4],
            _ => [0, 0, 0],
        })
    }

    fn voxel_lookup_delegate(&self) -> VoxelLookupDelegate<u8> {
        let seed = self.world_seed;
        Box::new(move |_chunk_pos, _lod, _previous| make_voxel_fn(seed))
    }
}
```

---

## World generation

`voxel_lookup_delegate` runs on a **background thread**. No Bevy resources accessible.

```rust
fn make_voxel_fn(seed: u64) -> Box<dyn FnMut(IVec3, Option<WorldVoxel>) -> WorldVoxel + Send + Sync> {
    // Create noise / state here — captured by the closure.
    let mut noise = HybridMulti::<Perlin>::new((seed ^ (seed >> 32)) as u32);
    let mut cache = HashMap::<(i32, i32), f64>::new();

    Box::new(move |pos: IVec3, _previous| {
        if pos.y < 1 {
            return WorldVoxel::Solid(0); // bedrock
        }
        let surface = *cache.entry((pos.x, pos.z)).or_insert_with(|| {
            noise.get([pos.x as f64 / 1000.0, pos.z as f64 / 1000.0]) * 50.0
        });
        let y = pos.y as f64;
        if y >= surface {
            WorldVoxel::Air
        } else if y >= surface - 1.0 {
            WorldVoxel::Solid(1) // surface material
        } else {
            WorldVoxel::Solid(2) // subsurface
        }
    })
}
```

Rules:
- Return `WorldVoxel::Air` for empty space (not `Unset`).
- Closure can be `FnMut` — use local mutable state for caches.
- Data needed from Bevy must be cloned/`Arc`-wrapped before the closure.

---

## Texture atlas setup

bevy_voxel_world needs a **stacked PNG** (each layer is one tile, stacked vertically). This must exist at startup.

Build the atlas before `App::run()`:

```rust
fn main() {
    let layers = build_block_atlas(); // writes assets/textures/blocks.png
    App::new()
        .insert_resource(BlockAtlasLayers(layers))
        .add_plugins((DefaultPlugins, WorldPlugin))
        .run();
}
```

`build_block_atlas` reads a manifest (`manifest.ron`: `["dirt", "grass_top", ...]`), loads each PNG, stacks them, writes the atlas PNG, returns layer count.

In `VoxelWorldConfig`:
```rust
fn voxel_texture(&self) -> Option<(String, u32)> {
    Some(("textures/blocks.png".into(), self.texture_layers))
}
```

**Atlas texture is loaded as `D2Array`** — incompatible with `StandardMaterial` (which expects `D2`). Do NOT bind the atlas handle to a `StandardMaterial`. For ghost/preview meshes, use a plain tinted `StandardMaterial` with no texture.

---

## Late texture init

When texture needs to load before material can be initialized (e.g., you need image dimensions):

```rust
impl VoxelWorldConfig for WorldConfig {
    fn init_custom_materials(&self) -> bool {
        false  // suppress auto-init
    }
}

// Later, once texture is loaded, insert manually:
commands.insert_resource(VoxelWorldMaterialHandle { handle: mat_handle });
```

---

## Raycast to hit block

```rust
fn update_look_target(
    camera_q: Query<&Transform, With<MainCamera>>,
    voxel_world: VoxelWorld<WorldConfig>,
    mut look_target: ResMut<LookTarget>,
) {
    let Ok(cam) = camera_q.single() else { return; };
    let ray = Ray3d::new(cam.translation, cam.forward());

    let hit = voxel_world
        .raycast(ray, &|(_pos, voxel)| matches!(voxel, WorldVoxel::Solid(_)))
        .filter(|h| h.position.distance(cam.translation) <= MAX_REACH);

    *look_target = match hit {
        None => LookTarget::Nothing,
        Some(h) => match h.voxel {
            WorldVoxel::Solid(mat) => LookTarget::Voxel {
                pos: h.voxel_pos(),          // IVec3
                normal: h.voxel_normal().unwrap_or(IVec3::Y), // IVec3
                material: mat,
            },
            _ => LookTarget::Nothing,
        },
    };
}
```

---

## Get and set voxels

```rust
// Read (no mut needed):
let voxel = voxel_world.get_voxel(IVec3::new(x, y, z));
if voxel.is_solid() { ... }

// Write (needs mut):
voxel_world.set_voxel(pos, WorldVoxel::Air);          // break block
voxel_world.set_voxel(pos + normal, WorldVoxel::Solid(mat_index)); // place block
```

`set_voxel` is buffered → applied next frame in `PreUpdate`. Reading immediately after in the same frame returns the old value (use `get_voxel_fn()` for reads-after-write in async contexts).

---

## Collision / blocked check

Sample 8 corners of an AABB:

```rust
pub fn is_blocked(voxel_world: &VoxelWorld<'_, WorldConfig>, center: Vec3, r: f32) -> bool {
    let offsets = [
        Vec3::new(-r, -r, -r), Vec3::new(-r, -r,  r),
        Vec3::new(-r,  r, -r), Vec3::new(-r,  r,  r),
        Vec3::new( r, -r, -r), Vec3::new( r, -r,  r),
        Vec3::new( r,  r, -r), Vec3::new( r,  r,  r),
    ];
    offsets.iter().any(|&o| {
        matches!(
            voxel_world.get_voxel((center + o).floor().as_ivec3()),
            WorldVoxel::Solid(_)
        )
    })
}

// In system, pass by reference:
if !is_blocked(&voxel_world, pos + delta, 0.35) {
    transform.translation += delta;
}
```

---

## Disabling world gen until ready

Return `spawning_distance() = 0` until the game is ready, then flip a flag on the config resource:

```rust
fn finish_loading(mut world_config: ResMut<WorldConfig>, mut next_state: ResMut<NextState<GameState>>) {
    world_config.active = true;          // spawning_distance now returns 12
    world_config.world_seed = seeds.world;
    next_state.set(GameState::Playing);
}
```

`WorldConfig` is a `Resource` — mutate it from any system.

---

## Chunk events

```rust
fn on_chunk_spawn(mut evr: MessageReader<ChunkWillSpawn<WorldConfig>>) {
    for ev in evr.read() {
        let chunk_world_pos = ev.chunk_key * 32; // chunk_key is in chunk units
        let entity = ev.entity;
    }
}

// Register like any message reader system:
app.add_systems(Update, on_chunk_spawn);
```

Available events: `ChunkWillSpawn`, `ChunkWillDespawn`, `ChunkWillRemesh`, `ChunkWillUpdate`, `ChunkWillChangeLod` — all generic over `C: VoxelWorldConfig`.

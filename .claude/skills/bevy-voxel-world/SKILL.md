---
name: bevy-voxel-world
description: >
  bevy_voxel_world 0.15 API reference and usage patterns for Exergon. Use when writing
  code that reads/writes voxels, implements world generation, raycasts into the voxel
  world, handles chunk events, or configures VoxelWorldPlugin. Triggers on: voxel world,
  VoxelWorld, WorldVoxel, set_voxel, get_voxel, raycast, VoxelWorldConfig, chunk spawn,
  texture_index_mapper, voxel_lookup_delegate.
---

# bevy_voxel_world 0.15

Crate: `bevy_voxel_world = "0.15"`. Import: `use bevy_voxel_world::prelude::*;`

Chunk size: **32 voxels**. 1 voxel = 1 meter. All coordinates are world-space `IVec3`.

## Quick reference

| What | Type |
|------|------|
| Config trait | `VoxelWorldConfig` |
| Plugin setup | `VoxelWorldPlugin::with_config(config)` |
| System param | `VoxelWorld<C>` / `mut VoxelWorld<C>` |
| Voxel value | `WorldVoxel::Unset / Air / Solid(mat_index)` |
| Camera marker | `VoxelWorldCamera::<C>::default()` |
| Raycast result | `VoxelRaycastResult { position, normal, voxel }` |

## Key files

**[api.md](api.md)** — VoxelWorldConfig methods, VoxelWorld methods, types, chunk events, constants

**[patterns.md](patterns.md)** — Setup, world generation, raycast, texture atlas, disabling gen, collision

## Critical gotchas

- `WorldVoxel::Unset` ≠ `Air`. Unset = chunk not yet generated. Never set a voxel to `Unset`.
- `set_voxel` writes to a buffer flushed in `PreUpdate`. Changes visible next frame.
- Atlas texture loads as `D2Array`. Can't bind to `StandardMaterial` (which expects `D2`). Use tinted `StandardMaterial` for ghost/preview meshes.
- `voxel_texture()` path loaded by the asset server — file must exist at `assets/` relative path at startup.
- Config struct must be `Resource + Default + Clone`. Plugin inserts it as a resource, so call `VoxelWorldPlugin::with_config(config)` in `Plugin::build()` where you can read other resources first.
- `spawning_distance()` returning `0` = world gen disabled (useful before the game starts).
- `VoxelWorld<C>` requires `&self` for reads, `&mut self` for `set_voxel`. In systems, declare `mut voxel_world: VoxelWorld<C>`.

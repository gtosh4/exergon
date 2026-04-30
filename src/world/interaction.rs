use bevy::ecs::message::MessageReader;
use bevy::input::mouse::MouseWheel;
use bevy::math::{Affine2, Mat2};
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;

use crate::textures::BlockAtlasLayers;

use super::generation::WorldConfig;
use super::player::MainCamera;

const MAX_REACH: f32 = 8.0;

#[derive(Component)]
pub struct GhostBlock;

#[derive(Resource)]
pub struct GhostPreview {
    entity: Entity,
}

#[derive(Resource)]
pub struct GhostMaterials(Vec<Handle<StandardMaterial>>);

#[derive(Resource, Clone, Copy, Default)]
pub enum LookTarget {
    #[default]
    Nothing,
    Voxel {
        material: u8,
        pos: IVec3,
        normal: IVec3,
    },
}

#[derive(Resource, Default)]
pub struct SelectedMaterial(pub u8);

// Mirrors texture_index_mapper in generation.rs — side face atlas index per material ID
fn side_atlas_index(mat: u8) -> usize {
    match mat {
        1 => 2,
        2 => 4,
        3 => 5,
        4 => 6,
        5 => 7,
        6 => 8,
        _ => 0,
    }
}

pub(super) fn setup_ghost_preview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    atlas_layers: Option<Res<BlockAtlasLayers>>,
) {
    let n = atlas_layers.map(|r| r.0).unwrap_or(1) as f32;
    let texture: Handle<Image> = asset_server.load("textures/blocks.png");

    // One semi-transparent material per block type (mat IDs 1..=6)
    let ghost_mats: Vec<Handle<StandardMaterial>> = (1u8..=6)
        .map(|mat| {
            let atlas_idx = side_atlas_index(mat) as f32;
            materials.add(StandardMaterial {
                base_color_texture: Some(texture.clone()),
                base_color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                // Slice the atlas: map V in [0,1] → [atlas_idx/n, (atlas_idx+1)/n]
                uv_transform: Affine2::from_mat2_translation(
                    Mat2::from_diagonal(Vec2::new(1.0, 1.0 / n)),
                    Vec2::new(0.0, atlas_idx / n),
                ),
                ..default()
            })
        })
        .collect();

    let initial_mat = ghost_mats[0].clone();

    let entity = commands
        .spawn((
            GhostBlock,
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(initial_mat),
            Transform::IDENTITY,
            Visibility::Hidden,
        ))
        .id();

    commands.insert_resource(GhostPreview { entity });
    commands.insert_resource(GhostMaterials(ghost_mats));
}

pub(super) fn update_ghost_preview(
    look_target: Res<LookTarget>,
    selected: Res<SelectedMaterial>,
    ghost: Option<Res<GhostPreview>>,
    ghost_mats: Option<Res<GhostMaterials>>,
    mut ghost_q: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<GhostBlock>,
    >,
) {
    let Some(ghost) = ghost else { return };
    let Ok((mut transform, mut vis, mut mat)) = ghost_q.get_mut(ghost.entity) else {
        return;
    };

    let show = selected.0 != 0;

    if selected.is_changed() && show {
        if let Some(mats) = &ghost_mats {
            let idx = (selected.0 as usize)
                .saturating_sub(1)
                .min(mats.0.len().saturating_sub(1));
            *mat = MeshMaterial3d(mats.0[idx].clone());
        }
    }

    match *look_target {
        LookTarget::Voxel { pos, normal, .. } if show => {
            transform.translation = (pos + normal).as_vec3() + Vec3::splat(0.5);
            *vis = Visibility::Visible;
        }
        _ => {
            *vis = Visibility::Hidden;
        }
    }
}

pub(super) fn hide_ghost_preview(
    ghost: Option<Res<GhostPreview>>,
    mut ghost_q: Query<&mut Visibility, With<GhostBlock>>,
) {
    let Some(ghost) = ghost else { return };
    if let Ok(mut vis) = ghost_q.get_mut(ghost.entity) {
        *vis = Visibility::Hidden;
    }
}

pub(super) fn update_look_target(
    camera_q: Query<&Transform, With<MainCamera>>,
    voxel_world: VoxelWorld<WorldConfig>,
    mut look_target: ResMut<LookTarget>,
) {
    let Ok(cam) = camera_q.single() else {
        *look_target = LookTarget::Nothing;
        return;
    };

    let ray = Ray3d::new(cam.translation, cam.forward());
    let hit = voxel_world
        .raycast(ray, &|(_pos, voxel)| matches!(voxel, WorldVoxel::Solid(_)))
        .filter(|hit| hit.position.distance(cam.translation) <= MAX_REACH);

    *look_target = match hit {
        None => LookTarget::Nothing,
        Some(hit) => match hit.voxel {
            WorldVoxel::Solid(mat) => LookTarget::Voxel {
                material: mat,
                pos: hit.voxel_pos(),
                normal: hit.voxel_normal().unwrap_or(IVec3::Y),
            },
            _ => LookTarget::Nothing,
        },
    };
}

pub(super) fn block_interaction(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut scroll: MessageReader<MouseWheel>,
    look_target: Res<LookTarget>,
    mut selected: ResMut<SelectedMaterial>,
    mut voxel_world: VoxelWorld<WorldConfig>,
) {
    for ev in scroll.read() {
        let delta = if ev.y > 0.0 { 1i8 } else { -1i8 };
        selected.0 = (selected.0 as i8 - 1 + delta).rem_euclid(6) as u8 + 1;
    }

    let LookTarget::Voxel { pos, normal, .. } = *look_target else {
        return;
    };

    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if mouse.just_pressed(MouseButton::Left) {
        if shift {
            voxel_world.set_voxel(pos, WorldVoxel::Air);
        } else {
            voxel_world.set_voxel(pos + normal, WorldVoxel::Solid(selected.0));
        }
    }
}

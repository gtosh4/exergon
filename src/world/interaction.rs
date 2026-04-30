use bevy::ecs::message::MessageReader;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;

use crate::inventory::{Hotbar, Inventory, InventoryOpen, ItemRegistry};

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

#[derive(Resource, Default)]
pub enum LookTarget {
    #[default]
    Nothing,
    Voxel {
        material: u8,
        pos: IVec3,
        normal: IVec3,
    },
}

pub(super) fn setup_ghost_preview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Distinct tint per material slot so placed-block type is visually distinguishable.
    // Do NOT use the atlas texture — bevy_voxel_world loads it as D2Array which is
    // incompatible with the D2 binding expected by StandardMaterial.
    let tints = [
        Color::srgba(0.75, 0.75, 0.75, 0.5), // 1
        Color::srgba(0.35, 0.75, 0.35, 0.5), // 2
        Color::srgba(0.80, 0.55, 0.25, 0.5), // 3
        Color::srgba(0.30, 0.55, 0.90, 0.5), // 4
        Color::srgba(0.90, 0.85, 0.25, 0.5), // 5
        Color::srgba(0.80, 0.30, 0.80, 0.5), // 6
    ];

    let ghost_mats: Vec<Handle<StandardMaterial>> = tints
        .iter()
        .map(|&color| {
            materials.add(StandardMaterial {
                base_color: color,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
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
    hotbar: Res<Hotbar>,
    item_registry: Option<Res<ItemRegistry>>,
    inventory_open: Option<Res<InventoryOpen>>,
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

    let inv_open = inventory_open.map(|o| o.0).unwrap_or(false);
    let active_voxel = hotbar
        .active_item_id()
        .and_then(|id| item_registry.as_ref().and_then(|r| r.voxel_id(id)));
    let show = active_voxel.is_some() && !inv_open;

    if hotbar.is_changed() {
        if let (Some(m), Some(mats)) = (active_voxel, &ghost_mats) {
            let idx = (m as usize).saturating_sub(1).min(mats.0.len().saturating_sub(1));
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
    mut hotbar: ResMut<Hotbar>,
    mut inventory: ResMut<Inventory>,
    item_registry: Option<Res<ItemRegistry>>,
    mut voxel_world: VoxelWorld<WorldConfig>,
) {
    // Scroll wheel or number keys to select hotbar slot
    for ev in scroll.read() {
        let delta = if ev.y > 0.0 { 1i32 } else { -1i32 };
        hotbar.selected = (hotbar.selected as i32 + delta).rem_euclid(9) as usize;
    }
    for (i, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .iter()
    .enumerate()
    {
        if keyboard.just_pressed(*key) {
            hotbar.selected = i;
        }
    }

    let LookTarget::Voxel {
        pos,
        normal,
        material: hit_voxel,
        ..
    } = *look_target
    else {
        return;
    };

    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if mouse.just_pressed(MouseButton::Left) {
        if shift {
            voxel_world.set_voxel(pos, WorldVoxel::Air);
            if let Some(item) = item_registry.as_ref().and_then(|r| r.item_for_voxel(hit_voxel)) {
                inventory.add(item.id.clone(), 1);
            }
        } else if let Some(voxel_id) = hotbar
            .active_item_id()
            .and_then(|id| item_registry.as_ref().and_then(|r| r.voxel_id(id)))
        {
            hotbar.consume_active();
            voxel_world.set_voxel(pos + normal, WorldVoxel::Solid(voxel_id));
        }
    }
}

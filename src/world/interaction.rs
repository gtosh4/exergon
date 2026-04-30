use bevy::ecs::message::MessageReader;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;

use super::generation::WorldConfig;
use super::player::MainCamera;

const MAX_REACH: f32 = 8.0;

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

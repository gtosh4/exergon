use avian3d::prelude::SpatialQuery;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::inventory::{Hotbar, Inventory, InventoryOpen};

use super::player::MainCamera;
use super::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

const MAX_REACH: f32 = 8.0;

#[derive(Component)]
pub struct PlacementGhost;

/// Holds the first-selected IO port position when the player is mid-way through
/// a two-click cable connection.
#[derive(Resource, Default)]
pub struct PendingCablePort {
    pub pos: Option<Vec3>,
    pub item_id: Option<String>,
}

#[derive(Resource)]
pub struct GhostPreview {
    entity: Entity,
}

#[derive(Resource, Default)]
pub enum LookTarget {
    #[default]
    Nothing,
    Surface {
        pos: Vec3,
        normal: Vec3,
    },
}

pub(super) fn setup_ghost_preview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let entity = commands
        .spawn((
            PlacementGhost,
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.75, 0.75, 0.75, 0.5),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
            Transform::IDENTITY,
            Visibility::Hidden,
        ))
        .id();

    commands.insert_resource(GhostPreview { entity });
    commands.init_resource::<PendingCablePort>();
}

pub(super) fn update_ghost_preview(
    look_target: Res<LookTarget>,
    hotbar: Res<Hotbar>,
    inventory_open: Option<Res<InventoryOpen>>,
    ghost: Option<Res<GhostPreview>>,
    mut ghost_q: Query<(&mut Transform, &mut Visibility), With<PlacementGhost>>,
) {
    let Some(ghost) = ghost else { return };
    let Ok((mut transform, mut vis)) = ghost_q.get_mut(ghost.entity) else {
        return;
    };

    let inv_open = inventory_open.is_some_and(|o| o.0);
    let has_item = hotbar.active_item_id().is_some();
    let show_ghost = has_item && !inv_open;

    match *look_target {
        LookTarget::Surface { pos, normal } if show_ghost => {
            transform.translation = pos + normal * 0.5;
            *vis = Visibility::Visible;
        }
        _ => {
            *vis = Visibility::Hidden;
        }
    }
}

pub(super) fn hide_ghost_preview(
    ghost: Option<Res<GhostPreview>>,
    mut ghost_q: Query<&mut Visibility, With<PlacementGhost>>,
) {
    let Some(ghost) = ghost else { return };
    if let Ok(mut vis) = ghost_q.get_mut(ghost.entity) {
        *vis = Visibility::Hidden;
    }
}

pub(super) fn update_look_target(
    camera_q: Query<&Transform, With<MainCamera>>,
    spatial_query: SpatialQuery,
    mut look_target: ResMut<LookTarget>,
) {
    let Ok(cam) = camera_q.single() else {
        *look_target = LookTarget::Nothing;
        return;
    };

    let dir = Dir3::new(*cam.forward()).unwrap_or(Dir3::NEG_Z);
    let hit = spatial_query.cast_ray(cam.translation, dir, MAX_REACH, true, &Default::default());

    *look_target = match hit {
        None => LookTarget::Nothing,
        Some(h) => LookTarget::Surface {
            pos: cam.translation + *dir * h.distance,
            normal: h.normal,
        },
    };
}

pub(super) fn object_interaction(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut scroll: MessageReader<MouseWheel>,
    look_target: Res<LookTarget>,
    mut hotbar: ResMut<Hotbar>,
    mut inventory: ResMut<Inventory>,
    mut world_events: MessageWriter<WorldObjectEvent>,
    mut cable_events: MessageWriter<CableConnectionEvent>,
    mut pending_cable: ResMut<PendingCablePort>,
) {
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

    let LookTarget::Surface { pos, normal } = *look_target else {
        return;
    };

    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if mouse.just_pressed(MouseButton::Left) {
        if shift {
            world_events.write(WorldObjectEvent {
                pos,
                item_id: String::new(),
                kind: WorldObjectKind::Removed,
            });
            pending_cable.pos = None;
            pending_cable.item_id = None;
        } else if let Some(item_id) = hotbar.active_item_id().map(str::to_owned) {
            if item_id.ends_with("_cable") {
                let place_pos = pos;
                match pending_cable.pos {
                    Some(from)
                        if pending_cable.item_id.as_deref() == Some(&item_id)
                            && from.distance(place_pos) > 0.1 =>
                    {
                        hotbar.consume_active();
                        inventory.add(item_id.clone(), 0);
                        cable_events.write(CableConnectionEvent {
                            from,
                            to: place_pos,
                            item_id,
                            kind: WorldObjectKind::Placed,
                        });
                        pending_cable.pos = None;
                        pending_cable.item_id = None;
                    }
                    _ => {
                        pending_cable.pos = Some(place_pos);
                        pending_cable.item_id = Some(item_id);
                    }
                }
            } else {
                hotbar.consume_active();
                inventory.add(item_id.clone(), 0);
                world_events.write(WorldObjectEvent {
                    pos: pos + normal * 0.5,
                    item_id,
                    kind: WorldObjectKind::Placed,
                });
            }
        }
    }
}

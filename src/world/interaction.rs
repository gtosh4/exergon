use avian3d::prelude::{Collider, Sensor, SpatialQuery, SpatialQueryFilter};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::inventory::{Hotbar, Inventory, InventoryOpen};
use crate::machine::{GhostAssets, IoPortMarker, Machine, Platform};

use super::player::MainCamera;
use super::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

const MAX_REACH: f32 = 16.0;

fn placement_half_extent(item_id: &str) -> f32 {
    match item_id {
        "platform" => 0.125,
        _ => 2.0,
    }
}

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
    last_item_id: String,
}

#[derive(Resource, Default)]
pub enum LookTarget {
    #[default]
    Nothing,
    Surface {
        pos: Vec3,
        normal: Vec3,
        entity: Entity,
    },
}

/// Marker for the red removal-preview ghost entity.
#[derive(Component)]
pub struct RemovalGhost;

#[derive(Resource)]
pub struct RemovalGhostPreview {
    entity: Entity,
}

pub(super) fn setup_ghost_preview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let placeholder_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.65, 0.65, 0.65, 0.5),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let entity = commands
        .spawn((
            PlacementGhost,
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(placeholder_mat),
            Transform::IDENTITY,
            Visibility::Hidden,
        ))
        .id();

    commands.insert_resource(GhostPreview {
        entity,
        last_item_id: String::new(),
    });
    commands.init_resource::<PendingCablePort>();

    let removal_entity = commands
        .spawn((
            RemovalGhost,
            Mesh3d(meshes.add(Cuboid::new(1.1, 1.1, 1.1))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.15, 0.15, 0.5),
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
    commands.insert_resource(RemovalGhostPreview {
        entity: removal_entity,
    });
}

pub(super) fn update_ghost_preview(
    look_target: Res<LookTarget>,
    hotbar: Res<Hotbar>,
    inventory_open: Option<Res<InventoryOpen>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ghost: Option<ResMut<GhostPreview>>,
    mut ghost_q: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<PlacementGhost>,
    >,
    ghost_assets: Option<Res<GhostAssets>>,
) {
    let (Some(ghost), Some(ghost_assets)) = (ghost.as_mut(), ghost_assets) else {
        return;
    };
    let Ok((mut transform, mut vis, mut mesh, mut mat)) = ghost_q.get_mut(ghost.entity) else {
        return;
    };

    let inv_open = inventory_open.is_some_and(|o| o.0);
    let has_item = hotbar.active_item_id().is_some();
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let show_ghost = has_item && !inv_open && !shift;

    match *look_target {
        LookTarget::Surface { pos, normal, .. } if show_ghost => {
            let item_id = hotbar.active_item_id().unwrap_or("");
            if ghost.last_item_id != item_id {
                let (new_mesh, new_mat) = if item_id == "platform" {
                    (
                        ghost_assets.platform_mesh.clone(),
                        ghost_assets.platform_material.clone(),
                    )
                } else if let Some(mat) = ghost_assets.materials.get(item_id) {
                    (ghost_assets.machine_mesh.clone(), mat.clone())
                } else {
                    (
                        ghost_assets.fallback_mesh.clone(),
                        ghost_assets.fallback_material.clone(),
                    )
                };
                *mesh = Mesh3d(new_mesh);
                *mat = MeshMaterial3d(new_mat);
                ghost.last_item_id = item_id.to_string();
            }
            transform.translation = pos + normal * placement_half_extent(item_id);
            transform.scale = Vec3::ONE;
            *vis = Visibility::Visible;
        }
        _ => {
            *vis = Visibility::Hidden;
        }
    }
}

pub(super) fn hide_ghost_preview(
    ghost: Option<Res<GhostPreview>>,
    removal_ghost: Option<Res<RemovalGhostPreview>>,
    mut ghost_q: Query<&mut Visibility, With<PlacementGhost>>,
    mut removal_ghost_q: Query<&mut Visibility, (With<RemovalGhost>, Without<PlacementGhost>)>,
) {
    if let Some(ghost) = ghost
        && let Ok(mut vis) = ghost_q.get_mut(ghost.entity)
    {
        *vis = Visibility::Hidden;
    }
    if let Some(rg) = removal_ghost
        && let Ok(mut vis) = removal_ghost_q.get_mut(rg.entity)
    {
        *vis = Visibility::Hidden;
    }
}

pub(super) fn update_removal_ghost(
    look_target: Res<LookTarget>,
    keyboard: Res<ButtonInput<KeyCode>>,
    removal_ghost: Option<Res<RemovalGhostPreview>>,
    machine_q: Query<&Transform, (With<Machine>, Without<RemovalGhost>)>,
    platform_q: Query<&Transform, (With<Platform>, Without<RemovalGhost>)>,
    mut ghost_q: Query<(&mut Transform, &mut Visibility), With<RemovalGhost>>,
) {
    let Some(rg) = removal_ghost else { return };
    let Ok((mut transform, mut vis)) = ghost_q.get_mut(rg.entity) else {
        return;
    };

    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    let target_pos = if shift {
        if let LookTarget::Surface { entity, .. } = *look_target {
            machine_q
                .get(entity)
                .map(|t| t.translation)
                .or_else(|_| platform_q.get(entity).map(|t| t.translation))
                .ok()
        } else {
            None
        }
    } else {
        None
    };

    match target_pos {
        Some(pos) => {
            transform.translation = pos;
            *vis = Visibility::Visible;
        }
        None => {
            *vis = Visibility::Hidden;
        }
    }
}

pub(super) fn update_look_target(
    camera_q: Query<&Transform, With<MainCamera>>,
    spatial_query: SpatialQuery,
    mut look_target: ResMut<LookTarget>,
    port_q: Query<&GlobalTransform, With<IoPortMarker>>,
) {
    let Ok(cam) = camera_q.single() else {
        *look_target = LookTarget::Nothing;
        return;
    };

    let dir = Dir3::new(*cam.forward()).unwrap_or(Dir3::NEG_Z);
    let hit = spatial_query.cast_ray(cam.translation, dir, MAX_REACH, true, &Default::default());

    *look_target = match hit {
        None => LookTarget::Nothing,
        Some(h) => {
            let pos = if let Ok(gt) = port_q.get(h.entity) {
                gt.translation()
            } else {
                cam.translation + *dir * h.distance
            };
            LookTarget::Surface {
                pos,
                normal: h.normal,
                entity: h.entity,
            }
        }
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
    spatial_query: SpatialQuery,
    sensor_q: Query<(), With<Sensor>>,
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

    let LookTarget::Surface { pos, normal, .. } = *look_target else {
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
                let half = placement_half_extent(&item_id);
                let place_pos = pos + normal * half;
                let check_half = (half - 0.05).max(0.05);
                let check = Collider::cuboid(check_half, check_half, check_half);
                let blocked = spatial_query
                    .shape_intersections(
                        &check,
                        place_pos,
                        Quat::IDENTITY,
                        &SpatialQueryFilter::default(),
                    )
                    .iter()
                    .any(|&e| !sensor_q.contains(e));
                if !blocked {
                    hotbar.consume_active();
                    inventory.add(item_id.clone(), 0);
                    world_events.write(WorldObjectEvent {
                        pos: place_pos,
                        item_id,
                        kind: WorldObjectKind::Placed,
                    });
                }
            }
        }
    }
}

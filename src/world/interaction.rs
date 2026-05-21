use avian3d::prelude::{Collider, Sensor, SpatialQuery, SpatialQueryFilter};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::inventory::{Hotbar, InventoryOpen};
use crate::logistics::StorageUnit;
use bevy::ecs::system::SystemParam;

use crate::machine::{
    GhostAssets, IoPortMarker, Machine, OrientationSupport, PlaceableColliderCache,
    PlaceableRegistry, Platform, SnapRule, TileSnap,
};

use super::player::{MainCamera, Player};
use super::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

const MAX_REACH: f32 = 16.0;

/// Returns (ix_min, ix_max, iz_min, iz_max) as tile offsets relative to `anchor`.
fn platform_tile_range(anchor: Vec3, cursor: Vec3, step: f32) -> (i32, i32, i32, i32) {
    let n_x = ((cursor.x - anchor.x) / step).round() as i32;
    let n_z = ((cursor.z - anchor.z) / step).round() as i32;
    (n_x.min(0), n_x.max(0), n_z.min(0), n_z.max(0))
}

#[derive(Resource, Default)]
pub enum PendingPlacement {
    #[default]
    Idle,
    TwoEndpoint {
        item_id: String,
        anchor: Vec3,
    },
    AreaRect {
        item_id: String,
        corner_a: Vec3,
        tile_step: f32,
    },
}

#[derive(Component)]
pub struct PlacementGhost;

/// Current pre-placement rotation, applied to single-point placed entities.
/// Stays sticky across hotbar swaps; forced to IDENTITY for Fixed placeables.
#[derive(Resource, Default)]
pub struct BuildOrientation(pub Quat);

const BUILD_ROT_STEP: f32 = 10.0;

/// Bundled placement metadata params — keeps `object_interaction` under the 16-param limit.
#[derive(SystemParam)]
pub(super) struct PlacementMeta<'w> {
    cache: Option<Res<'w, PlaceableColliderCache>>,
    registry: Option<Res<'w, PlaceableRegistry>>,
    orientation: Option<Res<'w, BuildOrientation>>,
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

pub(super) fn build_orientation_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    hotbar: Res<Hotbar>,
    registry: Option<Res<PlaceableRegistry>>,
    mut orientation: ResMut<BuildOrientation>,
) {
    let support = registry
        .as_ref()
        .and_then(|r| r.get(hotbar.active_item_id()?))
        .map(|d| d.orientation.clone())
        .unwrap_or(OrientationSupport::Fixed);

    match support {
        OrientationSupport::Fixed => {
            orientation.0 = Quat::IDENTITY;
        }
        OrientationSupport::AxisY | OrientationSupport::Free => {
            // VS simplification: Free treated same as AxisY (spec §13)
            if keyboard.just_pressed(KeyCode::KeyR) {
                let shift =
                    keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
                let step = if shift {
                    -BUILD_ROT_STEP
                } else {
                    BUILD_ROT_STEP
                };
                orientation.0 = Quat::from_rotation_y(step.to_radians()) * orientation.0;
            }
        }
    }
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
    commands.init_resource::<PendingPlacement>();
    commands.init_resource::<BuildOrientation>();

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
    pending: Option<Res<PendingPlacement>>,
    camera_q: Query<&Transform, (With<MainCamera>, Without<PlacementGhost>)>,
    placeable_cache: Option<Res<PlaceableColliderCache>>,
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
            if item_id == "platform" {
                let base = pos + normal * 0.125;
                let corner_info = pending.as_ref().and_then(|p| {
                    if let PendingPlacement::AreaRect {
                        corner_a,
                        tile_step,
                        ..
                    } = p.as_ref()
                    {
                        Some((*corner_a, *tile_step))
                    } else {
                        None
                    }
                });
                match corner_info {
                    Some((ca, step)) => {
                        let (ix_min, ix_max, iz_min, iz_max) = platform_tile_range(ca, base, step);
                        transform.translation = Vec3::new(
                            ca.x + (ix_min + ix_max) as f32 * 0.5 * step,
                            ca.y,
                            ca.z + (iz_min + iz_max) as f32 * 0.5 * step,
                        );
                        transform.scale = Vec3::new(
                            (ix_max - ix_min + 1) as f32,
                            1.0,
                            (iz_max - iz_min + 1) as f32,
                        );
                    }
                    None => {
                        transform.translation = base;
                        transform.scale = Vec3::ONE;
                    }
                }
            } else {
                let half_y = placeable_cache
                    .as_ref()
                    .and_then(|c| c.by_item.get(item_id))
                    .map(|c| c.aabb_half_extents.y)
                    .unwrap_or(2.0);
                transform.translation = pos + normal * half_y;
                transform.scale = Vec3::ONE;
            }
            *vis = Visibility::Visible;
        }
        _ if show_ghost
            && hotbar.active_item_id() == Some("platform")
            && matches!(pending.as_deref(), Some(PendingPlacement::AreaRect { .. })) =>
        {
            let Some(PendingPlacement::AreaRect {
                corner_a,
                tile_step,
                ..
            }) = pending.as_deref()
            else {
                return;
            };
            let (corner_a, step) = (*corner_a, *tile_step);
            if let Some(air_pos) = air_platform_pos(&camera_q, corner_a) {
                let (ix_min, ix_max, iz_min, iz_max) = platform_tile_range(corner_a, air_pos, step);
                if ghost.last_item_id != "platform" {
                    *mesh = Mesh3d(ghost_assets.platform_mesh.clone());
                    *mat = MeshMaterial3d(ghost_assets.platform_material.clone());
                    ghost.last_item_id = "platform".to_string();
                }
                transform.translation = Vec3::new(
                    corner_a.x + (ix_min + ix_max) as f32 * 0.5 * step,
                    corner_a.y,
                    corner_a.z + (iz_min + iz_max) as f32 * 0.5 * step,
                );
                transform.scale = Vec3::new(
                    (ix_max - ix_min + 1) as f32,
                    1.0,
                    (iz_max - iz_min + 1) as f32,
                );
                *vis = Visibility::Visible;
            } else {
                *vis = Visibility::Hidden;
            }
        }
        _ => {
            *vis = Visibility::Hidden;
        }
    }
}

/// Projects the camera ray onto the horizontal plane at `corner_y`, returning
/// the intersection point. Returns `None` if the ray is nearly horizontal or
/// points away from the plane.
fn air_platform_pos(
    camera_q: &Query<&Transform, (With<MainCamera>, Without<PlacementGhost>)>,
    corner: Vec3,
) -> Option<Vec3> {
    let cam = camera_q.single().ok()?;
    let dir = *cam.forward();
    if dir.y.abs() <= 1e-5 {
        return None;
    }
    let t = (corner.y - cam.translation.y) / dir.y;
    if t <= 0.0 || t > MAX_REACH * 4.0 {
        return None;
    }
    Some(cam.translation + dir * t)
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
    player_q: Query<Entity, With<Player>>,
) {
    let Ok(cam) = camera_q.single() else {
        *look_target = LookTarget::Nothing;
        return;
    };

    let mut filter = SpatialQueryFilter::default();
    if let Ok(player) = player_q.single() {
        filter.excluded_entities.insert(player);
    }

    let dir = Dir3::new(*cam.forward()).unwrap_or(Dir3::NEG_Z);
    let hit = spatial_query.cast_ray(cam.translation, dir, MAX_REACH, true, &filter);

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

fn take_from_any_storage(storage_q: &mut Query<&mut StorageUnit>, item_id: &str) -> bool {
    for mut unit in storage_q.iter_mut() {
        if let Some(count) = unit.items.get_mut(item_id)
            && *count > 0
        {
            *count -= 1;
            if *count == 0 {
                unit.items.remove(item_id);
            }
            return true;
        }
    }
    false
}

pub(super) fn object_interaction(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut scroll: MessageReader<MouseWheel>,
    look_target: Res<LookTarget>,
    mut hotbar: ResMut<Hotbar>,
    mut storage_q: Query<&mut StorageUnit>,
    player_q: Query<Entity, With<Player>>,
    mut world_events: MessageWriter<WorldObjectEvent>,
    mut cable_events: MessageWriter<CableConnectionEvent>,
    mut pending: ResMut<PendingPlacement>,
    camera_q: Query<&Transform, (With<MainCamera>, Without<PlacementGhost>)>,
    spatial_query: SpatialQuery,
    sensor_q: Query<(), With<Sensor>>,
    meta: PlacementMeta<'_>,
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

    if mouse.just_pressed(MouseButton::Right) {
        *pending = PendingPlacement::Idle;
    }
    match &*pending {
        PendingPlacement::AreaRect { item_id, .. }
            if hotbar.active_item_id() != Some(item_id.as_str()) =>
        {
            *pending = PendingPlacement::Idle;
        }
        PendingPlacement::TwoEndpoint { item_id, .. }
            if hotbar.active_item_id() != Some(item_id.as_str()) =>
        {
            *pending = PendingPlacement::Idle;
        }
        _ => {}
    }

    let surface: Option<(Vec3, Vec3)> = match *look_target {
        LookTarget::Surface { pos, normal, .. } => Some((pos, normal)),
        _ => None,
    };

    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if mouse.just_pressed(MouseButton::Left) {
        if shift {
            let Some((pos, _)) = surface else {
                return;
            };
            world_events.write(WorldObjectEvent {
                transform: Transform::from_translation(pos),
                item_id: String::new(),
                kind: WorldObjectKind::Removed,
            });
            *pending = PendingPlacement::Idle;
        } else if let Some(item_id) = hotbar.active_item_id().map(str::to_owned) {
            if item_id.ends_with("_cable") {
                let Some((pos, _)) = surface else {
                    return;
                };
                let place_pos = pos;
                match &*pending {
                    PendingPlacement::TwoEndpoint {
                        anchor,
                        item_id: pending_id,
                    } if pending_id == &item_id && anchor.distance(place_pos) > 0.1 => {
                        let from = *anchor;
                        if take_from_any_storage(&mut storage_q, &item_id) {
                            cable_events.write(CableConnectionEvent {
                                from,
                                to: place_pos,
                                item_id,
                                kind: WorldObjectKind::Placed,
                                from_port: None,
                                to_port: None,
                            });
                        }
                        *pending = PendingPlacement::Idle;
                    }
                    _ => {
                        *pending = PendingPlacement::TwoEndpoint {
                            item_id,
                            anchor: place_pos,
                        };
                    }
                }
            } else if item_id == "platform" {
                let existing = if let PendingPlacement::AreaRect {
                    corner_a,
                    tile_step,
                    ..
                } = &*pending
                {
                    Some((*corner_a, *tile_step))
                } else {
                    None
                };
                let raw = surface
                    .map(|(p, n)| p + n * 0.125)
                    .or_else(|| air_platform_pos(&camera_q, existing.map(|(ca, _)| ca)?));
                let Some(raw) = raw else {
                    return;
                };
                match existing {
                    None => {
                        let tile_step = meta
                            .registry
                            .as_deref()
                            .and_then(|r| r.get("platform"))
                            .and_then(|def| match &def.snap {
                                SnapRule::Tile(TileSnap::Horizontal { step }) => Some(*step),
                                _ => None,
                            })
                            .unwrap_or(8.0);
                        *pending = PendingPlacement::AreaRect {
                            item_id,
                            corner_a: raw,
                            tile_step,
                        };
                    }
                    Some((corner_a, tile_step)) => {
                        let (ix_min, ix_max, iz_min, iz_max) =
                            platform_tile_range(corner_a, raw, tile_step);
                        for ix in ix_min..=ix_max {
                            for iz in iz_min..=iz_max {
                                if !take_from_any_storage(&mut storage_q, &item_id) {
                                    *pending = PendingPlacement::Idle;
                                    return;
                                }
                                world_events.write(WorldObjectEvent {
                                    transform: Transform::from_translation(Vec3::new(
                                        corner_a.x + ix as f32 * tile_step,
                                        corner_a.y,
                                        corner_a.z + iz as f32 * tile_step,
                                    )),
                                    item_id: item_id.clone(),
                                    kind: WorldObjectKind::Placed,
                                });
                            }
                        }
                        *pending = PendingPlacement::Idle;
                    }
                }
            } else {
                let Some((pos, normal)) = surface else {
                    return;
                };
                let footprint = meta
                    .cache
                    .as_ref()
                    .and_then(|c| c.by_item.get(item_id.as_str()))
                    .map(|c| c.aabb_half_extents)
                    .unwrap_or(Vec3::splat(2.0));
                let place_pos = pos + normal * footprint.y;
                let check = Collider::cuboid(
                    (footprint.x - 0.05).max(0.05),
                    (footprint.y - 0.05).max(0.05),
                    (footprint.z - 0.05).max(0.05),
                );
                let mut place_filter = SpatialQueryFilter::default();
                if let Ok(player) = player_q.single() {
                    place_filter.excluded_entities.insert(player);
                }
                let blocked = spatial_query
                    .shape_intersections(&check, place_pos, Quat::IDENTITY, &place_filter)
                    .iter()
                    .any(|&e| !sensor_q.contains(e));
                if !blocked && take_from_any_storage(&mut storage_q, &item_id) {
                    let rotation = {
                        let support = meta
                            .registry
                            .as_deref()
                            .and_then(|r: &PlaceableRegistry| r.get(&item_id))
                            .map(|d| &d.orientation);
                        match support {
                            Some(OrientationSupport::AxisY | OrientationSupport::Free) => meta
                                .orientation
                                .as_deref()
                                .map(|o: &BuildOrientation| o.0)
                                .unwrap_or(Quat::IDENTITY),
                            _ => Quat::IDENTITY,
                        }
                    };
                    world_events.write(WorldObjectEvent {
                        transform: Transform {
                            translation: place_pos,
                            rotation,
                            scale: Vec3::ONE,
                        },
                        item_id,
                        kind: WorldObjectKind::Placed,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_tile_range_single_tile() {
        let a = Vec3::ZERO;
        assert_eq!(platform_tile_range(a, a, 8.0), (0, 0, 0, 0));
    }

    #[test]
    fn platform_tile_range_two_by_three() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(8.0, 0.0, 16.0);
        assert_eq!(platform_tile_range(a, b, 8.0), (0, 1, 0, 2));
    }

    #[test]
    fn platform_tile_range_negative_direction() {
        let anchor = Vec3::new(16.0, 0.0, 0.0);
        let cursor = Vec3::new(0.0, 0.0, 0.0);
        assert_eq!(platform_tile_range(anchor, cursor, 8.0), (-2, 0, 0, 0));
    }

    #[test]
    fn platform_tile_range_arbitrary_anchor() {
        let anchor = Vec3::new(3.5, 0.0, 7.2);
        let cursor = Vec3::new(3.5 + 16.0, 0.0, 7.2 + 8.0);
        assert_eq!(platform_tile_range(anchor, cursor, 8.0), (0, 2, 0, 1));
    }
}

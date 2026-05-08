use avian3d::prelude::{Collider, Sensor, SpatialQuery, SpatialQueryFilter};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::inventory::{Hotbar, InventoryOpen};
use crate::logistics::StorageUnit;
use crate::machine::{GhostAssets, IoPortMarker, Machine, Platform};

use super::player::{MainCamera, Player};
use super::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

const MAX_REACH: f32 = 16.0;
const PLATFORM_TILE: f32 = 8.0;

fn placement_half_extent(item_id: &str) -> f32 {
    match item_id {
        "platform" => 0.125,
        _ => 2.0,
    }
}

fn snap_platform_xz(pos: Vec3, y: f32) -> Vec3 {
    Vec3::new(
        (pos.x / PLATFORM_TILE).round() * PLATFORM_TILE,
        y,
        (pos.z / PLATFORM_TILE).round() * PLATFORM_TILE,
    )
}

/// Returns (ix_min, ix_max, iz_min, iz_max) in tile-index space.
fn platform_tile_range(a: Vec3, b: Vec3) -> (i32, i32, i32, i32) {
    let ix_a = (a.x / PLATFORM_TILE).round() as i32;
    let ix_b = (b.x / PLATFORM_TILE).round() as i32;
    let iz_a = (a.z / PLATFORM_TILE).round() as i32;
    let iz_b = (b.z / PLATFORM_TILE).round() as i32;
    (
        ix_a.min(ix_b),
        ix_a.max(ix_b),
        iz_a.min(iz_b),
        iz_a.max(iz_b),
    )
}

#[derive(Resource, Default)]
pub struct PendingPlatformCorner(pub Option<Vec3>);

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
    commands.init_resource::<PendingPlatformCorner>();

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
    pending_platform: Option<Res<PendingPlatformCorner>>,
    camera_q: Query<&Transform, (With<MainCamera>, Without<PlacementGhost>)>,
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
                let snapped = snap_platform_xz(base, base.y);
                let corner_a = pending_platform.as_ref().and_then(|p| p.0);
                match corner_a {
                    Some(ca) => {
                        let (ix_min, ix_max, iz_min, iz_max) = platform_tile_range(ca, snapped);
                        transform.translation = Vec3::new(
                            (ix_min + ix_max) as f32 * 0.5 * PLATFORM_TILE,
                            ca.y,
                            (iz_min + iz_max) as f32 * 0.5 * PLATFORM_TILE,
                        );
                        transform.scale = Vec3::new(
                            (ix_max - ix_min + 1) as f32,
                            1.0,
                            (iz_max - iz_min + 1) as f32,
                        );
                    }
                    None => {
                        transform.translation = snapped;
                        transform.scale = Vec3::ONE;
                    }
                }
            } else {
                transform.translation = pos + normal * placement_half_extent(item_id);
                transform.scale = Vec3::ONE;
            }
            *vis = Visibility::Visible;
        }
        _ if show_ghost
            && hotbar.active_item_id() == Some("platform")
            && pending_platform.as_ref().and_then(|p| p.0).is_some() =>
        {
            let Some(corner_a) = pending_platform.as_ref().and_then(|p| p.0) else {
                return;
            };
            if let Some(air_pos) = air_platform_pos(&camera_q, corner_a) {
                let snapped = snap_platform_xz(air_pos, corner_a.y);
                let (ix_min, ix_max, iz_min, iz_max) = platform_tile_range(corner_a, snapped);
                if ghost.last_item_id != "platform" {
                    *mesh = Mesh3d(ghost_assets.platform_mesh.clone());
                    *mat = MeshMaterial3d(ghost_assets.platform_material.clone());
                    ghost.last_item_id = "platform".to_string();
                }
                transform.translation = Vec3::new(
                    (ix_min + ix_max) as f32 * 0.5 * PLATFORM_TILE,
                    corner_a.y,
                    (iz_min + iz_max) as f32 * 0.5 * PLATFORM_TILE,
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
    mut pending_cable: ResMut<PendingCablePort>,
    mut pending_platform: ResMut<PendingPlatformCorner>,
    camera_q: Query<&Transform, (With<MainCamera>, Without<PlacementGhost>)>,
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

    if mouse.just_pressed(MouseButton::Right) {
        pending_platform.0 = None;
    }
    if pending_platform.0.is_some() && hotbar.active_item_id() != Some("platform") {
        pending_platform.0 = None;
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
                pos,
                item_id: String::new(),
                kind: WorldObjectKind::Removed,
            });
            pending_cable.pos = None;
            pending_cable.item_id = None;
        } else if let Some(item_id) = hotbar.active_item_id().map(str::to_owned) {
            if item_id.ends_with("_cable") {
                let Some((pos, _)) = surface else {
                    return;
                };
                let place_pos = pos;
                match pending_cable.pos {
                    Some(from)
                        if pending_cable.item_id.as_deref() == Some(&item_id)
                            && from.distance(place_pos) > 0.1 =>
                    {
                        if take_from_any_storage(&mut storage_q, &item_id) {
                            cable_events.write(CableConnectionEvent {
                                from,
                                to: place_pos,
                                item_id,
                                kind: WorldObjectKind::Placed,
                            });
                        }
                        pending_cable.pos = None;
                        pending_cable.item_id = None;
                    }
                    _ => {
                        pending_cable.pos = Some(place_pos);
                        pending_cable.item_id = Some(item_id);
                    }
                }
            } else if item_id == "platform" {
                // Use surface hit or, when a corner is already set, project into air
                let raw = surface
                    .map(|(p, n)| p + n * 0.125)
                    .or_else(|| air_platform_pos(&camera_q, pending_platform.0?));
                let Some(raw) = raw else {
                    return;
                };
                let snapped = snap_platform_xz(raw, raw.y);
                match pending_platform.0 {
                    None => {
                        pending_platform.0 = Some(snapped);
                    }
                    Some(corner_a) => {
                        let (ix_min, ix_max, iz_min, iz_max) =
                            platform_tile_range(corner_a, snapped);
                        for ix in ix_min..=ix_max {
                            for iz in iz_min..=iz_max {
                                if !take_from_any_storage(&mut storage_q, &item_id) {
                                    pending_platform.0 = None;
                                    return;
                                }
                                world_events.write(WorldObjectEvent {
                                    pos: Vec3::new(
                                        ix as f32 * PLATFORM_TILE,
                                        corner_a.y,
                                        iz as f32 * PLATFORM_TILE,
                                    ),
                                    item_id: item_id.clone(),
                                    kind: WorldObjectKind::Placed,
                                });
                            }
                        }
                        pending_platform.0 = None;
                    }
                }
            } else {
                let Some((pos, normal)) = surface else {
                    return;
                };
                let half = placement_half_extent(&item_id);
                let place_pos = pos + normal * half;
                let check_half = (half - 0.05).max(0.05);
                let check = Collider::cuboid(check_half, check_half, check_half);
                let mut place_filter = SpatialQueryFilter::default();
                if let Ok(player) = player_q.single() {
                    place_filter.excluded_entities.insert(player);
                }
                let blocked = spatial_query
                    .shape_intersections(&check, place_pos, Quat::IDENTITY, &place_filter)
                    .iter()
                    .any(|&e| !sensor_q.contains(e));
                if !blocked && take_from_any_storage(&mut storage_q, &item_id) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_half_extent_platform() {
        assert_eq!(placement_half_extent("platform"), 0.125);
    }

    #[test]
    fn placement_half_extent_default() {
        assert_eq!(placement_half_extent("smelter"), 2.0);
        assert_eq!(placement_half_extent("anything_else"), 2.0);
    }

    #[test]
    fn snap_platform_xz_rounds_to_nearest_tile() {
        let snapped = snap_platform_xz(Vec3::new(3.0, 5.0, 13.0), 5.0);
        assert_eq!(snapped, Vec3::new(0.0, 5.0, 16.0));

        let snapped2 = snap_platform_xz(Vec3::new(5.0, 0.0, -1.0), 0.0);
        assert_eq!(snapped2, Vec3::new(8.0, 0.0, 0.0));
    }

    #[test]
    fn platform_tile_range_single_tile() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        assert_eq!(platform_tile_range(a, a), (0, 0, 0, 0));
    }

    #[test]
    fn platform_tile_range_two_by_three() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(8.0, 0.0, 16.0);
        assert_eq!(platform_tile_range(a, b), (0, 1, 0, 2));
    }

    #[test]
    fn platform_tile_range_negative_direction() {
        let a = Vec3::new(16.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 0.0, 0.0);
        assert_eq!(platform_tile_range(a, b), (0, 2, 0, 0));
    }
}

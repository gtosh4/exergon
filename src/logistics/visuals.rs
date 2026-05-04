use bevy::prelude::*;

use crate::machine::Machine;
use crate::network::visuals::spawn_cable_children;

use super::{CABLE_RADIUS, LogisticsCableSegment};

#[derive(Resource)]
pub(super) struct LogisticsVisualAssets {
    pub(super) tube: Handle<Mesh>,
    pub(super) joint: Handle<Mesh>,
    pub(super) cable_material: Handle<StandardMaterial>,
}

pub(super) fn setup_logistics_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(LogisticsVisualAssets {
        tube: meshes.add(Cylinder::new(CABLE_RADIUS, 1.0)),
        joint: meshes.add(Sphere::new(CABLE_RADIUS)),
        cable_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.7, 0.3),
            ..default()
        }),
    });
}

pub(super) fn add_cable_visuals(
    mut commands: Commands,
    added: Query<(Entity, &LogisticsCableSegment), Added<LogisticsCableSegment>>,
    assets: Option<Res<LogisticsVisualAssets>>,
    machine_q: Query<(&Machine, &Transform)>,
) {
    let Some(assets) = assets else { return };
    for (entity, seg) in &added {
        let port_machine_positions: Vec<(Vec3, Vec3)> = [seg.from, seg.to]
            .into_iter()
            .filter_map(|port| {
                let port_center = port + Vec3::splat(0.5);
                let port_key = port.round().as_ivec3();
                machine_q
                    .iter()
                    .find(|(m, _)| {
                        m.logistics_ports
                            .iter()
                            .any(|p| p.round().as_ivec3() == port_key)
                    })
                    .map(|(_, t)| (port_center, t.translation))
            })
            .collect();

        commands
            .entity(entity)
            .insert((Transform::default(), Visibility::default()))
            .with_children(|parent| {
                spawn_cable_children(
                    parent,
                    &seg.path,
                    &port_machine_positions,
                    assets.tube.clone(),
                    assets.joint.clone(),
                    assets.cable_material.clone(),
                    CABLE_RADIUS,
                );
            });
    }
}

use avian3d::prelude::{Collider, Sensor};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

/// Spawns tube segments, corner joints, and machine-to-port connector tubes
/// as children of a cable entity. Both logistics and power cables use this.
///
/// `port_machine_positions`: `(port_center_world, machine_center_world)` for each
/// cable endpoint that connects to a machine body.
pub fn spawn_cable_children(
    parent: &mut ChildSpawnerCommands,
    path: &[IVec3],
    port_machine_positions: &[(Vec3, Vec3)],
    tube: Handle<Mesh>,
    joint: Handle<Mesh>,
    cable_material: Handle<StandardMaterial>,
    cable_radius: f32,
) {
    for window in path.windows(2) {
        let [a_pos, b_pos] = window else { continue };
        let a = a_pos.as_vec3() + Vec3::splat(0.5);
        let b = b_pos.as_vec3() + Vec3::splat(0.5);
        let dir = b - a;
        let rotation = if dir.x.abs() > 0.5 {
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
        } else if dir.z.abs() > 0.5 {
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
        } else {
            Quat::IDENTITY
        };
        parent.spawn((
            Mesh3d(tube.clone()),
            MeshMaterial3d(cable_material.clone()),
            Transform::from_translation((a + b) * 0.5).with_rotation(rotation),
            Collider::cylinder(cable_radius, 1.0),
            Sensor,
        ));
    }

    for window in path.windows(3) {
        let [prev, curr, next] = window else { continue };
        let prev_dir = *curr - *prev;
        let next_dir = *next - *curr;
        if prev_dir != next_dir {
            parent.spawn((
                Mesh3d(joint.clone()),
                MeshMaterial3d(cable_material.clone()),
                Transform::from_translation(curr.as_vec3() + Vec3::splat(0.5)),
            ));
        }
    }

    for &(port_center, mpos) in port_machine_positions {
        let diff = port_center - mpos;
        let length = diff.length();
        if length > 1e-4 {
            let rotation = Quat::from_rotation_arc(Vec3::Y, diff / length);
            parent.spawn((
                Mesh3d(tube.clone()),
                MeshMaterial3d(cable_material.clone()),
                Transform::from_translation((mpos + port_center) * 0.5)
                    .with_rotation(rotation)
                    .with_scale(Vec3::new(1.0, length, 1.0)),
            ));
        }
    }
}

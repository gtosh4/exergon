use avian3d::prelude::{
    Collider, ColliderConstructor, ColliderConstructorHierarchy, RigidBody, Sensor,
};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::world::{WorldObjectEvent, WorldObjectKind};

use super::registry::{MachineDef, MachineRegistry, MachineTierDef};
use super::visuals::{MachineColliders, MachineVisualAssets};
use super::{
    IoPortMarker, Machine, MachineNetworkChanged, MachineState, Mirror, Orientation, Platform,
    Rotation,
};

#[derive(Bundle)]
pub struct MachineBundle {
    pub machine: Machine,
    pub state: MachineState,
    pub transform: Transform,
    pub rigid_body: RigidBody,
}

impl MachineBundle {
    pub fn new(pos: Vec3, def: &MachineDef, tier: u8) -> Self {
        let fallback = MachineTierDef::default();
        let tier_def = def
            .tiers
            .iter()
            .find(|t| t.tier == tier)
            .unwrap_or(&fallback);
        let orientation = Orientation {
            rotation: Rotation::North,
            mirror: Mirror::Normal,
        };
        Self {
            machine: Machine {
                machine_type: def.id.clone(),
                tier: tier_def.tier,
                orientation,
                energy_ports: tier_def
                    .energy_io_offsets
                    .iter()
                    .map(|&o| pos + orientation.transform(o).as_vec3())
                    .collect(),
                logistics_ports: tier_def
                    .logistics_io_offsets
                    .iter()
                    .map(|&o| pos + orientation.transform(o).as_vec3())
                    .collect(),
            },
            state: MachineState::Idle,
            transform: Transform::from_translation(pos),
            rigid_body: RigidBody::Static,
        }
    }
}

pub fn spawn_port_markers(
    commands: &mut Commands,
    machine_entity: Entity,
    energy_ports: &[Vec3],
    logistics_ports: &[Vec3],
    visuals: Option<&MachineVisualAssets>,
) {
    for (&port_pos, is_energy) in energy_ports
        .iter()
        .map(|p| (p, true))
        .chain(logistics_ports.iter().map(|p| (p, false)))
    {
        let mut cmd = commands.spawn((
            IoPortMarker {
                owner: machine_entity,
            },
            Transform::from_translation(port_pos),
            Collider::sphere(0.4),
            Sensor,
        ));
        if let Some(v) = visuals {
            let mat = if is_energy {
                v.energy_port_mat.clone()
            } else {
                v.logistics_port_mat.clone()
            };
            cmd.insert((Mesh3d(v.port_mesh.clone()), MeshMaterial3d(mat)));
        }
    }
}

pub(super) fn place_machine_system(
    mut commands: Commands,
    mut events: MessageReader<WorldObjectEvent>,
    registry: Res<MachineRegistry>,
    mut network_changed: MessageWriter<MachineNetworkChanged>,
    visuals: Option<Res<MachineVisualAssets>>,
    machine_colliders: Option<Res<MachineColliders>>,
) {
    for ev in events.read() {
        if ev.kind != WorldObjectKind::Placed {
            continue;
        }
        let Some(def) = registry.machine_def(&ev.item_id) else {
            continue;
        };

        let tier = def.tiers.iter().map(|t| t.tier).max().unwrap_or(1);
        let bundle = MachineBundle::new(ev.pos, def, tier);
        let energy_ports = bundle.machine.energy_ports.clone();
        let logistics_ports = bundle.machine.logistics_ports.clone();
        let machine_entity = commands.spawn(bundle).id();

        let cached = machine_colliders
            .as_deref()
            .and_then(|mc| mc.colliders.get(&def.id))
            .cloned();
        if let Some(collider) = cached {
            commands.entity(machine_entity).insert(collider);
        } else {
            commands
                .entity(machine_entity)
                .insert(ColliderConstructorHierarchy::new(
                    ColliderConstructor::ConvexHullFromMesh,
                ));
        }

        if let Some(ref v) = visuals
            && let Some(scene) = v.scenes.get(&def.id)
        {
            commands
                .entity(machine_entity)
                .insert(SceneRoot(scene.clone()));
        }

        spawn_port_markers(
            &mut commands,
            machine_entity,
            &energy_ports,
            &logistics_ports,
            visuals.as_deref(),
        );

        network_changed.write(MachineNetworkChanged);
        info!("Machine '{}' tier {} placed at {:?}", def.id, tier, ev.pos);
    }
}

pub(super) fn place_platform_system(
    mut commands: Commands,
    mut events: MessageReader<WorldObjectEvent>,
    visuals: Option<Res<MachineVisualAssets>>,
) {
    for ev in events.read() {
        if ev.kind != WorldObjectKind::Placed || ev.item_id != "platform" {
            continue;
        }
        let mut entity_cmd = commands.spawn((
            Platform,
            Transform::from_translation(ev.pos),
            RigidBody::Static,
        ));
        if let Some(ref v) = visuals {
            entity_cmd.insert((
                SceneRoot(v.platform_scene.clone()),
                ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
            ));
        } else {
            entity_cmd.insert(Collider::cuboid(8.0, 0.25, 8.0));
        }
        info!("Platform placed at {:?}", ev.pos);
    }
}

pub(super) fn remove_placed_objects_system(
    mut commands: Commands,
    mut events: MessageReader<WorldObjectEvent>,
    machine_q: Query<(Entity, &Machine, &Transform)>,
    port_marker_q: Query<(Entity, &IoPortMarker)>,
    platform_q: Query<(Entity, &Transform), With<Platform>>,
    mut network_changed: MessageWriter<MachineNetworkChanged>,
) {
    for ev in events.read() {
        if ev.kind != WorldObjectKind::Removed || !ev.item_id.is_empty() {
            continue;
        }
        if let Some((entity, machine, _)) = machine_q
            .iter()
            .find(|(_, _, t)| t.translation.distance(ev.pos) < 1.5)
        {
            let machine_type = machine.machine_type.clone();
            for (marker_entity, marker) in port_marker_q.iter() {
                if marker.owner == entity {
                    commands.entity(marker_entity).despawn();
                }
            }
            commands.entity(entity).despawn();
            network_changed.write(MachineNetworkChanged);
            info!("Machine '{}' removed near {:?}", machine_type, ev.pos);
        } else if let Some((entity, _)) = platform_q
            .iter()
            .find(|(_, t)| t.translation.distance(ev.pos) < 1.5)
        {
            commands.entity(entity).despawn();
            info!("Platform removed near {:?}", ev.pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::registry::{MachineDef, MachineTierDef};

    fn def_with_tier(id: &str, tier: u8, energy: Vec<IVec3>, logistics: Vec<IVec3>) -> MachineDef {
        MachineDef {
            id: id.to_string(),
            tiers: vec![MachineTierDef {
                tier,
                energy_io_offsets: energy,
                logistics_io_offsets: logistics,
            }],
        }
    }

    #[test]
    fn machine_bundle_new_uses_matching_tier() {
        let def = def_with_tier(
            "smelter",
            1,
            vec![IVec3::new(1, 0, 0)],
            vec![IVec3::new(-1, 0, 0)],
        );
        let bundle = MachineBundle::new(Vec3::ZERO, &def, 1);
        assert_eq!(bundle.machine.tier, 1);
        assert_eq!(bundle.machine.energy_ports.len(), 1);
        assert_eq!(bundle.machine.logistics_ports.len(), 1);
        assert_eq!(bundle.machine.machine_type, "smelter");
    }

    #[test]
    fn machine_bundle_new_falls_back_when_tier_missing() {
        let def = def_with_tier("smelter", 1, vec![], vec![]);
        let bundle = MachineBundle::new(Vec3::ZERO, &def, 9);
        assert_eq!(bundle.machine.tier, 0); // MachineTierDef::default() has tier 0
    }

    #[test]
    fn machine_bundle_new_offsets_ports_by_position() {
        let def = def_with_tier("smelter", 1, vec![IVec3::new(2, 0, 0)], vec![]);
        let pos = Vec3::new(10.0, 0.0, 0.0);
        let bundle = MachineBundle::new(pos, &def, 1);
        assert_eq!(bundle.machine.energy_ports[0], Vec3::new(12.0, 0.0, 0.0));
        assert_eq!(bundle.transform.translation, pos);
    }
}

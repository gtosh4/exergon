use avian3d::prelude::{Collider, RigidBody, Sensor};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::world::{WorldObjectEvent, WorldObjectKind};

use super::registry::{MachineDef, MachineRegistry, MachineTierDef};
use super::visuals::MachineVisualAssets;
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
    pub collider: Collider,
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
            collider: Collider::cuboid(2.0, 2.0, 2.0),
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

        if let Some(ref v) = visuals {
            let mat = v
                .materials
                .get(&def.id)
                .cloned()
                .unwrap_or_else(|| v.fallback.clone());
            commands
                .entity(machine_entity)
                .insert((Mesh3d(v.mesh.clone()), MeshMaterial3d(mat)));
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
            Collider::cuboid(1.0, 0.125, 1.0),
        ));
        if let Some(ref v) = visuals {
            entity_cmd.insert((
                Mesh3d(v.platform_mesh.clone()),
                MeshMaterial3d(v.platform_mat.clone()),
            ));
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

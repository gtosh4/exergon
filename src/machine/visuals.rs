use std::collections::HashMap;

use avian3d::prelude::Collider;
use bevy::gltf::{Gltf, GltfMesh};
use bevy::prelude::*;

#[derive(Resource)]
pub struct MachineVisualAssets {
    pub(super) mesh: Handle<Mesh>,
    pub(super) port_mesh: Handle<Mesh>,
    pub(super) energy_port_mat: Handle<StandardMaterial>,
    pub(super) logistics_port_mat: Handle<StandardMaterial>,
    pub(super) platform_mesh: Handle<Mesh>,
    pub scenes: HashMap<String, Handle<Scene>>,
    pub(crate) platform_scene: Handle<Scene>,
    pub(crate) deposit_scene: Handle<Scene>,
    pub gltf_handles: HashMap<String, Handle<Gltf>>,
}

#[derive(Resource, Default)]
pub struct MachineColliders {
    pub colliders: HashMap<String, Collider>,
}

#[derive(Resource)]
pub(crate) struct GhostAssets {
    pub(crate) machine_mesh: Handle<Mesh>,
    pub(crate) platform_mesh: Handle<Mesh>,
    pub(crate) fallback_mesh: Handle<Mesh>,
    pub(crate) materials: HashMap<String, Handle<StandardMaterial>>,
    pub(crate) fallback_material: Handle<StandardMaterial>,
    pub(crate) platform_material: Handle<StandardMaterial>,
}

pub(super) fn setup_machine_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mesh = meshes.add(Cuboid::new(4.0, 4.0, 4.0));
    let port_mesh = meshes.add(Sphere::new(0.4));
    let energy_port_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.0),
        unlit: true,
        ..default()
    });
    let logistics_port_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.9, 0.2),
        unlit: true,
        ..default()
    });
    let platform_mesh = meshes.add(Cuboid::new(8.0, 0.25, 8.0));

    let machine_ids = [
        "smelter",
        "assembler",
        "analysis_station",
        "generator",
        "storage_crate",
        "refinery",
        "gateway",
    ];
    let mut scenes: HashMap<String, Handle<Scene>> = HashMap::new();
    let mut gltf_handles: HashMap<String, Handle<Gltf>> = HashMap::new();
    for id in machine_ids {
        scenes.insert(
            id.to_string(),
            asset_server.load(format!("models/machines/{id}.glb#Scene0")),
        );
        gltf_handles.insert(
            id.to_string(),
            asset_server.load(format!("models/machines/{id}.glb")),
        );
    }
    let platform_scene = asset_server.load("models/platforms/platform.glb#Scene0");
    let deposit_scene = asset_server.load("models/deposits/ore_deposit.glb#Scene0");

    commands.insert_resource(MachineVisualAssets {
        mesh,
        port_mesh,
        energy_port_mat,
        logistics_port_mat,
        platform_mesh,
        scenes,
        platform_scene,
        deposit_scene,
        gltf_handles,
    });
}

pub(super) fn setup_ghost_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    visuals: Res<MachineVisualAssets>,
) {
    let ghost_mat = |color: Color| StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    };
    let mut ghost_materials: HashMap<String, Handle<StandardMaterial>> = HashMap::new();
    for (id, color) in [
        ("smelter", Color::srgba(0.9, 0.45, 0.1, 0.5)),
        ("assembler", Color::srgba(0.2, 0.45, 0.9, 0.5)),
        ("analysis_station", Color::srgba(0.1, 0.75, 0.55, 0.5)),
        ("generator", Color::srgba(0.9, 0.8, 0.1, 0.5)),
        ("storage_crate", Color::srgba(0.55, 0.6, 0.65, 0.5)),
    ] {
        ghost_materials.insert(id.to_string(), materials.add(ghost_mat(color)));
    }
    commands.insert_resource(GhostAssets {
        machine_mesh: visuals.mesh.clone(),
        platform_mesh: visuals.platform_mesh.clone(),
        fallback_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        materials: ghost_materials,
        fallback_material: materials.add(ghost_mat(Color::srgba(0.65, 0.65, 0.65, 0.5))),
        platform_material: materials.add(ghost_mat(Color::srgba(0.5, 0.5, 0.55, 0.5))),
    });
}

pub(super) fn compute_machine_colliders(
    mut events: bevy::ecs::message::MessageReader<AssetEvent<Gltf>>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_mesh_assets: Res<Assets<GltfMesh>>,
    mesh_assets: Res<Assets<Mesh>>,
    visuals: Res<MachineVisualAssets>,
    mut machine_colliders: ResMut<MachineColliders>,
) {
    for event in events.read() {
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };
        let Some((machine_id, _)) = visuals.gltf_handles.iter().find(|(_, h)| h.id() == *id) else {
            continue;
        };
        let Some(gltf) = gltf_assets.get(*id) else {
            continue;
        };

        let mut shapes: Vec<(Vec3, Quat, Collider)> = vec![];
        for gltf_mesh_handle in &gltf.meshes {
            let Some(gltf_mesh) = gltf_mesh_assets.get(gltf_mesh_handle) else {
                continue;
            };
            for primitive in &gltf_mesh.primitives {
                let Some(mesh) = mesh_assets.get(&primitive.mesh) else {
                    continue;
                };
                if let Some(collider) = Collider::convex_hull_from_mesh(mesh) {
                    shapes.push((Vec3::ZERO, Quat::IDENTITY, collider));
                }
            }
        }

        let collider = match shapes.len() {
            0 => {
                warn!("No mesh data for machine '{machine_id}', skipping collider cache");
                continue;
            }
            1 => {
                let Some((_, _, shape)) = shapes.into_iter().next() else {
                    continue;
                };
                shape
            }
            _ => Collider::compound(shapes),
        };

        info!("Cached collider for machine '{machine_id}'");
        machine_colliders
            .colliders
            .insert(machine_id.clone(), collider);
    }
}

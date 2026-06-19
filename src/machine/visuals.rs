use std::collections::HashMap;

use avian3d::prelude::Collider;
use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::prelude::*;

#[derive(Resource)]
pub struct MachineVisualAssets {
    pub(super) mesh: Handle<Mesh>,
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

/// Per-machine port positions (canonical-space, pre-orientation) extracted from
/// GLTF named child nodes (`Port_Energy_<i>`, `Port_Logistics_<i>`). Populated
/// by `extract_machine_port_layouts` as each machine GLB finishes loading.
#[derive(Resource, Default, Debug)]
pub struct MachinePortLayouts {
    pub by_machine: HashMap<String, MachinePortLayout>,
}

#[derive(Default, Debug, Clone)]
pub struct MachinePortLayout {
    pub energy: Vec<Vec3>,
    pub logistics: Vec<Vec3>,
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
    asset_server: Res<AssetServer>,
) {
    let mesh = meshes.add(Cuboid::new(4.0, 4.0, 4.0));
    let platform_mesh = meshes.add(Cuboid::new(8.0, 0.25, 8.0));

    let machine_ids = [
        "smelter",
        "assembler",
        "analysis_station",
        "storage_crate",
        "refinery",
        "gateway",
        "solar_generator",
        "combustion_generator",
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
        ("storage_crate", Color::srgba(0.55, 0.6, 0.65, 0.5)),
        ("solar_generator", Color::srgba(0.15, 0.55, 0.95, 0.5)),
        ("combustion_generator", Color::srgba(0.7, 0.35, 0.1, 0.5)),
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
    gltf_node_assets: Res<Assets<GltfNode>>,
    mesh_assets: Res<Assets<Mesh>>,
    visuals: Res<MachineVisualAssets>,
    mut machine_colliders: ResMut<MachineColliders>,
    mut port_layouts: ResMut<MachinePortLayouts>,
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

        // Collider from body mesh primitives. Port stub nodes are skipped so
        // ports don't block placement.
        let body_handle = gltf
            .named_nodes
            .get("Body")
            .and_then(|h| gltf_node_assets.get(h))
            .and_then(|n| n.mesh.as_ref());
        let mut shapes: Vec<(Vec3, Quat, Collider)> = vec![];
        let mesh_handles: Vec<&Handle<GltfMesh>> = if let Some(body) = body_handle {
            vec![body]
        } else {
            gltf.meshes.iter().collect()
        };
        for gltf_mesh_handle in mesh_handles {
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

        // Port layout from named child nodes — Port_Energy_<i>, Port_Logistics_<i>.
        let layout = extract_layout(gltf, &gltf_node_assets);
        info!(
            "Extracted port layout for '{machine_id}': {} energy, {} logistics",
            layout.energy.len(),
            layout.logistics.len()
        );
        port_layouts.by_machine.insert(machine_id.clone(), layout);
    }
}

/// Register port layouts for machine types that have no GLTF model.
pub(super) fn register_fallback_port_layouts(mut port_layouts: ResMut<MachinePortLayouts>) {
    for machine_id in ["solar_generator", "combustion_generator"] {
        port_layouts
            .by_machine
            .entry(machine_id.to_string())
            .or_insert_with(|| MachinePortLayout {
                energy: vec![Vec3::new(1.0, 0.0, 0.0)],
                logistics: vec![Vec3::new(-1.0, 0.0, 0.0)],
            });
    }
}

fn extract_layout(gltf: &Gltf, nodes: &Assets<GltfNode>) -> MachinePortLayout {
    let mut energy: Vec<(usize, Vec3)> = vec![];
    let mut logistics: Vec<(usize, Vec3)> = vec![];
    for (name, handle) in gltf.named_nodes.iter() {
        let Some(node) = nodes.get(handle) else {
            continue;
        };
        if let Some(rest) = name.strip_prefix("Port_Energy_")
            && let Ok(idx) = rest.parse::<usize>()
        {
            energy.push((idx, node.transform.translation));
        } else if let Some(rest) = name.strip_prefix("Port_Logistics_")
            && let Ok(idx) = rest.parse::<usize>()
        {
            logistics.push((idx, node.transform.translation));
        }
    }
    energy.sort_by_key(|(i, _)| *i);
    logistics.sort_by_key(|(i, _)| *i);
    MachinePortLayout {
        energy: energy.into_iter().map(|(_, p)| p).collect(),
        logistics: logistics.into_iter().map(|(_, p)| p).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::platform::collections::HashMap as BvHashMap;

    #[test]
    fn extract_layout_groups_by_prefix_and_sorts_by_index() {
        let mut nodes_assets: Assets<GltfNode> = Assets::default();
        let mut gltf = Gltf {
            scenes: vec![],
            named_scenes: BvHashMap::default(),
            meshes: vec![],
            named_meshes: BvHashMap::default(),
            materials: vec![],
            named_materials: BvHashMap::default(),
            nodes: vec![],
            named_nodes: BvHashMap::default(),
            skins: vec![],
            named_skins: BvHashMap::default(),
            default_scene: None,
            animations: vec![],
            named_animations: BvHashMap::default(),
            source: None,
        };

        let mut insert_node = |name: &str, translation: Vec3| {
            let handle = nodes_assets.add(GltfNode {
                index: 0,
                name: name.to_string(),
                children: vec![],
                mesh: None,
                skin: None,
                transform: Transform::from_translation(translation),
                is_animation_root: false,
                extras: None,
            });
            gltf.named_nodes.insert(name.into(), handle);
        };

        insert_node("Port_Energy_1", Vec3::new(0.0, 0.0, 2.0));
        insert_node("Port_Energy_0", Vec3::new(3.0, 0.0, 0.0));
        insert_node("Port_Logistics_0", Vec3::new(-3.0, 0.0, 0.0));
        insert_node("Body", Vec3::ZERO);

        let layout = extract_layout(&gltf, &nodes_assets);
        assert_eq!(
            layout.energy,
            vec![Vec3::new(3.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0)]
        );
        assert_eq!(layout.logistics, vec![Vec3::new(-3.0, 0.0, 0.0)]);
    }
}

use avian3d::prelude::{
    Collider, ColliderConstructor, ColliderConstructorHierarchy, RigidBody, Sensor,
};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::logistics::{LogisticsNetworkMember, LogisticsNetworkMembers, StorageUnit, give_items};
use crate::power::{GENERATOR_DEFAULT_WATTS, GeneratorUnit};
use crate::recipe_graph::RecipeGraph;
use crate::world::{WorldObjectEvent, WorldObjectKind};

use super::registry::{MachineDef, MachineRegistry, MachineTierDef};
use super::visuals::{MachineColliders, MachineVisualAssets};
use super::{
    EnergyPortOf, IoPortMarker, LogisticsPortOf, Machine, MachineActivity, MachineLogisticsPorts,
    MachineNetworkChanged, MachineState, Mirror, Orientation, Platform, Rotation,
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

fn spawn_port_markers(
    commands: &mut Commands,
    machine_entity: Entity,
    energy_ports: &[Vec3],
    logistics_ports: &[Vec3],
    visuals: Option<&MachineVisualAssets>,
) {
    for &port_pos in energy_ports {
        let mut cmd = commands.spawn((
            IoPortMarker {
                owner: machine_entity,
            },
            EnergyPortOf(machine_entity),
            Transform::from_translation(port_pos),
            Collider::sphere(0.4),
            Sensor,
        ));
        if let Some(v) = visuals {
            cmd.insert((
                Mesh3d(v.port_mesh.clone()),
                MeshMaterial3d(v.energy_port_mat.clone()),
            ));
        }
    }
    for &port_pos in logistics_ports {
        let mut cmd = commands.spawn((
            IoPortMarker {
                owner: machine_entity,
            },
            LogisticsPortOf(machine_entity),
            Transform::from_translation(port_pos),
            Collider::sphere(0.4),
            Sensor,
        ));
        if let Some(v) = visuals {
            cmd.insert((
                Mesh3d(v.port_mesh.clone()),
                MeshMaterial3d(v.logistics_port_mat.clone()),
            ));
        }
    }
}

pub(super) fn on_machine_added(
    trigger: On<Add, Machine>,
    machines: Query<&Machine>,
    visuals: Option<Res<MachineVisualAssets>>,
    mut commands: Commands,
) {
    let entity = trigger.event_target();
    let Ok(machine) = machines.get(entity) else {
        return;
    };
    let energy_ports = machine.energy_ports.clone();
    let logistics_ports = machine.logistics_ports.clone();
    spawn_port_markers(
        &mut commands,
        entity,
        &energy_ports,
        &logistics_ports,
        visuals.as_deref(),
    );
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

        if def.id == "generator" {
            commands.entity(machine_entity).insert(GeneratorUnit {
                pos: ev.pos,
                watts: GENERATOR_DEFAULT_WATTS,
                buffer_joules: 0.0,
                max_buffer_joules: GENERATOR_DEFAULT_WATTS * 10.0,
            });
        }

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
    activity_q: Query<(&MachineActivity, &MachineLogisticsPorts)>,
    recipe_graph: Option<Res<RecipeGraph>>,
    port_net_q: Query<&LogisticsNetworkMember>,
    net_q: Query<&LogisticsNetworkMembers>,
    mut storage_q: Query<&mut StorageUnit>,
    port_of_q: Query<&LogisticsPortOf>,
) {
    for ev in events.read() {
        if ev.kind != WorldObjectKind::Removed || !ev.item_id.is_empty() {
            continue;
        }
        if let Some((entity, machine, _)) = machine_q
            .iter()
            .find(|(_, _, t)| t.translation.distance(ev.pos) < 1.5)
        {
            // Return recipe inputs to the network if a recipe was in progress.
            if let Ok((activity, logistics_ports)) = activity_q.get(entity)
                && let Some(recipe) = recipe_graph
                    .as_ref()
                    .and_then(|rg| rg.recipes.get(&activity.recipe_id))
            {
                let net_e = logistics_ports
                    .ports()
                    .iter()
                    .find_map(|&p| port_net_q.get(p).ok().map(|m| m.0));
                if let Some(net_e) = net_e
                    && let Ok(members) = net_q.get(net_e)
                {
                    for input in &recipe.inputs {
                        give_items(
                            members,
                            &mut storage_q,
                            &port_of_q,
                            &input.item,
                            input.quantity as u32,
                        );
                    }
                }
            }

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
    use crate::logistics::{LogisticsNetwork, LogisticsNetworkMember, StorageUnit};
    use crate::machine::registry::{MachineDef, MachineTierDef};
    use crate::recipe_graph::{ConcreteRecipe, ItemStack, RecipeGraph};
    use crate::world::WorldObjectKind;

    fn make_recipe_graph(recipe: ConcreteRecipe) -> RecipeGraph {
        let id = recipe.id.clone();
        RecipeGraph {
            materials: Default::default(),
            form_groups: Default::default(),
            templates: Default::default(),
            items: Default::default(),
            recipes: [(id, recipe)].into(),
            terminal: Default::default(),
            producers: Default::default(),
            consumers: Default::default(),
        }
    }

    #[test]
    fn machine_removed_while_running_returns_inputs_to_network() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>()
            .add_systems(Update, remove_placed_objects_system);

        app.insert_resource(make_recipe_graph(ConcreteRecipe {
            id: "smelt".to_string(),
            inputs: vec![ItemStack {
                item: "iron_ore".to_string(),
                quantity: 3.0,
            }],
            outputs: vec![ItemStack {
                item: "iron_ingot".to_string(),
                quantity: 1.0,
            }],
            byproducts: vec![],
            machine_type: "smelter".to_string(),
            machine_tier: 1,
            processing_time: 5.0,
            energy_cost: 0.0,
        }));

        let net_e = app.world_mut().spawn(LogisticsNetwork).id();
        let storage_e = app
            .world_mut()
            .spawn(StorageUnit {
                items: Default::default(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_e),
            Transform::default(),
            LogisticsNetworkMember(net_e),
        ));

        let machine_pos = Vec3::ZERO;
        let machine_e = app
            .world_mut()
            .spawn((
                Machine {
                    machine_type: "smelter".to_string(),
                    tier: 1,
                    orientation: Orientation {
                        rotation: Rotation::North,
                        mirror: Mirror::Normal,
                    },
                    energy_ports: vec![],
                    logistics_ports: vec![Vec3::new(1.0, 0.0, 0.0)],
                },
                MachineState::Running,
                MachineActivity {
                    recipe_id: "smelt".to_string(),
                    progress: 2.5,
                    speed_factor: 1.0,
                },
                Transform::from_translation(machine_pos),
            ))
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(machine_e),
            Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
            LogisticsNetworkMember(net_e),
        ));

        app.world_mut().write_message(WorldObjectEvent {
            pos: machine_pos,
            item_id: String::new(),
            kind: WorldObjectKind::Removed,
        });
        app.update();

        let storage = app.world().get::<StorageUnit>(storage_e).unwrap();
        assert_eq!(storage.items.get("iron_ore").copied().unwrap_or(0), 3);
        assert!(
            app.world().get_entity(machine_e).is_err(),
            "machine should be despawned"
        );
    }

    #[test]
    fn idle_machine_removed_returns_nothing() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WorldObjectEvent>()
            .add_message::<MachineNetworkChanged>()
            .add_systems(Update, remove_placed_objects_system);

        let net_e = app.world_mut().spawn(LogisticsNetwork).id();
        let storage_e = app
            .world_mut()
            .spawn(StorageUnit {
                items: Default::default(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_e),
            Transform::default(),
            LogisticsNetworkMember(net_e),
        ));

        let machine_pos = Vec3::ZERO;
        app.world_mut().spawn((
            Machine {
                machine_type: "smelter".to_string(),
                tier: 1,
                orientation: Orientation {
                    rotation: Rotation::North,
                    mirror: Mirror::Normal,
                },
                energy_ports: vec![],
                logistics_ports: vec![],
            },
            MachineState::Idle,
            Transform::from_translation(machine_pos),
        ));

        app.world_mut().write_message(WorldObjectEvent {
            pos: machine_pos,
            item_id: String::new(),
            kind: WorldObjectKind::Removed,
        });
        app.update();

        let storage = app.world().get::<StorageUnit>(storage_e).unwrap();
        assert!(storage.items.is_empty());
    }

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

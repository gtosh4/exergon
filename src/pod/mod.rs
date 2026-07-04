use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use moonshine_save::prelude::Save;

use crate::aegis::{AEGIS_RADIUS, AegisActive, AegisEmitter, AegisRadius, aegis_sphere_collider};
use crate::inventory::{Hotbar, HotbarSlot};
use crate::logistics::{LogisticsNetwork, LogisticsNetworkMember, StorageUnit};
use crate::machine::LogisticsPortOf;
use crate::network::{Logistics, NetworkChanged};
use crate::power::PodPowered;
use crate::world::generation::{TerrainSampler, WorldConfig};
use crate::{GameLayer, GameState};

pub struct PodPlugin;

/// Marks the escape pod's private logistics network entity so it survives
/// save/load and can be located on resume without spawning a duplicate.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct PodNetwork;

/// Holds the entity of the pod's internal logistics network.
#[derive(Resource)]
struct PodLogisticsNetwork(Entity);

impl Plugin for PodPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PodNetwork>()
            .add_systems(
                OnTransition {
                    exited: GameState::Loading,
                    entered: GameState::Playing,
                },
                spawn_escape_pod,
            )
            .add_systems(OnEnter(GameState::Playing), wire_pod_machines)
            .add_systems(OnExit(GameState::Playing), |mut commands: Commands| {
                commands.remove_resource::<PodLogisticsNetwork>()
            });
    }
}

/// Marker for the escape pod entity (Aegis Emitter + visual hull).
#[derive(Component)]
pub struct EscapePod;

/// Placeable items the lander drops with on a fresh run: enough to stand up a
/// self-sustaining base (mine → power → assemble → research) from zero raw materials,
/// so a run can never brick for lack of the right starting parts.
fn starting_kit() -> HashMap<String, u32> {
    HashMap::from([
        ("assembler".to_string(), 1),
        ("solar_generator".to_string(), 1),
        ("miner".to_string(), 1),
        ("analysis_station".to_string(), 1),
        ("logistics_cable".to_string(), 100),
        ("power_cable".to_string(), 100),
    ])
}

fn spawn_escape_pod(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world_config: Res<WorldConfig>,
    mut hotbar: ResMut<Hotbar>,
    pod_net_q: Query<Entity, With<PodNetwork>>,
) {
    let sampler = TerrainSampler::new(world_config.world_seed);
    let ground_y = sampler.height_at(0.0, 0.0);

    // Pod hull always spawns fresh — it is not saved
    commands.spawn((
        EscapePod,
        AegisEmitter,
        AegisRadius(AEGIS_RADIUS),
        AegisActive,
        Transform::from_translation(Vec3::new(0.0, ground_y + 1.5, 0.0)),
        RigidBody::Static,
        aegis_sphere_collider(AEGIS_RADIUS),
        CollisionLayers::new(GameLayer::AegisBoundary, GameLayer::Player),
        Mesh3d(meshes.add(Cuboid::new(3.0, 3.0, 3.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.55, 0.65),
            metallic: 0.9,
            perceptual_roughness: 0.2,
            ..default()
        })),
        DespawnOnExit(GameState::Playing),
    ));

    // On a loaded run the pod network entity is deserialized from the save and found here.
    // On a new run it must be created. wire_pod_machines (OnEnter Playing) handles port wiring
    // in both cases — ports already have LogisticsNetworkMember on load so it's a no-op there.
    let new_run = pod_net_q.single().is_err();
    let pod_net = match pod_net_q.single() {
        Ok(e) => e,
        Err(_) => commands.spawn((LogisticsNetwork, PodNetwork, Save)).id(),
    };
    commands.insert_resource(PodLogisticsNetwork(pod_net));

    if new_run {
        // Fresh run: the lander carries a starting kit of placeable machines + cables.
        // Placement draws items from any StorageUnit, so this stash need not be networked.
        commands.spawn((
            StorageUnit {
                items: starting_kit(),
            },
            Transform::from_translation(Vec3::new(0.0, ground_y + 1.5, 0.0)),
            DespawnOnExit(GameState::Playing),
        ));
        // Bind the hotbar to the kit so the player can place immediately after landing.
        let kit_order = [
            "miner",
            "solar_generator",
            "assembler",
            "analysis_station",
            "logistics_cable",
            "power_cable",
        ];
        for (slot, item) in hotbar.slots.iter_mut().zip(kit_order) {
            *slot = Some(HotbarSlot {
                item_id: item.to_string(),
            });
        }
    }
    // Loaded run: pod machines already in world from save. wire_pod_machines handles port assignment.
}

/// Assigns all PodPowered machine ports to the pod's private logistics network.
/// Runs OnEnter(Playing) after spawn_escape_pod (OnTransition) has flushed commands,
/// ensuring port entities created by on_machine_added are available.
fn wire_pod_machines(
    mut commands: Commands,
    pod_net: Option<Res<PodLogisticsNetwork>>,
    port_q: Query<(Entity, &LogisticsPortOf), Without<LogisticsNetworkMember>>,
    pod_powered_q: Query<(), With<PodPowered>>,
    mut changed: MessageWriter<NetworkChanged<Logistics>>,
) {
    let Some(pod_net) = pod_net else { return };
    let mut assigned = false;
    for (port_e, port_of) in &port_q {
        if pod_powered_q.get(port_of.0).is_ok() {
            commands
                .entity(port_e)
                .insert(LogisticsNetworkMember(pod_net.0));
            assigned = true;
        }
    }
    if assigned {
        changed.write(NetworkChanged::<Logistics>::new(pod_net.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_kit_has_bootstrap_essentials() {
        let kit = starting_kit();
        assert!(kit.values().all(|&v| v > 0));
        // The kit must let the player stand up the full research bootstrap loop.
        for essential in ["miner", "solar_generator", "assembler", "analysis_station"] {
            assert!(kit.contains_key(essential), "kit missing {essential}");
        }
    }
}

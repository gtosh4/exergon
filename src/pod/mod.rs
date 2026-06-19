use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use moonshine_save::prelude::Save;

use crate::aegis::{AEGIS_RADIUS, AegisActive, AegisEmitter, AegisRadius, aegis_sphere_collider};
use crate::logistics::{LogisticsNetwork, LogisticsNetworkMember, StorageUnit};
use crate::machine::{
    LogisticsPortOf, Machine, MachineState, ManualCraftOnly, Mirror, Orientation, Rotation,
};
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

fn starting_stock() -> HashMap<String, u32> {
    HashMap::from([
        ("iron_ore".to_string(), 50),
        ("copper_ore".to_string(), 50),
        ("stone".to_string(), 100),
        ("coal".to_string(), 50),
    ])
}

fn pod_orientation() -> Orientation {
    Orientation {
        rotation: Rotation::North,
        mirror: Mirror::Normal,
    }
}

fn spawn_escape_pod(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world_config: Res<WorldConfig>,
    pod_machines_q: Query<&Machine, With<PodPowered>>,
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
    let pod_net = match pod_net_q.single() {
        Ok(e) => e,
        Err(_) => commands.spawn((LogisticsNetwork, PodNetwork, Save)).id(),
    };
    commands.insert_resource(PodLogisticsNetwork(pod_net));

    if pod_machines_q.is_empty() {
        let machine_y = ground_y + 1.0;
        // Single all-in-one pod terminal: stores starting items and can run assembler recipes.
        commands.spawn((
            Machine {
                machine_type: "assembler".to_string(),
                tier: 1,
                orientation: pod_orientation(),
                energy_ports: vec![],
                logistics_ports: vec![
                    Vec3::new(-4.0, machine_y, 0.0),
                    Vec3::new(-8.0, machine_y, 0.0),
                    Vec3::new(-6.0, machine_y, -2.0),
                    Vec3::new(-6.0, machine_y, 2.0),
                ],
            },
            MachineState::Idle,
            StorageUnit {
                items: starting_stock(),
            },
            PodPowered,
            ManualCraftOnly,
            Transform::from_translation(Vec3::new(-6.0, machine_y, 0.0)),
            RigidBody::Static,
            Collider::cuboid(2.0, 2.0, 2.0),
        ));
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
    fn starting_stock_nonempty_positive_quantities() {
        let stock = starting_stock();
        assert!(!stock.is_empty());
        assert!(stock.values().all(|&v| v > 0));
    }
}

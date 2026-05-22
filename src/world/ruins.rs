use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::drone::Drone;
use crate::escape::EscapeObjective;
use crate::machine::{Machine, MachineState, Mirror, Orientation, Rotation};
use crate::research::{Discovered, DiscoveryEvent};
use crate::save::Run;
use crate::seed::{DomainSeeds, derive};
use crate::world::generation::TerrainSampler;

const RUINS_DISCOVERY_RADIUS: f32 = 8.0;

#[derive(Component)]
pub struct GatewayRuins;

#[derive(Component)]
pub struct ScoutSite {
    pub site_id: String,
}

/// World position of the gateway ruins — populated on `OnEnter(Playing)`.
#[derive(Resource)]
pub struct GatewayRuinsPosition(pub Vec3);

pub fn spawn_gateway_ruins_system(mut commands: Commands, run_q: Query<&DomainSeeds, With<Run>>) {
    let world_seed = run_q.single().map(|s| s.world).unwrap_or(0);
    // Deterministic XZ within [-100, 100) → max 141 units from origin, well within 200
    let x = (derive(world_seed, "ruins_x") % 200) as f32 - 100.0;
    let z = (derive(world_seed, "ruins_z") % 200) as f32 - 100.0;
    let sampler = TerrainSampler::new(world_seed);
    let y = sampler.height_at(x as f64, z as f64);
    let pos = Vec3::new(x, y, z);

    commands.insert_resource(GatewayRuinsPosition(pos));
    commands.spawn((GatewayRuins, Transform::from_translation(pos)));

    // Spawn gateway machine at ruins — players cable it into their network to activate escape.
    // Hardcode one logistics port (+X) and one energy port (-X) relative to machine center.
    commands.spawn((
        Machine {
            machine_type: "gateway".to_string(),
            tier: 1,
            orientation: Orientation {
                rotation: Rotation::North,
                mirror: Mirror::Normal,
            },
            energy_ports: vec![Vec3::new(pos.x - 2.0, pos.y, pos.z)],
            logistics_ports: vec![Vec3::new(pos.x + 2.0, pos.y, pos.z)],
        },
        MachineState::Idle,
        Transform::from_translation(pos),
        EscapeObjective,
    ));
}

pub fn spawn_scout_sites(mut commands: Commands, run_q: Query<&DomainSeeds, With<Run>>) {
    let world_seed = run_q.single().map(|s| s.world).unwrap_or(0);
    let sampler = TerrainSampler::new(world_seed);

    // Site 1: mineral-rich deposit (high value, low risk)
    let x1 = (derive(world_seed, "site1_x") % 300) as f32 - 150.0;
    let z1 = (derive(world_seed, "site1_z") % 300) as f32 - 150.0;
    let y1 = sampler.height_at(x1 as f64, z1 as f64);
    commands.spawn((
        ScoutSite {
            site_id: "mineral_deposit".to_string(),
        },
        Transform::from_xyz(x1, y1, z1),
    ));

    // Site 2: alien artifact (high value, high-risk flavor)
    let x2 = (derive(world_seed, "site2_x") % 400) as f32 - 200.0;
    let z2 = (derive(world_seed, "site2_z") % 400) as f32 - 200.0;
    let y2 = sampler.height_at(x2 as f64, z2 as f64);
    commands.spawn((
        ScoutSite {
            site_id: "alien_artifact".to_string(),
        },
        Transform::from_xyz(x2, y2, z2),
    ));
}

pub fn scout_site_discovery_system(
    mut commands: Commands,
    drone_q: Query<&Transform, With<Drone>>,
    sites_q: Query<(Entity, &Transform, &ScoutSite), Without<Discovered>>,
    mut events: MessageWriter<DiscoveryEvent>,
) {
    let Ok(drone) = drone_q.single() else {
        return;
    };
    for (entity, site_transform, site) in &sites_q {
        if drone.translation.distance(site_transform.translation) <= RUINS_DISCOVERY_RADIUS {
            events.write(DiscoveryEvent(site.site_id.clone()));
            commands.entity(entity).insert(Discovered);
        }
    }
}

pub fn ruins_discovery_system(
    mut commands: Commands,
    drone_q: Query<&Transform, With<Drone>>,
    ruins_q: Query<(Entity, &Transform), (With<GatewayRuins>, Without<Discovered>)>,
    mut events: MessageWriter<DiscoveryEvent>,
) {
    let Ok(drone) = drone_q.single() else {
        return;
    };
    for (entity, ruins_transform) in &ruins_q {
        if drone.translation.distance(ruins_transform.translation) <= RUINS_DISCOVERY_RADIUS {
            events.write(DiscoveryEvent("gateway_ruins".to_string()));
            commands.entity(entity).insert(Discovered);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruins_spawn_is_deterministic() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Startup, spawn_gateway_ruins_system);
        app.update();
        let pos_a = app.world().resource::<GatewayRuinsPosition>().0;

        let mut app2 = App::new();
        app2.add_message::<DiscoveryEvent>()
            .add_systems(Startup, spawn_gateway_ruins_system);
        app2.update();
        let pos_b = app2.world().resource::<GatewayRuinsPosition>().0;

        assert_eq!(pos_a, pos_b, "ruins position must be deterministic");
    }

    #[test]
    fn ruins_discovery_fires_once_on_approach() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, ruins_discovery_system);

        let ruins = app
            .world_mut()
            .spawn((GatewayRuins, Transform::from_xyz(0.0, 0.0, 0.0)))
            .id();
        app.world_mut()
            .spawn((Drone, Transform::from_xyz(1.0, 0.0, 0.0)));

        // First frame — within radius → Discovered inserted
        app.update();
        assert!(
            app.world().get::<Discovered>(ruins).is_some(),
            "ruins should be marked Discovered"
        );

        // Second frame — Without<Discovered> excludes ruins → no re-processing
        app.update();
        assert!(app.world().get::<Discovered>(ruins).is_some());
    }

    #[test]
    fn ruins_discovery_not_fired_when_out_of_range() {
        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, ruins_discovery_system);

        let ruins = app
            .world_mut()
            .spawn((GatewayRuins, Transform::from_xyz(0.0, 0.0, 0.0)))
            .id();
        app.world_mut()
            .spawn((Drone, Transform::from_xyz(100.0, 0.0, 0.0)));

        app.update();

        assert!(
            app.world().get::<Discovered>(ruins).is_none(),
            "drone too far — ruins should not be marked Discovered"
        );
    }
}

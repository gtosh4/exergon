use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::drone::Drone;
use crate::research::{Discovered, DiscoveryEvent};
use crate::seed::{DomainSeeds, derive};
use crate::world::generation::TerrainSampler;

const RUINS_DISCOVERY_RADIUS: f32 = 8.0;

#[derive(Component)]
pub struct GatewayRuins;

/// World position of the gateway ruins — populated on `OnEnter(Playing)`.
#[derive(Resource)]
pub struct GatewayRuinsPosition(pub Vec3);

pub fn spawn_gateway_ruins_system(mut commands: Commands, domain_seeds: Option<Res<DomainSeeds>>) {
    let world_seed = domain_seeds.map(|s| s.world).unwrap_or(0);
    // Deterministic XZ within [-100, 100) → max 141 units from origin, well within 200
    let x = (derive(world_seed, "ruins_x") % 200) as f32 - 100.0;
    let z = (derive(world_seed, "ruins_z") % 200) as f32 - 100.0;
    let sampler = TerrainSampler::new(world_seed);
    let y = sampler.height_at(x as f64, z as f64);
    let pos = Vec3::new(x, y, z);

    commands.insert_resource(GatewayRuinsPosition(pos));
    commands.spawn((GatewayRuins, Transform::from_translation(pos)));
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

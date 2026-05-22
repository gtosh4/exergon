use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy_tnua::TnuaUserControlsSystems;
use bevy_tnua::builtins::{TnuaBuiltinWalk, TnuaBuiltinWalkConfig};
use bevy_tnua::prelude::*;
use bevy_tnua_avian3d::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;

use bevy::ecs::message::MessageWriter;

use crate::aegis::AegisActive;
use crate::logistics::StorageUnit;
use crate::research::{Discovered, DiscoveryEvent};
use crate::world::MainCamera;
use crate::world::generation::OreDeposit;
use crate::{GameState, PlayMode};

const MINE_REACH: f32 = 4.0;
const DEPOSIT_RADIUS: f32 = 15.0;
const CHARACTER_FOG_REVEAL_RADIUS: f32 = 4.0;

pub struct DronePlugin;

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
pub enum DroneScheme {}

#[derive(Component)]
pub struct Drone;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DroneState {
    #[default]
    Idle,
    ActivelyControlled,
}

#[derive(Component, Default, Debug)]
pub struct DroneInventory {
    pub items: HashMap<String, u32>,
}

#[derive(Component, Debug)]
pub struct FogRevealRadius(pub f32);

/// Grid-based fog of war. One cell per 4m×4m world tile.
/// Indexed by chunk (16m×16m = 4×4 cells) → bitmask of 4×4 sub-tiles.
#[derive(Resource, Default)]
pub struct FogOfWar {
    pub revealed: HashMap<IVec2, u16>,
}

/// Entity of the currently controlled drone. None when in Local mode.
#[derive(Resource, Default)]
pub struct ActiveDrone(pub Option<Entity>);

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct FogCellRevealedEvent {
    pub cell: IVec2,
}

impl Plugin for DronePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            TnuaControllerPlugin::<DroneScheme>::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .init_resource::<FogOfWar>()
        .init_resource::<ActiveDrone>()
        .add_message::<FogCellRevealedEvent>()
        .add_systems(OnEnter(GameState::Playing), spawn_land_drone)
        .add_systems(
            Update,
            (
                toggle_drone_mode,
                drone_pilot_input
                    .in_set(TnuaUserControlsSystems)
                    .run_if(in_state(PlayMode::DronePilot)),
                drone_mine_system.run_if(in_state(PlayMode::DronePilot)),
                deposit_discovery_system.run_if(in_state(PlayMode::DronePilot)),
                fog_reveal_system.run_if(in_state(PlayMode::DronePilot)),
                character_fog_reveal_system.run_if(in_state(PlayMode::Exploring)),
                drone_deposit_system.run_if(in_state(PlayMode::DronePilot)),
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn spawn_land_drone(mut commands: Commands, mut scheme_configs: ResMut<Assets<DroneSchemeConfig>>) {
    commands.spawn((
        Transform::from_xyz(0.0, 100.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.8),
        TnuaController::<DroneScheme>::default(),
        TnuaConfig::<DroneScheme>(scheme_configs.add(DroneSchemeConfig {
            basis: TnuaBuiltinWalkConfig {
                float_height: 1.0,
                speed: 15.0,
                ..Default::default()
            },
        })),
        TnuaAvian3dSensorShape(Collider::cylinder(0.39, 0.0)),
        LockedAxes::ROTATION_LOCKED,
        Drone,
        DroneState::Idle,
        DroneInventory::default(),
        FogRevealRadius(12.0),
    ));
}

fn toggle_drone_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    mode: Res<State<PlayMode>>,
    mut next_mode: ResMut<NextState<PlayMode>>,
    drone_q: Query<Entity, With<Drone>>,
    mut active_drone: ResMut<ActiveDrone>,
    mut drone_state_q: Query<&mut DroneState>,
) {
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    match mode.get() {
        PlayMode::Exploring => {
            let Ok(drone_entity) = drone_q.single() else {
                return;
            };
            active_drone.0 = Some(drone_entity);
            if let Ok(mut state) = drone_state_q.get_mut(drone_entity) {
                *state = DroneState::ActivelyControlled;
            }
            next_mode.set(PlayMode::DronePilot);
        }
        PlayMode::DronePilot => {
            if let Some(drone_entity) = active_drone.0
                && let Ok(mut state) = drone_state_q.get_mut(drone_entity)
            {
                *state = DroneState::Idle;
            }
            next_mode.set(PlayMode::Exploring);
        }
        _ => {}
    }
}

fn drone_pilot_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<AccumulatedMouseMotion>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    drone_q: Query<&Transform, (With<Drone>, Without<MainCamera>)>,
    mut controller_q: Query<&mut TnuaController<DroneScheme>, With<Drone>>,
) {
    let Ok(drone_transform) = drone_q.single() else {
        return;
    };
    let Ok(mut camera) = camera_q.single_mut() else {
        return;
    };
    let Ok(mut controller) = controller_q.single_mut() else {
        return;
    };

    let yaw = -mouse.delta.x * 0.003;
    let pitch = -mouse.delta.y * 0.003;
    if yaw != 0.0 || pitch != 0.0 {
        let (current_yaw, current_pitch, _) = camera.rotation.to_euler(EulerRot::YXZ);
        let new_pitch = (current_pitch + pitch).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        camera.rotation = Quat::from_euler(EulerRot::YXZ, current_yaw + yaw, new_pitch, 0.0);
    }

    camera.translation = drone_transform.translation + Vec3::Y * 0.5;

    let cam_fwd = *camera.forward();
    let forward = Vec3::new(cam_fwd.x, 0.0, cam_fwd.z).normalize_or_zero();
    let cam_right = *camera.right();
    let right = Vec3::new(cam_right.x, 0.0, cam_right.z).normalize_or_zero();

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += right;
    }

    controller.basis = TnuaBuiltinWalk {
        desired_motion: direction.normalize_or_zero(),
        ..Default::default()
    };
}

/// Asymptotic yield factor: starts near 1.0, decays toward a floor > 0.
/// Floor and decay rate vary per deposit via `depletion_seed`.
pub(crate) fn yield_factor(total_extracted: f32, depletion_seed: u64) -> f32 {
    let floor = 0.1 + (depletion_seed % 100) as f32 * 0.001;
    let k = 0.02 + (depletion_seed % 50) as f32 * 0.001;
    floor + (1.0 - floor) * (-k * total_extracted).exp()
}

pub(crate) fn sample_ore<R: Rng>(ores: &[(String, f32)], rng: &mut R) -> Option<String> {
    let total: f32 = ores.iter().map(|(_, w)| w).sum();
    if total <= 0.0 {
        return None;
    }
    let roll = rng.gen_range(0.0f32..total);
    let mut acc = 0.0;
    for (id, w) in ores {
        acc += w;
        if roll < acc {
            return Some(id.clone());
        }
    }
    ores.last().map(|(id, _)| id.clone())
}

fn drone_mine_system(
    mouse: Res<ButtonInput<MouseButton>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    spatial_query: SpatialQuery,
    mut deposit_q: Query<&mut OreDeposit>,
    mut drone_q: Query<&mut DroneInventory, With<Drone>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(cam) = camera_q.single() else {
        return;
    };
    let dir = Dir3::new(*cam.forward()).unwrap_or(Dir3::NEG_Z);
    let Some(hit) =
        spatial_query.cast_ray(cam.translation, dir, MINE_REACH, true, &Default::default())
    else {
        return;
    };
    let Ok(mut deposit) = deposit_q.get_mut(hit.entity) else {
        return;
    };
    let Ok(mut inventory) = drone_q.single_mut() else {
        return;
    };
    let rng_seed = deposit.depletion_seed ^ deposit.total_extracted.to_bits() as u64;
    let mut rng = Pcg64::seed_from_u64(rng_seed);
    if let Some(ore_id) = sample_ore(&deposit.ores, &mut rng) {
        *inventory.items.entry(ore_id).or_insert(0) += 1;
        deposit.total_extracted += 1.0;
    }
}

/// When drone is within DEPOSIT_RADIUS of the aegis emitter and player presses E,
/// move all DroneInventory items into the nearest StorageUnit.
fn drone_deposit_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut drone_q: Query<(&Transform, &mut DroneInventory), With<Drone>>,
    aegis_q: Query<&Transform, (With<crate::aegis::AegisEmitter>, With<AegisActive>)>,
    mut storage_q: Query<&mut StorageUnit>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Ok((drone_transform, mut inventory)) = drone_q.single_mut() else {
        return;
    };
    if inventory.items.is_empty() {
        return;
    }
    let near_base = aegis_q.iter().any(|aegis_transform| {
        drone_transform
            .translation
            .distance(aegis_transform.translation)
            <= DEPOSIT_RADIUS
    });
    if !near_base {
        return;
    }
    let Ok(mut storage) = storage_q.single_mut() else {
        return;
    };
    for (item_id, count) in inventory.items.drain() {
        *storage.items.entry(item_id).or_insert(0) += count;
    }
}

fn fog_reveal_system(
    drone_q: Query<(&Transform, &FogRevealRadius), With<Drone>>,
    mut fog: ResMut<FogOfWar>,
    mut revealed_events: MessageWriter<FogCellRevealedEvent>,
) {
    let Ok((transform, FogRevealRadius(radius))) = drone_q.single() else {
        return;
    };
    reveal_area(
        &mut fog,
        transform.translation,
        *radius,
        &mut revealed_events,
    );
}

fn character_fog_reveal_system(
    player_q: Query<&Transform, With<crate::world::Player>>,
    mut fog: ResMut<FogOfWar>,
    mut revealed_events: MessageWriter<FogCellRevealedEvent>,
) {
    let Ok(transform) = player_q.single() else {
        return;
    };
    reveal_area(
        &mut fog,
        transform.translation,
        CHARACTER_FOG_REVEAL_RADIUS,
        &mut revealed_events,
    );
}

fn reveal_area(
    fog: &mut FogOfWar,
    center: Vec3,
    radius: f32,
    events: &mut MessageWriter<FogCellRevealedEvent>,
) {
    let center_cell = world_to_cell(center);
    let cell_radius = (radius / 4.0).ceil() as i32;
    for dz in -cell_radius..=cell_radius {
        for dx in -cell_radius..=cell_radius {
            let cell = center_cell + IVec2::new(dx, dz);
            let cell_world = Vec2::new((cell.x as f32 + 0.5) * 4.0, (cell.y as f32 + 0.5) * 4.0);
            if cell_world.distance(Vec2::new(center.x, center.z)) > radius {
                continue;
            }
            let chunk = IVec2::new(cell.x.div_euclid(4), cell.y.div_euclid(4));
            let bit = (cell.x.rem_euclid(4) + cell.y.rem_euclid(4) * 4) as u32;
            let entry = fog.revealed.entry(chunk).or_insert(0);
            if *entry & (1 << bit) == 0 {
                *entry |= 1 << bit;
                events.write(FogCellRevealedEvent { cell });
            }
        }
    }
}

fn world_to_cell(pos: Vec3) -> IVec2 {
    IVec2::new((pos.x / 4.0).floor() as i32, (pos.z / 4.0).floor() as i32)
}

const DISCOVERY_RADIUS: f32 = 8.0;

fn deposit_discovery_system(
    mut commands: Commands,
    drone_q: Query<&Transform, With<Drone>>,
    deposit_q: Query<(Entity, &Transform, &OreDeposit), Without<Discovered>>,
    mut events: MessageWriter<DiscoveryEvent>,
) {
    let Ok(drone) = drone_q.single() else {
        return;
    };
    for (entity, deposit_transform, deposit) in &deposit_q {
        if !deposit.ores.iter().any(|(id, _)| id == "xalite") {
            continue;
        }
        if drone.translation.distance(deposit_transform.translation) <= DISCOVERY_RADIUS {
            events.write(DiscoveryEvent("xalite_deposit".to_string()));
            commands.entity(entity).insert(Discovered);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[test]
    fn f_key_enters_drone_pilot_mode() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ActiveDrone>()
            .add_systems(
                Update,
                toggle_drone_mode.run_if(in_state(GameState::Playing)),
            );

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<PlayMode>>()
            .set(PlayMode::Exploring);
        app.update();

        app.world_mut().spawn((Drone, DroneState::Idle));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::DronePilot
        );
    }

    #[test]
    fn f_key_exits_drone_pilot_mode() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<GameState>()
            .add_sub_state::<PlayMode>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ActiveDrone>()
            .add_systems(
                Update,
                toggle_drone_mode.run_if(in_state(GameState::Playing)),
            );

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<PlayMode>>()
            .set(PlayMode::Exploring);
        app.update();

        app.world_mut().spawn((Drone, DroneState::Idle));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::DronePilot
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyF);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<PlayMode>>().get(),
            PlayMode::Exploring
        );
    }

    #[test]
    fn yield_factor_decreases_monotonically() {
        let seed = 12345u64;
        let y: Vec<f32> = (0..5)
            .map(|i| yield_factor(i as f32 * 20.0, seed))
            .collect();
        for w in y.windows(2) {
            assert!(w[0] > w[1], "yield should decrease: {} > {}", w[0], w[1]);
        }
        assert!(y[4] > 0.0, "yield floor must be above zero");
    }

    #[test]
    fn sample_ore_single_entry_always_returns_it() {
        let ores = vec![("iron_ore".to_string(), 1.0f32)];
        let mut rng = Pcg64::seed_from_u64(0);
        assert_eq!(sample_ore(&ores, &mut rng), Some("iron_ore".to_string()));
    }

    #[test]
    fn sample_ore_empty_returns_none() {
        let ores: Vec<(String, f32)> = vec![];
        let mut rng = Pcg64::seed_from_u64(0);
        assert_eq!(sample_ore(&ores, &mut rng), None);
    }

    #[test]
    fn mine_samples_ore_and_increments_extracted() {
        use crate::world::generation::OreDeposit;

        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("copper_ore".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 0,
        };

        let rng_seed = deposit.depletion_seed ^ deposit.total_extracted.to_bits() as u64;
        let mut rng = Pcg64::seed_from_u64(rng_seed);
        let ore = sample_ore(&deposit.ores, &mut rng);
        if ore.is_some() {
            deposit.total_extracted += 1.0;
        }

        assert_eq!(ore.as_deref(), Some("copper_ore"));
        assert_eq!(deposit.total_extracted, 1.0);
        assert!(!deposit.ores.is_empty(), "deposit must persist");
    }

    #[test]
    fn repeated_mining_degrades_yield() {
        use crate::world::generation::OreDeposit;

        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("iron_ore".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 42,
        };
        let y_before = yield_factor(deposit.total_extracted, deposit.depletion_seed);
        for _ in 0..10 {
            deposit.total_extracted += 1.0;
        }
        let y_after = yield_factor(deposit.total_extracted, deposit.depletion_seed);
        assert!(
            y_before > y_after,
            "yield must degrade: {} > {}",
            y_before,
            y_after
        );
        assert!(y_after > 0.0, "yield floor must remain above zero");
    }

    #[test]
    fn deposit_discovery_fires_once_for_xalite() {
        use crate::research::{Discovered, DiscoveryEvent};
        use crate::world::generation::OreDeposit;

        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, deposit_discovery_system);

        app.world_mut()
            .spawn((Drone, Transform::from_xyz(0.0, 0.0, 0.0)));
        let deposit = app
            .world_mut()
            .spawn((
                OreDeposit {
                    chunk_pos: IVec2::ZERO,
                    ores: vec![("xalite".to_string(), 1.0)],
                    total_extracted: 0.0,
                    depletion_seed: 0,
                },
                Transform::from_xyz(1.0, 0.0, 0.0),
            ))
            .id();

        app.update();
        assert!(
            app.world().get::<Discovered>(deposit).is_some(),
            "deposit should be marked Discovered"
        );

        app.update();
        assert!(app.world().get::<Discovered>(deposit).is_some());
    }

    #[test]
    fn deposit_discovery_ignores_non_xalite() {
        use crate::research::{Discovered, DiscoveryEvent};
        use crate::world::generation::OreDeposit;

        let mut app = App::new();
        app.add_message::<DiscoveryEvent>()
            .add_systems(Update, deposit_discovery_system);

        app.world_mut()
            .spawn((Drone, Transform::from_xyz(0.0, 0.0, 0.0)));
        let deposit = app
            .world_mut()
            .spawn((
                OreDeposit {
                    chunk_pos: IVec2::ZERO,
                    ores: vec![("iron".to_string(), 1.0)],
                    total_extracted: 0.0,
                    depletion_seed: 0,
                },
                Transform::from_xyz(1.0, 0.0, 0.0),
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<Discovered>(deposit).is_none(),
            "iron deposit should not trigger discovery"
        );
    }

    #[test]
    fn fog_reveal_marks_cells_in_radius() {
        let mut app = App::new();
        app.init_resource::<FogOfWar>()
            .add_message::<FogCellRevealedEvent>()
            .add_systems(Update, fog_reveal_system);

        app.world_mut().spawn((
            Drone,
            Transform::from_xyz(0.0, 0.0, 0.0),
            FogRevealRadius(8.0),
        ));
        app.update();

        let fog = app.world().resource::<FogOfWar>();
        assert!(!fog.revealed.is_empty(), "fog should have revealed cells");
    }

    #[test]
    fn drone_deposit_moves_items_to_storage() {
        use crate::aegis::{AegisActive, AegisEmitter, AegisRadius};

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_message::<FogCellRevealedEvent>()
            .add_systems(Update, drone_deposit_system);

        let mut inventory = DroneInventory::default();
        inventory.items.insert("iron_ore".to_string(), 5);
        let drone_e = app
            .world_mut()
            .spawn((Drone, Transform::from_xyz(0.0, 0.0, 0.0), inventory))
            .id();

        app.world_mut().spawn((
            AegisEmitter,
            AegisActive,
            AegisRadius(60.0),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));

        let mut storage = StorageUnit {
            items: HashMap::new(),
        };
        storage.items.insert("copper_ore".to_string(), 3);
        let storage_e = app.world_mut().spawn(storage).id();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyE);
        app.update();

        assert!(
            app.world()
                .get::<DroneInventory>(drone_e)
                .unwrap()
                .items
                .is_empty(),
            "drone inventory should be empty after deposit"
        );
        assert_eq!(
            app.world()
                .get::<StorageUnit>(storage_e)
                .unwrap()
                .items
                .get("iron_ore")
                .copied()
                .unwrap_or(0),
            5,
            "storage should have received iron_ore"
        );
    }
}

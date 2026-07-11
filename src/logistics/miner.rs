use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use rand::SeedableRng;
use rand_pcg::Pcg64;

use crate::drone::{sample_ore, yield_factor};
use crate::machine::{LogisticsPortOf, MachineLogisticsPorts, MinerMachine};
use crate::research::ProductionTally;
use crate::world::generation::OreDeposit;

use super::items::give_items;
use super::{LogisticsNetworkMember, LogisticsNetworkMembers, NetworkStorageChanged, StorageUnit};

fn tick_miner(deposit: &mut OreDeposit, accumulator: &mut f32, dt: f32) -> Option<String> {
    let yf = yield_factor(deposit.total_extracted, deposit.depletion_seed);
    *accumulator += yf * dt;
    if *accumulator >= 1.0 {
        *accumulator -= 1.0;
        let rng_seed = deposit.depletion_seed ^ deposit.total_extracted.to_bits() as u64;
        let mut rng = Pcg64::seed_from_u64(rng_seed);
        if let Some(ore_id) = sample_ore(&deposit.ores, &mut rng) {
            deposit.total_extracted += 1.0;
            return Some(ore_id);
        }
    }
    None
}

pub(super) fn miner_tick_system(
    time: Res<Time>,
    mut miner_q: Query<(&mut MinerMachine, &MachineLogisticsPorts)>,
    port_net_q: Query<&LogisticsNetworkMember>,
    mut deposit_q: Query<&mut OreDeposit>,
    net_q: Query<&LogisticsNetworkMembers>,
    mut storage_q: Query<&mut StorageUnit>,
    port_of_q: Query<&LogisticsPortOf>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
    mut tally: Option<ResMut<ProductionTally>>,
) {
    let dt = time.delta_secs();
    for (mut miner, ports) in &mut miner_q {
        // Resolve the miner's logistics network via any wired port. Skip (and don't
        // deplete the deposit) while unconnected — extracted ore would have nowhere to go.
        let Some(net_e) = ports
            .ports()
            .iter()
            .find_map(|&p| port_net_q.get(p).ok().map(|m| m.0))
        else {
            continue;
        };
        let Ok(members) = net_q.get(net_e) else {
            continue;
        };
        let Ok(mut deposit) = deposit_q.get_mut(miner.deposit) else {
            continue;
        };
        if let Some(ore_id) = tick_miner(&mut deposit, &mut miner.accumulator, dt) {
            give_items(members, &mut storage_q, &port_of_q, &ore_id, 1);
            if let Some(ref mut tally) = tally {
                tally.record(&ore_id, 1.0);
            }
            storage_changed.write(NetworkStorageChanged { network: net_e });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bevy::prelude::*;

    use super::*;
    use crate::logistics::{LogisticsNetworkMember, NetworkStorageChanged, StorageUnit};
    use crate::machine::MinerMachine;

    #[test]
    fn tick_miner_outputs_ore_when_accumulator_overflows() {
        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("iron_ore".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 0,
        };
        let mut acc = 0.5;
        let result = tick_miner(&mut deposit, &mut acc, 0.6);
        assert_eq!(result.as_deref(), Some("iron_ore"));
        assert!(acc < 1.0, "accumulator drained after output");
        assert_eq!(deposit.total_extracted, 1.0);
    }

    #[test]
    fn tick_miner_no_output_below_threshold() {
        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("iron_ore".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 0,
        };
        let mut acc = 0.0;
        let result = tick_miner(&mut deposit, &mut acc, 0.5);
        assert!(result.is_none());
        assert_eq!(deposit.total_extracted, 0.0);
    }

    #[test]
    fn tick_miner_yield_floor_still_produces() {
        let mut deposit = OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("iron_ore".to_string(), 1.0)],
            total_extracted: 1_000_000.0,
            depletion_seed: 99,
        };
        let mut acc = 0.0;
        let result = tick_miner(&mut deposit, &mut acc, 10.0);
        assert!(result.is_some(), "floor yield must still produce over time");
    }

    /// Wires a miner and a storage machine onto a shared logistics network via ports,
    /// mirroring how cable connections assign membership in the real game.
    fn spawn_miner_and_storage(
        app: &mut App,
        deposit_entity: Entity,
        accumulator: f32,
    ) -> (Entity, Entity) {
        let net_entity = app.world_mut().spawn_empty().id();
        let storage_machine = app
            .world_mut()
            .spawn(StorageUnit {
                items: HashMap::new(),
            })
            .id();
        app.world_mut().spawn((
            LogisticsPortOf(storage_machine),
            LogisticsNetworkMember(net_entity),
        ));
        let miner = app
            .world_mut()
            .spawn(MinerMachine {
                deposit: deposit_entity,
                accumulator,
            })
            .id();
        // Miner's own logistics port joins the same network → MachineLogisticsPorts is set.
        app.world_mut()
            .spawn((LogisticsPortOf(miner), LogisticsNetworkMember(net_entity)));
        (miner, storage_machine)
    }

    #[test]
    fn miner_tick_system_produces_and_stores_ore() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<NetworkStorageChanged>()
            .add_systems(Update, miner_tick_system);

        let deposit_entity = app
            .world_mut()
            .spawn(OreDeposit {
                chunk_pos: IVec2::ZERO,
                ores: vec![("iron_ore".to_string(), 1.0)],
                total_extracted: 0.0,
                depletion_seed: 0,
            })
            .id();
        // accumulator >= 1.0 triggers immediately even at dt=0
        let (_miner, storage_machine) = spawn_miner_and_storage(&mut app, deposit_entity, 1.0);

        app.update();

        let world = app.world();
        let storage = world.get::<StorageUnit>(storage_machine).unwrap();
        assert_eq!(storage.items.get("iron_ore").copied().unwrap_or(0), 1);
        assert_eq!(
            world
                .get::<OreDeposit>(deposit_entity)
                .unwrap()
                .total_extracted,
            1.0
        );
    }

    #[test]
    fn miner_tick_system_records_production() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<NetworkStorageChanged>()
            .init_resource::<ProductionTally>()
            .add_systems(Update, miner_tick_system);

        let deposit_entity = app
            .world_mut()
            .spawn(OreDeposit {
                chunk_pos: IVec2::ZERO,
                ores: vec![("iron_ore".to_string(), 1.0)],
                total_extracted: 0.0,
                depletion_seed: 0,
            })
            .id();
        spawn_miner_and_storage(&mut app, deposit_entity, 1.0);

        app.update();

        assert_eq!(
            app.world().resource::<ProductionTally>().get("iron_ore"),
            1.0,
            "mined ore must be tallied as production"
        );
    }

    #[test]
    fn miner_tick_system_skips_when_deposit_missing() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<NetworkStorageChanged>()
            .add_systems(Update, miner_tick_system);

        // deposit_entity has no OreDeposit component → continue branch
        let deposit_entity = app.world_mut().spawn_empty().id();
        spawn_miner_and_storage(&mut app, deposit_entity, 1.0);

        app.update(); // should not panic
    }

    #[test]
    fn miner_tick_system_skips_when_unconnected() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<NetworkStorageChanged>()
            .add_systems(Update, miner_tick_system);

        let deposit_entity = app
            .world_mut()
            .spawn(OreDeposit {
                chunk_pos: IVec2::ZERO,
                ores: vec![("iron_ore".to_string(), 1.0)],
                total_extracted: 0.0,
                depletion_seed: 0,
            })
            .id();
        // Miner with no logistics port → no MachineLogisticsPorts → not matched, deposit untouched.
        app.world_mut().spawn(MinerMachine {
            deposit: deposit_entity,
            accumulator: 1.0,
        });

        app.update();

        assert_eq!(
            app.world()
                .get::<OreDeposit>(deposit_entity)
                .unwrap()
                .total_extracted,
            0.0,
            "unconnected miner must not deplete the deposit"
        );
    }
}

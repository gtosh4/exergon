use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use rand::SeedableRng;
use rand_pcg::Pcg64;

use crate::drone::{sample_ore, yield_factor};
use crate::machine::MinerMachine;
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
    mut miner_q: Query<(&mut MinerMachine, &LogisticsNetworkMember)>,
    mut deposit_q: Query<&mut OreDeposit>,
    net_q: Query<&LogisticsNetworkMembers>,
    mut storage_q: Query<&mut StorageUnit>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
) {
    let dt = time.delta_secs();
    for (mut miner, member) in &mut miner_q {
        let Ok(mut deposit) = deposit_q.get_mut(miner.deposit) else {
            continue;
        };
        if let Some(ore_id) = tick_miner(&mut deposit, &mut miner.accumulator, dt) {
            let Ok(members) = net_q.get(member.0) else {
                continue;
            };
            give_items(members, &mut storage_q, &ore_id, 1);
            storage_changed.write(NetworkStorageChanged { network: member.0 });
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

    #[test]
    fn miner_tick_system_produces_and_stores_ore() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<NetworkStorageChanged>()
            .add_systems(Update, miner_tick_system);

        let net_entity = app.world_mut().spawn_empty().id();
        let deposit_entity = app
            .world_mut()
            .spawn(OreDeposit {
                chunk_pos: IVec2::ZERO,
                ores: vec![("iron_ore".to_string(), 1.0)],
                total_extracted: 0.0,
                depletion_seed: 0,
            })
            .id();
        let storage_entity = app
            .world_mut()
            .spawn((
                StorageUnit {
                    items: HashMap::new(),
                },
                LogisticsNetworkMember(net_entity),
            ))
            .id();
        app.world_mut().spawn((
            MinerMachine {
                deposit: deposit_entity,
                // accumulator >= 1.0 triggers immediately even at dt=0
                accumulator: 1.0,
            },
            LogisticsNetworkMember(net_entity),
        ));

        app.update();

        let world = app.world();
        let storage = world.get::<StorageUnit>(storage_entity).unwrap();
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
    fn miner_tick_system_skips_when_deposit_missing() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<NetworkStorageChanged>()
            .add_systems(Update, miner_tick_system);

        let net_entity = app.world_mut().spawn_empty().id();
        // deposit_entity has no OreDeposit component → continue branch
        let deposit_entity = app.world_mut().spawn_empty().id();
        app.world_mut().spawn((
            MinerMachine {
                deposit: deposit_entity,
                accumulator: 1.0,
            },
            LogisticsNetworkMember(net_entity),
        ));

        app.update(); // should not panic
    }
}

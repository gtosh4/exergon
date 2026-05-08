use std::collections::HashMap;

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::machine::{LogisticsPortOf, Machine};

use super::{LogisticsNetworkMember, NetworkStorageChanged, STORAGE_CRATE_ID, StorageUnit};

pub(super) fn storage_unit_system(
    mut commands: Commands,
    added_machines: Query<(Entity, &Machine), (Added<Machine>, Without<StorageUnit>)>,
    added_port_members: Query<
        (&LogisticsPortOf, &LogisticsNetworkMember),
        Added<LogisticsNetworkMember>,
    >,
    storage_q: Query<(), With<StorageUnit>>,
    mut changed: MessageWriter<NetworkStorageChanged>,
) {
    for (entity, machine) in &added_machines {
        if machine.machine_type != STORAGE_CRATE_ID {
            continue;
        }
        debug!("storage_unit: spawning StorageUnit for {:?}", entity);
        commands.entity(entity).insert(StorageUnit {
            items: HashMap::new(),
        });
    }
    for (port_of, member) in &added_port_members {
        let machine_e = port_of.0;
        if storage_q.get(machine_e).is_ok() {
            debug!(
                "storage_unit: storage machine {:?} port joined network {:?}, firing NetworkStorageChanged",
                machine_e, member.0
            );
            changed.write(NetworkStorageChanged { network: member.0 });
        }
    }
}

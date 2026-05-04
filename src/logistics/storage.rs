use std::collections::HashMap;

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::machine::Machine;

use super::{LogisticsNetworkMember, NetworkStorageChanged, STORAGE_CRATE_ID, StorageUnit};

pub(super) fn storage_unit_system(
    mut commands: Commands,
    added_machines: Query<(Entity, &Machine), (Added<Machine>, Without<StorageUnit>)>,
    added_members: Query<
        (Entity, &LogisticsNetworkMember),
        (Added<LogisticsNetworkMember>, With<StorageUnit>),
    >,
    member_q: Query<&LogisticsNetworkMember>,
    mut changed: MessageWriter<NetworkStorageChanged>,
) {
    for (entity, machine) in &added_machines {
        if machine.machine_type != STORAGE_CRATE_ID {
            continue;
        }
        commands.entity(entity).insert(StorageUnit {
            items: HashMap::new(),
        });
        if let Ok(member) = member_q.get(entity) {
            changed.write(NetworkStorageChanged { network: member.0 });
        }
    }
    for (_, member) in &added_members {
        changed.write(NetworkStorageChanged { network: member.0 });
    }
}

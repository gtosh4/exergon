use bevy::prelude::*;

use super::{LogisticsNetworkMembers, StorageUnit};

pub fn has_items(
    members: &LogisticsNetworkMembers,
    storage_q: &Query<&StorageUnit>,
    item_id: &str,
    count: u32,
) -> bool {
    let available: u32 = members
        .0
        .iter()
        .filter_map(|&e| storage_q.get(e).ok())
        .map(|s| s.items.get(item_id).copied().unwrap_or(0))
        .sum();
    available >= count
}

pub fn take_items(
    members: &LogisticsNetworkMembers,
    storage_q: &mut Query<&mut StorageUnit>,
    item_id: &str,
    count: u32,
) {
    let mut remaining = count;
    for &e in &members.0 {
        if remaining == 0 {
            break;
        }
        if let Ok(mut block) = storage_q.get_mut(e) {
            let avail = *block.items.get(item_id).unwrap_or(&0);
            let take = remaining.min(avail);
            if take > 0 {
                let v = block.items.entry(item_id.to_owned()).or_insert(0);
                *v -= take;
                if *v == 0 {
                    block.items.remove(item_id);
                }
                remaining -= take;
            }
        }
    }
}

pub fn give_items(
    members: &LogisticsNetworkMembers,
    storage_q: &mut Query<&mut StorageUnit>,
    item_id: &str,
    count: u32,
) {
    for &e in &members.0 {
        if let Ok(mut block) = storage_q.get_mut(e) {
            *block.items.entry(item_id.to_owned()).or_insert(0) += count;
            return;
        }
    }
    warn!("No storage for network; {item_id} ×{count} lost");
}

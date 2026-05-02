use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hotbar>()
            .init_resource::<Inventory>()
            .init_resource::<InventoryOpen>();
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct BlockProps {
    pub voxel_id: u8,
    pub hardness: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub block: Option<BlockProps>,
}

#[derive(Resource, Default)]
pub struct ItemRegistry {
    items: HashMap<String, ItemDef>,
    voxel_to_item: HashMap<u8, String>,
}

impl ItemRegistry {
    pub fn register(&mut self, def: ItemDef) {
        if let Some(bp) = &def.block {
            self.voxel_to_item.insert(bp.voxel_id, def.id.clone());
        }
        self.items.insert(def.id.clone(), def);
    }

    pub fn get(&self, id: &str) -> Option<&ItemDef> {
        self.items.get(id)
    }

    pub fn item_for_voxel(&self, voxel_id: u8) -> Option<&ItemDef> {
        let id = self.voxel_to_item.get(&voxel_id)?;
        self.items.get(id)
    }

    pub fn voxel_id(&self, item_id: &str) -> Option<u8> {
        self.items.get(item_id)?.block.as_ref().map(|b| b.voxel_id)
    }
}

#[derive(Clone, Debug)]
pub struct HotbarSlot {
    pub item_id: String,
    pub count: u32,
}

#[derive(Resource)]
pub struct Hotbar {
    pub slots: [Option<HotbarSlot>; 9],
    pub selected: usize,
}

impl Default for Hotbar {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            selected: 0,
        }
    }
}

impl Hotbar {
    pub fn active_item_id(&self) -> Option<&str> {
        self.slots
            .get(self.selected)?
            .as_ref()
            .map(|s| s.item_id.as_str())
    }

    pub fn consume_active(&mut self) -> Option<String> {
        let slot = self.slots.get_mut(self.selected)?;
        let s = slot.as_mut()?;
        let id = s.item_id.clone();
        s.count -= 1;
        if s.count == 0 {
            *slot = None;
        }
        Some(id)
    }
}

#[derive(Resource, Default)]
pub struct Inventory(pub HashMap<String, u32>);

impl Inventory {
    pub fn add(&mut self, item_id: impl Into<String>, count: u32) {
        *self.0.entry(item_id.into()).or_insert(0) += count;
    }
}

#[derive(Resource, Default)]
pub struct InventoryOpen(pub bool);

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, voxel_id: u8) -> ItemDef {
        ItemDef {
            id: id.to_string(),
            name: id.to_string(),
            block: Some(BlockProps {
                voxel_id,
                hardness: 1.0,
            }),
        }
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = ItemRegistry::default();
        reg.register(item("iron", 1));
        assert!(reg.get("iron").is_some());
        assert!(reg.get("gold").is_none());
    }

    #[test]
    fn registry_voxel_id_roundtrip() {
        let mut reg = ItemRegistry::default();
        reg.register(item("iron", 5));
        assert_eq!(reg.voxel_id("iron"), Some(5));
        assert_eq!(reg.item_for_voxel(5).map(|i| i.id.as_str()), Some("iron"));
    }

    #[test]
    fn hotbar_active_empty() {
        let h = Hotbar::default();
        assert!(h.active_item_id().is_none());
    }

    #[test]
    fn hotbar_active_returns_selected_slot() {
        let mut h = Hotbar::default();
        h.slots[0] = Some(HotbarSlot {
            item_id: "iron".to_string(),
            count: 3,
        });
        assert_eq!(h.active_item_id(), Some("iron"));
    }

    #[test]
    fn hotbar_consume_decrements_count() {
        let mut h = Hotbar::default();
        h.slots[0] = Some(HotbarSlot {
            item_id: "iron".to_string(),
            count: 3,
        });
        assert_eq!(h.consume_active().as_deref(), Some("iron"));
        assert_eq!(h.slots[0].as_ref().unwrap().count, 2);
    }

    #[test]
    fn hotbar_consume_clears_slot_at_zero() {
        let mut h = Hotbar::default();
        h.slots[0] = Some(HotbarSlot {
            item_id: "iron".to_string(),
            count: 1,
        });
        h.consume_active();
        assert!(h.slots[0].is_none());
    }

    #[test]
    fn inventory_add_accumulates() {
        let mut inv = Inventory::default();
        inv.add("iron", 3);
        inv.add("iron", 5);
        assert_eq!(inv.0["iron"], 8);
    }
}

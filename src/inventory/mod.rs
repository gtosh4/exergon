use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

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
        self.slots[self.selected].as_ref().map(|s| s.item_id.as_str())
    }

    pub fn consume_active(&mut self) -> Option<String> {
        let slot = &mut self.slots[self.selected];
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

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hotbar>()
            .init_resource::<Inventory>()
            .init_resource::<InventoryOpen>();
    }
}

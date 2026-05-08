use std::collections::HashMap;

use bevy::prelude::*;

use crate::recipe_graph::ItemDef;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hotbar>()
            .init_resource::<InventoryOpen>();
    }
}

#[derive(Resource, Default)]
pub struct ItemRegistry {
    items: HashMap<String, ItemDef>,
}

impl ItemRegistry {
    pub fn register(&mut self, def: ItemDef) {
        self.items.insert(def.id.clone(), def);
    }

    pub fn get(&self, id: &str) -> Option<&ItemDef> {
        self.items.get(id)
    }
}

#[derive(Clone, Debug)]
pub struct HotbarSlot {
    pub item_id: String,
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
}

#[derive(Resource, Default)]
pub struct InventoryOpen(pub bool);

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> ItemDef {
        use crate::recipe_graph::ItemKind;
        ItemDef {
            id: id.to_string(),
            name: id.to_string(),
            kind: ItemKind::Unique,
            is_terminal: false,
        }
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = ItemRegistry::default();
        reg.register(item("iron"));
        assert!(reg.get("iron").is_some());
        assert!(reg.get("gold").is_none());
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
        });
        assert_eq!(h.active_item_id(), Some("iron"));
    }
}

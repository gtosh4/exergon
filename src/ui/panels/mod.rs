use bevy::prelude::*;

pub mod hotbar;
pub mod inventory;
pub mod machine;
pub mod storage;
pub mod tech_tree;

pub struct PanelsPlugin;

impl Plugin for PanelsPlugin {
    fn build(&self, app: &mut App) {
        hotbar::plugin(app);
        inventory::plugin(app);
        machine::plugin(app);
        storage::plugin(app);
        tech_tree::plugin(app);
    }
}

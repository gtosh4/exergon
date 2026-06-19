use bevy::prelude::*;

pub mod craft_modal;
pub mod escape_completion;
pub mod hotbar;
pub mod inventory;
pub mod machine;
pub mod planner;
pub mod storage;
pub mod tech_tree;

pub struct PanelsPlugin;

impl Plugin for PanelsPlugin {
    fn build(&self, app: &mut App) {
        craft_modal::plugin(app);
        escape_completion::plugin(app);
        hotbar::plugin(app);
        inventory::plugin(app);
        machine::plugin(app);
        storage::plugin(app);
        tech_tree::plugin(app);
        app.add_plugins(planner::PlannerPlugin);
    }
}

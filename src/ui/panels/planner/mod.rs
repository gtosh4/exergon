use bevy::prelude::*;

use crate::recipe_graph::{ItemId, RecipeId};

pub mod dep_graph;
pub mod panel;
pub mod recipe_picker;
pub mod topology;

pub use dep_graph::{
    ApplyAltRecipe, DepNode, LockMachineCount, PlanList, PlanName, PlanState, RateUnit,
    SelectSankeyNode, SetDepGraphTarget,
};
pub use topology::TopologyOverlay;

#[derive(Resource, Default)]
pub struct PlannerOpen {
    pub open: bool,
}

#[derive(Resource, Default)]
pub struct InspectorState {
    pub selected: Option<ItemId>,
}

#[derive(Resource, Default)]
pub struct RecipePickerState {
    pub open: bool,
    pub node: Option<ItemId>,
    pub search: String,
    pub filter_unlocked: bool,
    pub selected_alt: Option<RecipeId>,
}

pub struct PlannerPlugin;

impl Plugin for PlannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlannerOpen>()
            .init_resource::<InspectorState>()
            .init_resource::<RecipePickerState>()
            .init_resource::<PlanList>()
            .add_plugins((
                dep_graph::DepGraphPlugin,
                panel::PlannerPanelPlugin,
                recipe_picker::RecipePickerPlugin,
                topology::TopologyPlugin,
            ));
    }
}

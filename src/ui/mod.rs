use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::{
    PlayMode,
    inventory::InventoryOpen,
    logistics::StorageUnit,
    machine::{IoPortMarker, Machine},
    world::{MainCamera, Player},
};

pub mod hud;
pub mod input;
pub mod menus;
pub mod panels;
pub mod theme;
pub mod widgets;

pub struct UiPlugin;

#[derive(Resource, Default)]
pub struct MachineStatusPanel {
    pub entity: Option<Entity>,
    pub recipe_filter: String,
}

#[derive(Resource, Default)]
pub struct StorageStatusPanel(pub Option<Entity>);

#[derive(Resource, Default)]
pub struct TechTreePanelOpen {
    pub open: bool,
    pub selected_node: Option<String>,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            hud::HudPlugin,
            menus::MenusPlugin,
            panels::PanelsPlugin,
            widgets::WidgetsPlugin,
            input::TextInputPlugin,
        ))
        .init_resource::<MachineStatusPanel>()
        .init_resource::<StorageStatusPanel>()
        .init_resource::<TechTreePanelOpen>()
        .add_systems(
            Update,
            inspect_input
                .run_if(in_state(PlayMode::Exploring))
                .run_if(|o: Option<Res<InventoryOpen>>| !o.is_some_and(|r| r.0)),
        );
    }
}

fn inspect_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    spatial_query: SpatialQuery,
    machine_q: Query<(), With<Machine>>,
    storage_q: Query<(), With<StorageUnit>>,
    port_q: Query<&IoPortMarker>,
    parent_q: Query<&ChildOf>,
    player_q: Query<Entity, With<Player>>,
    mut panel: ResMut<MachineStatusPanel>,
    mut storage_panel: ResMut<StorageStatusPanel>,
    mut tech_tree_open: ResMut<TechTreePanelOpen>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) || keyboard.just_pressed(KeyCode::F4) {
        tech_tree_open.open = !tech_tree_open.open;
    }

    if mouse.just_pressed(MouseButton::Right) {
        if panel.entity.is_some() || storage_panel.0.is_some() {
            panel.entity = None;
            storage_panel.0 = None;
            return;
        }

        let Ok(cam) = camera_q.single() else { return };
        let dir = Dir3::new(*cam.forward()).unwrap_or(Dir3::NEG_Z);
        let mut filter = SpatialQueryFilter::default();
        if let Ok(player) = player_q.single() {
            filter.excluded_entities.insert(player);
        }
        let hit = spatial_query.cast_ray(cam.translation, dir, 8.0, true, &filter);

        panel.entity = None;
        storage_panel.0 = None;

        if let Some(h) = hit {
            let mut entity = h.entity;
            loop {
                if storage_q.contains(entity) {
                    storage_panel.0 = Some(entity);
                    break;
                } else if machine_q.contains(entity) {
                    panel.entity = Some(entity);
                    break;
                } else if let Ok(m) = port_q.get(entity) {
                    if storage_q.contains(m.owner) {
                        storage_panel.0 = Some(m.owner);
                    } else if machine_q.contains(m.owner) {
                        panel.entity = Some(m.owner);
                    }
                    break;
                } else if let Ok(child_of) = parent_q.get(entity) {
                    entity = child_of.0;
                } else {
                    break;
                }
            }
        }
    }
}

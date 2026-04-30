mod generation;
mod interaction;
mod player;

pub use interaction::LookTarget;

use bevy::prelude::*;
use bevy_voxel_world::prelude::*;

use crate::inventory::InventoryOpen;
use crate::textures::BlockAtlasLayers;
use crate::GameState;

use generation::WorldConfig;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let texture_layers = app
            .world()
            .get_resource::<BlockAtlasLayers>()
            .map(|r| r.0)
            .unwrap_or(0);
        app.add_plugins(VoxelWorldPlugin::with_config(WorldConfig {
            texture_layers,
            ..Default::default()
        }))
            .init_resource::<LookTarget>()
            .add_systems(Startup, (player::spawn_camera, interaction::setup_ghost_preview))
            .add_systems(OnEnter(GameState::Loading), generation::finish_loading)
            .add_systems(
                OnEnter(GameState::Playing),
                (player::setup_world_once, player::lock_cursor),
            )
            .add_systems(
                OnEnter(GameState::Paused),
                (player::unlock_cursor, interaction::hide_ghost_preview),
            )
            .add_systems(
                Update,
                (
                    player::toggle_pause,
                    player::toggle_inventory,
                    player::camera_input
                        .run_if(|o: Option<Res<InventoryOpen>>| !o.map(|r| r.0).unwrap_or(false)),
                    interaction::update_look_target.after(player::camera_input),
                    interaction::block_interaction
                        .after(interaction::update_look_target)
                        .run_if(|o: Option<Res<InventoryOpen>>| !o.map(|r| r.0).unwrap_or(false)),
                    interaction::update_ghost_preview.after(interaction::update_look_target),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

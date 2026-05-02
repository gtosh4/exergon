pub(crate) mod generation;
mod interaction;
mod player;

pub use interaction::LookTarget;
pub use player::MainCamera;

#[derive(Debug, Clone, Copy)]
pub enum BlockChangeKind {
    Placed { voxel_id: u8 },
    Removed { voxel_id: u8 },
    Replaced { old_voxel_id: u8, new_voxel_id: u8 },
}

#[derive(Message, Debug, Clone, Copy)]
pub struct BlockChangedMessage {
    pub pos: IVec3,
    pub kind: BlockChangeKind,
}

use bevy::prelude::*;
use bevy_voxel_world::prelude::*;

use crate::inventory::InventoryOpen;
use crate::textures::BlockAtlasLayers;
use crate::{GameState, PlayMode};

use generation::WorldConfig;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let texture_layers = app
            .world()
            .get_resource::<BlockAtlasLayers>()
            .map_or(0, |r| r.0);
        app.add_plugins(VoxelWorldPlugin::with_config(WorldConfig {
            texture_layers,
            ..Default::default()
        }))
        .add_message::<BlockChangedMessage>()
        .init_resource::<LookTarget>()
        .add_systems(
            Startup,
            (player::spawn_camera, interaction::setup_ghost_preview),
        )
        .add_systems(OnEnter(GameState::Loading), generation::finish_loading)
        .add_systems(Update, generation::add_chunk_colliders)
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
                    .run_if(|o: Option<Res<InventoryOpen>>| !o.is_some_and(|r| r.0))
                    .run_if(in_state(PlayMode::Exploring)),
                interaction::update_look_target.after(player::camera_input),
                interaction::block_interaction
                    .after(interaction::update_look_target)
                    .in_set(crate::GameSystems::Input)
                    .run_if(|o: Option<Res<InventoryOpen>>| !o.is_some_and(|r| r.0)),
                interaction::update_ghost_preview.after(interaction::update_look_target),
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

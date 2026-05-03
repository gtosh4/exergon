pub(crate) mod generation;
mod interaction;
mod player;

pub use interaction::LookTarget;
pub use player::MainCamera;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldObjectKind {
    Placed,
    Removed,
}

#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct WorldObjectEvent {
    pub pos: Vec3,
    pub item_id: String,
    pub kind: WorldObjectKind,
}

/// Fired when the player connects or disconnects two IO ports with a cable.
#[derive(bevy::ecs::message::Message, Debug, Clone)]
pub struct CableConnectionEvent {
    pub from: Vec3,
    pub to: Vec3,
    pub item_id: String,
    pub kind: WorldObjectKind,
}

use bevy::prelude::*;

use crate::inventory::InventoryOpen;
use crate::{GameState, PlayMode};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WorldObjectEvent>()
            .add_message::<CableConnectionEvent>()
            .init_resource::<LookTarget>()
            .init_resource::<generation::SpawnedChunks>()
            .insert_resource(generation::WorldConfig::default())
            .add_systems(
                Startup,
                (player::spawn_camera, interaction::setup_ghost_preview),
            )
            .add_systems(OnEnter(GameState::Loading), generation::finish_loading)
            .add_systems(
                Update,
                (
                    generation::spawn_chunks,
                    generation::despawn_chunks,
                    generation::add_chunk_colliders,
                )
                    .run_if(resource_exists::<generation::WorldConfig>),
            )
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
                    interaction::object_interaction
                        .after(interaction::update_look_target)
                        .in_set(crate::GameSystems::Input)
                        .run_if(|o: Option<Res<InventoryOpen>>| !o.is_some_and(|r| r.0)),
                    interaction::update_ghost_preview.after(interaction::update_look_target),
                    interaction::update_removal_ghost.after(interaction::update_look_target),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

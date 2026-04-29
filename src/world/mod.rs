use bevy::prelude::*;

use crate::GameState;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(OnEnter(GameState::Loading), finish_loading);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera3d::default());
}

fn finish_loading(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}

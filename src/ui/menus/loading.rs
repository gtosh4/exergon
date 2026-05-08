use bevy::prelude::*;

use crate::{GameState, ui::theme::COLOR_GOLD};

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), spawn);
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            DespawnOnExit(GameState::Loading),
        ))
        .with_child((
            Text::new("Loading..."),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(COLOR_GOLD),
        ));
}

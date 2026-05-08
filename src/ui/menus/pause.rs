use bevy::{app::AppExit, ecs::message::MessageWriter, prelude::*};

use crate::{
    GameState,
    ui::theme::{COLOR_DIM, COLOR_GOLD, COLOR_OVERLAY_BG},
};

#[derive(Component)]
enum PauseButton {
    Resume,
    Quit,
}

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Paused), spawn)
        .add_systems(Update, handle_buttons.run_if(in_state(GameState::Paused)));
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            DespawnOnExit(GameState::Paused),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("Paused"),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                TextColor(COLOR_GOLD),
            ));

            p.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(32.0), Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(COLOR_DIM),
                BackgroundColor(COLOR_OVERLAY_BG),
                PauseButton::Resume,
            ))
            .with_child((
                Text::new("Resume"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            p.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(32.0), Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(COLOR_DIM),
                BackgroundColor(COLOR_OVERLAY_BG),
                PauseButton::Quit,
            ))
            .with_child((
                Text::new("Quit"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn handle_buttons(
    q: Query<(&Interaction, &PauseButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, btn) in &q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match btn {
            PauseButton::Resume => next_state.set(GameState::Playing),
            PauseButton::Quit => {
                app_exit.write(AppExit::Success);
            }
        }
    }
}

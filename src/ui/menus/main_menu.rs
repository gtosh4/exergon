use bevy::{app::AppExit, prelude::*};

use crate::{
    GameState,
    ui::{
        theme::{font_size, palette, space},
        widgets::{UiButton, button_label, caption, divider, panel},
    },
};

#[derive(Component)]
enum MenuButton {
    NewRun,
    Quit,
}

const TAGLINE: &str = "Decode the world.\nBuild the factory.\nEscape.";

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::MainMenu), spawn)
        .add_systems(Update, handle_buttons.run_if(in_state(GameState::MainMenu)));
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(80.0),
                ..default()
            },
            BackgroundColor(palette::BG),
            DespawnOnExit(GameState::MainMenu),
        ))
        .with_children(|root| {
            // Left: logo + action buttons
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(space::XL),
                width: Val::Px(280.0),
                ..default()
            })
            .with_children(|left| {
                left.spawn((
                    Text::new("EXERGON"),
                    TextFont {
                        font_size: 52.0,
                        ..default()
                    },
                    TextColor(palette::ACCENT),
                ));

                left.spawn(divider());

                left.spawn(caption("BUILD YOUR ESCAPE"));

                left.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(space::MD),
                    ..default()
                })
                .with_children(|btns| {
                    btns.spawn((UiButton::accent(), MenuButton::NewRun))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("NEW RUN"),
                                TextFont {
                                    font_size: font_size::BUTTON,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    btns.spawn((UiButton::default(), MenuButton::Quit))
                        .with_children(|b| {
                            b.spawn(button_label("QUIT TO DESKTOP"));
                        });
                });
            });

            // Right: tagline card
            root.spawn(panel()).with_children(|right| {
                right.spawn((
                    Node {
                        max_width: Val::Px(220.0),
                        ..default()
                    },
                    Text::new(TAGLINE),
                    TextFont {
                        font_size: font_size::H_MD,
                        ..default()
                    },
                    TextColor(palette::DIM),
                ));
            });
        });
}

fn handle_buttons(
    btn_q: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, btn) in &btn_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match btn {
            MenuButton::NewRun => next_state.set(GameState::NewRunWizard),
            MenuButton::Quit => {
                app_exit.write(AppExit::Success);
            }
        }
    }
}

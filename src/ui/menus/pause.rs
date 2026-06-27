use bevy::{app::AppExit, prelude::*};

use crate::{
    GameState, PlayMode,
    save::{Run, RunSaveHeader, SaveRoot, trigger_run_save},
    ui::{
        theme::{font_size, palette, space},
        widgets::{UiButton, button_label, caption, divider, panel},
    },
};

#[derive(Component)]
enum PauseButton {
    Resume,
    LoadRun,
    SaveQuit,
    Quit,
}

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(PlayMode::Paused), spawn)
        .add_systems(Update, handle_buttons.run_if(in_state(PlayMode::Paused)));
}

fn spawn(mut commands: Commands, run_q: Query<&RunSaveHeader, With<Run>>) {
    let header = run_q.single().ok();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(48.0),
                ..default()
            },
            BackgroundColor(palette::OVERLAY_SCRIM),
            DespawnOnExit(PlayMode::Paused),
        ))
        .with_children(|root| {
            // Left: pause title + action buttons
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(space::MD),
                width: Val::Px(240.0),
                ..default()
            })
            .with_children(|left| {
                left.spawn((
                    Text::new("PAUSED"),
                    TextFont {
                        font_size: FontSize::Px(font_size::H_XL),
                        ..default()
                    },
                    TextColor(palette::ACCENT),
                ));

                left.spawn(divider());

                left.spawn((UiButton::accent(), PauseButton::Resume))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("RESUME"),
                            TextFont {
                                font_size: FontSize::Px(font_size::BUTTON),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                left.spawn((UiButton::default(), PauseButton::LoadRun))
                    .with_children(|b| {
                        b.spawn(button_label("LOAD RUN"));
                    });
                left.spawn((UiButton::default(), PauseButton::SaveQuit))
                    .with_children(|b| {
                        b.spawn(button_label("SAVE & QUIT TO MENU"));
                    });
                left.spawn((UiButton::default(), PauseButton::Quit))
                    .with_children(|b| {
                        b.spawn(button_label("SAVE & QUIT TO DESKTOP"));
                    });
            });

            // Right: run-at-a-glance panel
            root.spawn(panel()).with_children(|right| {
                right
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(space::SM),
                        min_width: Val::Px(180.0),
                        ..default()
                    })
                    .with_children(|info| {
                        if let Some(h) = header {
                            let seed = if h.seed_text.is_empty() {
                                "—"
                            } else {
                                &h.seed_text
                            };
                            info.spawn(caption("SEED"));
                            info.spawn((
                                Text::new(seed.to_string()),
                                TextFont {
                                    font_size: FontSize::Px(font_size::H_SM),
                                    ..default()
                                },
                                TextColor(palette::TEXT),
                            ));

                            info.spawn(divider());

                            let secs = h.total_playtime_secs as u64;
                            let playtime = if secs >= 3600 {
                                format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
                            } else {
                                format!("{}m {:02}s", secs / 60, secs % 60)
                            };
                            info.spawn(caption("PLAYTIME"));
                            info.spawn((
                                Text::new(playtime),
                                TextFont {
                                    font_size: FontSize::Px(font_size::H_SM),
                                    ..default()
                                },
                                TextColor(palette::TEXT),
                            ));
                        } else {
                            info.spawn(caption("NO ACTIVE RUN"));
                        }
                    });
            });
        });
}

fn handle_buttons(
    mut commands: Commands,
    q: Query<(&Interaction, &PauseButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut next_mode: ResMut<NextState<PlayMode>>,
    mut app_exit: MessageWriter<AppExit>,
    save_root: Res<SaveRoot>,
    header_q: Query<&RunSaveHeader, With<Run>>,
) {
    for (interaction, btn) in &q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match btn {
            PauseButton::Resume => next_mode.set(PlayMode::Exploring),
            PauseButton::LoadRun => next_state.set(GameState::LoadRun),
            PauseButton::SaveQuit => next_state.set(GameState::MainMenu),
            PauseButton::Quit => {
                if let Ok(header) = header_q.single() {
                    trigger_run_save(&mut commands, &save_root, &header.run_id);
                }
                app_exit.write(AppExit::Success);
            }
        }
    }
}

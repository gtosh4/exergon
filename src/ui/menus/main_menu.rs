use bevy::{app::AppExit, prelude::*};

use crate::{
    GameState,
    save::{
        DifficultyTier, LoadRunEvent, RunSaveHeader, RunStatus, SaveRoot, list_run_ids,
        read_run_header,
    },
    ui::{
        theme::{font_size, palette, space},
        widgets::{UiButton, button_label, caption, divider, panel},
    },
};

#[derive(Component)]
enum MenuButton {
    NewRun,
    LoadRun,
    ResumeRun(String),
    Quit,
}

const TAGLINE: &str = "Decode the world.\nBuild the factory.\nEscape.";

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::MainMenu), spawn)
        .add_systems(Update, handle_buttons.run_if(in_state(GameState::MainMenu)));
}

fn latest_in_progress(
    save_root: &SaveRoot,
    registry: &bevy::reflect::TypeRegistry,
) -> Option<RunSaveHeader> {
    list_run_ids(save_root)
        .into_iter()
        .filter_map(|id| read_run_header(save_root, &id, registry))
        .filter(|h| h.status == RunStatus::InProgress)
        .max_by_key(|h| h.start_time_secs)
}

fn difficulty_label(d: &DifficultyTier) -> &'static str {
    match d {
        DifficultyTier::Initiation => "Initiation",
        DifficultyTier::Standard => "Standard",
        DifficultyTier::Advanced => "Advanced",
        DifficultyTier::Pinnacle => "Pinnacle",
    }
}

fn format_playtime(secs: f64) -> String {
    let s = secs as u64;
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    } else {
        format!("{}m {:02}s", s / 60, s % 60)
    }
}

fn spawn(mut commands: Commands, save_root: Res<SaveRoot>, app_registry: Res<AppTypeRegistry>) {
    let resume = {
        let registry = app_registry.read();
        latest_in_progress(&save_root, &registry)
    };

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
                    if let Some(ref h) = resume {
                        btns.spawn((UiButton::accent(), MenuButton::ResumeRun(h.run_id.clone())))
                            .with_children(|b| {
                                b.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|inner| {
                                    inner.spawn((
                                        Text::new("RESUME RUN"),
                                        TextFont {
                                            font_size: font_size::BUTTON,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                    let hint = format!(
                                        "{} · {} · {}",
                                        h.seed_text,
                                        difficulty_label(&h.difficulty),
                                        format_playtime(h.total_playtime_secs),
                                    );
                                    inner.spawn((
                                        Text::new(hint),
                                        TextFont {
                                            font_size: font_size::LABEL_SM,
                                            ..default()
                                        },
                                        TextColor(palette::DIM),
                                    ));
                                });
                            });
                    }

                    let new_run_btn = if resume.is_some() {
                        UiButton::default()
                    } else {
                        UiButton::accent()
                    };
                    btns.spawn((new_run_btn, MenuButton::NewRun))
                        .with_children(|b| {
                            if resume.is_some() {
                                b.spawn(button_label("NEW RUN"));
                            } else {
                                b.spawn((
                                    Text::new("NEW RUN"),
                                    TextFont {
                                        font_size: font_size::BUTTON,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            }
                        });

                    btns.spawn((UiButton::default(), MenuButton::LoadRun))
                        .with_children(|b| {
                            b.spawn(button_label("LOAD RUN"));
                        });
                    btns.spawn((UiButton::default(), MenuButton::Quit))
                        .with_children(|b| {
                            b.spawn(button_label("QUIT TO DESKTOP"));
                        });
                });
            });

            root.spawn(panel()).with_children(|right| {
                if let Some(ref h) = resume {
                    spawn_run_glance(right, h);
                } else {
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
                }
            });
        });
}

fn spawn_run_glance(parent: &mut ChildSpawnerCommands, h: &RunSaveHeader) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(space::MD),
            max_width: Val::Px(260.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn(caption("LAST RUN · IN PROGRESS"));

            let seed = if h.seed_text.is_empty() {
                "—"
            } else {
                &h.seed_text
            };
            col.spawn((
                Text::new(seed.to_string()),
                TextFont {
                    font_size: font_size::H_MD,
                    ..default()
                },
                TextColor(palette::TEXT),
            ));

            col.spawn((
                Text::new(format!(
                    "{} · {}",
                    difficulty_label(&h.difficulty),
                    format_playtime(h.total_playtime_secs),
                )),
                TextFont {
                    font_size: font_size::LABEL,
                    ..default()
                },
                TextColor(palette::DIM),
            ));
        });
}

fn handle_buttons(
    btn_q: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut load_events: MessageWriter<LoadRunEvent>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, btn) in &btn_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match btn {
            MenuButton::NewRun => next_state.set(GameState::NewRunWizard),
            MenuButton::LoadRun => next_state.set(GameState::LoadRun),
            MenuButton::ResumeRun(run_id) => {
                load_events.write(LoadRunEvent {
                    run_id: run_id.clone(),
                });
            }
            MenuButton::Quit => {
                app_exit.write(AppExit::Success);
            }
        }
    }
}

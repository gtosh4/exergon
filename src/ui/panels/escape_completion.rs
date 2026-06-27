use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::{GameState, escape::EscapeStats, ui::theme::palette};

#[derive(Component)]
struct CompletionRoot;

#[derive(Component)]
struct NewRunButton;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Escaped), (unlock_cursor, spawn).chain())
        .add_systems(
            Update,
            on_new_run_click.run_if(in_state(GameState::Escaped)),
        );
}

fn unlock_cursor(mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn spawn(mut commands: Commands, stats: Res<EscapeStats>) {
    let mins = (stats.playtime_secs / 60.0) as u32;
    let secs = (stats.playtime_secs % 60.0) as u32;
    let archetype = if stats.archetype.is_empty() {
        "Unknown".to_string()
    } else {
        stats.archetype.clone()
    };
    let seed = if stats.seed_text.is_empty() {
        "—".to_string()
    } else {
        stats.seed_text.clone()
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(palette::PANEL_SCRIM),
            ZIndex(200),
            DespawnOnExit(GameState::Escaped),
            CompletionRoot,
        ))
        .with_children(|p| {
            // Title
            p.spawn((
                Text::new("ESCAPE COMPLETE"),
                TextFont {
                    font_size: FontSize::Px(40.0),
                    ..default()
                },
                TextColor(palette::WARN),
            ));

            // Stats card
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(48.0), Val::Px(24.0)),
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(palette::P1),
            ))
            .with_children(|s| {
                s.spawn((
                    Text::new(format!("Planet: {archetype}")),
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(palette::TEXT),
                ));
                s.spawn((
                    Text::new(format!("Seed:   {seed}")),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(palette::DIM),
                ));
                s.spawn((
                    Text::new(format!("Time:   {mins}m {secs}s")),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(palette::DIM),
                ));
            });

            // New run button
            p.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(32.0), Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(palette::OK),
                Button,
                NewRunButton,
            ))
            .with_child((
                Text::new("Main Menu"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Pickable::IGNORE,
            ));
        });
}

fn on_new_run_click(
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<NewRunButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for &interaction in &interaction_q {
        if interaction == Interaction::Pressed {
            next_state.set(GameState::MainMenu);
        }
    }
}

use bevy::prelude::*;

use crate::{
    GameState,
    seed::{DomainSeeds, RunSeed, hash_text},
    ui::{
        input::{FocusedInput, TextInput},
        theme::{COLOR_DIM, COLOR_GOLD, COLOR_OVERLAY_BG},
    },
};

#[derive(Component)]
struct SeedInput;

#[derive(Component)]
struct StartButton;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::MainMenu), (spawn, set_focus).chain())
        .add_systems(Update, handle_start.run_if(in_state(GameState::MainMenu)));
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
            DespawnOnExit(GameState::MainMenu),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("EXERGON"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(COLOR_GOLD),
            ));
            p.spawn((
                Text::new("Seed"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(COLOR_DIM),
            ));
            p.spawn((
                Node {
                    width: Val::Px(260.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    ..default()
                },
                BorderColor::all(COLOR_DIM),
                BackgroundColor(COLOR_OVERLAY_BG),
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TextInput::default(),
                SeedInput,
            ));
            p.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(24.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(COLOR_GOLD),
                BackgroundColor(COLOR_OVERLAY_BG),
                StartButton,
            ))
            .with_child((
                Text::new("Start Run"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(COLOR_GOLD),
            ));
        });
}

fn set_focus(seed_q: Query<Entity, With<SeedInput>>, mut focus: ResMut<FocusedInput>) {
    if let Ok(entity) = seed_q.single() {
        focus.0 = Some(entity);
    }
}

fn handle_start(
    seed_q: Query<&TextInput, With<SeedInput>>,
    btn_q: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut focus: ResMut<FocusedInput>,
) {
    let Ok(input) = seed_q.single() else { return };

    let start = input.submitted || btn_q.iter().any(|i| *i == Interaction::Pressed);

    if start {
        let hash = hash_text(&input.value);
        commands.insert_resource(RunSeed {
            text: input.value.clone(),
            hash,
        });
        commands.insert_resource(DomainSeeds::from_master(hash));
        focus.0 = None;
        next_state.set(GameState::Loading);
    }
}

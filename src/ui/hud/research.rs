use bevy::prelude::*;

use crate::{
    GameState,
    inventory::InventoryOpen,
    research::ResearchPool,
    ui::theme::{font_size, palette},
};

#[derive(Component)]
struct ResearchHudRoot;

#[derive(Component)]
struct ResearchHudText;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(Update, update.run_if(in_state(GameState::Playing)));
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(38.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(palette::OVERLAY_SCRIM),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            ResearchHudRoot,
        ))
        .with_child((
            Text::new("⚗ 0 RP"),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(palette::ACCENT),
            Pickable::IGNORE,
            ResearchHudText,
        ));
}

fn update(
    pool: Option<Res<ResearchPool>>,
    inv_open: Option<Res<InventoryOpen>>,
    mut root_q: Query<&mut Visibility, With<ResearchHudRoot>>,
    mut text_q: Query<&mut Text, With<ResearchHudText>>,
) {
    let hidden = inv_open.is_some_and(|o| o.0);
    for mut v in &mut root_q {
        *v = if hidden {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    if hidden {
        return;
    }

    let Some(pool) = pool else { return };
    if !pool.is_changed() {
        return;
    }
    if let Ok(mut t) = text_q.single_mut() {
        **t = format!("⚗ {:.0} RP", pool.points);
    }
}

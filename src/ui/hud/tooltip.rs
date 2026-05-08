use bevy::prelude::*;

use crate::{GameState, ui::theme::COLOR_PANEL_BG, world::LookTarget};

#[derive(Component)]
struct TooltipRoot;

#[derive(Component)]
struct TooltipText;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(Update, update.run_if(in_state(GameState::Playing)));
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::End,
                justify_content: JustifyContent::Center,
                padding: UiRect::bottom(Val::Px(80.0)),
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            TooltipRoot,
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_BG),
                Pickable::IGNORE,
            ))
            .with_child((
                Text::new(""),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Pickable::IGNORE,
                TooltipText,
            ));
        });
}

fn update(
    look_target: Option<Res<LookTarget>>,
    mut root_q: Query<&mut Visibility, With<TooltipRoot>>,
    mut text_q: Query<&mut Text, With<TooltipText>>,
) {
    let label = match look_target.as_deref() {
        Some(LookTarget::Surface { pos, .. }) => {
            let snapped = pos.floor().as_ivec3();
            Some(format!("{}, {}, {}", snapped.x, snapped.y, snapped.z))
        }
        _ => None,
    };

    for mut v in &mut root_q {
        *v = if label.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if let (Some(label), Ok(mut text)) = (label, text_q.single_mut()) {
        **text = label;
    }
}

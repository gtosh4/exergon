use bevy::prelude::*;

use crate::{
    GameState,
    escape::EscapeObjective,
    machine::{MachineActivity, MachineState},
    recipe_graph::RecipeGraph,
    ui::theme::palette,
};

#[derive(Component)]
struct EscapeHudRoot;

#[derive(Component)]
struct EscapeHudBar;

#[derive(Component)]
struct EscapeHudText;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(Update, update.run_if(in_state(GameState::Playing)));
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                bottom: Val::Px(60.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(palette::P1),
            Visibility::Hidden,
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            EscapeHudRoot,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("GATEWAY ACTIVATION"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::WARN),
                Pickable::IGNORE,
            ));
            // Bar container
            p.spawn((
                Node {
                    width: Val::Px(160.0),
                    height: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(palette::P3),
                Pickable::IGNORE,
            ))
            .with_child((
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(palette::OK),
                Pickable::IGNORE,
                EscapeHudBar,
            ));
            p.spawn((
                Text::new(""),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::TEXT),
                Pickable::IGNORE,
                EscapeHudText,
            ));
        });
}

fn update(
    escape_q: Query<(Option<&MachineActivity>, &MachineState), With<EscapeObjective>>,
    recipe_graph: Option<Res<RecipeGraph>>,
    mut root_q: Query<&mut Visibility, With<EscapeHudRoot>>,
    mut bar_q: Query<&mut Node, With<EscapeHudBar>>,
    mut text_q: Query<&mut Text, With<EscapeHudText>>,
) {
    let Ok((activity, state)) = escape_q.single() else {
        return;
    };

    let (visible, pct, label) = match (activity, state) {
        (Some(act), MachineState::Running) => {
            let (pct, secs_left) = recipe_graph
                .as_ref()
                .and_then(|rg| rg.recipes.get(&act.recipe_id))
                .map(|r| {
                    let p = (act.progress / r.processing_time * 100.0).clamp(0.0, 100.0);
                    let s = (r.processing_time - act.progress).max(0.0) as u32;
                    (p, s)
                })
                .unwrap_or((0.0, 0));
            (true, pct, format!("{secs_left}s remaining"))
        }
        _ => (false, 0.0, String::new()),
    };

    for mut v in &mut root_q {
        *v = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if visible {
        for mut node in &mut bar_q {
            node.width = Val::Percent(pct);
        }
        for mut text in &mut text_q {
            **text = label.clone();
        }
    }
}

use bevy::prelude::*;

use crate::{
    GameState,
    inventory::InventoryOpen,
    machine::{MachineActivity, MachineState},
    power::PowerNetwork,
    recipe_graph::RecipeGraph,
    ui::theme::{COLOR_GOLD, COLOR_OVERLAY_BG},
};

#[derive(Component)]
struct PowerHudRoot;

#[derive(Component)]
struct PowerHudText;

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
                bottom: Val::Px(10.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(COLOR_OVERLAY_BG),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            PowerHudRoot,
        ))
        .with_child((
            Text::new("⚡ 0W / 0W"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(COLOR_GOLD),
            Pickable::IGNORE,
            PowerHudText,
        ));
}

fn update(
    net_q: Query<&PowerNetwork>,
    machine_q: Query<(&MachineState, Option<&MachineActivity>)>,
    recipe_graph: Option<Res<RecipeGraph>>,
    inv_open: Option<Res<InventoryOpen>>,
    mut root_q: Query<&mut Visibility, With<PowerHudRoot>>,
    mut text_q: Query<&mut Text, With<PowerHudText>>,
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

    let produced: f32 = net_q.iter().map(|n| n.capacity_watts).sum();
    let demanded: f32 = recipe_graph
        .as_ref()
        .map(|rg| {
            machine_q
                .iter()
                .filter_map(|(state, activity)| {
                    if *state != MachineState::Running {
                        return None;
                    }
                    let act = activity?;
                    let recipe = rg.recipes.get(&act.recipe_id)?;
                    Some(recipe.energy_cost / recipe.processing_time)
                })
                .sum()
        })
        .unwrap_or(0.0);

    let label = if demanded > 0.0 {
        let pct = (produced / demanded * 100.0).min(100.0);
        format!("⚡ {produced:.0}W / {demanded:.0}W ({pct:.0}%)")
    } else {
        format!("⚡ {produced:.0}W / 0W")
    };

    if let Ok(mut text) = text_q.single_mut() {
        **text = label;
    }
}

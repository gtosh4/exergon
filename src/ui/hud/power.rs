use bevy::prelude::*;

use crate::{
    GameState,
    inventory::InventoryOpen,
    machine::{MachineActivity, MachineState},
    power::GeneratorUnit,
    recipe_graph::RecipeGraph,
    ui::theme::{COLOR_GOLD, COLOR_OVERLAY_BG},
};

const COLOR_DEFICIT: Color = Color::srgb(1.0, 0.3, 0.3);

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
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(COLOR_GOLD),
            Pickable::IGNORE,
            PowerHudText,
        ));
}

fn update(
    gen_q: Query<&GeneratorUnit>,
    machine_q: Query<(&MachineState, Option<&MachineActivity>)>,
    recipe_graph: Option<Res<RecipeGraph>>,
    inv_open: Option<Res<InventoryOpen>>,
    mut root_q: Query<&mut Visibility, With<PowerHudRoot>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<PowerHudText>>,
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

    let produced: f32 = gen_q.iter().map(|g| g.watts).sum();
    let (buf_cur, buf_max): (f32, f32) = gen_q.iter().fold((0.0, 0.0), |(c, m), g| {
        (c + g.buffer_joules, m + g.max_buffer_joules)
    });
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

    let deficit = demanded > 0.0 && produced < demanded;
    let pwr_line = if demanded > 0.0 {
        let pct = (produced / demanded * 100.0).min(100.0);
        if deficit {
            format!("⚡ {produced:.0}W / {demanded:.0}W ({pct:.0}%) !")
        } else {
            format!("⚡ {produced:.0}W / {demanded:.0}W ({pct:.0}%)")
        }
    } else {
        format!("⚡ {produced:.0}W / 0W")
    };
    let buf_line = if buf_max > 0.0 {
        format!(
            "\n▪ buf {:.0}kJ / {:.0}kJ",
            buf_cur / 1000.0,
            buf_max / 1000.0
        )
    } else {
        String::new()
    };
    let label = format!("{pwr_line}{buf_line}");

    if let Ok((mut text, mut color)) = text_q.single_mut() {
        **text = label;
        color.0 = if deficit { COLOR_DEFICIT } else { COLOR_GOLD };
    }
}

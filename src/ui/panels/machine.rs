use bevy::prelude::*;

use crate::{
    GameState,
    inventory::ItemRegistry,
    machine::{Machine, MachineActivity, MachineState},
    recipe_graph::RecipeGraph,
    ui::{
        MachineStatusPanel,
        theme::{COLOR_DIM, COLOR_GOLD, COLOR_GREEN, COLOR_OVERLAY_BG},
        widgets::ScrollableContent,
    },
};

#[derive(Component)]
struct MachinePanelRoot;

#[derive(Component)]
struct MachinePanelTitle;

#[derive(Component)]
struct MachineStatusText;

#[derive(Component)]
struct CurrentCraftText;

#[derive(Component)]
struct ProgressBar;

#[derive(Component)]
struct PortsText;

#[derive(Component)]
struct RecipeListRoot;

#[derive(Component)]
struct RecipeFilterInput;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(
            Update,
            (sync_visibility, update_content, handle_close, handle_filter)
                .run_if(in_state(GameState::Playing)),
        );
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                width: Val::Px(1100.0),
                min_height: Val::Px(620.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.039, 0.039, 0.039)),
            BorderColor::all(COLOR_DIM),
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            MachinePanelRoot,
        ))
        .with_children(|root| {
            // Title bar
            root.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            })
            .with_children(|h| {
                h.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(COLOR_GOLD),
                    MachinePanelTitle,
                ));
                h.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    MachinePanelCloseButton,
                ))
                .with_child((
                    Text::new("✕"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

            // Body: left rail + recipe table
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|body| {
                // Left rail
                body.spawn(Node {
                    width: Val::Px(380.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::right(Val::Px(1.0)),
                    margin: UiRect::right(Val::Px(16.0)),
                    ..default()
                })
                .insert(BorderColor::all(COLOR_DIM))
                .with_children(|rail| {
                    rail.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(COLOR_GOLD),
                        MachineStatusText,
                    ));
                    rail.spawn(Node {
                        margin: UiRect::vertical(Val::Px(8.0)),
                        height: Val::Px(1.0),
                        ..default()
                    })
                    .insert(BackgroundColor(COLOR_DIM));

                    // Current craft
                    rail.spawn((
                        Text::new("CURRENT CRAFT"),
                        TextFont {
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(COLOR_DIM),
                    ));
                    rail.spawn((
                        Text::new("— no active recipe"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(COLOR_DIM),
                        CurrentCraftText,
                    ));

                    // Progress bar (thin node)
                    rail.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Px(4.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.392, 0.294, 0.047)),
                        ProgressBar,
                    ));

                    rail.spawn(Node {
                        margin: UiRect::vertical(Val::Px(8.0)),
                        height: Val::Px(1.0),
                        ..default()
                    })
                    .insert(BackgroundColor(COLOR_DIM));

                    // Ports
                    rail.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(COLOR_DIM),
                        PortsText,
                    ));
                });

                // Right: recipe table
                body.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|right| {
                    // Filter row
                    right
                        .spawn(Node {
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(6.0)),
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|fr| {
                            fr.spawn((
                                Text::new("RECIPES"),
                                TextFont {
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(COLOR_GOLD),
                            ));
                            fr.spawn((
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
                                crate::ui::input::TextInput::default(),
                                RecipeFilterInput,
                            ));
                        });

                    // Recipe list (scrollable)
                    right.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            flex_grow: 1.0,
                            ..default()
                        },
                        ScrollableContent,
                        ScrollPosition::default(),
                        RecipeListRoot,
                    ));
                });
            });
        });
}

#[derive(Component)]
struct MachinePanelCloseButton;

fn sync_visibility(
    panel: Res<MachineStatusPanel>,
    mut q: Query<&mut Visibility, With<MachinePanelRoot>>,
    mut focus: ResMut<crate::ui::input::FocusedInput>,
    filter_q: Query<Entity, With<RecipeFilterInput>>,
) {
    let visible = panel.entity.is_some();
    for mut v in &mut q {
        *v = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if visible {
        if let Ok(filter_entity) = filter_q.single() {
            focus.0 = Some(filter_entity);
        }
    } else {
        focus.0 = None;
    }
}

fn update_content(
    panel: Res<MachineStatusPanel>,
    machine_q: Query<(&Machine, &MachineState, Option<&MachineActivity>)>,
    recipe_graph: Option<Res<RecipeGraph>>,
    item_registry: Option<Res<ItemRegistry>>,
    filter_q: Query<&crate::ui::input::TextInput, With<RecipeFilterInput>>,
    mut title_q: Query<&mut Text, With<MachinePanelTitle>>,
    mut status_q: Query<&mut Text, (With<MachineStatusText>, Without<MachinePanelTitle>)>,
    mut craft_q: Query<
        &mut Text,
        (
            With<CurrentCraftText>,
            Without<MachinePanelTitle>,
            Without<MachineStatusText>,
        ),
    >,
    mut progress_q: Query<&mut Node, With<ProgressBar>>,
    mut ports_q: Query<
        &mut Text,
        (
            With<PortsText>,
            Without<MachinePanelTitle>,
            Without<MachineStatusText>,
            Without<CurrentCraftText>,
        ),
    >,
    list_q: Query<Entity, With<RecipeListRoot>>,
    mut commands: Commands,
) {
    if !panel.is_changed() {
        return;
    }
    let Some(entity) = panel.entity else { return };
    let Ok((machine, state, activity)) = machine_q.get(entity) else {
        return;
    };

    // Title
    if let Ok(mut t) = title_q.single_mut() {
        **t = format!(
            "{} · LV{}",
            machine.machine_type.to_uppercase().replace('_', " "),
            machine.tier
        );
    }

    // Status
    let status = match state {
        MachineState::Idle => "◌ IDLE",
        MachineState::Running => "● RUNNING",
    };
    let status_color = match state {
        MachineState::Idle => Color::srgb(0.6, 0.6, 0.6),
        MachineState::Running => COLOR_GREEN,
    };
    if let Ok(mut t) = status_q.single_mut() {
        **t = format!(
            "{status}\n{} · LV{}",
            machine.machine_type.to_uppercase().replace('_', " "),
            machine.tier
        );
    }

    // Current craft + progress
    let current_recipe_id = activity.as_ref().map(|a| a.recipe_id.clone());
    let progress = activity
        .as_ref()
        .and_then(|a| {
            recipe_graph
                .as_ref()
                .and_then(|rg| rg.recipes.get(&a.recipe_id))
                .map(|r| a.progress / r.processing_time)
        })
        .unwrap_or(0.0);

    if let Ok(mut t) = craft_q.single_mut() {
        **t = match &current_recipe_id {
            Some(id) => format!("{} ({:.0}%)", id.replace('_', " "), progress * 100.0),
            None => "— no active recipe".to_string(),
        };
    }
    if let Ok(mut node) = progress_q.single_mut() {
        node.width = Val::Percent(progress * 100.0);
    }

    // Ports
    if let Ok(mut t) = ports_q.single_mut() {
        **t = format!(
            "▸ {} logistics port(s)\n⚡ {} power port(s)",
            machine.logistics_ports.len(),
            machine.energy_ports.len()
        );
    }

    // Status text color
    let _ = status_color; // used for future styling

    // Rebuild recipe list
    let filter = filter_q
        .single()
        .map(|f| f.value.to_lowercase())
        .unwrap_or_default();

    let Ok(list_entity) = list_q.single() else {
        return;
    };
    commands.entity(list_entity).despawn_children();

    let Some(rg) = &recipe_graph else { return };
    let mut recipes: Vec<_> = rg
        .recipes
        .values()
        .filter(|r| r.machine_type == machine.machine_type && r.machine_tier <= machine.tier)
        .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
        .collect();
    recipes.sort_by_key(|r| r.id.as_str());

    commands.entity(list_entity).with_children(|list| {
        if recipes.is_empty() {
            list.spawn((
                Text::new("(no matching recipes)"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(COLOR_DIM),
            ));
            return;
        }

        for recipe in &recipes {
            let is_active = current_recipe_id.as_deref() == Some(recipe.id.as_str());
            let row_color = if is_active {
                Color::srgb(0.086, 0.173, 0.039)
            } else {
                Color::NONE
            };
            let text_color = if is_active { COLOR_GREEN } else { COLOR_GOLD };

            let inputs: Vec<String> = recipe
                .inputs
                .iter()
                .map(|inp| {
                    let name = item_registry
                        .as_ref()
                        .and_then(|ir| ir.get(&inp.item))
                        .map_or(inp.item.as_str(), |d| d.name.as_str());
                    format!("{:.0}× {}", inp.quantity, name)
                })
                .collect();
            let outputs: Vec<String> = recipe
                .outputs
                .iter()
                .map(|out| {
                    let name = item_registry
                        .as_ref()
                        .and_then(|ir| ir.get(&out.item))
                        .map_or(out.item.as_str(), |d| d.name.as_str());
                    format!("{:.0}× {}", out.quantity, name)
                })
                .collect();

            list.spawn(Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            })
            .insert((BackgroundColor(row_color), BorderColor::all(COLOR_DIM)))
            .with_children(|row| {
                row.spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        Text::new(format!(
                            "{} {}",
                            if is_active { "▶" } else { "·" },
                            recipe.id.replace('_', " ")
                        )),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(text_color),
                    ));
                    header.spawn((
                        Text::new(format!(
                            "{:.1}s · {:.0}W",
                            recipe.processing_time,
                            recipe.energy_cost / recipe.processing_time
                        )),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(COLOR_DIM),
                    ));
                });
                row.spawn((
                    Text::new(format!("{} → {}", inputs.join(" + "), outputs.join(", "))),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(COLOR_DIM),
                ));
            });
        }
    });
}

fn handle_close(
    q: Query<&Interaction, (Changed<Interaction>, With<MachinePanelCloseButton>)>,
    mut panel: ResMut<MachineStatusPanel>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            panel.entity = None;
        }
    }
}

fn handle_filter(
    filter_q: Query<
        &crate::ui::input::TextInput,
        (
            With<RecipeFilterInput>,
            Changed<crate::ui::input::TextInput>,
        ),
    >,
    mut panel: ResMut<MachineStatusPanel>,
) {
    if filter_q.single().is_ok() {
        panel.set_changed();
    }
}

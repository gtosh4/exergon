use bevy::prelude::*;

use crate::{
    GameState,
    recipe_graph::{RecipeGraph, RecipeId},
    research::TechTreeProgress,
    ui::theme::{font_size, palette, space},
};

use super::{
    PlanList, RecipePickerState,
    dep_graph::{ApplyAltRecipe, PlanState},
};

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct RecipePickerRoot;

#[derive(Component)]
struct RecipePickerList;

#[derive(Component)]
struct RecipePickerCompare;

#[derive(Component)]
struct RecipeRowButton(RecipeId);

#[derive(Component)]
struct PickerApplyButton;

#[derive(Component)]
struct PickerCancelButton;

#[derive(Component)]
struct PickerUnlockedToggle;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct RecipePickerPlugin;

impl Plugin for RecipePickerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_picker)
            .add_systems(
                Update,
                (
                    sync_picker_visibility,
                    rebuild_picker,
                    handle_recipe_row_click,
                    handle_picker_apply,
                    handle_picker_cancel,
                    handle_unlocked_toggle,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

fn spawn_picker(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(palette::OVERLAY_SCRIM),
            Visibility::Hidden,
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            RecipePickerRoot,
        ))
        .with_children(|outer| {
            outer
                .spawn((
                    Node {
                        width: Val::Px(700.0),
                        height: Val::Px(460.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(space::MD)),
                        ..default()
                    },
                    BackgroundColor(palette::BG),
                    BorderColor::all(palette::BORDER_STRONG),
                    Pickable::default(),
                ))
                .with_children(|panel| {
                    // Header
                    panel.spawn((
                        Text::new("RECIPE PICKER"),
                        TextFont {
                            font_size: FontSize::Px(font_size::H_SM),
                            ..default()
                        },
                        TextColor(palette::TEXT),
                    ));

                    // Body row
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_grow: 1.0,
                            min_height: Val::Px(0.0),
                            column_gap: Val::Px(space::MD),
                            margin: UiRect::top(Val::Px(space::MD)),
                            ..default()
                        })
                        .with_children(|body| {
                            // Left: filters
                            body.spawn(Node {
                                width: Val::Px(140.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(space::SM),
                                ..default()
                            })
                            .with_children(|left| {
                                left.spawn((
                                    Text::new("Filter"),
                                    TextFont {
                                        font_size: FontSize::Px(font_size::LABEL),
                                        ..default()
                                    },
                                    TextColor(palette::DIM),
                                ));
                                left.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(
                                            Val::Px(space::SM),
                                            Val::Px(space::XS),
                                        ),
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(palette::P2),
                                    BorderColor::all(palette::BORDER),
                                    PickerUnlockedToggle,
                                ))
                                .with_child((
                                    Text::new("Unlocked only"),
                                    TextFont {
                                        font_size: FontSize::Px(font_size::LABEL),
                                        ..default()
                                    },
                                    TextColor(palette::TEXT),
                                ));
                            });

                            // Center: recipe list
                            body.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    flex_direction: FlexDirection::Column,
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                                RecipePickerList,
                            ));

                            // Right: compare
                            body.spawn((
                                Node {
                                    width: Val::Px(180.0),
                                    flex_direction: FlexDirection::Column,
                                    border: UiRect::left(Val::Px(1.0)),
                                    padding: UiRect::left(Val::Px(space::MD)),
                                    ..default()
                                },
                                BorderColor::all(palette::BORDER),
                                RecipePickerCompare,
                            ));
                        });

                    // Footer
                    panel
                        .spawn(Node {
                            justify_content: JustifyContent::FlexEnd,
                            column_gap: Val::Px(space::SM),
                            margin: UiRect::top(Val::Px(space::MD)),
                            ..default()
                        })
                        .with_children(|footer| {
                            footer
                                .spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(
                                            Val::Px(space::LG),
                                            Val::Px(space::SM),
                                        ),
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(palette::P2),
                                    BorderColor::all(palette::BORDER),
                                    PickerCancelButton,
                                ))
                                .with_child((
                                    Text::new("Cancel"),
                                    TextFont {
                                        font_size: FontSize::Px(font_size::BUTTON),
                                        ..default()
                                    },
                                    TextColor(palette::TEXT),
                                ));
                            footer
                                .spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(
                                            Val::Px(space::LG),
                                            Val::Px(space::SM),
                                        ),
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(palette::ACCENT),
                                    BorderColor::all(palette::BORDER_STRONG),
                                    PickerApplyButton,
                                ))
                                .with_child((
                                    Text::new("Apply"),
                                    TextFont {
                                        font_size: FontSize::Px(font_size::BUTTON),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                        });
                });
        });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn sync_picker_visibility(
    picker_state: Res<RecipePickerState>,
    mut q: Query<&mut Visibility, With<RecipePickerRoot>>,
) {
    if !picker_state.is_changed() {
        return;
    }
    let visible = picker_state.open;
    for mut v in &mut q {
        *v = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn rebuild_picker(
    picker_state: Res<RecipePickerState>,
    graph: Option<Res<RecipeGraph>>,
    progress: Option<Res<TechTreeProgress>>,
    list_q: Query<Entity, With<RecipePickerList>>,
    compare_q: Query<Entity, With<RecipePickerCompare>>,
    plan_list: Res<PlanList>,
    plan_q: Query<&PlanState>,
    mut commands: Commands,
) {
    if !picker_state.is_changed() {
        return;
    }
    if !picker_state.open {
        return;
    }

    let Ok(list_entity) = list_q.single() else {
        return;
    };
    let Ok(compare_entity) = compare_q.single() else {
        return;
    };
    commands.entity(list_entity).despawn_children();
    commands.entity(compare_entity).despawn_children();

    let Some(node_item) = &picker_state.node else {
        return;
    };
    let Some(graph) = graph else { return };
    let empty_prog = TechTreeProgress::default();
    let progress_ref = progress.as_deref().unwrap_or(&empty_prog);

    let producers = graph.producers.get(node_item).cloned().unwrap_or_default();

    commands.entity(list_entity).with_children(|list| {
        for rid in &producers {
            let Some(recipe) = graph.recipes.get(rid) else {
                continue;
            };
            let is_unlocked = progress_ref.unlocked_recipes.contains(rid);
            if picker_state.filter_unlocked && !is_unlocked {
                continue;
            }
            let is_selected = picker_state.selected_alt.as_deref() == Some(rid.as_str());

            let border_col = if is_selected {
                palette::ACCENT
            } else {
                palette::BORDER
            };
            list.spawn((
                Button,
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(space::SM)),
                    border: UiRect::all(Val::Px(1.0)),
                    margin: UiRect::bottom(Val::Px(space::XS)),
                    ..default()
                },
                BackgroundColor(if is_selected {
                    palette::P3
                } else {
                    palette::P1
                }),
                BorderColor::all(border_col),
                RecipeRowButton(rid.clone()),
            ))
            .with_children(|row| {
                let tier_str = format!("T{}", recipe.machine_tier);
                let name = rid.replace('_', " ");
                row.spawn((
                    Text::new(format!("{name}  [{tier_str}]")),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL),
                        ..default()
                    },
                    TextColor(if is_unlocked {
                        palette::TEXT
                    } else {
                        palette::DIM
                    }),
                ));
                let inputs: Vec<String> = recipe
                    .inputs
                    .iter()
                    .map(|s| format!("{}×{}", s.quantity, s.item))
                    .collect();
                let outputs: Vec<String> = recipe
                    .outputs
                    .iter()
                    .map(|s| format!("{}×{}", s.quantity, s.item))
                    .collect();
                row.spawn((
                    Text::new(format!("{} → {}", inputs.join(" + "), outputs.join(" + "))),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(palette::DIM),
                ));
            });
        }
    });

    // Rebuild compare panel
    let selected_alt = picker_state.selected_alt.clone();
    if let Some(alt_id) = selected_alt {
        let active = plan_list.active;
        let current_recipe_id = active.and_then(|e| {
            plan_q.get(e).ok().and_then(|plan| {
                plan.nodes
                    .iter()
                    .find(|n| {
                        plan.alt_recipes
                            .get(&n.item)
                            .map_or(n.item == *node_item, |_| n.item == *node_item)
                    })
                    .and_then(|n| n.recipe.clone())
            })
        });

        commands.entity(compare_entity).with_children(|comp| {
            comp.spawn((
                Text::new("Compare"),
                TextFont {
                    font_size: FontSize::Px(font_size::H_XS),
                    ..default()
                },
                TextColor(palette::DIM),
            ));

            if let Some(current_id) = &current_recipe_id
                && let Some(current) = graph.recipes.get(current_id)
            {
                comp.spawn((
                    Text::new(format!(
                        "Current: {}s, {:.0}kJ",
                        current.processing_time, current.energy_cost
                    )),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(palette::TEXT),
                ));
            }

            if let Some(alt) = graph.recipes.get(&alt_id) {
                comp.spawn((
                    Text::new(format!(
                        "Alt: {}s, {:.0}kJ",
                        alt.processing_time, alt.energy_cost
                    )),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(palette::ACCENT),
                ));
            }
        });
    }
}

fn handle_recipe_row_click(
    q: Query<(&Interaction, &RecipeRowButton), Changed<Interaction>>,
    mut picker_state: ResMut<RecipePickerState>,
) {
    for (interaction, btn) in &q {
        if *interaction == Interaction::Pressed {
            picker_state.selected_alt = Some(btn.0.clone());
        }
    }
}

fn handle_picker_apply(
    q: Query<&Interaction, (Changed<Interaction>, With<PickerApplyButton>)>,
    mut picker_state: ResMut<RecipePickerState>,
    mut commands: Commands,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            if let (Some(node), Some(recipe)) =
                (picker_state.node.clone(), picker_state.selected_alt.clone())
            {
                commands.write_message(ApplyAltRecipe { node, recipe });
            }
            picker_state.open = false;
        }
    }
}

fn handle_picker_cancel(
    q: Query<&Interaction, (Changed<Interaction>, With<PickerCancelButton>)>,
    mut picker_state: ResMut<RecipePickerState>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            picker_state.open = false;
        }
    }
}

fn handle_unlocked_toggle(
    q: Query<&Interaction, (Changed<Interaction>, With<PickerUnlockedToggle>)>,
    mut picker_state: ResMut<RecipePickerState>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            picker_state.filter_unlocked = !picker_state.filter_unlocked;
        }
    }
}

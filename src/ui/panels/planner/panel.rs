use bevy::prelude::*;

use crate::{
    GameState,
    recipe_graph::RecipeGraph,
    ui::theme::{font_size, palette, space},
};

use super::{
    InspectorState, PlanList, PlannerOpen, RecipePickerState,
    dep_graph::{PlanState, RateUnit, SelectSankeyNode},
};

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct PlannerRoot;

#[derive(Component)]
struct PlannerTopbar;

#[derive(Component)]
struct PlannerTopbarName;

#[derive(Component)]
struct PlannerTopbarTarget;

#[derive(Component)]
struct PlannerSankeyCanvas;

#[derive(Component)]
struct PlannerInspector;

#[derive(Component)]
struct PlannerStatusMachines;

#[derive(Component)]
struct PlannerStatusIssues;

#[derive(Component)]
struct NodeCard(String);

#[derive(Component)]
struct RateUnitPerSecond;

#[derive(Component)]
struct RateUnitPerMinute;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct PlannerPanelPlugin;

impl Plugin for PlannerPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_planner)
            .add_systems(
                Update,
                (
                    sync_visibility,
                    rebuild_sankey,
                    rebuild_inspector,
                    handle_node_card_click,
                    handle_close,
                    handle_rate_unit_toggle,
                    update_statusbar,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

fn spawn_planner(mut commands: Commands) {
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
            Pickable::IGNORE,
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            PlannerRoot,
        ))
        .with_children(|outer| {
            outer
                .spawn((
                    Node {
                        width: Val::Percent(90.0),
                        height: Val::Percent(90.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(palette::BG),
                    BorderColor::all(palette::BORDER_STRONG),
                    Pickable::default(),
                ))
                .with_children(|panel| {
                    // Topbar
                    panel
                        .spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::SM)),
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                border: UiRect::bottom(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(palette::BORDER),
                            PlannerTopbar,
                        ))
                        .with_children(|topbar| {
                            // Left: plan name + target
                            topbar
                                .spawn(Node {
                                    column_gap: Val::Px(space::MD),
                                    align_items: AlignItems::Center,
                                    ..default()
                                })
                                .with_children(|left| {
                                    left.spawn((
                                        Text::new("PLANNER · Plan A"),
                                        TextFont {
                                            font_size: FontSize::Px(font_size::H_SM),
                                            ..default()
                                        },
                                        TextColor(palette::TEXT),
                                        PlannerTopbarName,
                                    ));
                                    left.spawn((
                                        Text::new("—"),
                                        TextFont {
                                            font_size: FontSize::Px(font_size::LABEL),
                                            ..default()
                                        },
                                        TextColor(palette::DIM),
                                        PlannerTopbarTarget,
                                    ));
                                });

                            // Right: rate unit buttons
                            topbar
                                .spawn(Node {
                                    column_gap: Val::Px(space::XS),
                                    ..default()
                                })
                                .with_children(|right| {
                                    right
                                        .spawn((
                                            Button,
                                            Node {
                                                padding: UiRect::axes(
                                                    Val::Px(space::SM),
                                                    Val::Px(space::XS),
                                                ),
                                                border: UiRect::all(Val::Px(1.0)),
                                                ..default()
                                            },
                                            BackgroundColor(palette::ACCENT),
                                            BorderColor::all(palette::BORDER_STRONG),
                                            RateUnitPerSecond,
                                        ))
                                        .with_child((
                                            Text::new("/s"),
                                            TextFont {
                                                font_size: FontSize::Px(font_size::LABEL),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    right
                                        .spawn((
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
                                            RateUnitPerMinute,
                                        ))
                                        .with_child((
                                            Text::new("/min"),
                                            TextFont {
                                                font_size: FontSize::Px(font_size::LABEL),
                                                ..default()
                                            },
                                            TextColor(palette::DIM),
                                        ));
                                });
                        });

                    // Body
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_grow: 1.0,
                            min_height: Val::Px(0.0),
                            ..default()
                        })
                        .with_children(|body| {
                            // Sankey canvas (scrollable)
                            body.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    flex_direction: FlexDirection::Row,
                                    overflow: Overflow::scroll_x(),
                                    padding: UiRect::all(Val::Px(space::MD)),
                                    column_gap: Val::Px(space::SM),
                                    ..default()
                                },
                                PlannerSankeyCanvas,
                            ));

                            // Inspector rail
                            body.spawn((
                                Node {
                                    width: Val::Px(280.0),
                                    flex_direction: FlexDirection::Column,
                                    border: UiRect::left(Val::Px(1.0)),
                                    padding: UiRect::all(Val::Px(space::MD)),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                                BorderColor::all(palette::BORDER),
                                PlannerInspector,
                            ));
                        });

                    // Statusbar
                    panel
                        .spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::SM)),
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                border: UiRect::top(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(palette::BORDER),
                        ))
                        .with_children(|bar| {
                            bar.spawn(Node {
                                column_gap: Val::Px(space::MD),
                                ..default()
                            })
                            .with_children(|left| {
                                left.spawn((
                                    Text::new("0 machines"),
                                    TextFont {
                                        font_size: FontSize::Px(font_size::LABEL),
                                        ..default()
                                    },
                                    TextColor(palette::DIM),
                                    PlannerStatusMachines,
                                ));
                                left.spawn((
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(font_size::LABEL),
                                        ..default()
                                    },
                                    TextColor(palette::WARN),
                                    PlannerStatusIssues,
                                ));
                            });
                            bar.spawn((
                                Text::new("[Esc to close]"),
                                TextFont {
                                    font_size: FontSize::Px(font_size::LABEL),
                                    ..default()
                                },
                                TextColor(palette::DIM),
                            ));
                        });
                });
        });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn sync_visibility(
    planner_open: Res<PlannerOpen>,
    mut q: Query<&mut Visibility, With<PlannerRoot>>,
) {
    if !planner_open.is_changed() {
        return;
    }
    for mut v in &mut q {
        *v = if planner_open.open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn rebuild_sankey(
    planner_open: Res<PlannerOpen>,
    plan_list: Res<PlanList>,
    plan_q: Query<&PlanState>,
    canvas_q: Query<Entity, With<PlannerSankeyCanvas>>,
    mut commands: Commands,
) {
    if !planner_open.open {
        return;
    }
    let Some(active) = plan_list.active else {
        return;
    };
    let Ok(plan) = plan_q.get(active) else { return };
    // Only rebuild when the plan was just rebuilt (dirty just became false)
    if plan.dirty {
        return;
    }
    if !plan_list.is_changed() && !plan_q.get(active).is_ok_and(|_| true) {
        return;
    }
    // Rebuild unconditionally when open + plan changed
    let Ok(canvas_entity) = canvas_q.single() else {
        return;
    };
    commands.entity(canvas_entity).despawn_children();

    if plan.nodes.is_empty() {
        commands.entity(canvas_entity).with_child((
            Text::new("No target set"),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(palette::DIM),
        ));
        return;
    }

    // Group nodes by column
    let max_col = plan.nodes.iter().map(|n| n.column).max().unwrap_or(0);

    commands.entity(canvas_entity).with_children(|canvas| {
        for col in 0..=max_col {
            let col_nodes: Vec<_> = plan.nodes.iter().filter(|n| n.column == col).collect();
            if col_nodes.is_empty() {
                continue;
            }

            canvas
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(space::SM),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|col_node| {
                    for node in col_nodes {
                        let border_color = palette::BORDER;

                        col_node
                            .spawn((
                                Button,
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(space::SM)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    width: Val::Px(120.0),
                                    ..default()
                                },
                                BackgroundColor(palette::P1),
                                BorderColor::all(border_color),
                                NodeCard(node.item.clone()),
                            ))
                            .with_children(|card| {
                                // Item name + machine count
                                card.spawn(Node {
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..default()
                                })
                                .with_children(|row| {
                                    let item_name = node.item.replace('_', " ");
                                    row.spawn((
                                        Text::new(item_name),
                                        TextFont {
                                            font_size: FontSize::Px(font_size::LABEL),
                                            ..default()
                                        },
                                        TextColor(palette::TEXT),
                                    ));
                                    row.spawn((
                                        Text::new(format!("×{}", node.machine_count)),
                                        TextFont {
                                            font_size: FontSize::Px(font_size::LABEL),
                                            ..default()
                                        },
                                        TextColor(palette::WARN),
                                    ));
                                });

                                // Recipe
                                if let Some(ref recipe_id) = node.recipe {
                                    card.spawn((
                                        Text::new(recipe_id.replace('_', " ")),
                                        TextFont {
                                            font_size: FontSize::Px(font_size::LABEL_SM),
                                            ..default()
                                        },
                                        TextColor(palette::DIM),
                                    ));
                                }

                                // Rate
                                let rate_str = format!("{:.2}/s", node.required_rate);
                                card.spawn((
                                    Text::new(rate_str),
                                    TextFont {
                                        font_size: FontSize::Px(font_size::LABEL_SM),
                                        ..default()
                                    },
                                    TextColor(palette::TEXT),
                                ));
                            });
                    }
                });

            // Connector strip between columns (except after last)
            if col < max_col {
                canvas.spawn((
                    Node {
                        width: Val::Px(16.0),
                        align_self: AlignSelf::Stretch,
                        ..default()
                    },
                    BackgroundColor(palette::BORDER),
                ));
            }
        }
    });
}

fn rebuild_inspector(
    planner_open: Res<PlannerOpen>,
    inspector: Res<InspectorState>,
    plan_list: Res<PlanList>,
    plan_q: Query<&PlanState>,
    graph: Option<Res<RecipeGraph>>,
    inspector_q: Query<Entity, With<PlannerInspector>>,
    mut commands: Commands,
) {
    if !planner_open.open {
        return;
    }
    if !inspector.is_changed() && !planner_open.is_changed() && !plan_list.is_changed() {
        return;
    }

    let Ok(inspector_entity) = inspector_q.single() else {
        return;
    };
    commands.entity(inspector_entity).despawn_children();

    let Some(active) = plan_list.active else {
        return;
    };
    let Ok(plan) = plan_q.get(active) else { return };

    let Some(ref selected_item) = inspector.selected else {
        commands.entity(inspector_entity).with_child((
            Text::new("Click a node to inspect"),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(palette::DIM),
        ));
        return;
    };

    let node = plan.nodes.iter().find(|n| &n.item == selected_item);
    let Some(node) = node else {
        commands.entity(inspector_entity).with_child((
            Text::new("Node not found"),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(palette::ERR),
        ));
        return;
    };

    let graph_ref = graph.as_deref();
    let item_name = graph_ref
        .and_then(|g| g.items.get(selected_item))
        .map(|i| i.name.as_str())
        .unwrap_or(selected_item.as_str());

    let is_goal = plan.target == *selected_item;
    let machine_type = node
        .recipe
        .as_ref()
        .and_then(|rid| graph_ref.and_then(|g| g.recipes.get(rid)))
        .map(|r| r.machine_type.as_str())
        .unwrap_or("—");

    let throughput = node
        .recipe
        .as_ref()
        .and_then(|rid| {
            graph_ref.and_then(|g| g.recipes.get(rid)).map(|r| {
                let out_qty = r
                    .outputs
                    .iter()
                    .find(|s| s.item == *selected_item)
                    .map(|s| s.quantity)
                    .unwrap_or(1.0);
                (out_qty / r.processing_time) * node.machine_count as f32
            })
        })
        .unwrap_or(0.0);

    let under_planned = node.machine_count > 0 && throughput < node.required_rate - 1e-5;

    let status_text = if is_goal {
        "GOAL"
    } else if under_planned {
        "UNDER-PLANNED"
    } else {
        "OK"
    };
    let status_color = if is_goal {
        palette::ACCENT
    } else if under_planned {
        palette::ERR
    } else {
        palette::OK
    };

    let node_item = node.item.clone();
    let node_recipe = node.recipe.clone();
    let node_required_rate = node.required_rate;
    let node_machine_count = node.machine_count;
    let locked = plan.locked_counts.contains_key(&node.item);

    let alt_count = graph_ref
        .and_then(|g| g.producers.get(selected_item))
        .map(|v| v.len())
        .unwrap_or(0);

    commands.entity(inspector_entity).with_children(|insp| {
        // Header
        insp.spawn((
            Text::new(item_name.to_string()),
            TextFont {
                font_size: FontSize::Px(font_size::H_MD),
                ..default()
            },
            TextColor(palette::TEXT),
        ));
        insp.spawn((
            Text::new(machine_type.to_string()),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(palette::DIM),
        ));
        insp.spawn((
            Text::new(status_text),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(status_color),
        ));

        divider(insp);

        // Recipe section
        if let Some(ref rid) = node_recipe {
            insp.spawn((
                Text::new("RECIPE"),
                TextFont {
                    font_size: FontSize::Px(font_size::H_XS),
                    ..default()
                },
                TextColor(palette::DIM),
            ));
            if let Some(g) = graph_ref
                && let Some(recipe) = g.recipes.get(rid)
            {
                let inputs: Vec<String> = recipe
                    .inputs
                    .iter()
                    .map(|s| format!("{} {}", s.quantity, s.item))
                    .collect();
                let outputs: Vec<String> = recipe
                    .outputs
                    .iter()
                    .map(|s| format!("{} {}", s.quantity, s.item))
                    .collect();
                insp.spawn((
                    Text::new(format!("{} → {}", inputs.join(", "), outputs.join(", "))),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(palette::TEXT),
                ));
            }
            if alt_count > 1 {
                let label = format!("swap ({} alts)", alt_count - 1);
                let node_item_clone = node_item.clone();
                insp.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(space::SM), Val::Px(space::XS)),
                        border: UiRect::all(Val::Px(1.0)),
                        margin: UiRect::top(Val::Px(space::XS)),
                        ..default()
                    },
                    BackgroundColor(palette::P2),
                    BorderColor::all(palette::BORDER),
                    OpenRecipePickerButton(node_item_clone),
                ))
                .with_child((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL),
                        ..default()
                    },
                    TextColor(palette::ACCENT),
                ));
            }
        }

        divider(insp);

        // Throughput section
        insp.spawn((
            Text::new("THROUGHPUT"),
            TextFont {
                font_size: FontSize::Px(font_size::H_XS),
                ..default()
            },
            TextColor(palette::DIM),
        ));
        insp.spawn((
            Text::new(format!("Rate: {:.3}/s", node_required_rate)),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(palette::TEXT),
        ));
        insp.spawn((
            Text::new(format!(
                "Machines: {} {}",
                node_machine_count,
                if locked { "(locked)" } else { "" }
            )),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(if locked { palette::WARN } else { palette::TEXT }),
        ));

        // Under-planned alert
        if under_planned {
            divider(insp);
            insp.spawn((
                Text::new(format!(
                    "⚠ supply {throughput:.2}/s, demand {node_required_rate:.2}/s"
                )),
                TextFont {
                    font_size: FontSize::Px(font_size::LABEL),
                    ..default()
                },
                TextColor(palette::WARN),
            ));
        }

        divider(insp);

        // Modules placeholder
        insp.spawn((
            Text::new("MODULES"),
            TextFont {
                font_size: FontSize::Px(font_size::H_XS),
                ..default()
            },
            TextColor(palette::DIM),
        ));
        insp.spawn((
            Text::new("— no modules (MVP)"),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(palette::DIM),
        ));
    });
}

fn divider(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(space::SM)),
            ..default()
        })
        .insert(BackgroundColor(palette::BORDER));
}

#[derive(Component)]
struct OpenRecipePickerButton(String);

fn handle_node_card_click(
    q: Query<(&Interaction, &NodeCard), Changed<Interaction>>,
    mut inspector: ResMut<InspectorState>,
    mut commands: Commands,
) {
    for (interaction, card) in &q {
        if *interaction == Interaction::Pressed {
            commands.write_message(SelectSankeyNode(card.0.clone()));
            inspector.selected = Some(card.0.clone());
        }
    }
}

fn handle_close(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut planner_open: ResMut<PlannerOpen>,
    picker_state: Res<RecipePickerState>,
) {
    if keyboard.just_pressed(KeyCode::Escape) && planner_open.open && !picker_state.open {
        planner_open.open = false;
    }
}

fn handle_rate_unit_toggle(
    s_q: Query<&Interaction, (Changed<Interaction>, With<RateUnitPerSecond>)>,
    m_q: Query<&Interaction, (Changed<Interaction>, With<RateUnitPerMinute>)>,
    plan_list: Res<PlanList>,
    mut plan_q: Query<&mut PlanState>,
) {
    let Some(active) = plan_list.active else {
        return;
    };
    let Ok(mut plan) = plan_q.get_mut(active) else {
        return;
    };
    for interaction in &s_q {
        if *interaction == Interaction::Pressed {
            plan.rate_unit = RateUnit::PerSecond;
        }
    }
    for interaction in &m_q {
        if *interaction == Interaction::Pressed {
            plan.rate_unit = RateUnit::PerMinute;
        }
    }
}

fn update_statusbar(
    planner_open: Res<PlannerOpen>,
    plan_list: Res<PlanList>,
    plan_q: Query<&PlanState>,
    mut machines_q: Query<&mut Text, (With<PlannerStatusMachines>, Without<PlannerStatusIssues>)>,
    mut issues_q: Query<&mut Text, (With<PlannerStatusIssues>, Without<PlannerStatusMachines>)>,
) {
    if !planner_open.open {
        return;
    }
    let Some(active) = plan_list.active else {
        return;
    };
    let Ok(plan) = plan_q.get(active) else { return };
    if !plan_list.is_changed() && !plan_q.get(active).is_ok_and(|_| true) {
        return;
    }

    let total_machines: u32 = plan.nodes.iter().map(|n| n.machine_count).sum();
    let issues: u32 = plan
        .nodes
        .iter()
        .filter(|n| {
            // under-planned check using recipe throughput
            n.machine_count > 0 && n.machine_count as f32 * 1.0 < n.required_rate
        })
        .count() as u32;

    if let Ok(mut t) = machines_q.single_mut() {
        **t = format!("{total_machines} machines");
    }
    if let Ok(mut t) = issues_q.single_mut() {
        **t = if issues > 0 {
            format!("⚠{issues} issues")
        } else {
            String::new()
        };
    }
}

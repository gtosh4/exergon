use bevy::prelude::*;

use crate::{
    GameState,
    research::{ResearchPool, TechTreeProgress},
    tech_tree::{NodeEffect, TechTree, UnlockVector},
    ui::{
        TechTreePanelOpen,
        theme::{COLOR_DIM, COLOR_GOLD, COLOR_GREEN, COLOR_OVERLAY_BG},
        widgets::ScrollableContent,
    },
};

#[derive(Resource, Default)]
struct TechCurrentTier(u8);

#[derive(Component)]
struct TechTreePanelRoot;

#[derive(Component)]
struct TierTabsRoot;

#[derive(Component)]
struct TierTabButton(u8);

#[derive(Component)]
struct TechRPText;

#[derive(Component)]
struct TechNodeCanvas;

#[derive(Component)]
struct TechNodeButton(String);

#[derive(Component)]
struct TechDetailRoot;

#[derive(Component)]
struct TechDetailContent;

#[derive(Component)]
struct TechCloseButton;

pub fn plugin(app: &mut App) {
    app.init_resource::<TechCurrentTier>()
        .add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(
            Update,
            (
                sync_visibility,
                rebuild,
                rebuild_detail,
                update_rp,
                handle_tier_tab,
                handle_node_click,
                handle_close,
            )
                .run_if(in_state(GameState::Playing)),
        );
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            TechTreePanelRoot,
        ))
        .with_children(|outer| {
            outer
                .spawn((
                    Node {
                        width: Val::Px(860.0),
                        height: Val::Px(540.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.039, 0.039, 0.039)),
                    BorderColor::all(COLOR_DIM),
                ))
                .with_children(|panel| {
                    // Header row
                    panel
                        .spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|h| {
                            h.spawn((
                                Text::new("TECH TREE"),
                                TextFont {
                                    font_size: 15.0,
                                    ..default()
                                },
                                TextColor(COLOR_GOLD),
                            ));
                            h.spawn((
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                TechCloseButton,
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

                    // Tier tabs + RP row
                    panel
                        .spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Node {
                                    column_gap: Val::Px(4.0),
                                    ..default()
                                },
                                TierTabsRoot,
                            ));
                            row.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(COLOR_GOLD),
                                TechRPText,
                            ));
                        });

                    // Body: canvas + detail
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_grow: 1.0,
                            min_height: Val::Px(0.0),
                            ..default()
                        })
                        .with_children(|body| {
                            // Node canvas (scrollable flex-wrap)
                            body.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    flex_wrap: FlexWrap::Wrap,
                                    align_content: AlignContent::FlexStart,
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                                ScrollableContent,
                                ScrollPosition::default(),
                                TechNodeCanvas,
                            ));

                            // Detail panel (right side)
                            body.spawn((
                                Node {
                                    width: Val::Px(220.0),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(10.0)),
                                    border: UiRect::left(Val::Px(1.0)),
                                    margin: UiRect::left(Val::Px(8.0)),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                                BorderColor::all(COLOR_DIM),
                                TechDetailRoot,
                                Visibility::Hidden,
                            ))
                            .with_child((Node::default(), TechDetailContent));
                        });
                });
        });
}

fn sync_visibility(
    panel: Res<TechTreePanelOpen>,
    mut q: Query<&mut Visibility, With<TechTreePanelRoot>>,
) {
    let visible = panel.open;
    for mut v in &mut q {
        *v = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn rebuild(
    panel: Res<TechTreePanelOpen>,
    tier: Res<TechCurrentTier>,
    tech_tree: Option<Res<TechTree>>,
    progress: Option<Res<TechTreeProgress>>,
    tabs_q: Query<Entity, With<TierTabsRoot>>,
    canvas_q: Query<Entity, With<TechNodeCanvas>>,
    mut commands: Commands,
) {
    let changed = panel.is_changed()
        || tier.is_changed()
        || progress.as_ref().map(|r| r.is_changed()).unwrap_or(false)
        || tech_tree.as_ref().map(|r| r.is_changed()).unwrap_or(false);
    if !changed {
        return;
    }
    if !panel.open {
        return;
    }

    let Ok(tabs_entity) = tabs_q.single() else {
        return;
    };
    let Ok(canvas_entity) = canvas_q.single() else {
        return;
    };

    commands.entity(tabs_entity).despawn_children();
    commands.entity(canvas_entity).despawn_children();

    let Some(tree) = &tech_tree else {
        commands.entity(canvas_entity).with_child((
            Text::new("(no tech tree loaded)"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        return;
    };

    let max_tier = tree.nodes.values().map(|n| n.tier).max().unwrap_or(0);
    let current_tier = tier.0.min(max_tier);
    let empty_prog = TechTreeProgress::default();
    let prog = progress.as_deref().unwrap_or(&empty_prog);

    // Rebuild tier tabs
    commands.entity(tabs_entity).with_children(|tabs| {
        for t in 0..=max_tier {
            let active = t == current_tier;
            tabs.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(if active {
                    Color::srgb(0.118, 0.098, 0.031)
                } else {
                    Color::NONE
                }),
                BorderColor::all(if active { COLOR_GOLD } else { Color::NONE }),
                TierTabButton(t),
            ))
            .with_child((
                Text::new(format!("T{t}")),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(if active { COLOR_GOLD } else { COLOR_DIM }),
            ));
        }
    });

    // Rebuild node canvas for current tier
    let tier_nodes: Vec<_> = tree
        .tier_order
        .iter()
        .filter_map(|id| tree.nodes.get(id))
        .filter(|n| n.tier == current_tier)
        .collect();

    if tier_nodes.is_empty() {
        commands.entity(canvas_entity).with_child((
            Text::new("(no nodes for this tier)"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        return;
    }

    commands.entity(canvas_entity).with_children(|canvas| {
        for node in &tier_nodes {
            let unlocked = prog.unlocked_nodes.contains(&node.id);
            let selected = panel.selected_node.as_deref() == Some(node.id.as_str());
            let prereqs_met = node
                .prerequisites
                .iter()
                .all(|p| prog.unlocked_nodes.contains(p));

            let border_col = if selected {
                COLOR_GOLD
            } else if unlocked {
                COLOR_GREEN
            } else if prereqs_met {
                COLOR_DIM
            } else {
                Color::srgb(0.172, 0.149, 0.071)
            };
            let text_col = if unlocked {
                COLOR_GREEN
            } else if prereqs_met {
                Color::WHITE
            } else {
                COLOR_DIM
            };
            let bg = if selected {
                Color::srgb(0.157, 0.204, 0.055)
            } else if unlocked {
                Color::srgb(0.071, 0.141, 0.031)
            } else {
                COLOR_OVERLAY_BG
            };

            canvas
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(140.0),
                        height: Val::Px(44.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        margin: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    BorderColor::all(border_col),
                    BackgroundColor(bg),
                    TechNodeButton(node.id.clone()),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(&node.name),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(text_col),
                    ));
                    if unlocked {
                        btn.spawn((
                            Text::new("✓"),
                            TextFont {
                                font_size: 9.0,
                                ..default()
                            },
                            TextColor(COLOR_GREEN),
                        ));
                    }
                });
        }
    });
}

fn rebuild_detail(
    panel: Res<TechTreePanelOpen>,
    tech_tree: Option<Res<TechTree>>,
    progress: Option<Res<TechTreeProgress>>,
    pool: Option<Res<ResearchPool>>,
    detail_root_q: Query<Entity, With<TechDetailRoot>>,
    detail_content_q: Query<Entity, With<TechDetailContent>>,
    mut visibility_q: Query<&mut Visibility, With<TechDetailRoot>>,
    mut commands: Commands,
) {
    if !panel.is_changed()
        && !tech_tree.as_ref().map(|r| r.is_changed()).unwrap_or(false)
        && !progress.as_ref().map(|r| r.is_changed()).unwrap_or(false)
    {
        return;
    }

    let Ok(content_entity) = detail_content_q.single() else {
        return;
    };
    let Ok(root_entity) = detail_root_q.single() else {
        return;
    };

    commands.entity(content_entity).despawn_children();

    let Some(sel_id) = &panel.selected_node else {
        if let Ok(mut v) = visibility_q.get_mut(root_entity) {
            *v = Visibility::Hidden;
        }
        return;
    };

    if let Ok(mut v) = visibility_q.get_mut(root_entity) {
        *v = Visibility::Inherited;
    }

    let Some(tree) = &tech_tree else { return };
    let Some(node) = tree.nodes.get(sel_id) else {
        return;
    };

    let empty_prog = TechTreeProgress::default();
    let prog = progress.as_deref().unwrap_or(&empty_prog);
    let unlocked = prog.unlocked_nodes.contains(sel_id);

    commands.entity(content_entity).with_children(|c| {
        c.spawn((
            Text::new(&node.name),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_GOLD),
        ));
        c.spawn((
            Text::new(format!("Tier {}", node.tier)),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        c.spawn(Node {
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(6.0)),
            ..default()
        })
        .insert(BackgroundColor(COLOR_DIM));

        let (status_text, status_color) = if unlocked {
            ("✓ Unlocked", COLOR_GREEN)
        } else {
            ("Locked", COLOR_DIM)
        };
        c.spawn((
            Text::new(status_text),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(status_color),
        ));

        c.spawn(Node {
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(6.0)),
            ..default()
        })
        .insert(BackgroundColor(COLOR_DIM));

        c.spawn((
            Text::new("UNLOCK"),
            TextFont {
                font_size: 9.0,
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        let unlock_text = match &node.primary_unlock {
            UnlockVector::ResearchSpend(cost) => {
                let pts = pool.as_ref().map(|p| p.points).unwrap_or(0.0);
                let can = pts >= *cost as f32;
                let _ = can;
                format!("{:.0} / {} RP", pts, cost)
            }
            UnlockVector::ExplorationDiscovery(loc) => format!("Discover: {loc}"),
            UnlockVector::PrerequisiteChain => "Complete prerequisites".to_string(),
            UnlockVector::ProductionMilestone { material, quantity } => {
                format!("Produce {quantity:.0}× {material}")
            }
            UnlockVector::Observation(loc) => format!("Observe: {loc}"),
        };
        c.spawn((
            Text::new(unlock_text),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));

        if !node.prerequisites.is_empty() {
            c.spawn(Node {
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(6.0)),
                ..default()
            })
            .insert(BackgroundColor(COLOR_DIM));

            c.spawn((
                Text::new("REQUIRES"),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(COLOR_DIM),
            ));
            for prereq_id in &node.prerequisites {
                let done = prog.unlocked_nodes.contains(prereq_id);
                let name = tree
                    .nodes
                    .get(prereq_id)
                    .map_or(prereq_id.as_str(), |n| n.name.as_str());
                c.spawn((
                    Text::new(format!("{} {name}", if done { "✓" } else { "·" })),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(if done { COLOR_GREEN } else { COLOR_DIM }),
                ));
            }
        }

        if !node.effects.is_empty() {
            c.spawn(Node {
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(6.0)),
                ..default()
            })
            .insert(BackgroundColor(COLOR_DIM));

            c.spawn((
                Text::new("EFFECTS"),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(COLOR_DIM),
            ));
            for effect in &node.effects {
                match effect {
                    NodeEffect::UnlockRecipes(recipes) => {
                        for r in recipes {
                            c.spawn((
                                Text::new(format!("Recipe: {}", r.replace('_', " "))),
                                TextFont {
                                    font_size: 10.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        }
                    }
                    NodeEffect::UnlockMachine(m) => {
                        c.spawn((
                            Text::new(format!("Machine: {}", m.replace('_', " "))),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    }
                }
            }
        }
    });
}

fn update_rp(pool: Option<Res<ResearchPool>>, mut text_q: Query<&mut Text, With<TechRPText>>) {
    let Some(pool) = pool else { return };
    if !pool.is_changed() {
        return;
    }
    if let Ok(mut t) = text_q.single_mut() {
        **t = format!("{:.0} RP", pool.points);
    }
}

fn handle_tier_tab(
    q: Query<(&Interaction, &TierTabButton), Changed<Interaction>>,
    mut tier: ResMut<TechCurrentTier>,
) {
    for (interaction, btn) in &q {
        if *interaction == Interaction::Pressed {
            tier.0 = btn.0;
        }
    }
}

fn handle_node_click(
    q: Query<(&Interaction, &TechNodeButton), Changed<Interaction>>,
    mut panel: ResMut<TechTreePanelOpen>,
) {
    for (interaction, btn) in &q {
        if *interaction == Interaction::Pressed {
            if panel.selected_node.as_deref() == Some(btn.0.as_str()) {
                panel.selected_node = None;
            } else {
                panel.selected_node = Some(btn.0.clone());
            }
        }
    }
}

fn handle_close(
    q: Query<&Interaction, (Changed<Interaction>, With<TechCloseButton>)>,
    mut panel: ResMut<TechTreePanelOpen>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            panel.open = false;
        }
    }
}

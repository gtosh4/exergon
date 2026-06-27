use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::{
    GameState,
    recipe_graph::RecipeGraph,
    research::{ResearchPool, TechTreeProgress, UnlockNodeRequest},
    tech_tree::{NodeCategory, NodeDef, NodeEffect, TechTree, UnlockVector},
    ui::{
        TechTreePanelOpen,
        theme::{COLOR_DIM, COLOR_GOLD, COLOR_GREEN, COLOR_OVERLAY_BG, font_size, palette},
        widgets::{ScrollableContent, caption, divider, label},
    },
};

#[derive(Resource, Default)]
struct TechCurrentTier(u8);

/// Pending exclusive-group unlock: node_id awaiting player confirmation.
#[derive(Resource, Default)]
struct PendingExclusiveUnlock(Option<String>);

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

/// UNLOCK button in detail panel — node_id to request.
#[derive(Component)]
struct TechUnlockButton(String);

/// CONFIRM button in exclusive-group modal.
#[derive(Component)]
struct TechConfirmUnlockButton(String);

/// CANCEL button in exclusive-group modal.
#[derive(Component)]
struct TechCancelUnlockButton;

fn category_label(cat: &NodeCategory) -> &'static str {
    match cat {
        NodeCategory::Power => "POWER",
        NodeCategory::Processing => "PROCESSING",
        NodeCategory::Logistics => "LOGISTICS",
        NodeCategory::Science => "SCIENCE",
        NodeCategory::Exploration => "EXPLORATION",
        NodeCategory::Escape => "ESCAPE",
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<TechCurrentTier>()
        .init_resource::<PendingExclusiveUnlock>()
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
                handle_unlock_button,
                handle_confirm_unlock,
                handle_cancel_unlock,
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
                                    font_size: FontSize::Px(15.0),
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
                                    font_size: FontSize::Px(14.0),
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
                                    font_size: FontSize::Px(12.0),
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
    pool: Option<Res<ResearchPool>>,
    tabs_q: Query<Entity, With<TierTabsRoot>>,
    canvas_q: Query<Entity, With<TechNodeCanvas>>,
    mut commands: Commands,
) {
    let changed = panel.is_changed()
        || tier.is_changed()
        || progress.as_ref().map(|r| r.is_changed()).unwrap_or(false)
        || pool.as_ref().map(|r| r.is_changed()).unwrap_or(false)
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
                font_size: FontSize::Px(13.0),
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

    // Tier tabs
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
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(if active { COLOR_GOLD } else { COLOR_DIM }),
            ));
        }
    });

    // Node canvas for current tier
    let tier_nodes: Vec<&NodeDef> = tree
        .tier_order
        .iter()
        .filter_map(|id| tree.nodes.get(id))
        .filter(|n| n.tier == current_tier)
        .collect();

    if tier_nodes.is_empty() {
        commands.entity(canvas_entity).with_child((
            Text::new("(no nodes for this tier)"),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        return;
    }

    commands.entity(canvas_entity).with_children(|canvas| {
        for node in &tier_nodes {
            let unlocked = prog.unlocked_nodes.contains(&node.id);
            let disabled = prog.disabled_nodes.contains(&node.id);
            let selected = panel.selected_node.as_deref() == Some(node.id.as_str());
            let prereqs_met = node
                .prerequisites
                .iter()
                .all(|p| prog.unlocked_nodes.contains(p));
            let pts = pool.as_ref().map(|p| p.points).unwrap_or(0.0);
            let can_afford = prereqs_met
                && matches!(&node.primary_unlock, UnlockVector::ResearchSpend(c) if pts >= *c as f32);

            // Visual state: Unlocked > Disabled > Selected > Unlockable > Revealed > Shadow
            let (border_col, text_col, bg) = if unlocked {
                (
                    COLOR_GREEN,
                    COLOR_GREEN,
                    Color::srgb(0.071, 0.141, 0.031),
                )
            } else if disabled {
                (
                    palette::ERR,
                    palette::ERR,
                    Color::srgb(0.12, 0.04, 0.04),
                )
            } else if selected {
                (
                    COLOR_GOLD,
                    Color::WHITE,
                    Color::srgb(0.157, 0.204, 0.055),
                )
            } else if can_afford {
                // Unlockable: bright white border
                (Color::WHITE, Color::WHITE, COLOR_OVERLAY_BG)
            } else if prereqs_met {
                // Revealed: prereqs met, can't afford yet
                (COLOR_DIM, Color::WHITE, COLOR_OVERLAY_BG)
            } else {
                // Shadow: prereqs not met — name hidden
                (
                    Color::srgb(0.12, 0.10, 0.04),
                    COLOR_DIM,
                    COLOR_OVERLAY_BG,
                )
            };

            let dep_count = tree
                .dependents
                .get(&node.id)
                .map(|d| d.len())
                .unwrap_or(0);

            canvas
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(140.0),
                        height: Val::Px(52.0),
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
                    // Name row: hide name in Shadow state
                    let display_name = if !unlocked && !disabled && !prereqs_met {
                        format!("?? {}", category_label(&node.category))
                    } else if disabled {
                        format!("✗ {}", node.name)
                    } else {
                        node.name.clone()
                    };
                    btn.spawn((
                        Text::new(display_name),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(text_col),
                    ));

                    // Status subtitle
                    if unlocked {
                        btn.spawn((
                            Text::new("✓"),
                            TextFont {
                                font_size: FontSize::Px(9.0),
                                ..default()
                            },
                            TextColor(COLOR_GREEN),
                        ));
                    } else if disabled {
                        btn.spawn((
                            Text::new("locked out"),
                            TextFont {
                                font_size: FontSize::Px(8.0),
                                ..default()
                            },
                            TextColor(palette::ERR),
                        ));
                    } else if let UnlockVector::ResearchSpend(cost) = &node.primary_unlock
                        && prereqs_met {
                            btn.spawn((
                                Text::new(format!("{cost} RP")),
                                TextFont {
                                    font_size: FontSize::Px(font_size::MONO_XS),
                                    ..default()
                                },
                                TextColor(if can_afford { palette::OK } else { palette::DIM }),
                            ));
                        }

                    // Cross-tier stub badge
                    if dep_count > 0 && !disabled {
                        btn.spawn((
                            Text::new(format!("→ {dep_count}")),
                            TextFont {
                                font_size: FontSize::Px(8.0),
                                ..default()
                            },
                            TextColor(COLOR_DIM),
                        ));
                    }
                });
        }
    });
}

fn rebuild_detail(
    panel: Res<TechTreePanelOpen>,
    pending: Res<PendingExclusiveUnlock>,
    tech_tree: Option<Res<TechTree>>,
    progress: Option<Res<TechTreeProgress>>,
    pool: Option<Res<ResearchPool>>,
    recipe_graph: Option<Res<RecipeGraph>>,
    detail_root_q: Query<Entity, With<TechDetailRoot>>,
    detail_content_q: Query<Entity, With<TechDetailContent>>,
    mut visibility_q: Query<&mut Visibility, With<TechDetailRoot>>,
    mut commands: Commands,
) {
    if !panel.is_changed()
        && !pending.is_changed()
        && !tech_tree.as_ref().map(|r| r.is_changed()).unwrap_or(false)
        && !progress.as_ref().map(|r| r.is_changed()).unwrap_or(false)
        && !pool.as_ref().map(|r| r.is_changed()).unwrap_or(false)
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
    let disabled = prog.disabled_nodes.contains(sel_id);
    let prereqs_met = node
        .prerequisites
        .iter()
        .all(|p| prog.unlocked_nodes.contains(p));
    let pts = pool.as_ref().map(|p| p.points).unwrap_or(0.0);

    // Exclusive-group confirmation modal
    if pending.0.as_deref() == Some(sel_id.as_str()) {
        commands.entity(content_entity).with_children(|c| {
            c.spawn((
                Text::new(&node.name),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(COLOR_GOLD),
            ));
            c.spawn(divider());
            c.spawn((
                Text::new("⚠ EXCLUSIVE CHOICE"),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(palette::WARN),
            ));
            if let Some(group) = &node.exclusive_group {
                let peers: Vec<&str> = tree
                    .nodes
                    .iter()
                    .filter(|(id, n)| {
                        *id != sel_id && n.exclusive_group.as_deref() == Some(group.as_str())
                    })
                    .map(|(_, n)| n.name.as_str())
                    .collect();
                for peer in &peers {
                    c.spawn((
                        Text::new(format!("✗ {peer} (locked out)")),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(palette::ERR),
                    ));
                }
            }
            c.spawn(divider());
            c.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    margin: UiRect::bottom(Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.05, 0.05)),
                BorderColor::all(palette::ERR),
                TechConfirmUnlockButton(sel_id.clone()),
            ))
            .with_child((
                Text::new("CONFIRM UNLOCK"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(palette::ERR),
            ));
            c.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(COLOR_DIM),
                TechCancelUnlockButton,
            ))
            .with_child((
                Text::new("CANCEL"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(COLOR_DIM),
            ));
        });
        return;
    }

    commands.entity(content_entity).with_children(|c| {
        // Header
        let display_name = if !unlocked && !disabled && !prereqs_met {
            format!("?? {}", category_label(&node.category))
        } else {
            node.name.clone()
        };
        c.spawn((
            Text::new(display_name),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(COLOR_GOLD),
        ));
        c.spawn((
            Text::new(format!("Tier {} · {:?}", node.tier, node.rarity)),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        c.spawn(divider());

        // Status
        let (status_text, status_color) = if unlocked {
            ("✓ Unlocked", COLOR_GREEN)
        } else if disabled {
            ("✗ Locked Out", palette::ERR)
        } else if can_afford_node(node, pts, prereqs_met) {
            ("UNLOCKABLE", palette::OK)
        } else if prereqs_met {
            ("Revealed", Color::WHITE)
        } else {
            ("Shadow", COLOR_DIM)
        };
        c.spawn((
            Text::new(status_text),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(status_color),
        ));

        // Locked-out reason
        if disabled {
            if let Some(group) = &node.exclusive_group {
                let chosen = tree.nodes.iter().find(|(id, n)| {
                    *id != sel_id
                        && n.exclusive_group.as_deref() == Some(group.as_str())
                        && prog.unlocked_nodes.contains(*id)
                });
                if let Some((_, chosen_node)) = chosen {
                    c.spawn((
                        Text::new(format!("{} chosen instead", chosen_node.name)),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(COLOR_DIM),
                    ));
                }
            }
            return;
        }

        c.spawn(divider());

        // Unlock section
        c.spawn(caption("UNLOCK"));
        let unlock_text = match &node.primary_unlock {
            UnlockVector::ResearchSpend(cost) => {
                format!("{pts:.0} / {cost} RP")
            }
            UnlockVector::ExplorationDiscovery(loc) => format!("Discover: {loc}"),
            UnlockVector::PrerequisiteChain => "Complete prerequisites".to_string(),
            UnlockVector::ProductionMilestone { material, quantity } => {
                format!("Produce {quantity:.0}× {material}")
            }
            UnlockVector::Observation(loc) => format!("Observe: {loc}"),
        };
        c.spawn(label(&unlock_text));

        if !unlocked && let UnlockVector::ResearchSpend(cost) = &node.primary_unlock {
            if !prereqs_met {
                c.spawn((
                    Text::new("↑ Prereqs not met"),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(palette::ERR),
                ));
            } else if pts < *cost as f32 {
                let deficit = *cost as f32 - pts;
                c.spawn((
                    Text::new(format!("Need {deficit:.0} more RP")),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(palette::WARN),
                ));
            } else {
                // Exclusive group warning
                if node.exclusive_group.is_some() {
                    c.spawn((
                        Text::new("⚠ Exclusive choice — locks out peers"),
                        TextFont {
                            font_size: FontSize::Px(font_size::LABEL_SM),
                            ..default()
                        },
                        TextColor(palette::WARN),
                    ));
                }
                // UNLOCK button
                c.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        margin: UiRect::top(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.06, 0.14, 0.04)),
                    BorderColor::all(palette::OK),
                    TechUnlockButton(sel_id.clone()),
                ))
                .with_child((
                    Text::new("UNLOCK"),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(palette::OK),
                ));
            }
        }

        // Prerequisites
        if !node.prerequisites.is_empty() {
            c.spawn(divider());
            c.spawn(caption("REQUIRES"));
            for prereq_id in &node.prerequisites {
                let done = prog.unlocked_nodes.contains(prereq_id);
                let name = tree
                    .nodes
                    .get(prereq_id)
                    .map_or(prereq_id.as_str(), |n| n.name.as_str());
                c.spawn((
                    Text::new(format!("{} {name}", if done { "✓" } else { "·" })),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(if done { COLOR_GREEN } else { COLOR_DIM }),
                ));
            }
        }

        // Effects (only shown when prereqs met or unlocked)
        if (prereqs_met || unlocked) && !node.effects.is_empty() {
            c.spawn(divider());
            c.spawn(caption("EFFECTS"));
            for effect in &node.effects {
                match effect {
                    NodeEffect::UnlockRecipes(recipes) => {
                        for r in recipes {
                            c.spawn(label(&format!("Recipe: {}", r.replace('_', " "))));
                        }
                    }
                    NodeEffect::UnlockMachine(m) => {
                        c.spawn(label(&format!("Machine: {}", m.replace('_', " "))));
                    }
                    NodeEffect::UnlockRecipeTemplate(t) => {
                        c.spawn(label(&format!(
                            "Recipes: {} (all materials)",
                            t.replace('_', " ")
                        )));
                    }
                }
            }
        }

        // Cross-tier: nodes that depend on this one
        let dependent_nodes: Vec<(&String, &NodeDef)> = tree
            .dependents
            .get(sel_id)
            .map(|deps| {
                deps.iter()
                    .filter_map(|id| tree.nodes.get(id).map(|n| (id, n)))
                    .collect()
            })
            .unwrap_or_default();

        if !dependent_nodes.is_empty() {
            c.spawn(divider());
            c.spawn(caption("LEADS TO"));
            for (dep_id, dep_node) in &dependent_nodes {
                let dep_unlocked = prog.unlocked_nodes.contains(*dep_id);
                let dep_disabled = prog.disabled_nodes.contains(*dep_id);
                let prefix = if dep_unlocked {
                    "✓"
                } else if dep_disabled {
                    "✗"
                } else {
                    "→"
                };
                let color = if dep_unlocked {
                    COLOR_GREEN
                } else if dep_disabled {
                    palette::ERR
                } else {
                    COLOR_DIM
                };
                c.spawn((
                    Text::new(format!("{prefix} {} (T{})", dep_node.name, dep_node.tier)),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(color),
                ));
            }
        }

        // Research source
        if let UnlockVector::ResearchSpend(_) = &node.primary_unlock
            && let Some(rg) = &recipe_graph
        {
            let sources: Vec<String> = rg
                .producers
                .get("research_points")
                .map(|ids| {
                    let mut machine_types: Vec<String> = ids
                        .iter()
                        .filter_map(|id| rg.recipes.get(id))
                        .map(|r| humanize_id(&r.machine_type))
                        .collect();
                    machine_types.sort();
                    machine_types.dedup();
                    machine_types
                })
                .unwrap_or_default();

            if !sources.is_empty() {
                c.spawn(divider());
                c.spawn(caption("SOURCE"));
                for src in &sources {
                    c.spawn(label(src.as_str()));
                }
            }
        }
    });
}

fn can_afford_node(node: &NodeDef, pts: f32, prereqs_met: bool) -> bool {
    prereqs_met
        && matches!(&node.primary_unlock, UnlockVector::ResearchSpend(c) if pts >= *c as f32)
}

fn humanize_id(id: &str) -> String {
    id.split('_')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
    mut pending: ResMut<PendingExclusiveUnlock>,
) {
    for (interaction, btn) in &q {
        if *interaction == Interaction::Pressed {
            if panel.selected_node.as_deref() == Some(btn.0.as_str()) {
                panel.selected_node = None;
            } else {
                panel.selected_node = Some(btn.0.clone());
            }
            pending.0 = None;
        }
    }
}

fn handle_close(
    q: Query<&Interaction, (Changed<Interaction>, With<TechCloseButton>)>,
    mut panel: ResMut<TechTreePanelOpen>,
    mut pending: ResMut<PendingExclusiveUnlock>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            panel.open = false;
            pending.0 = None;
        }
    }
}

fn handle_unlock_button(
    q: Query<(&Interaction, &TechUnlockButton), Changed<Interaction>>,
    tech_tree: Option<Res<TechTree>>,
    progress: Option<Res<TechTreeProgress>>,
    mut pending: ResMut<PendingExclusiveUnlock>,
    mut unlock_requests: MessageWriter<UnlockNodeRequest>,
) {
    for (interaction, btn) in &q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let node_id = &btn.0;
        let Some(tree) = &tech_tree else { continue };
        let Some(node) = tree.nodes.get(node_id) else {
            continue;
        };
        if let Some(group) = &node.exclusive_group {
            let has_unlockable_peers = tree.nodes.iter().any(|(id, n)| {
                id != node_id
                    && n.exclusive_group.as_deref() == Some(group.as_str())
                    && !progress
                        .as_ref()
                        .is_some_and(|p| p.disabled_nodes.contains(id))
                    && !progress
                        .as_ref()
                        .is_some_and(|p| p.unlocked_nodes.contains(id))
            });
            if has_unlockable_peers {
                pending.0 = Some(node_id.clone());
                return;
            }
        }
        unlock_requests.write(UnlockNodeRequest(node_id.clone()));
    }
}

fn handle_confirm_unlock(
    q: Query<(&Interaction, &TechConfirmUnlockButton), Changed<Interaction>>,
    mut pending: ResMut<PendingExclusiveUnlock>,
    mut unlock_requests: MessageWriter<UnlockNodeRequest>,
) {
    for (interaction, btn) in &q {
        if *interaction == Interaction::Pressed {
            unlock_requests.write(UnlockNodeRequest(btn.0.clone()));
            pending.0 = None;
        }
    }
}

fn handle_cancel_unlock(
    q: Query<&Interaction, (Changed<Interaction>, With<TechCancelUnlockButton>)>,
    mut pending: ResMut<PendingExclusiveUnlock>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            pending.0 = None;
        }
    }
}

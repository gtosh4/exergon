use std::collections::HashSet;

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::{
    GameState,
    inventory::ItemRegistry,
    logistics::{
        LogisticsNetworkMember, LogisticsNetworkMembers, NetworkCraftQueue, NetworkStorageChanged,
        StorageUnit,
    },
    machine::{LogisticsPortOf, Machine, MachineActivity, MachineLogisticsPorts, ManualCraftOnly},
    network::NetworkMembersComponent,
    recipe_graph::RecipeGraph,
    research::TechTreeProgress,
    ui::theme::{COLOR_DIM, border, font_size, palette, space},
};

// ---------------------------------------------------------------------------
// Public resource — open by setting Some(data), close by setting None
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct CraftModal(pub Option<CraftModalData>);

pub struct CraftModalData {
    pub item_id: String,
    pub quantity: u32,
    pub phase: CraftPhase,
}

#[derive(Default, PartialEq)]
pub enum CraftPhase {
    #[default]
    Quantity,
    Plan,
}

// ---------------------------------------------------------------------------
// Resolved ingredient (computed on RESOLVE click)
// ---------------------------------------------------------------------------

struct ResolvedIngredient {
    item_id: String,
    needed: u32,
    have: u32,
}

// ---------------------------------------------------------------------------
// UI component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CraftModalRoot;

#[derive(Component)]
struct CraftModalContent;

#[derive(Component)]
struct CraftQtyMinus;

#[derive(Component)]
struct CraftQtyPlus;

#[derive(Component)]
struct CraftQtyPreset(u32);

#[derive(Component)]
struct CraftResolveBtn;

#[derive(Component)]
struct CraftEnqueueBtn;

#[derive(Component)]
struct CraftBackBtn;

#[derive(Component)]
struct CraftCloseBtn;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub fn plugin(app: &mut App) {
    app.init_resource::<CraftModal>()
        .add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(
            Update,
            (
                sync_visibility,
                handle_close,
                handle_qty_buttons,
                handle_resolve,
                handle_back,
                handle_enqueue,
                update_content,
            )
                .run_if(in_state(GameState::Playing)),
        );
}

// ---------------------------------------------------------------------------
// Spawn skeleton — one root + one content node rebuilt by update_content
// ---------------------------------------------------------------------------

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(10),
            Pickable::IGNORE,
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            CraftModalRoot,
        ))
        .with_children(|outer| {
            outer.spawn((
                Node {
                    width: Val::Px(480.0),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(border::THIN)),
                    ..default()
                },
                BackgroundColor(palette::P1),
                BorderColor::all(palette::BORDER_STRONG),
                CraftModalContent,
            ));
        });
}

// ---------------------------------------------------------------------------
// Visibility
// ---------------------------------------------------------------------------

fn sync_visibility(modal: Res<CraftModal>, mut q: Query<&mut Visibility, With<CraftModalRoot>>) {
    let visible = modal.0.is_some();
    for mut v in &mut q {
        *v = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

// ---------------------------------------------------------------------------
// Button handlers
// ---------------------------------------------------------------------------

fn handle_close(
    q: Query<&Interaction, (Changed<Interaction>, With<CraftCloseBtn>)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut modal: ResMut<CraftModal>,
) {
    let esc = keyboard.just_pressed(KeyCode::Escape);
    let clicked = q.iter().any(|i| *i == Interaction::Pressed);
    if (esc || clicked) && modal.0.is_some() {
        modal.0 = None;
    }
}

fn handle_qty_buttons(
    minus_q: Query<&Interaction, (Changed<Interaction>, With<CraftQtyMinus>)>,
    plus_q: Query<&Interaction, (Changed<Interaction>, With<CraftQtyPlus>)>,
    preset_q: Query<(&Interaction, &CraftQtyPreset), Changed<Interaction>>,
    mut modal: ResMut<CraftModal>,
) {
    let Some(data) = modal.0.as_mut() else { return };
    for i in &minus_q {
        if *i == Interaction::Pressed {
            data.quantity = data.quantity.saturating_sub(1).max(1);
        }
    }
    for i in &plus_q {
        if *i == Interaction::Pressed {
            data.quantity = data.quantity.saturating_add(1).min(9999);
        }
    }
    for (i, preset) in &preset_q {
        if *i == Interaction::Pressed {
            data.quantity = preset.0;
        }
    }
}

fn handle_resolve(
    q: Query<&Interaction, (Changed<Interaction>, With<CraftResolveBtn>)>,
    mut modal: ResMut<CraftModal>,
) {
    let Some(data) = modal.0.as_mut() else { return };
    if data.phase != CraftPhase::Quantity {
        return;
    }
    for i in &q {
        if *i == Interaction::Pressed {
            data.phase = CraftPhase::Plan;
        }
    }
}

fn handle_back(
    q: Query<&Interaction, (Changed<Interaction>, With<CraftBackBtn>)>,
    mut modal: ResMut<CraftModal>,
) {
    let Some(data) = modal.0.as_mut() else { return };
    for i in &q {
        if *i == Interaction::Pressed {
            data.phase = CraftPhase::Quantity;
        }
    }
}

fn handle_enqueue(
    q: Query<&Interaction, (Changed<Interaction>, With<CraftEnqueueBtn>)>,
    port_q: Query<&LogisticsNetworkMember>,
    manual_machine_q: Query<&MachineLogisticsPorts, With<ManualCraftOnly>>,
    net_members_q: Query<&LogisticsNetworkMembers>,
    port_of_q: Query<&LogisticsPortOf>,
    storage_unit_q: Query<&StorageUnit>,
    recipe_graph: Option<Res<RecipeGraph>>,
    mut queue_q: Query<&mut NetworkCraftQueue>,
    mut storage_changed: MessageWriter<NetworkStorageChanged>,
    mut modal_mut: ResMut<CraftModal>,
) {
    let clicked = q.iter().any(|i| *i == Interaction::Pressed);
    if !clicked {
        return;
    }

    // Find the logistics network containing the ManualCraftOnly machine (pod assembler).
    let pod_net_e: Option<Entity> = manual_machine_q.iter().find_map(|lports| {
        lports
            .ports()
            .iter()
            .find_map(|&port_e| port_q.get(port_e).ok().map(|m| m.0))
    });

    if let (Some(data), Some(graph), Some(net_e)) =
        (modal_mut.0.as_ref(), recipe_graph.as_ref(), pod_net_e)
    {
        // Aggregate storage for this network only — only items accessible to the
        // pod assembler should count toward reservation.
        let mut storage: std::collections::HashMap<String, u32> = Default::default();
        if let Ok(members) = net_members_q.get(net_e) {
            for &port_e in members.members() {
                let Ok(port_of) = port_of_q.get(port_e) else {
                    continue;
                };
                let Ok(unit) = storage_unit_q.get(port_of.0) else {
                    continue;
                };
                for (id, &count) in &unit.items {
                    *storage.entry(id.clone()).or_insert(0) += count;
                }
            }
        }
        if let Ok(mut queue) = queue_q.get_mut(net_e) {
            queue.enqueue_item(&data.item_id, data.quantity, graph, &storage);
        }
    }

    // Fire NetworkStorageChanged for all networks so recipe_check_system picks up the new jobs.
    let mut networks: HashSet<Entity> = HashSet::new();
    for member in &port_q {
        networks.insert(member.0);
    }
    for net in networks {
        storage_changed.write(NetworkStorageChanged { network: net });
    }
    modal_mut.0 = None;
}

// ---------------------------------------------------------------------------
// Content rebuild
// ---------------------------------------------------------------------------

fn update_content(
    modal: Res<CraftModal>,
    recipe_graph: Option<Res<RecipeGraph>>,
    storage_q: Query<&StorageUnit>,
    machine_q: Query<(&Machine, Option<&MachineActivity>)>,
    item_registry: Option<Res<ItemRegistry>>,
    progress: Option<Res<TechTreeProgress>>,
    content_q: Query<Entity, With<CraftModalContent>>,
    mut commands: Commands,
) {
    if !modal.is_changed() {
        return;
    }
    let Ok(content_entity) = content_q.single() else {
        return;
    };

    commands.entity(content_entity).despawn_children();

    let Some(data) = &modal.0 else { return };
    let Some(graph) = recipe_graph.as_ref() else {
        return;
    };

    let item_name = item_registry
        .as_ref()
        .and_then(|r| r.get(&data.item_id))
        .map_or(data.item_id.as_str(), |d| d.name.as_str())
        .to_string();

    // Aggregate storage
    let mut storage: std::collections::HashMap<String, u32> = Default::default();
    for unit in &storage_q {
        for (id, &count) in &unit.items {
            *storage.entry(id.clone()).or_insert(0) += count;
        }
    }
    let in_storage = storage.get(&data.item_id).copied().unwrap_or(0);

    // Find recipe
    let recipe = graph
        .producers
        .get(&data.item_id)
        .and_then(|ids| ids.first())
        .and_then(|id| graph.recipes.get(id));

    match data.phase {
        CraftPhase::Quantity => {
            build_phase1(
                &mut commands,
                content_entity,
                data,
                &item_name,
                in_storage,
                recipe.map(|r| r.machine_type.as_str()).unwrap_or("—"),
            );
        }
        CraftPhase::Plan => {
            let ingredients = recipe
                .map(|r| {
                    r.inputs
                        .iter()
                        .map(|inp| {
                            let needed = (inp.quantity * data.quantity as f32).ceil() as u32;
                            let have = storage.get(&inp.item).copied().unwrap_or(0);
                            ResolvedIngredient {
                                item_id: inp.item.clone(),
                                needed,
                                have,
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let machine_type = recipe.map(|r| r.machine_type.as_str()).unwrap_or("—");
            let missing_count = ingredients.iter().filter(|i| i.have < i.needed).count();

            // Find a capable idle machine
            let capable_machine = machine_q.iter().find_map(|(m, act)| {
                (m.machine_type == machine_type && act.is_none()).then(|| m.machine_type.clone())
            });

            // Check unlock status
            let unlocked = recipe
                .map(|r| {
                    progress
                        .as_ref()
                        .map(|p| p.unlocked_recipes.contains(&r.id))
                        .unwrap_or(true)
                })
                .unwrap_or(false);

            build_phase2(
                &mut commands,
                content_entity,
                data,
                &item_name,
                in_storage,
                &ingredients,
                machine_type,
                capable_machine.is_some(),
                missing_count,
                unlocked,
                item_registry.as_ref(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 1 layout
// ---------------------------------------------------------------------------

fn build_phase1(
    commands: &mut Commands,
    content: Entity,
    data: &CraftModalData,
    item_name: &str,
    in_storage: u32,
    machine_type: &str,
) {
    commands.entity(content).with_children(|root| {
        // Header
        spawn_modal_header(root, "CRAFT  —  set quantity", false, 0);

        // Body
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(space::XL)),
            row_gap: Val::Px(space::XL),
            ..default()
        })
        .with_children(|body| {
            // Item info
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(space::SM),
                ..default()
            })
            .with_children(|info| {
                info.spawn((
                    Text::new(item_name),
                    TextFont {
                        font_size: font_size::H_MD,
                        ..default()
                    },
                    TextColor(palette::TEXT),
                ));
                info.spawn(Node {
                    column_gap: Val::Px(space::MD),
                    ..default()
                })
                .with_children(|row| {
                    spawn_tag(row, "MACHINE", machine_type.to_uppercase().as_str());
                    row.spawn((
                        Text::new(format!("in storage: {in_storage}")),
                        TextFont {
                            font_size: font_size::LABEL,
                            ..default()
                        },
                        TextColor(if in_storage > 0 {
                            palette::OK
                        } else {
                            COLOR_DIM
                        }),
                    ));
                });
            });

            // Quantity stepper
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(space::MD),
                ..default()
            })
            .with_children(|qty_col| {
                qty_col.spawn((
                    Text::new("QUANTITY"),
                    TextFont {
                        font_size: font_size::H_XS,
                        ..default()
                    },
                    TextColor(COLOR_DIM),
                ));

                // -/qty/+ row
                qty_col
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(space::MD),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_stepper_btn(row, "−", CraftQtyMinus);
                        row.spawn((
                            Node {
                                min_width: Val::Px(60.0),
                                justify_content: JustifyContent::Center,
                                border: UiRect::all(Val::Px(border::THIN)),
                                padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::SM)),
                                ..default()
                            },
                            BackgroundColor(palette::P2),
                            BorderColor::all(palette::BORDER_STRONG),
                        ))
                        .with_child((
                            Text::new(data.quantity.to_string()),
                            TextFont {
                                font_size: font_size::H_MD,
                                ..default()
                            },
                            TextColor(palette::TEXT),
                        ));
                        spawn_stepper_btn(row, "+", CraftQtyPlus);
                    });

                // Preset row
                qty_col
                    .spawn(Node {
                        column_gap: Val::Px(space::SM),
                        ..default()
                    })
                    .with_children(|presets| {
                        for &p in &[1u32, 4, 16, 64, 256] {
                            let active = data.quantity == p;
                            presets
                                .spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(
                                            Val::Px(space::LG),
                                            Val::Px(space::XS + 1.0),
                                        ),
                                        border: UiRect::all(Val::Px(border::THIN)),
                                        ..default()
                                    },
                                    BackgroundColor(if active {
                                        palette::ACCENT
                                    } else {
                                        palette::P2
                                    }),
                                    BorderColor::all(if active {
                                        palette::ACCENT
                                    } else {
                                        palette::BORDER
                                    }),
                                    CraftQtyPreset(p),
                                ))
                                .with_child((
                                    Text::new(p.to_string()),
                                    TextFont {
                                        font_size: font_size::LABEL,
                                        ..default()
                                    },
                                    TextColor(if active { Color::WHITE } else { palette::DIM }),
                                    Pickable::IGNORE,
                                ));
                        }
                    });
            });

            // Footer note
            body.spawn((
                Text::new("plan resolves from current network state + machine priorities"),
                TextFont {
                    font_size: font_size::LABEL_SM,
                    ..default()
                },
                TextColor(COLOR_DIM),
            ));
        });

        // Action bar
        root.spawn(Node {
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(space::MD),
            padding: UiRect {
                left: Val::Px(space::XL),
                right: Val::Px(space::XL),
                top: Val::Px(0.0),
                bottom: Val::Px(space::XL),
            },
            ..default()
        })
        .with_children(|bar| {
            spawn_action_btn(bar, "CANCEL", false, false, CraftCloseBtn);
            spawn_action_btn(bar, "RESOLVE PLAN  →", true, false, CraftResolveBtn);
        });
    });
}

// ---------------------------------------------------------------------------
// Phase 2 layout
// ---------------------------------------------------------------------------

fn build_phase2(
    commands: &mut Commands,
    content: Entity,
    data: &CraftModalData,
    item_name: &str,
    in_storage: u32,
    ingredients: &[ResolvedIngredient],
    machine_type: &str,
    machine_available: bool,
    missing_count: usize,
    unlocked: bool,
    item_registry: Option<&Res<ItemRegistry>>,
) {
    let _ = in_storage;
    commands.entity(content).with_children(|root| {
        // Header
        spawn_modal_header(
            root,
            "CRAFT  —  execution plan",
            missing_count > 0,
            missing_count,
        );

        // Body
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(space::XL)),
            row_gap: Val::Px(space::XL),
            ..default()
        })
        .with_children(|body| {
            // Legend
            body.spawn(Node {
                column_gap: Val::Px(space::MD),
                ..default()
            })
            .with_children(|legend| {
                legend.spawn((
                    Text::new("DEPENDENCY TREE  —  read-only"),
                    TextFont {
                        font_size: font_size::H_XS,
                        ..default()
                    },
                    TextColor(COLOR_DIM),
                ));
            });

            // Target item row
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(space::XS),
                ..default()
            })
            .with_children(|tree| {
                // Root item
                tree.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(space::MD),
                    padding: UiRect::axes(Val::Px(space::SM), Val::Px(space::XS)),
                    border: UiRect::all(Val::Px(border::THIN)),
                    ..default()
                })
                .insert(BackgroundColor(palette::P2))
                .insert(BorderColor::all(palette::BORDER))
                .with_children(|row| {
                    row.spawn((
                        Text::new("☐"),
                        TextFont {
                            font_size: font_size::LABEL,
                            ..default()
                        },
                        TextColor(palette::ACCENT),
                    ));
                    row.spawn((
                        Text::new(item_name),
                        TextFont {
                            font_size: font_size::LABEL,
                            ..default()
                        },
                        TextColor(palette::TEXT),
                    ));
                    row.spawn((
                        Text::new(format!("×{}", data.quantity)),
                        TextFont {
                            font_size: font_size::LABEL,
                            ..default()
                        },
                        TextColor(COLOR_DIM),
                    ));
                    row.spawn(Node {
                        flex_grow: 1.0,
                        ..default()
                    });
                    spawn_tag(row, "→ craft", machine_type);
                });

                // Ingredient rows (indented)
                for ing in ingredients {
                    let stocked = ing.have >= ing.needed;
                    let name = item_registry
                        .and_then(|r| r.get(&ing.item_id))
                        .map_or(ing.item_id.as_str(), |d| d.name.as_str())
                        .to_string();

                    tree.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(space::MD),
                        padding: UiRect {
                            left: Val::Px(space::XXL),
                            right: Val::Px(space::SM),
                            top: Val::Px(space::XS),
                            bottom: Val::Px(space::XS),
                        },
                        border: UiRect {
                            left: Val::Px(2.0),
                            ..UiRect::DEFAULT
                        },
                        ..default()
                    })
                    .insert(BackgroundColor(Color::NONE))
                    .insert(BorderColor::all(if stocked {
                        palette::OK
                    } else {
                        palette::ERR
                    }))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(if stocked { "✓" } else { "✗" }),
                            TextFont {
                                font_size: font_size::LABEL,
                                ..default()
                            },
                            TextColor(if stocked { palette::OK } else { palette::ERR }),
                        ));
                        row.spawn((
                            Text::new(name),
                            TextFont {
                                font_size: font_size::LABEL,
                                ..default()
                            },
                            TextColor(palette::TEXT),
                        ));
                        row.spawn((
                            Text::new(format!("×{}", ing.needed)),
                            TextFont {
                                font_size: font_size::LABEL,
                                ..default()
                            },
                            TextColor(COLOR_DIM),
                        ));
                        row.spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        });
                        if stocked {
                            row.spawn((
                                Text::new(format!("✓ stocked ({})", ing.have)),
                                TextFont {
                                    font_size: font_size::LABEL,
                                    ..default()
                                },
                                TextColor(palette::OK),
                            ));
                        } else {
                            row.spawn((
                                Text::new(format!("✗ MISSING  (have {})", ing.have)),
                                TextFont {
                                    font_size: font_size::LABEL,
                                    ..default()
                                },
                                TextColor(palette::ERR),
                            ));
                        }
                    });
                }
            });

            // Machine plan section
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(space::MD),
                ..default()
            })
            .with_children(|section| {
                section.spawn((
                    Text::new("MACHINE PLAN"),
                    TextFont {
                        font_size: font_size::H_XS,
                        ..default()
                    },
                    TextColor(COLOR_DIM),
                ));
                section
                    .spawn(Node {
                        padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::SM)),
                        border: UiRect::all(Val::Px(border::THIN)),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(space::MD),
                        ..default()
                    })
                    .insert(BackgroundColor(palette::P2))
                    .insert(BorderColor::all(if machine_available {
                        palette::BORDER
                    } else {
                        palette::ERR
                    }))
                    .with_children(|row| {
                        let machine_label = machine_type.to_uppercase();
                        row.spawn((
                            Text::new(if machine_available {
                                format!("{machine_label}  (idle)")
                            } else {
                                format!("{machine_label}  (none available)")
                            }),
                            TextFont {
                                font_size: font_size::LABEL,
                                ..default()
                            },
                            TextColor(if machine_available {
                                palette::TEXT
                            } else {
                                palette::ERR
                            }),
                        ));
                        row.spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        });
                        row.spawn((
                            Text::new(format!("→ {item_name}")),
                            TextFont {
                                font_size: font_size::LABEL,
                                ..default()
                            },
                            TextColor(COLOR_DIM),
                        ));
                    });

                if !unlocked {
                    section.spawn((
                        Text::new("⚠ recipe not yet unlocked in tech tree"),
                        TextFont {
                            font_size: font_size::LABEL,
                            ..default()
                        },
                        TextColor(palette::WARN),
                    ));
                }

                section.spawn((
                    Text::new("based on current machine priorities"),
                    TextFont {
                        font_size: font_size::LABEL_SM,
                        ..default()
                    },
                    TextColor(COLOR_DIM),
                ));
            });
        });

        // Action bar
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(space::SM),
            padding: UiRect {
                left: Val::Px(space::XL),
                right: Val::Px(space::XL),
                top: Val::Px(0.0),
                bottom: Val::Px(space::XL),
            },
            ..default()
        })
        .with_children(|bar| {
            let stall = missing_count > 0 || !machine_available || !unlocked;
            let enqueue_label = if stall {
                "⚠  ENQUEUE  ·  will stall".to_string()
            } else {
                "▶  ENQUEUE".to_string()
            };
            spawn_action_btn(bar, &enqueue_label, true, stall, CraftEnqueueBtn);
            spawn_action_btn(bar, "←  BACK", false, false, CraftBackBtn);
        });
    });
}

// ---------------------------------------------------------------------------
// Shared layout helpers
// ---------------------------------------------------------------------------

fn spawn_modal_header(
    parent: &mut ChildSpawnerCommands<'_>,
    title: &str,
    has_warning: bool,
    missing_count: usize,
) {
    parent
        .spawn(Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::axes(Val::Px(space::XL), Val::Px(space::LG)),
            border: UiRect::bottom(Val::Px(border::THIN)),
            ..default()
        })
        .insert(BorderColor::all(palette::BORDER))
        .with_children(|h| {
            // Left: title + optional warning badge
            h.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(space::MD),
                ..default()
            })
            .with_children(|left| {
                left.spawn((
                    Text::new(title),
                    TextFont {
                        font_size: font_size::H_SM,
                        ..default()
                    },
                    TextColor(palette::DIM),
                ));
                if has_warning {
                    left.spawn(Node {
                        padding: UiRect::axes(Val::Px(space::SM), Val::Px(space::XS)),
                        border: UiRect::all(Val::Px(border::THIN)),
                        ..default()
                    })
                    .insert(BackgroundColor(Color::srgba(0.72, 0.29, 0.29, 0.2)))
                    .insert(BorderColor::all(palette::ERR))
                    .with_child((
                        Text::new(format!("▲ {missing_count} MISSING")),
                        TextFont {
                            font_size: font_size::LABEL_SM,
                            ..default()
                        },
                        TextColor(palette::ERR),
                    ));
                }
            });

            // Right: ESC CLOSE
            h.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::XS)),
                    border: UiRect::all(Val::Px(border::THIN)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(palette::BORDER),
                CraftCloseBtn,
            ))
            .with_child((
                Text::new("ESC  CLOSE"),
                TextFont {
                    font_size: font_size::LABEL_SM,
                    ..default()
                },
                TextColor(COLOR_DIM),
                Pickable::IGNORE,
            ));
        });
}

fn spawn_tag(parent: &mut ChildSpawnerCommands<'_>, label: &str, value: &str) {
    parent
        .spawn(Node {
            padding: UiRect::axes(Val::Px(space::SM), Val::Px(1.0)),
            border: UiRect::all(Val::Px(border::THIN)),
            align_items: AlignItems::Center,
            column_gap: Val::Px(space::XS),
            ..default()
        })
        .insert(BackgroundColor(palette::P3))
        .insert(BorderColor::all(palette::BORDER))
        .with_children(|t| {
            if !label.is_empty() {
                t.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: font_size::TAG,
                        ..default()
                    },
                    TextColor(COLOR_DIM),
                ));
            }
            t.spawn((
                Text::new(value),
                TextFont {
                    font_size: font_size::TAG,
                    ..default()
                },
                TextColor(palette::TEXT),
            ));
        });
}

fn spawn_stepper_btn<T: Component>(parent: &mut ChildSpawnerCommands<'_>, label: &str, marker: T) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(32.0),
                height: Val::Px(32.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(border::THIN)),
                ..default()
            },
            BackgroundColor(palette::P2),
            BorderColor::all(palette::BORDER_STRONG),
            marker,
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: font_size::H_MD,
                ..default()
            },
            TextColor(palette::TEXT),
            Pickable::IGNORE,
        ));
}

fn spawn_action_btn<T: Component>(
    parent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    primary: bool,
    warn: bool,
    marker: T,
) {
    let (bg, border_color, text_color) = if warn {
        (
            Color::srgba(0.72, 0.29, 0.29, 0.3),
            palette::ERR,
            palette::ERR,
        )
    } else if primary {
        (palette::ACCENT, palette::ACCENT, Color::WHITE)
    } else {
        (Color::NONE, palette::BORDER_STRONG, COLOR_DIM)
    };

    parent
        .spawn((
            Button,
            Node {
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(Val::Px(space::XL), Val::Px(space::MD)),
                border: UiRect::all(Val::Px(border::THIN)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border_color),
            marker,
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: font_size::BUTTON,
                ..default()
            },
            TextColor(text_color),
            Pickable::IGNORE,
        ));
}

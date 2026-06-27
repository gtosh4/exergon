use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use crate::planet::{
    PlanetProperties, PlanetPropertyKey, PlanetPropertyViewLog, PlanetPropertyViewed,
    PlanetPropertyVisibility, PropertyVisibility, ViewContext, qualitative_label,
};
use crate::world::Player;
use crate::{
    GameState,
    inventory::{Hotbar, HotbarSlot, InventoryOpen, ItemRegistry},
    logistics::StorageUnit,
    recipe_graph::RecipeGraph,
    research::TechTreeProgress,
    ui::{
        panels::craft_modal::{CraftModal, CraftModalData, CraftPhase},
        theme::{COLOR_DIM, COLOR_GOLD, COLOR_OVERLAY_BG, border, font_size, palette, space},
    },
};

#[derive(Component)]
struct InventoryPanel;

#[derive(Component)]
struct NetworkListRoot;

#[derive(Component)]
struct PlanetTabRoot;

#[derive(Component)]
struct TabButton(TerminalTab);

#[derive(Component)]
struct PlanetRow(PlanetPropertyKey);

#[derive(Resource, Default)]
struct TerminalTabState(TerminalTab);

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum TerminalTab {
    #[default]
    Network,
    Planet,
}

#[derive(Component)]
struct CraftItemButton(String);

#[derive(Component)]
struct HotbarSlotDrop(usize);

#[derive(Component)]
struct DraggableItem(String);

#[derive(Component)]
struct DragCursor;

#[derive(Component)]
struct DragCursorText;

#[derive(Resource, Default)]
struct InventoryDrag(Option<String>);

pub fn plugin(app: &mut App) {
    app.init_resource::<InventoryDrag>()
        .init_resource::<TerminalTabState>()
        .add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(
            Update,
            (
                sync_visibility,
                handle_tab_click,
                handle_craft_click,
                start_drag,
                handle_drop,
                update_drag_cursor,
                update_items,
                update_planet_tab,
                planet_view_tracker,
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
            InventoryPanel,
        ))
        .with_children(|outer| {
            outer
                .spawn((
                    Node {
                        width: Val::Percent(80.0),
                        min_height: Val::Percent(70.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.039, 0.039, 0.039)),
                    BorderColor::all(COLOR_DIM),
                ))
                .with_children(|root| {
                    // Header
                    root.spawn(Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..default()
                    })
                    .with_children(|h| {
                        h.spawn((
                            Text::new("TERMINAL"),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(COLOR_GOLD),
                        ));
                        h.spawn((
                            Text::new("[Tab / Esc]"),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(COLOR_DIM),
                        ));
                    });

                    // Tab strip
                    root.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    })
                    .with_children(|tabs| {
                        spawn_tab(tabs, TerminalTab::Network, "NETWORK");
                        spawn_tab(tabs, TerminalTab::Planet, "PLANET");
                    });

                    // Network item list
                    root.spawn((
                        Node {
                            overflow: Overflow::scroll_y(),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        crate::ui::widgets::ScrollableContent,
                        ScrollPosition::default(),
                        NetworkListRoot,
                    ));

                    // Planet tab content (hidden until active)
                    root.spawn((
                        Node {
                            overflow: Overflow::scroll_y(),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            display: Display::None,
                            ..default()
                        },
                        crate::ui::widgets::ScrollableContent,
                        ScrollPosition::default(),
                        PlanetTabRoot,
                    ));

                    // Separator
                    root.spawn(Node {
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(12.0)),
                        ..default()
                    })
                    .insert(BackgroundColor(COLOR_DIM));

                    // Hotbar
                    root.spawn((
                        Text::new("HOTBAR  —  drag items here to assign"),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(COLOR_DIM),
                    ));
                    root.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    })
                    .with_children(|bar| {
                        for i in 0..9usize {
                            bar.spawn((
                                Node {
                                    width: Val::Px(96.0),
                                    height: Val::Px(96.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    margin: UiRect::right(Val::Px(4.0)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgb(0.392, 0.392, 0.392)),
                                BackgroundColor(COLOR_OVERLAY_BG),
                                HotbarSlotDrop(i),
                            ));
                        }
                    });
                });
        });

    // Drag cursor — spawned last, renders on top via GlobalZIndex
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.039, 0.039, 0.039)),
            BorderColor::all(COLOR_GOLD),
            GlobalZIndex(100),
            Pickable::IGNORE,
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            DragCursor,
        ))
        .with_child((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Pickable::IGNORE,
            DragCursorText,
        ));
}

fn sync_visibility(
    inv_open: Option<Res<InventoryOpen>>,
    mut drag: ResMut<InventoryDrag>,
    mut q: Query<&mut Visibility, With<InventoryPanel>>,
) {
    let visible = inv_open.is_some_and(|o| o.0);
    if !visible {
        drag.0 = None;
    }
    for mut v in &mut q {
        *v = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn start_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    inv_open: Option<Res<InventoryOpen>>,
    q: Query<(&Interaction, &DraggableItem), Changed<Interaction>>,
    mut drag: ResMut<InventoryDrag>,
) {
    if !inv_open.is_some_and(|o| o.0) || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    for (interaction, item) in &q {
        if *interaction == Interaction::Pressed {
            drag.0 = Some(item.0.clone());
            return;
        }
    }
}

fn handle_drop(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut drag: ResMut<InventoryDrag>,
    hotbar_slots: Query<(&HotbarSlotDrop, &UiGlobalTransform, &ComputedNode)>,
    mut hotbar: ResMut<Hotbar>,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    let Some(item_id) = drag.0.take() else { return };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    for (slot, gt, computed) in &hotbar_slots {
        if computed.contains_point(*gt, cursor) {
            if let Some(s) = hotbar.slots.get_mut(slot.0) {
                *s = Some(HotbarSlot { item_id });
            }
            return;
        }
    }
}

fn update_drag_cursor(
    drag: Res<InventoryDrag>,
    item_registry: Option<Res<ItemRegistry>>,
    windows: Query<&Window>,
    mut cursor_q: Query<(&mut Node, &mut Visibility), With<DragCursor>>,
    mut text_q: Query<&mut Text, With<DragCursorText>>,
) {
    let Ok((mut node, mut vis)) = cursor_q.single_mut() else {
        return;
    };
    match &drag.0 {
        Some(item_id) => {
            *vis = Visibility::Inherited;
            if let Ok(mut text) = text_q.single_mut() {
                let name = item_registry
                    .as_ref()
                    .and_then(|r| r.get(item_id.as_str()))
                    .map_or(item_id.as_str(), |d| d.name.as_str())
                    .to_string();
                **text = name;
            }
            if let Ok(window) = windows.single()
                && let Some(cursor) = window.cursor_position()
            {
                node.left = Val::Px(cursor.x + 12.0);
                node.top = Val::Px(cursor.y + 12.0);
            }
        }
        None => {
            *vis = Visibility::Hidden;
        }
    }
}

fn spawn_tab(parent: &mut ChildSpawnerCommands<'_>, tab: TerminalTab, label: &str) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            Button,
            BackgroundColor(palette::P2),
            BorderColor::all(palette::BORDER),
            TabButton(tab),
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(COLOR_DIM),
            Pickable::IGNORE,
        ));
}

fn handle_tab_click(
    mut interactions: Query<(&Interaction, &TabButton), Changed<Interaction>>,
    mut state: ResMut<TerminalTabState>,
) {
    for (interaction, btn) in &mut interactions {
        if *interaction == Interaction::Pressed {
            state.0 = btn.0;
        }
    }
}

fn update_planet_tab(
    inv_open: Option<Res<InventoryOpen>>,
    tab_state: Res<TerminalTabState>,
    planet_q: Query<(&PlanetProperties, &PlanetPropertyVisibility)>,
    mut net_root_q: Query<&mut Node, (With<NetworkListRoot>, Without<PlanetTabRoot>)>,
    mut planet_root_q: Query<(Entity, &mut Node), (With<PlanetTabRoot>, Without<NetworkListRoot>)>,
    mut tab_buttons: Query<(&TabButton, &mut BorderColor, &Children)>,
    mut text_q: Query<&mut TextColor>,
    mut commands: Commands,
) {
    if !inv_open.is_some_and(|o| o.0) {
        return;
    }
    let planet_active = matches!(tab_state.0, TerminalTab::Planet);

    if let Ok(mut net_node) = net_root_q.single_mut() {
        net_node.display = if planet_active {
            Display::None
        } else {
            Display::Flex
        };
    }
    let Ok((planet_entity, mut planet_node)) = planet_root_q.single_mut() else {
        return;
    };
    planet_node.display = if planet_active {
        Display::Flex
    } else {
        Display::None
    };

    // Highlight active tab
    for (btn, mut border, children) in &mut tab_buttons {
        let active = btn.0 == tab_state.0;
        *border = if active {
            BorderColor::all(COLOR_GOLD)
        } else {
            BorderColor::all(palette::BORDER)
        };
        if let Some(&child) = children.first()
            && let Ok(mut tc) = text_q.get_mut(child)
        {
            tc.0 = if active { COLOR_GOLD } else { COLOR_DIM };
        }
    }

    if !planet_active {
        return;
    }

    commands.entity(planet_entity).despawn_children();
    let Ok((props, vis)) = planet_q.single() else {
        commands.entity(planet_entity).with_child((
            Text::new("(no planet data)"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        return;
    };

    commands.entity(planet_entity).with_children(|root| {
        let title = if props.name.epithet.is_empty() {
            props.name.catalog.clone()
        } else {
            format!("{}  \"{}\"", props.name.catalog, props.name.epithet)
        };
        root.spawn((
            Text::new(title),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(COLOR_GOLD),
        ));
        for key in PlanetPropertyKey::ALL {
            spawn_planet_row(root, key, props, vis);
        }
    });
}

fn spawn_planet_row(
    parent: &mut ChildSpawnerCommands<'_>,
    key: PlanetPropertyKey,
    props: &PlanetProperties,
    vis: &PlanetPropertyVisibility,
) {
    let visibility = vis.get(key);
    let (value, value_text, color) = match (visibility, key) {
        (PropertyVisibility::Hidden, _) => (0.0, key.hidden_hint().to_string(), COLOR_DIM),
        (_, PlanetPropertyKey::HazardType) => {
            (0.0, props.hazard_type.display().to_string(), palette::TEXT)
        }
        (PropertyVisibility::Qualitative, k) => {
            let v = planet_value(k, props);
            (v, qualitative_label(k, v).to_string(), palette::TEXT)
        }
        (PropertyVisibility::Revealed, k) => {
            let v = planet_value(k, props);
            (
                v,
                format!("{} [{:.2}]", qualitative_label(k, v), v),
                palette::TEXT,
            )
        }
    };
    let _ = value;

    parent
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                ..default()
            },
            Button,
            BackgroundColor(Color::NONE),
            Interaction::None,
            PlanetRow(key),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(key.display_name()),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(palette::TEXT),
                Pickable::IGNORE,
            ));
            row.spawn((
                Text::new(value_text),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(color),
                Pickable::IGNORE,
            ));
        });
}

fn planet_value(key: PlanetPropertyKey, props: &PlanetProperties) -> f32 {
    match key {
        PlanetPropertyKey::StellarDistance => props.stellar_distance,
        PlanetPropertyKey::AtmosphericOxygen => props.atmospheric_oxygen,
        PlanetPropertyKey::GeologicalActivity => props.geological_activity,
        PlanetPropertyKey::Temperature => props.temperature,
        PlanetPropertyKey::AtmosphericPressure => props.atmospheric_pressure,
        PlanetPropertyKey::WindIntensity => props.wind_intensity,
        PlanetPropertyKey::HazardType => 0.0,
    }
}

fn planet_view_tracker(
    rows: Query<(&Interaction, &PlanetRow), Changed<Interaction>>,
    mut viewed: MessageWriter<PlanetPropertyViewed>,
    mut player_q: Query<&mut PlanetPropertyViewLog, With<Player>>,
) {
    for (interaction, row) in &rows {
        if matches!(interaction, Interaction::Hovered | Interaction::Pressed) {
            viewed.write(PlanetPropertyViewed {
                property: row.0,
                context: ViewContext::Terminal,
            });
            if let Ok(mut log) = player_q.single_mut() {
                log.record(row.0);
            }
        }
    }
}

fn handle_craft_click(
    q: Query<(&Interaction, &CraftItemButton), Changed<Interaction>>,
    mut craft_modal: ResMut<CraftModal>,
) {
    for (interaction, btn) in &q {
        if *interaction == Interaction::Pressed {
            craft_modal.0 = Some(CraftModalData {
                item_id: btn.0.clone(),
                quantity: 1,
                phase: CraftPhase::Quantity,
            });
            return;
        }
    }
}

fn update_items(
    inv_open: Option<Res<InventoryOpen>>,
    hotbar: Option<Res<Hotbar>>,
    item_registry: Option<Res<ItemRegistry>>,
    recipe_graph: Option<Res<RecipeGraph>>,
    progress: Option<Res<TechTreeProgress>>,
    storage_q: Query<&StorageUnit>,
    net_list_q: Query<Entity, With<NetworkListRoot>>,
    hotbar_drop_q: Query<(Entity, &HotbarSlotDrop)>,
    drag: Res<InventoryDrag>,
    mut commands: Commands,
) {
    if !inv_open.is_some_and(|o| o.0) {
        return;
    }

    // Update hotbar drop slots
    if let Some(hotbar) = &hotbar {
        for (entity, slot) in &hotbar_drop_q {
            let i = slot.0;
            let selected = i == hotbar.selected;
            commands.entity(entity).insert(if selected {
                BorderColor::all(Color::srgb(1.0, 0.863, 0.196))
            } else {
                BorderColor::all(Color::srgb(0.392, 0.392, 0.392))
            });
            commands.entity(entity).despawn_children();

            match hotbar.slots.get(i).and_then(|s| s.as_ref()) {
                Some(s) => {
                    let name = item_registry
                        .as_ref()
                        .and_then(|r| r.get(&s.item_id))
                        .map_or(s.item_id.as_str(), |d| d.name.as_str())
                        .to_string();
                    let count: u32 = storage_q
                        .iter()
                        .filter_map(|u| u.items.get(&s.item_id))
                        .sum();
                    commands.entity(entity).with_children(|p| {
                        p.spawn((
                            Text::new(format!("{}\n×{}", name, count)),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                }
                None => {
                    commands.entity(entity).with_child((
                        Text::new(format!("{}", i + 1)),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.392, 0.392, 0.392)),
                    ));
                }
            }
        }
    }

    // Skip rebuilding item list during drag to preserve draggable entity state
    if drag.0.is_some() {
        return;
    }

    let Ok(net_list) = net_list_q.single() else {
        return;
    };
    commands.entity(net_list).despawn_children();

    // Aggregate storage
    let mut net_items: std::collections::HashMap<String, u32> = Default::default();
    for unit in &storage_q {
        for (id, &count) in &unit.items {
            *net_items.entry(id.clone()).or_insert(0) += count;
        }
    }

    // Build craftable set: items with at least one unlocked producing recipe
    let craftable: std::collections::HashSet<String> = recipe_graph
        .as_ref()
        .map(|g| {
            g.producers
                .iter()
                .filter(|(_, recipe_ids)| {
                    recipe_ids.iter().any(|rid| {
                        progress
                            .as_ref()
                            .map(|p| p.unlocked_recipes.contains(rid))
                            .unwrap_or(false)
                    })
                })
                .map(|(item_id, _)| item_id.clone())
                .collect()
        })
        .unwrap_or_default();

    // Rows: in-storage items first (sorted by name), then craftable-only items (qty=0)
    let mut rows: Vec<(String, u32)> = net_items.into_iter().collect();
    rows.sort_by_key(|(k, _)| k.clone());

    // Add craftable items not already in storage
    let in_storage_ids: std::collections::HashSet<_> =
        rows.iter().map(|(id, _)| id.clone()).collect();
    let mut craftable_only: Vec<(String, u32)> = craftable
        .iter()
        .filter(|id| !in_storage_ids.contains(*id))
        .map(|id| (id.clone(), 0u32))
        .collect();
    craftable_only.sort_by_key(|(k, _)| k.clone());
    rows.extend(craftable_only);

    if rows.is_empty() {
        commands.entity(net_list).with_child((
            Text::new("(network empty — no craftable items)"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        return;
    }

    // Table header
    commands.entity(net_list).with_children(|l| {
        l.spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect {
                left: Val::Px(space::SM),
                right: Val::Px(space::SM),
                top: Val::Px(space::XS),
                bottom: Val::Px(space::XS),
            },
            border: UiRect::bottom(Val::Px(border::THIN)),
            margin: UiRect::bottom(Val::Px(space::SM)),
            ..default()
        })
        .insert(BorderColor::all(palette::BORDER))
        .with_children(|hdr| {
            hdr.spawn((
                Text::new("NAME"),
                TextFont {
                    font_size: FontSize::Px(font_size::H_XS),
                    ..default()
                },
                TextColor(COLOR_DIM),
            ));
            hdr.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            hdr.spawn((
                Text::new("QTY"),
                TextFont {
                    font_size: FontSize::Px(font_size::H_XS),
                    ..default()
                },
                TextColor(COLOR_DIM),
            ));
            hdr.spawn(Node {
                width: Val::Px(28.0),
                ..default()
            }); // C badge space
            hdr.spawn(Node {
                width: Val::Px(56.0),
                ..default()
            }); // CRAFT btn space
        });

        // Item rows
        for (item_id, count) in &rows {
            let name = item_registry
                .as_ref()
                .and_then(|r| r.get(item_id.as_str()))
                .map_or(item_id.as_str(), |d| d.name.as_str())
                .to_string();
            let is_craftable = craftable.contains(item_id.as_str());
            let in_stock = *count > 0;
            let text_color = if in_stock { palette::TEXT } else { COLOR_DIM };

            l.spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(space::SM),
                    margin: UiRect::bottom(Val::Px(2.0)),
                    padding: UiRect::axes(Val::Px(space::SM), Val::Px(space::XS)),
                    ..default()
                },
                Button,
                BackgroundColor(Color::NONE),
                DraggableItem(item_id.clone()),
            ))
            .with_children(|row| {
                // Name
                row.spawn((
                    Text::new(name),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(text_color),
                    Pickable::IGNORE,
                ));

                row.spawn((
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));

                // Qty
                row.spawn((
                    Text::new(if in_stock {
                        count.to_string()
                    } else {
                        "—".to_string()
                    }),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(if in_stock { COLOR_GOLD } else { COLOR_DIM }),
                    Pickable::IGNORE,
                ));

                // C badge
                if is_craftable {
                    row.spawn(Node {
                        padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
                        border: UiRect::all(Val::Px(border::THIN)),
                        margin: UiRect::left(Val::Px(space::SM)),
                        ..default()
                    })
                    .insert(BackgroundColor(Color::srgba(0.541, 0.447, 0.667, 0.15)))
                    .insert(BorderColor::all(palette::ACCENT))
                    .with_child((
                        Text::new("C"),
                        TextFont {
                            font_size: FontSize::Px(font_size::TAG),
                            ..default()
                        },
                        TextColor(palette::ACCENT),
                        Pickable::IGNORE,
                    ));
                } else {
                    row.spawn((
                        Node {
                            width: Val::Px(20.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                }

                // CRAFT button
                if is_craftable {
                    row.spawn((
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::XS)),
                            border: UiRect::all(Val::Px(border::THIN)),
                            margin: UiRect::left(Val::Px(space::SM)),
                            ..default()
                        },
                        BackgroundColor(palette::ACCENT),
                        BorderColor::all(palette::ACCENT),
                        CraftItemButton(item_id.clone()),
                    ))
                    .with_child((
                        Text::new("CRAFT"),
                        TextFont {
                            font_size: FontSize::Px(font_size::TAG),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Pickable::IGNORE,
                    ));
                }
            });
        }
    });
}

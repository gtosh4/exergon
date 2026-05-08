use bevy::prelude::*;

use crate::{
    GameState,
    inventory::{Hotbar, HotbarSlot, InventoryOpen, ItemRegistry},
    logistics::StorageUnit,
    ui::theme::{COLOR_DIM, COLOR_GOLD, COLOR_OVERLAY_BG},
};

#[derive(Component)]
struct InventoryPanel;

#[derive(Component)]
struct NetworkListRoot;

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
        .add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(
            Update,
            (
                sync_visibility,
                start_drag,
                handle_drop,
                update_drag_cursor,
                update_items,
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
                            Text::new("NETWORK STORAGE"),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(COLOR_GOLD),
                        ));
                        h.spawn((
                            Text::new("[Tab / Esc]"),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(COLOR_DIM),
                        ));
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
                            font_size: 12.0,
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
                font_size: 14.0,
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
    hotbar_slots: Query<(&HotbarSlotDrop, &GlobalTransform, &ComputedNode)>,
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
        let center = gt.translation().truncate();
        let half = computed.size() / 2.0;
        if cursor.x >= center.x - half.x
            && cursor.x <= center.x + half.x
            && cursor.y >= center.y - half.y
            && cursor.y <= center.y + half.y
        {
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

fn update_items(
    inv_open: Option<Res<InventoryOpen>>,
    hotbar: Option<Res<Hotbar>>,
    item_registry: Option<Res<ItemRegistry>>,
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
                                font_size: 12.0,
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
                            font_size: 12.0,
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

    // Rebuild network item list
    let Ok(net_list) = net_list_q.single() else {
        return;
    };
    commands.entity(net_list).despawn_children();

    let mut net_items: std::collections::HashMap<String, u32> = Default::default();
    for unit in &storage_q {
        for (id, &count) in &unit.items {
            *net_items.entry(id.clone()).or_insert(0) += count;
        }
    }
    let mut net_sorted: Vec<_> = net_items.into_iter().collect();
    net_sorted.sort_by_key(|(k, _)| k.clone());

    if net_sorted.is_empty() {
        commands.entity(net_list).with_child((
            Text::new("(network empty)"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
    } else {
        commands.entity(net_list).with_children(|l| {
            for (item_id, count) in &net_sorted {
                let name = item_registry
                    .as_ref()
                    .and_then(|r| r.get(item_id.as_str()))
                    .map_or(item_id.as_str(), |d| d.name.as_str())
                    .to_string();
                l.spawn((
                    Node {
                        justify_content: JustifyContent::SpaceBetween,
                        margin: UiRect::bottom(Val::Px(4.0)),
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                        ..default()
                    },
                    Button,
                    BackgroundColor(Color::NONE),
                    DraggableItem(item_id.clone()),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new(name),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Pickable::IGNORE,
                    ));
                    row.spawn((
                        Text::new(format!("{count}")),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(COLOR_GOLD),
                        Pickable::IGNORE,
                    ));
                });
            }
        });
    }
}

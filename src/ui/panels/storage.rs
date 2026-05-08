use bevy::prelude::*;

use crate::{
    GameState,
    inventory::ItemRegistry,
    logistics::{LogisticsNetworkMember, StorageUnit},
    machine::{Machine, MachineEnergyPorts, MachineLogisticsPorts},
    power::PowerNetworkMember,
    ui::{
        MachineStatusPanel, StorageStatusPanel,
        theme::{COLOR_DIM, COLOR_GOLD},
        widgets::ScrollableContent,
    },
};

#[derive(Component)]
struct StoragePanel;

#[derive(Component)]
struct StorageCloseButton;

#[derive(Component)]
struct StoragePortsText;

#[derive(Component)]
struct StorageListRoot;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(
            Update,
            (sync_visibility, update_ports, update_list, handle_close)
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
            StoragePanel,
        ))
        .with_children(|outer| {
            outer
                .spawn((
                    Node {
                        width: Val::Percent(60.0),
                        min_height: Val::Percent(40.0),
                        max_height: Val::Percent(80.0),
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
                            Text::new("STORAGE CRATE"),
                            TextFont {
                                font_size: 18.0,
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
                            StorageCloseButton,
                        ))
                        .with_child((
                            Text::new("✕"),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(COLOR_DIM),
                        ));
                    });

                    // IO ports
                    root.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(COLOR_DIM),
                        Node {
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        },
                        StoragePortsText,
                    ));

                    // Scrollable item list
                    root.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            flex_grow: 1.0,
                            ..default()
                        },
                        ScrollableContent,
                        ScrollPosition::default(),
                        StorageListRoot,
                    ));
                });
        });
}

fn sync_visibility(
    panel: Res<StorageStatusPanel>,
    machine_panel: Res<MachineStatusPanel>,
    mut q: Query<&mut Visibility, With<StoragePanel>>,
) {
    let visible = panel.0.is_some() && machine_panel.entity.is_none();
    for mut v in &mut q {
        *v = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn update_ports(
    panel: Res<StorageStatusPanel>,
    storage_q: Query<(
        &Machine,
        Option<&MachineLogisticsPorts>,
        Option<&MachineEnergyPorts>,
    )>,
    port_log_q: Query<Option<&LogisticsNetworkMember>>,
    port_pwr_q: Query<Option<&PowerNetworkMember>>,
    mut ports_q: Query<&mut Text, With<StoragePortsText>>,
) {
    if !panel.is_changed() {
        return;
    }
    let Ok(mut t) = ports_q.single_mut() else {
        return;
    };
    let Some(entity) = panel.0 else {
        **t = String::new();
        return;
    };
    let Ok((machine, logistics_ports, energy_ports)) = storage_q.get(entity) else {
        **t = String::new();
        return;
    };
    let log_lines: Vec<String> = logistics_ports
        .map(|lp| {
            lp.ports()
                .iter()
                .enumerate()
                .map(|(i, &port_e)| {
                    let net_str = port_log_q
                        .get(port_e)
                        .ok()
                        .flatten()
                        .map(|m| format!("{:?}", m.0))
                        .unwrap_or_else(|| "—".to_string());
                    format!("PORT {}: {}", i, net_str)
                })
                .collect()
        })
        .unwrap_or_default();
    let pwr_lines: Vec<String> = energy_ports
        .map(|ep| {
            ep.ports()
                .iter()
                .enumerate()
                .map(|(i, &port_e)| {
                    let net_str = port_pwr_q
                        .get(port_e)
                        .ok()
                        .flatten()
                        .map(|m| format!("{:?}", m.0))
                        .unwrap_or_else(|| "—".to_string());
                    format!("PORT {}: {}", i, net_str)
                })
                .collect()
        })
        .unwrap_or_default();
    let log_text = if log_lines.is_empty() {
        "—".to_string()
    } else {
        log_lines.join(", ")
    };
    let pwr_text = if pwr_lines.is_empty() {
        "—".to_string()
    } else {
        pwr_lines.join(", ")
    };
    **t = format!(
        "▸ {} logistics port(s) · {}\n⚡ {} power port(s) · {}",
        machine.logistics_ports.len(),
        log_text,
        machine.energy_ports.len(),
        pwr_text,
    );
}

fn update_list(
    panel: Res<StorageStatusPanel>,
    storage_q: Query<&StorageUnit>,
    item_registry: Option<Res<ItemRegistry>>,
    list_q: Query<Entity, With<StorageListRoot>>,
    mut commands: Commands,
) {
    if !panel.is_changed() {
        return;
    }
    let Ok(list_entity) = list_q.single() else {
        return;
    };

    commands.entity(list_entity).despawn_children();

    let Some(storage_entity) = panel.0 else {
        return;
    };
    let Ok(unit) = storage_q.get(storage_entity) else {
        return;
    };

    if unit.items.is_empty() {
        commands.entity(list_entity).with_child((
            Text::new("(empty)"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(COLOR_DIM),
        ));
        return;
    }

    let mut items: Vec<(&String, u32)> = unit.items.iter().map(|(k, &c)| (k, c)).collect();
    items.sort_by_key(|(k, _)| k.as_str());

    commands.entity(list_entity).with_children(|list| {
        for (item_id, count) in &items {
            let name = item_registry
                .as_ref()
                .and_then(|r| r.get(item_id))
                .map_or(item_id.as_str(), |d| d.name.as_str())
                .to_string();
            list.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::bottom(Val::Px(4.0)),
                padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Text::new(name),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                row.spawn((
                    Text::new(format!("{count}")),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(COLOR_GOLD),
                ));
            });
        }
    });
}

fn handle_close(
    q: Query<&Interaction, (Changed<Interaction>, With<StorageCloseButton>)>,
    mut panel: ResMut<StorageStatusPanel>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            panel.0 = None;
        }
    }
}

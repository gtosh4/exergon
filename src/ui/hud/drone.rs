use bevy::prelude::*;

use crate::aegis::{AegisActive, AegisEmitter, AegisRadius};
use crate::drone::{Drone, DroneCargoOpen, DroneInventory};
use crate::ui::theme::{COLOR_DIM, COLOR_GOLD, COLOR_OVERLAY_BG};
use crate::{GameState, PlayMode};

const COLOR_REMOTE: Color = Color::srgb(0.2, 0.8, 0.9);
const COLOR_LOCAL: Color = Color::srgb(0.9, 0.9, 0.9);
const DEPOSIT_RADIUS: f32 = 15.0;

#[derive(Component)]
struct ModeIndicatorText;

#[derive(Component)]
struct DroneCargoRoot;

#[derive(Component)]
struct DroneCargoText;

#[derive(Component)]
struct DepositPromptRoot;

#[derive(Component)]
struct DroneCargoPanel;

#[derive(Component)]
struct DroneCargoPanelText;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn_widgets)
        .add_systems(
            Update,
            (
                update_mode_indicator,
                update_cargo_hud,
                update_deposit_prompt,
                sync_cargo_panel_visibility,
                update_cargo_panel_items,
            )
                .run_if(in_state(GameState::Playing)),
        );
}

fn spawn_widgets(mut commands: Commands) {
    // Mode indicator — top-right corner
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(COLOR_OVERLAY_BG),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
        ))
        .with_child((
            Text::new("◈ LOCAL"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(COLOR_LOCAL),
            Pickable::IGNORE,
            ModeIndicatorText,
        ));

    // Drone cargo — right side, below mode indicator, hidden when not in drone mode
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(42.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(COLOR_OVERLAY_BG),
            Pickable::IGNORE,
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            DroneCargoRoot,
        ))
        .with_child((
            Text::new(""),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(COLOR_GOLD),
            Pickable::IGNORE,
            DroneCargoText,
        ));

    // Deposit prompt — bottom center, hidden unless near base
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(80.0),
                left: Val::Percent(50.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(COLOR_OVERLAY_BG),
            Pickable::IGNORE,
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            DepositPromptRoot,
        ))
        .with_child((
            Text::new("E — deposit samples"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_REMOTE),
            Pickable::IGNORE,
        ));

    // Drone cargo panel — full-screen overlay, centered, shown when DroneCargoOpen
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
            DroneCargoPanel,
        ))
        .with_children(|outer| {
            outer
                .spawn((
                    Node {
                        width: Val::Px(360.0),
                        min_height: Val::Px(200.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.039, 0.039, 0.039)),
                    BorderColor::all(COLOR_DIM),
                ))
                .with_children(|root| {
                    root.spawn(Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    })
                    .with_children(|h| {
                        h.spawn((
                            Text::new("DRONE CARGO"),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(COLOR_REMOTE),
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
                    root.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(COLOR_GOLD),
                        DroneCargoPanelText,
                    ));
                });
        });
}

fn sync_cargo_panel_visibility(
    cargo_open: Option<Res<DroneCargoOpen>>,
    mut panel_q: Query<&mut Visibility, With<DroneCargoPanel>>,
) {
    let Ok(mut vis) = panel_q.single_mut() else {
        return;
    };
    let open = cargo_open.is_some_and(|o| o.0);
    *vis = if open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
}

fn update_cargo_panel_items(
    cargo_open: Option<Res<DroneCargoOpen>>,
    drone_q: Query<&DroneInventory, With<Drone>>,
    mut text_q: Query<&mut Text, With<DroneCargoPanelText>>,
) {
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };
    if !cargo_open.is_some_and(|o| o.0) {
        return;
    }
    let Ok(inventory) = drone_q.single() else {
        **text = "(no drone)".to_string();
        return;
    };
    if inventory.items.is_empty() {
        **text = "(empty)".to_string();
        return;
    }
    let mut lines: Vec<String> = inventory
        .items
        .iter()
        .map(|(id, count)| format!("{id}  ×{count}"))
        .collect();
    lines.sort();
    **text = lines.join("\n");
}

fn update_mode_indicator(
    play_mode: Res<State<PlayMode>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<ModeIndicatorText>>,
) {
    let Ok((mut text, mut color)) = text_q.single_mut() else {
        return;
    };
    match play_mode.get() {
        PlayMode::DronePilot => {
            **text = "◈ REMOTE".to_string();
            color.0 = COLOR_REMOTE;
        }
        _ => {
            **text = "◈ LOCAL".to_string();
            color.0 = COLOR_LOCAL;
        }
    }
}

fn update_cargo_hud(
    play_mode: Res<State<PlayMode>>,
    drone_q: Query<&DroneInventory, With<Drone>>,
    mut root_q: Query<&mut Visibility, With<DroneCargoRoot>>,
    mut text_q: Query<&mut Text, With<DroneCargoText>>,
) {
    let Ok(mut vis) = root_q.single_mut() else {
        return;
    };
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    if *play_mode.get() != PlayMode::DronePilot {
        *vis = Visibility::Hidden;
        return;
    }

    let Ok(inventory) = drone_q.single() else {
        *vis = Visibility::Hidden;
        return;
    };

    if inventory.items.is_empty() {
        **text = "Cargo: (empty)".to_string();
        *vis = Visibility::Inherited;
        return;
    }

    let mut lines: Vec<String> = inventory
        .items
        .iter()
        .map(|(id, count)| format!("  {id}: {count}"))
        .collect();
    lines.sort();
    let label = format!("Cargo:\n{}", lines.join("\n"));
    **text = label;
    *vis = Visibility::Inherited;
}

fn update_deposit_prompt(
    play_mode: Res<State<PlayMode>>,
    drone_q: Query<(&Transform, &DroneInventory), With<Drone>>,
    aegis_q: Query<(&Transform, &AegisRadius), (With<AegisEmitter>, With<AegisActive>)>,
    mut root_q: Query<&mut Visibility, With<DepositPromptRoot>>,
) {
    let Ok(mut vis) = root_q.single_mut() else {
        return;
    };
    if *play_mode.get() != PlayMode::DronePilot {
        *vis = Visibility::Hidden;
        return;
    }
    let Ok((drone_transform, inventory)) = drone_q.single() else {
        *vis = Visibility::Hidden;
        return;
    };
    if inventory.items.is_empty() {
        *vis = Visibility::Hidden;
        return;
    }
    let near_base = aegis_q.iter().any(|(aegis_transform, _)| {
        drone_transform
            .translation
            .distance(aegis_transform.translation)
            <= DEPOSIT_RADIUS
    });
    *vis = if near_base {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
}

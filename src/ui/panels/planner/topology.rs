use bevy::ecs::message::{Message, MessageReader};
use bevy::prelude::*;

use crate::{
    GameState,
    logistics::LogisticsCableSegment,
    machine::{Machine, MachineState},
    power::PowerCableSegment,
    ui::theme::palette,
};

// ---------------------------------------------------------------------------
// Resources / types
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy)]
pub struct NetworkFilter {
    pub logistics: bool,
    pub power: bool,
}

#[derive(Resource, Default)]
pub struct TopologyOverlay {
    pub enabled: bool,
    pub filter: NetworkFilter,
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ToggleTopologyOverlay;
impl Message for ToggleTopologyOverlay {}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct TopoHudRoot;

#[derive(Component)]
struct TopoFilterL;

#[derive(Component)]
struct TopoFilterP;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TopologyPlugin;

impl Plugin for TopologyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TopologyOverlay>()
            .add_message::<ToggleTopologyOverlay>()
            .add_systems(OnEnter(GameState::Playing), spawn_topo_hud)
            .add_systems(
                Update,
                (
                    handle_toggle,
                    topology_draw_system,
                    filter_hud_sync,
                    topo_key_n,
                    handle_filter_buttons,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn spawn_topo_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(8.0),
                column_gap: Val::Px(4.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(palette::P1),
            BorderColor::all(palette::BORDER),
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            TopoHudRoot,
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("TOPO"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::DIM),
            ));
            row.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(palette::P2),
                BorderColor::all(palette::BORDER),
                TopoFilterL,
            ))
            .with_child((
                Text::new("L"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::OK),
            ));
            row.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(palette::P2),
                BorderColor::all(palette::BORDER),
                TopoFilterP,
            ))
            .with_child((
                Text::new("P"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(palette::WARN),
            ));
        });
}

fn handle_toggle(
    mut reader: MessageReader<ToggleTopologyOverlay>,
    mut overlay: ResMut<TopologyOverlay>,
) {
    for _ in reader.read() {
        overlay.enabled = !overlay.enabled;
        if overlay.enabled && !overlay.filter.logistics && !overlay.filter.power {
            overlay.filter.logistics = true;
        }
    }
}

fn topo_key_n(keyboard: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keyboard.just_pressed(KeyCode::KeyN) {
        commands.write_message(ToggleTopologyOverlay);
    }
}

fn handle_filter_buttons(
    l_q: Query<&Interaction, (Changed<Interaction>, With<TopoFilterL>)>,
    p_q: Query<&Interaction, (Changed<Interaction>, With<TopoFilterP>)>,
    mut overlay: ResMut<TopologyOverlay>,
) {
    for interaction in &l_q {
        if *interaction == Interaction::Pressed {
            overlay.filter.logistics = !overlay.filter.logistics;
        }
    }
    for interaction in &p_q {
        if *interaction == Interaction::Pressed {
            overlay.filter.power = !overlay.filter.power;
        }
    }
}

fn topology_draw_system(
    overlay: Res<TopologyOverlay>,
    logistics_q: Query<&LogisticsCableSegment>,
    power_q: Query<&PowerCableSegment>,
    machine_q: Query<(&Transform, &MachineState), With<Machine>>,
    mut gizmos: Gizmos,
) {
    if !overlay.enabled {
        return;
    }

    if overlay.filter.logistics {
        for seg in &logistics_q {
            gizmos.line(seg.from, seg.to, palette::OK);
        }
    }

    if overlay.filter.power {
        for seg in &power_q {
            gizmos.line(seg.from, seg.to, palette::WARN);
        }
    }

    for (transform, state) in &machine_q {
        let color = match state {
            MachineState::Running => palette::OK,
            MachineState::Idle => palette::WARN,
        };
        gizmos.sphere(
            Isometry3d::from_translation(transform.translation),
            0.4,
            color,
        );
    }
}

fn filter_hud_sync(
    overlay: Res<TopologyOverlay>,
    mut hud_q: Query<&mut Visibility, With<TopoHudRoot>>,
) {
    if !overlay.is_changed() {
        return;
    }
    for mut v in &mut hud_q {
        *v = if overlay.enabled {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

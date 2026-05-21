use std::f32::consts::FRAC_PI_2;

use bevy::ecs::message::{Message, MessageReader};
use bevy::prelude::*;

use crate::{
    GameState,
    logistics::LogisticsCableSegment,
    machine::{Machine, MachineActivity, MachineState},
    power::PowerCableSegment,
    ui::theme::palette,
    world::Player,
};

/// Distance beyond which topology elements are not drawn (world units).
const TOPO_DRAW_RADIUS: f32 = 50.0;
/// Dark amber: signals power brownout (speed_factor < 1). Distinct from palette::WARN.
const COLOR_POWER_PAUSED: Color = Color::srgb(0.65, 0.35, 0.05);

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
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    logistics_q: Query<&LogisticsCableSegment>,
    power_q: Query<&PowerCableSegment>,
    machine_q: Query<(Entity, &Transform, &MachineState, &Machine)>,
    activity_q: Query<&MachineActivity>,
    mut gizmos: Gizmos,
) {
    if !overlay.enabled {
        return;
    }

    let player_pos = player_q
        .single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    if overlay.filter.logistics {
        for seg in &logistics_q {
            if seg.from.distance(player_pos) > TOPO_DRAW_RADIUS
                && seg.to.distance(player_pos) > TOPO_DRAW_RADIUS
            {
                continue;
            }
            gizmos.line(seg.from, seg.to, palette::OK);
        }
    }

    if overlay.filter.power {
        for seg in &power_q {
            if seg.from.distance(player_pos) > TOPO_DRAW_RADIUS
                && seg.to.distance(player_pos) > TOPO_DRAW_RADIUS
            {
                continue;
            }
            gizmos.line(seg.from, seg.to, palette::WARN);
        }
    }

    // Pulse: alpha 0.3–0.9 at 1 Hz, used for brownout ring
    let pulse_alpha = 0.3 + 0.6 * (0.5 + 0.5 * (std::f32::consts::TAU * time.elapsed_secs()).sin());

    for (entity, transform, state, machine) in &machine_q {
        let pos = transform.translation;
        if pos.distance(player_pos) > TOPO_DRAW_RADIUS {
            continue;
        }

        let speed_factor = activity_q
            .get(entity)
            .map(|a| a.speed_factor)
            .unwrap_or(1.0);
        let brownout = matches!(state, MachineState::Running) && speed_factor < 1.0;

        let color = if brownout {
            COLOR_POWER_PAUSED
        } else {
            match state {
                MachineState::Running => palette::OK,
                MachineState::Idle => palette::WARN,
            }
        };

        gizmos.sphere(Isometry3d::from_translation(pos), 0.4, color);

        // Pulsing ring for power-brownout machines (§8.3)
        if brownout {
            gizmos.circle(
                Isometry3d::new(pos, Quat::from_rotation_x(-FRAC_PI_2)),
                0.6,
                COLOR_POWER_PAUSED.with_alpha(pulse_alpha),
            );
        }

        // Short lines from machine center toward each IO port (§8.4)
        if overlay.filter.logistics {
            for &port in &machine.logistics_ports {
                gizmos.line(pos, port, palette::OK.with_alpha(0.4));
            }
        }
        if overlay.filter.power {
            for &port in &machine.energy_ports {
                gizmos.line(pos, port, palette::WARN.with_alpha(0.4));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_defaults_disabled() {
        let overlay = TopologyOverlay::default();
        assert!(!overlay.enabled);
        assert!(!overlay.filter.logistics);
        assert!(!overlay.filter.power);
    }

    #[test]
    fn first_toggle_enables_logistics_filter() {
        let mut overlay = TopologyOverlay::default();
        // Simulate handle_toggle logic: enable + default to logistics when no filter set
        overlay.enabled = !overlay.enabled;
        if overlay.enabled && !overlay.filter.logistics && !overlay.filter.power {
            overlay.filter.logistics = true;
        }
        assert!(overlay.enabled);
        assert!(overlay.filter.logistics);
        assert!(!overlay.filter.power);
    }

    #[test]
    fn second_toggle_disables_without_changing_filter() {
        let mut overlay = TopologyOverlay {
            enabled: true,
            filter: NetworkFilter {
                logistics: true,
                power: false,
            },
        };
        overlay.enabled = !overlay.enabled;
        if overlay.enabled && !overlay.filter.logistics && !overlay.filter.power {
            overlay.filter.logistics = true;
        }
        assert!(!overlay.enabled);
        // Filter preserved for next enable
        assert!(overlay.filter.logistics);
    }

    #[test]
    fn pulse_alpha_range() {
        // alpha = 0.3 + 0.6 * (0.5 + 0.5 * sin(x))
        // min when sin = -1: 0.3 + 0.6 * 0.0 = 0.3
        // max when sin =  1: 0.3 + 0.6 * 1.0 = 0.9
        let min_alpha = 0.3_f32 + 0.6 * (0.5 + 0.5 * (-1.0_f32));
        let max_alpha = 0.3_f32 + 0.6 * (0.5 + 0.5 * 1.0_f32);
        assert!((min_alpha - 0.3).abs() < 1e-6);
        assert!((max_alpha - 0.9).abs() < 1e-6);
    }

    #[test]
    fn distance_culling_constant_positive() {
        assert!(TOPO_DRAW_RADIUS > 0.0);
    }
}

use bevy::prelude::*;

use crate::{
    GameState,
    machine::Machine,
    power::{SlotBlockReason, SlotBlocked},
    ui::{
        MachineStatusPanel,
        theme::{COLOR_OVERLAY_BG, palette},
    },
};

/// Root of the top-left alerts panel. Hidden when nothing is blocked.
#[derive(Component)]
struct AlertsRoot;

/// A clickable alert row; jumps to `.0`'s machine panel on click.
#[derive(Component)]
struct AlertRow(Entity);

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(
            Update,
            (rebuild, handle_click).run_if(in_state(GameState::Playing)),
        );
}

/// One row's display text: machine name, tier, and block reason.
fn alert_label(machine: &Machine, reason: SlotBlockReason) -> String {
    format!(
        "⚠ {} LV{} — {}",
        machine.machine_type.to_uppercase().replace('_', " "),
        machine.tier,
        reason.label(),
    )
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(COLOR_OVERLAY_BG),
        Visibility::Hidden,
        DespawnOnExit(GameState::Playing),
        AlertsRoot,
    ));
}

fn rebuild(
    blocked_q: Query<(Entity, &Machine, &SlotBlocked)>,
    root_q: Query<Entity, With<AlertsRoot>>,
    mut visibility_q: Query<&mut Visibility, With<AlertsRoot>>,
    mut last: Local<Vec<Entity>>,
    mut commands: Commands,
) {
    let mut current: Vec<Entity> = blocked_q.iter().map(|(e, _, _)| e).collect();
    current.sort();
    if *last == current {
        return;
    }
    *last = current.clone();

    let Ok(root) = root_q.single() else {
        return;
    };
    commands.entity(root).despawn_children();

    if let Ok(mut vis) = visibility_q.single_mut() {
        *vis = if current.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }

    let mut rows: Vec<(Entity, String)> = blocked_q
        .iter()
        .map(|(e, m, b)| (e, alert_label(m, b.0)))
        .collect();
    rows.sort_by(|a, b| a.1.cmp(&b.1));

    commands.entity(root).with_children(|parent| {
        for (machine, label) in rows {
            parent
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    AlertRow(machine),
                ))
                .with_child((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(palette::ERR),
                ));
        }
    });
}

fn handle_click(
    q: Query<(&Interaction, &AlertRow), Changed<Interaction>>,
    mut panel: ResMut<MachineStatusPanel>,
) {
    for (interaction, row) in &q {
        if *interaction == Interaction::Pressed {
            panel.entity = Some(row.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::{Mirror, Orientation, Rotation};

    fn machine(machine_type: &str, tier: u8) -> Machine {
        Machine {
            machine_type: machine_type.into(),
            tier,
            orientation: Orientation {
                rotation: Rotation::North,
                mirror: Mirror::Normal,
            },
            energy_ports: vec![],
            logistics_ports: vec![],
        }
    }

    #[test]
    fn alert_label_formats_name_tier_reason() {
        let m = machine("arc_smelter", 2);
        assert_eq!(
            alert_label(&m, SlotBlockReason::NoPower),
            "⚠ ARC SMELTER LV2 — no power"
        );
    }

    #[test]
    fn click_jumps_to_blocked_machine() {
        let mut app = App::new();
        app.init_resource::<MachineStatusPanel>();
        app.add_systems(Update, handle_click);

        let machine_e = app.world_mut().spawn(machine("smelter", 1)).id();
        app.world_mut()
            .spawn((Interaction::Pressed, AlertRow(machine_e)));

        app.update();

        assert_eq!(
            app.world().resource::<MachineStatusPanel>().entity,
            Some(machine_e)
        );
    }
}

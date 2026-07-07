use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::{
    GameState, PlayMode,
    inventory::InventoryOpen,
    planet::{
        PlanetProperties, PlanetPropertyRevealed, PropertyDecisionValidated, qualitative_label,
    },
    research::{DiscoveryEvent, TechNodeUnlocked},
    ui::theme::{font_size, palette, space},
};

const AUTO_DISMISS_SECS: f32 = 8.0;
const MAX_VISIBLE: usize = 4;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Message)]
pub struct FieldComputerMessage {
    pub text: String,
    pub category: MessageCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageCategory {
    System,
    Scan,
    Research,
    Alert,
}

impl MessageCategory {
    fn tag(self) -> &'static str {
        match self {
            MessageCategory::System => "SYS",
            MessageCategory::Scan => "SCAN",
            MessageCategory::Research => "RES",
            MessageCategory::Alert => "ALERT",
        }
    }

    fn color(self) -> Color {
        match self {
            MessageCategory::System => palette::DIM,
            MessageCategory::Scan => palette::OK,
            MessageCategory::Research => palette::ACCENT,
            MessageCategory::Alert => palette::WARN,
        }
    }
}

#[derive(Resource, Default)]
pub struct FieldComputerLog {
    pub history: Vec<(MessageCategory, String)>,
}

// ---------------------------------------------------------------------------
// Private resources
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct FcState {
    active: Vec<FcActiveMsg>,
    history_open: bool,
    next_id: u32,
    dirty: bool,
}

impl FcState {
    fn push(&mut self, msg: FieldComputerMessage) {
        let id = self.next_id;
        self.next_id += 1;
        self.active.push(FcActiveMsg {
            id,
            text: msg.text,
            category: msg.category,
            timer: AUTO_DISMISS_SECS,
        });
        if self.active.len() > MAX_VISIBLE {
            self.active.remove(0);
        }
        self.dirty = true;
    }

    fn dismiss(&mut self, id: u32) {
        self.active.retain(|m| m.id != id);
        self.dirty = true;
    }
}

struct FcActiveMsg {
    id: u32,
    text: String,
    category: MessageCategory,
    timer: f32,
}

#[derive(Resource, Default)]
struct FcTriggers {
    property_reveal: bool,
    research: bool,
    drone_deploy: bool,
    discovery: bool,
    escape: bool,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct FcRoot;

#[derive(Component)]
struct FcMessageStack;

#[derive(Component)]
struct FcDismissBtn(u32);

#[derive(Component)]
struct FcHistoryBtn;

#[derive(Component)]
struct FcHistoryPanel;

#[derive(Component)]
struct FcHistoryList;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub fn plugin(app: &mut App) {
    app.add_message::<FieldComputerMessage>()
        .init_resource::<FieldComputerLog>()
        .init_resource::<FcState>()
        .init_resource::<FcTriggers>()
        .add_systems(OnEnter(GameState::Playing), (spawn, fire_arrival))
        .add_systems(
            Update,
            (
                receive_messages,
                trigger_messages,
                fire_insight_validation,
                handle_dismiss_click,
                handle_history_click,
                tick_dismiss,
                rebuild_stack,
                rebuild_history,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

fn spawn(mut commands: Commands) {
    // Bottom-right anchor: history button above message stack
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                bottom: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(4.0),
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            FcRoot,
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::SM)),
                    ..default()
                },
                BackgroundColor(palette::P2),
                Button,
                FcHistoryBtn,
            ))
            .with_child((
                Text::new("⌨ Log"),
                TextFont {
                    font_size: FontSize::Px(font_size::LABEL_SM),
                    ..default()
                },
                TextColor(palette::DIM),
                Pickable::IGNORE,
            ));

            p.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
                Pickable::IGNORE,
                FcMessageStack,
            ));
        });

    // History panel (right-side overlay, hidden by default)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(space::LG)),
                row_gap: Val::Px(space::SM),
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(palette::PANEL_SCRIM),
            Visibility::Hidden,
            DespawnOnExit(GameState::Playing),
            FcHistoryPanel,
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(space::MD)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new("Field Computer Log"),
                    TextFont {
                        font_size: FontSize::Px(font_size::H_SM),
                        ..default()
                    },
                    TextColor(palette::TEXT),
                    Pickable::IGNORE,
                ));
                row.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::XS)),
                        ..default()
                    },
                    BackgroundColor(palette::P2),
                    Button,
                    FcHistoryBtn,
                ))
                .with_child((
                    Text::new("✕"),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL),
                        ..default()
                    },
                    TextColor(palette::DIM),
                    Pickable::IGNORE,
                ));
            });

            p.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(space::SM),
                    overflow: Overflow::clip_y(),
                    flex_grow: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
                FcHistoryList,
            ));
        });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn receive_messages(
    mut reader: MessageReader<FieldComputerMessage>,
    mut state: ResMut<FcState>,
    mut log: ResMut<FieldComputerLog>,
) {
    for msg in reader.read() {
        log.history.push((msg.category, msg.text.clone()));
        state.push(msg.clone());
    }
}

/// Field-computer confirmation when the player's power choice matches a planet
/// hint (planet-identity design §8). Text: "Well-suited — {label} {property}
/// supports {machine} here."
fn fire_insight_validation(
    mut writer: MessageWriter<FieldComputerMessage>,
    mut validated: MessageReader<PropertyDecisionValidated>,
    planet_q: Query<&PlanetProperties>,
) {
    let Ok(planet) = planet_q.single() else {
        return;
    };
    for ev in validated.read() {
        let label = qualitative_label(ev.property, planet.axis(ev.property));
        writer.write(FieldComputerMessage {
            text: format!(
                "Well-suited — {label} {} supports {} here.",
                ev.property.display_name(),
                ev.kind.display(),
            ),
            category: MessageCategory::System,
        });
    }
}

fn fire_arrival(mut writer: MessageWriter<FieldComputerMessage>) {
    writer.write(FieldComputerMessage {
        text: "Systems online. Begin survey of planetary conditions.".into(),
        category: MessageCategory::System,
    });
}

fn trigger_messages(
    mut writer: MessageWriter<FieldComputerMessage>,
    mut triggers: ResMut<FcTriggers>,
    mut property_revealed: MessageReader<PlanetPropertyRevealed>,
    mut node_unlocked: MessageReader<TechNodeUnlocked>,
    mut discovery: MessageReader<DiscoveryEvent>,
    mode: Res<State<PlayMode>>,
) {
    let has_property = property_revealed.read().count() > 0;
    let has_research = node_unlocked.read().filter(|n| n.via_research).count() > 0;
    let discoveries: Vec<String> = discovery.read().map(|e| e.0.clone()).collect();

    if !triggers.property_reveal && has_property {
        triggers.property_reveal = true;
        writer.write(FieldComputerMessage {
            text: "New sensor data acquired. Cross-reference with power options.".into(),
            category: MessageCategory::Scan,
        });
    }

    if !triggers.research && has_research {
        triggers.research = true;
        writer.write(FieldComputerMessage {
            text: "Research node unlocked. Production paths updated.".into(),
            category: MessageCategory::Research,
        });
    }

    if !triggers.drone_deploy && *mode.get() == PlayMode::DronePilot {
        triggers.drone_deploy = true;
        writer.write(FieldComputerMessage {
            text: "Remote mode active. Maintain contact range.".into(),
            category: MessageCategory::System,
        });
    }

    for key in &discoveries {
        if key == "gateway_ruins" && !triggers.escape {
            triggers.escape = true;
            triggers.discovery = true;
            writer.write(FieldComputerMessage {
                text: "Gateway ruins located. Escape vector identified.".into(),
                category: MessageCategory::Alert,
            });
        } else if !triggers.discovery {
            triggers.discovery = true;
            writer.write(FieldComputerMessage {
                text: "Unknown structure detected. Cataloguing for analysis.".into(),
                category: MessageCategory::Scan,
            });
        }
    }
}

fn handle_dismiss_click(
    dismiss_q: Query<(&Interaction, &FcDismissBtn), Changed<Interaction>>,
    mut state: ResMut<FcState>,
) {
    for (interaction, btn) in &dismiss_q {
        if *interaction == Interaction::Pressed {
            state.dismiss(btn.0);
        }
    }
}

fn handle_history_click(
    btn_q: Query<&Interaction, (Changed<Interaction>, With<FcHistoryBtn>)>,
    mut state: ResMut<FcState>,
) {
    for interaction in &btn_q {
        if *interaction == Interaction::Pressed {
            state.history_open = !state.history_open;
            state.dirty = true;
        }
    }
}

fn tick_dismiss(time: Res<Time>, mut state: ResMut<FcState>) {
    let dt = time.delta_secs();
    let before = state.active.len();
    state.active.retain_mut(|m| {
        m.timer -= dt;
        m.timer > 0.0
    });
    if state.active.len() != before {
        state.dirty = true;
    }
}

fn rebuild_stack(
    mut commands: Commands,
    mut state: ResMut<FcState>,
    stack_q: Query<Entity, With<FcMessageStack>>,
    inv_open: Option<Res<InventoryOpen>>,
    mut root_q: Query<&mut Visibility, With<FcRoot>>,
) {
    let hidden = inv_open.is_some_and(|o| o.0);
    for mut v in &mut root_q {
        *v = if hidden {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }

    if !state.dirty {
        return;
    }
    state.dirty = false;

    let Ok(stack) = stack_q.single() else {
        return;
    };

    commands.entity(stack).despawn_children();

    for msg in &state.active {
        let card_id = msg.id;
        let cat = msg.category;
        let text = msg.text.clone();

        commands.entity(stack).with_children(|p| {
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(space::LG), Val::Px(space::SM)),
                    column_gap: Val::Px(space::SM),
                    max_width: Val::Px(300.0),
                    ..default()
                },
                BackgroundColor(palette::PANEL_SCRIM),
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(cat.tag()),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(cat.color()),
                    Pickable::IGNORE,
                ));
                row.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL),
                        ..default()
                    },
                    TextColor(palette::TEXT),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                row.spawn((
                    Node {
                        padding: UiRect::all(Val::Px(space::XS)),
                        ..default()
                    },
                    Button,
                    FcDismissBtn(card_id),
                ))
                .with_child((
                    Text::new("×"),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(palette::DIM),
                    Pickable::IGNORE,
                ));
            });
        });
    }
}

fn rebuild_history(
    mut commands: Commands,
    state: Res<FcState>,
    log: Res<FieldComputerLog>,
    list_q: Query<Entity, With<FcHistoryList>>,
    mut vis_q: Query<&mut Visibility, With<FcHistoryPanel>>,
) {
    for mut v in &mut vis_q {
        *v = if state.history_open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if !state.dirty {
        return;
    }
    if !state.history_open {
        return;
    }

    let Ok(list) = list_q.single() else {
        return;
    };

    commands.entity(list).despawn_children();

    if log.history.is_empty() {
        commands.entity(list).with_child((
            Text::new("No messages yet."),
            TextFont {
                font_size: FontSize::Px(font_size::LABEL),
                ..default()
            },
            TextColor(palette::DIM),
        ));
        return;
    }

    for (cat, text) in log.history.iter().rev() {
        let cat = *cat;
        let text = text.clone();
        commands.entity(list).with_children(|p| {
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(space::SM),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(cat.tag()),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL_SM),
                        ..default()
                    },
                    TextColor(cat.color()),
                    Pickable::IGNORE,
                ));
                row.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: FontSize::Px(font_size::LABEL),
                        ..default()
                    },
                    TextColor(palette::TEXT),
                    Pickable::IGNORE,
                ));
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sys_msg(text: &str) -> FieldComputerMessage {
        FieldComputerMessage {
            text: text.into(),
            category: MessageCategory::System,
        }
    }

    #[test]
    fn push_adds_message() {
        let mut state = FcState::default();
        state.push(sys_msg("hello"));
        assert_eq!(state.active.len(), 1);
        assert_eq!(state.active[0].text, "hello");
        assert!(state.dirty);
    }

    #[test]
    fn push_caps_at_max_visible() {
        let mut state = FcState::default();
        for i in 0..=MAX_VISIBLE {
            state.push(sys_msg(&format!("msg {i}")));
        }
        assert_eq!(state.active.len(), MAX_VISIBLE);
        assert_eq!(
            state.active.last().unwrap().text,
            format!("msg {MAX_VISIBLE}")
        );
    }

    #[test]
    fn dismiss_removes_by_id() {
        let mut state = FcState::default();
        state.push(sys_msg("a"));
        state.push(sys_msg("b"));
        let id_a = state.active[0].id;
        state.dismiss(id_a);
        assert_eq!(state.active.len(), 1);
        assert_eq!(state.active[0].text, "b");
    }

    #[test]
    fn dismiss_noop_on_unknown_id() {
        let mut state = FcState::default();
        state.push(sys_msg("a"));
        state.dismiss(999);
        assert_eq!(state.active.len(), 1);
    }

    #[test]
    fn fire_insight_validation_emits_message() {
        use crate::machine::PowerProducerKind;
        use crate::planet::PlanetPropertyKey;

        let mut app = App::new();
        app.add_message::<FieldComputerMessage>()
            .add_message::<PropertyDecisionValidated>()
            .add_systems(Update, fire_insight_validation);
        app.world_mut().spawn(PlanetProperties {
            stellar_distance: 0.1,
            ..Default::default()
        });
        app.world_mut().write_message(PropertyDecisionValidated {
            property: PlanetPropertyKey::StellarDistance,
            kind: PowerProducerKind::Solar,
            modifier: 1.48,
        });
        app.update();

        let msgs = app
            .world()
            .resource::<bevy::ecs::message::Messages<FieldComputerMessage>>();
        let mut cursor = msgs.get_cursor();
        let got: Vec<_> = cursor.read(msgs).cloned().collect();
        assert_eq!(got.len(), 1);
        assert!(got[0].text.contains("Solar Irradiance"));
        assert!(got[0].text.contains("Solar Generator"));
    }

    #[test]
    fn history_log_accumulates() {
        let mut log = FieldComputerLog::default();
        log.history.push((MessageCategory::System, "first".into()));
        log.history.push((MessageCategory::Scan, "second".into()));
        assert_eq!(log.history.len(), 2);
        assert_eq!(log.history[0].1, "first");
    }
}

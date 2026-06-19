use bevy::{ecs::message::MessageReader, prelude::*};

use crate::{
    GameState,
    inventory::InventoryOpen,
    planet::{
        Planet, PlanetProperties, PlanetPropertyKey, PlanetPropertyRevealed,
        PlanetPropertyVisibility, PropertyVisibility, qualitative_label,
    },
    ui::theme::{font_size, palette, space},
};

#[derive(Resource)]
struct PlanetHudState {
    expanded: bool,
    dirty: bool,
}

impl Default for PlanetHudState {
    fn default() -> Self {
        Self {
            expanded: true,
            dirty: true,
        }
    }
}

#[derive(Component)]
struct PlanetHudRoot;

#[derive(Component)]
struct PlanetHudBody;

#[derive(Component)]
struct PlanetHudToggle;

#[derive(Component)]
struct PlanetHudNameText;

pub fn plugin(app: &mut App) {
    app.init_resource::<PlanetHudState>()
        .add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(
            Update,
            (mark_dirty, handle_toggle, rebuild)
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(240.0),
                ..default()
            },
            BackgroundColor(palette::PANEL_SCRIM),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            PlanetHudRoot,
        ))
        .with_children(|p| {
            // Header — click to toggle expand/collapse
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::SM)),
                    column_gap: Val::Px(space::MD),
                    ..default()
                },
                Button,
                PlanetHudToggle,
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new("◉ PLANET"),
                    TextFont {
                        font_size: font_size::LABEL_SM,
                        ..default()
                    },
                    TextColor(palette::DIM),
                    Pickable::IGNORE,
                ));
                row.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: font_size::H_SM,
                        ..default()
                    },
                    TextColor(palette::ACCENT),
                    Pickable::IGNORE,
                    PlanetHudNameText,
                ));
            });

            // Body — property rows, rebuilt on reveal
            p.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(1.0),
                    padding: UiRect {
                        left: Val::Px(space::MD),
                        right: Val::Px(space::MD),
                        top: Val::Px(0.0),
                        bottom: Val::Px(space::SM),
                    },
                    ..default()
                },
                Pickable::IGNORE,
                PlanetHudBody,
            ));
        });
}

fn mark_dirty(
    mut revealed: MessageReader<PlanetPropertyRevealed>,
    mut state: ResMut<PlanetHudState>,
) {
    if revealed.read().count() > 0 {
        state.dirty = true;
    }
}

fn handle_toggle(
    toggle_q: Query<&Interaction, (Changed<Interaction>, With<PlanetHudToggle>)>,
    mut state: ResMut<PlanetHudState>,
) {
    for interaction in &toggle_q {
        if *interaction == Interaction::Pressed {
            state.expanded = !state.expanded;
            state.dirty = true;
        }
    }
}

fn rebuild(
    mut commands: Commands,
    mut state: ResMut<PlanetHudState>,
    inv_open: Option<Res<InventoryOpen>>,
    mut root_q: Query<&mut Visibility, With<PlanetHudRoot>>,
    body_q: Query<Entity, With<PlanetHudBody>>,
    mut name_q: Query<&mut Text, With<PlanetHudNameText>>,
    planet_q: Query<(&PlanetProperties, &PlanetPropertyVisibility), With<Planet>>,
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

    let Ok((props, vis)) = planet_q.single() else {
        return;
    };
    let Ok(body) = body_q.single() else { return };

    if let Ok(mut name_text) = name_q.single_mut() {
        **name_text = if props.name.epithet.is_empty() {
            props.name.catalog.clone()
        } else {
            format!("{}  \"{}\"", props.name.catalog, props.name.epithet)
        };
    }

    commands.entity(body).despawn_children();

    if !state.expanded {
        return;
    }

    let props = props.clone();
    let vis = vis.clone();

    commands.entity(body).with_children(|p| {
        for key in PlanetPropertyKey::ALL {
            spawn_row(p, key, &props, &vis);
        }
    });
}

fn spawn_row(
    parent: &mut ChildSpawnerCommands<'_>,
    key: PlanetPropertyKey,
    props: &PlanetProperties,
    vis: &PlanetPropertyVisibility,
) {
    let visibility = vis.get(key);

    let (value_text, value_color) = match visibility {
        PropertyVisibility::Hidden => (key.hidden_hint().to_string(), palette::DIM),
        PropertyVisibility::Qualitative | PropertyVisibility::Revealed => {
            let q = if key == PlanetPropertyKey::HazardType {
                props.hazard_type.display().to_string()
            } else {
                qualitative_label(key, property_value(key, props)).to_string()
            };
            (q, palette::TEXT)
        }
    };

    let name_color = if matches!(visibility, PropertyVisibility::Hidden) {
        palette::DIM
    } else {
        palette::TEXT
    };

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|col| {
            col.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(key.display_name()),
                    TextFont {
                        font_size: font_size::LABEL_SM,
                        ..default()
                    },
                    TextColor(name_color),
                    Pickable::IGNORE,
                ));
                row.spawn((
                    Text::new(value_text),
                    TextFont {
                        font_size: font_size::LABEL_SM,
                        ..default()
                    },
                    TextColor(value_color),
                    Pickable::IGNORE,
                ));
            });

            if !matches!(visibility, PropertyVisibility::Hidden) {
                col.spawn((
                    Text::new(format!("  \u{2192} {}", key.power_hint())),
                    TextFont {
                        font_size: font_size::LABEL_SM,
                        ..default()
                    },
                    TextColor(palette::DIM),
                    Pickable::IGNORE,
                ));
            }
        });
}

fn property_value(key: PlanetPropertyKey, props: &PlanetProperties) -> f32 {
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

//! Landing panel UI. Shown on `PlayMode::Landing`; dismissed by player.

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

use super::{
    LandingPanelDismissed, PlanetProperties, PlanetPropertyKey, PlanetPropertyViewLog,
    PlanetPropertyViewed, PlanetPropertyVisibility, PropertyVisibility, ViewContext,
    qualitative_label,
};
use crate::PlayMode;
use crate::ui::theme::{
    COLOR_DIM, COLOR_GOLD,
    font_size::{H_LG, H_SM, LABEL},
    palette, space,
};
use crate::world::Player;

pub struct LandingPanelPlugin;

impl Plugin for LandingPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(PlayMode::Landing), despawn_landing_panel)
            .add_systems(
                Update,
                (
                    spawn_landing_panel,
                    landing_panel_button,
                    landing_panel_view_tracker,
                )
                    .run_if(in_state(PlayMode::Landing)),
            );
    }
}

#[derive(Component)]
pub struct PlanetLandingPanel;

#[derive(Component)]
struct BeginButton;

#[derive(Component)]
struct PropertyRow(PlanetPropertyKey);

fn spawn_landing_panel(
    mut commands: Commands,
    planet_q: Query<(&PlanetProperties, &PlanetPropertyVisibility)>,
    existing_q: Query<(), With<PlanetLandingPanel>>,
) {
    if !existing_q.is_empty() {
        return;
    }
    let Ok((props, vis)) = planet_q.single() else {
        return;
    };

    let title = if props.name.epithet.is_empty() {
        props.name.catalog.clone()
    } else {
        format!("{}  \"{}\"", props.name.catalog, props.name.epithet)
    };

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
            BackgroundColor(palette::OVERLAY_SCRIM),
            GlobalZIndex(50),
            PlanetLandingPanel,
        ))
        .with_children(|outer| {
            outer
                .spawn((
                    Node {
                        width: Val::Px(640.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(space::XL)),
                        border: UiRect::all(Val::Px(1.0)),
                        row_gap: Val::Px(space::MD),
                        ..default()
                    },
                    BackgroundColor(palette::P1),
                    BorderColor::all(palette::BORDER_STRONG),
                ))
                .with_children(|root| {
                    root.spawn((
                        Text::new(title),
                        TextFont {
                            font_size: H_LG,
                            ..default()
                        },
                        TextColor(COLOR_GOLD),
                    ));
                    let temp_color = lerp_temp_color(props.temperature);
                    root.spawn((
                        Node {
                            height: Val::Px(6.0),
                            margin: UiRect::bottom(Val::Px(space::MD)),
                            ..default()
                        },
                        BackgroundColor(temp_color),
                    ));

                    for key in PlanetPropertyKey::ALL {
                        spawn_property_row(root, key, props, vis);
                    }

                    root.spawn((
                        Node {
                            margin: UiRect::top(Val::Px(space::LG)),
                            align_self: AlignSelf::FlexEnd,
                            padding: UiRect::axes(Val::Px(space::LG), Val::Px(space::SM)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        Button,
                        BackgroundColor(palette::P3),
                        BorderColor::all(COLOR_GOLD),
                        BeginButton,
                    ))
                    .with_child((
                        Text::new("Begin Run \u{2192}"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(COLOR_GOLD),
                        Pickable::IGNORE,
                    ));
                });
        });
}

fn spawn_property_row(
    parent: &mut ChildSpawnerCommands<'_>,
    key: PlanetPropertyKey,
    props: &PlanetProperties,
    vis: &PlanetPropertyVisibility,
) {
    let visibility = vis.get(key);
    let value = property_value(key, props);

    let (label, color, hint) = match visibility {
        PropertyVisibility::Hidden => (
            key.hidden_hint().to_string(),
            COLOR_DIM,
            hidden_effect_hint(key),
        ),
        PropertyVisibility::Qualitative | PropertyVisibility::Revealed => {
            let q = if key == PlanetPropertyKey::HazardType {
                props.hazard_type.display().to_string()
            } else {
                qualitative_label(key, value).to_string()
            };
            (q, palette::TEXT, effect_hint(key))
        }
    };

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(space::SM), Val::Px(space::XS)),
                row_gap: Val::Px(space::XS),
                ..default()
            },
            Button,
            BackgroundColor(Color::NONE),
            Interaction::None,
            PropertyRow(key),
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|r| {
                let bullet = if matches!(visibility, PropertyVisibility::Hidden) {
                    "\u{25CB} "
                } else {
                    "\u{25CF} "
                };
                r.spawn((
                    Text::new(format!("{}{}", bullet, key.display_name())),
                    TextFont {
                        font_size: H_SM,
                        ..default()
                    },
                    TextColor(palette::TEXT),
                    Pickable::IGNORE,
                ));
                r.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: H_SM,
                        ..default()
                    },
                    TextColor(color),
                    Pickable::IGNORE,
                ));
            });
            row.spawn((
                Text::new(hint),
                TextFont {
                    font_size: LABEL,
                    ..default()
                },
                TextColor(COLOR_DIM),
                Pickable::IGNORE,
            ));
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

fn effect_hint(key: PlanetPropertyKey) -> String {
    match key {
        PlanetPropertyKey::StellarDistance => "Drives solar generator output here.".into(),
        PlanetPropertyKey::Temperature => "Affects cooling cost and ice deposits.".into(),
        PlanetPropertyKey::WindIntensity => "Wind generation viability.".into(),
        PlanetPropertyKey::HazardType => "Operations outside aegis field are disrupted.".into(),
        PlanetPropertyKey::GeologicalActivity => "Geothermal sites and deep metals.".into(),
        PlanetPropertyKey::AtmosphericOxygen => "Combustion and material chemistry.".into(),
        PlanetPropertyKey::AtmosphericPressure => "Fluid pocket and pump conditions.".into(),
    }
}

fn hidden_effect_hint(key: PlanetPropertyKey) -> String {
    match key {
        PlanetPropertyKey::GeologicalActivity => "Deep metals and geothermal sites unknown.".into(),
        PlanetPropertyKey::AtmosphericOxygen => "Combustion and material chemistry unknown.".into(),
        PlanetPropertyKey::AtmosphericPressure => "Fluid pocket conditions unknown.".into(),
        _ => String::new(),
    }
}

fn lerp_temp_color(t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::srgb(0.20 + 0.55 * t, 0.30 + 0.20 * (1.0 - t), 0.65 - 0.45 * t)
}

fn despawn_landing_panel(mut commands: Commands, panel_q: Query<Entity, With<PlanetLandingPanel>>) {
    for e in &panel_q {
        commands.entity(e).despawn();
    }
}

fn landing_panel_button(
    mut interactions: Query<&Interaction, (Changed<Interaction>, With<BeginButton>)>,
    mut next_mode: ResMut<NextState<PlayMode>>,
    mut dismissed: MessageWriter<LandingPanelDismissed>,
) {
    for interaction in &mut interactions {
        if *interaction == Interaction::Pressed {
            next_mode.set(PlayMode::Exploring);
            dismissed.write(LandingPanelDismissed);
        }
    }
}

fn landing_panel_view_tracker(
    rows: Query<(&Interaction, &PropertyRow), Changed<Interaction>>,
    mut viewed: MessageWriter<PlanetPropertyViewed>,
    mut player_q: Query<&mut PlanetPropertyViewLog, With<Player>>,
) {
    for (interaction, row) in &rows {
        if matches!(interaction, Interaction::Hovered | Interaction::Pressed) {
            viewed.write(PlanetPropertyViewed {
                property: row.0,
                context: ViewContext::LandingPanel,
            });
            if let Ok(mut log) = player_q.single_mut() {
                log.record(row.0);
            }
        }
    }
}

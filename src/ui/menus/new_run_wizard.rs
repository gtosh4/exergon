use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use rand::Rng;

use crate::{
    GameState,
    planet::{
        PlanetArchetypes, PlanetProperties, PlanetPropertyKey, PlanetPropertyVisibility,
        PropertyVisibility, generate_properties_from, qualitative_label,
    },
    save::NewRunEvent,
    seed::{CuratedSeeds, DomainSeeds, hash_text},
    ui::{
        input::{FocusedInput, TextInput},
        theme::{font_size, palette, space},
        widgets::{H, UiButton, button_label, caption, divider, heading, hstack},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum WizardStep {
    #[default]
    Difficulty,
    Modifiers,
    Planet,
}

#[derive(Resource, Default)]
pub struct WizardDraft {
    pub step: WizardStep,
    pub seed_text: String,
    pub planet_preview: Option<PlanetProperties>,
    pub test_mode: bool,
}

#[derive(Component)]
struct WizardRoot;

#[derive(Component)]
struct WizardSeedInput;

#[derive(Component)]
enum WizardNav {
    Back,
    Next,
    Land,
    Roll,
    ToggleTestMode,
    SelectCuratedSeed(String),
}

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::NewRunWizard), init_draft)
        .add_systems(OnExit(GameState::NewRunWizard), cleanup_draft)
        .add_systems(
            Update,
            (handle_nav, rebuild_ui)
                .chain()
                .run_if(in_state(GameState::NewRunWizard)),
        );
}

fn init_draft(mut commands: Commands) {
    commands.insert_resource(WizardDraft::default());
}

fn cleanup_draft(mut commands: Commands) {
    commands.remove_resource::<WizardDraft>();
}

fn rebuild_ui(
    mut commands: Commands,
    draft: Res<WizardDraft>,
    roots: Query<Entity, With<WizardRoot>>,
    curated_seeds: Res<CuratedSeeds>,
) {
    if !draft.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(palette::BG),
            WizardRoot,
            DespawnOnExit(GameState::NewRunWizard),
        ))
        .with_children(|root| {
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(space::XL),
                width: Val::Px(560.0),
                ..default()
            })
            .with_children(|col| {
                let step_label = match draft.step {
                    WizardStep::Difficulty => "STEP 1 OF 3 — DIFFICULTY",
                    WizardStep::Modifiers => "STEP 2 OF 3 — SEED & MODIFIERS",
                    WizardStep::Planet => "STEP 3 OF 3 — PLANET",
                };
                col.spawn(caption(step_label));
                col.spawn(heading("NEW RUN", H::Xl));
                col.spawn(divider());

                match draft.step {
                    WizardStep::Difficulty => spawn_difficulty(col),
                    WizardStep::Modifiers => spawn_modifiers(col, &draft, &curated_seeds),
                    WizardStep::Planet => spawn_planet(col, &draft),
                }

                col.spawn(divider());

                col.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|nav| {
                    let back_label = if draft.step == WizardStep::Difficulty {
                        "CANCEL"
                    } else {
                        "BACK"
                    };
                    nav.spawn((UiButton::default(), WizardNav::Back))
                        .with_children(|b| {
                            b.spawn(button_label(back_label));
                        });
                    if draft.step == WizardStep::Planet {
                        nav.spawn((UiButton::accent(), WizardNav::Land))
                            .with_children(|b| {
                                b.spawn(button_label("LAND"));
                            });
                    } else {
                        nav.spawn((UiButton::accent(), WizardNav::Next))
                            .with_children(|b| {
                                b.spawn(button_label("NEXT"));
                            });
                    }
                });
            });
        });
}

fn spawn_difficulty(col: &mut ChildSpawnerCommands<'_>) {
    col.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(space::MD),
        ..default()
    })
    .with_children(|cards| {
        spawn_diff_card(
            cards,
            "INITIATION",
            "Shallow graph. Generous research. Gateway escape. 4–6 hours.",
            true,
            true,
        );
        for (name, desc) in [
            (
                "STANDARD",
                "Moderate depth. Meaningful modifiers. Derelict ship escape. 10–15 hours.",
            ),
            (
                "ADVANCED",
                "Deep graph. Strong modifiers. Relay escape. 20–30 hours.",
            ),
            (
                "PINNACLE",
                "Maximum variance. Build your own FTL. 30–50+ hours.",
            ),
        ] {
            spawn_diff_card(cards, name, desc, false, false);
        }
    });
}

fn spawn_diff_card(
    parent: &mut ChildSpawnerCommands<'_>,
    name: &str,
    desc: &str,
    unlocked: bool,
    selected: bool,
) {
    let bg = if selected {
        Color::srgba(0.541, 0.447, 0.667, 0.10)
    } else {
        palette::P1
    };
    let border_color = if selected {
        palette::ACCENT
    } else {
        palette::BORDER
    };
    let name_color = if selected {
        palette::ACCENT
    } else if unlocked {
        palette::TEXT
    } else {
        palette::DIM
    };

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(space::XL), Val::Px(space::LG)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                width: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border_color),
        ))
        .with_children(|card| {
            card.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(space::XS),
                ..default()
            })
            .with_children(|info| {
                info.spawn((
                    Text::new(name),
                    TextFont {
                        font_size: font_size::H_SM,
                        ..default()
                    },
                    TextColor(name_color),
                ));
                info.spawn((
                    Text::new(desc),
                    TextFont {
                        font_size: font_size::LABEL,
                        ..default()
                    },
                    TextColor(palette::DIM),
                ));
            });

            if selected {
                card.spawn((
                    Text::new("✓"),
                    TextFont {
                        font_size: font_size::H_MD,
                        ..default()
                    },
                    TextColor(palette::ACCENT),
                ));
            } else if !unlocked {
                card.spawn((
                    Text::new("LOCKED"),
                    TextFont {
                        font_size: font_size::TAG,
                        ..default()
                    },
                    TextColor(palette::DIM),
                ));
            }
        });
}

fn spawn_modifiers(
    col: &mut ChildSpawnerCommands<'_>,
    draft: &WizardDraft,
    curated: &CuratedSeeds,
) {
    if !curated.entries.is_empty() {
        col.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(space::SM),
            ..default()
        })
        .with_children(|section| {
            section.spawn(caption("CURATED SEEDS"));
            for entry in &curated.entries {
                let selected = draft.seed_text == entry.seed;
                section
                    .spawn((
                        Button,
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            border: UiRect::all(Val::Px(1.0)),
                            padding: UiRect::axes(Val::Px(space::LG), Val::Px(space::SM)),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            width: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(if selected {
                            Color::srgba(0.541, 0.447, 0.667, 0.10)
                        } else {
                            palette::P1
                        }),
                        BorderColor::all(if selected {
                            palette::ACCENT
                        } else {
                            palette::BORDER
                        }),
                        WizardNav::SelectCuratedSeed(entry.seed.clone()),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|info| {
                            info.spawn((
                                Text::new(entry.name.clone()),
                                TextFont {
                                    font_size: font_size::H_SM,
                                    ..default()
                                },
                                TextColor(if selected {
                                    palette::ACCENT
                                } else {
                                    palette::TEXT
                                }),
                            ));
                            info.spawn((
                                Text::new(entry.description.clone()),
                                TextFont {
                                    font_size: font_size::LABEL,
                                    ..default()
                                },
                                TextColor(palette::DIM),
                            ));
                        });
                        if selected {
                            row.spawn((
                                Text::new("✓"),
                                TextFont {
                                    font_size: font_size::H_MD,
                                    ..default()
                                },
                                TextColor(palette::ACCENT),
                            ));
                        }
                    });
            }
        });
    }

    // Seed strip
    col.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(space::SM),
        ..default()
    })
    .with_children(|section| {
        section.spawn(caption("SEED"));
        section.spawn(hstack(space::MD)).with_children(|row| {
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::SM + 1.0)),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                },
                BorderColor::all(palette::BORDER_STRONG),
                BackgroundColor(palette::P2),
                Text::new(draft.seed_text.clone()),
                TextFont {
                    font_size: font_size::LABEL,
                    ..default()
                },
                TextColor(palette::TEXT),
                TextInput {
                    value: draft.seed_text.clone(),
                    ..default()
                },
                WizardSeedInput,
            ));
            row.spawn((UiButton::default(), WizardNav::Roll))
                .with_children(|b| {
                    b.spawn(button_label("ROLL"));
                });
        });
    });

    col.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(space::SM),
        ..default()
    })
    .with_children(|section| {
        section.spawn(caption("MODIFIERS"));

        #[cfg(debug_assertions)]
        section
            .spawn((
                Button,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::axes(Val::Px(space::XL), Val::Px(space::LG)),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    width: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(if draft.test_mode {
                    Color::srgba(0.541, 0.447, 0.667, 0.10)
                } else {
                    palette::P1
                }),
                BorderColor::all(if draft.test_mode {
                    palette::ACCENT
                } else {
                    palette::BORDER
                }),
                WizardNav::ToggleTestMode,
            ))
            .with_children(|card| {
                card.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(space::XS),
                    ..default()
                })
                .with_children(|info| {
                    info.spawn((
                        Text::new("DEV: TEST LOADOUT"),
                        TextFont {
                            font_size: font_size::H_SM,
                            ..default()
                        },
                        TextColor(if draft.test_mode {
                            palette::ACCENT
                        } else {
                            palette::TEXT
                        }),
                    ));
                    info.spawn((
                        Text::new("Start with all machines, items, and research unlocked."),
                        TextFont {
                            font_size: font_size::LABEL,
                            ..default()
                        },
                        TextColor(palette::DIM),
                    ));
                });
                card.spawn((
                    Text::new("FREE"),
                    TextFont {
                        font_size: font_size::TAG,
                        ..default()
                    },
                    TextColor(palette::ACCENT),
                ));
            });

        #[cfg(not(debug_assertions))]
        section
            .spawn((
                Node {
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(space::XL)),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(palette::P1),
                BorderColor::all(palette::BORDER),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("No modifiers in this version"),
                    TextFont {
                        font_size: font_size::LABEL,
                        ..default()
                    },
                    TextColor(palette::DIM),
                ));
            });
    });
}

fn lerp_temp_color(t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::srgb(0.20 + 0.55 * t, 0.30 + 0.20 * (1.0 - t), 0.65 - 0.45 * t)
}

fn spawn_planet(col: &mut ChildSpawnerCommands<'_>, draft: &WizardDraft) {
    let Some(props) = &draft.planet_preview else {
        col.spawn((
            Text::new("Generating planet survey…"),
            TextFont {
                font_size: font_size::LABEL,
                ..default()
            },
            TextColor(palette::DIM),
        ));
        return;
    };

    let vis = PlanetPropertyVisibility::default();

    // Planet name
    let title = if props.name.epithet.is_empty() {
        props.name.catalog.clone()
    } else {
        format!("{}  \"{}\"", props.name.catalog, props.name.epithet)
    };
    col.spawn((
        Text::new(title),
        TextFont {
            font_size: font_size::H_LG,
            ..default()
        },
        TextColor(palette::ACCENT),
    ));

    // Temperature color bar
    col.spawn((
        Node {
            height: Val::Px(6.0),
            width: Val::Percent(100.0),
            border_radius: BorderRadius::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(lerp_temp_color(props.temperature)),
    ));

    // Property rows
    col.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        ..default()
    })
    .with_children(|rows| {
        for key in PlanetPropertyKey::ALL {
            spawn_property_row(rows, key, props, &vis);
        }
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

    let (label_text, label_color) = match visibility {
        PropertyVisibility::Hidden => (key.hidden_hint().to_string(), palette::DIM),
        PropertyVisibility::Qualitative | PropertyVisibility::Revealed => {
            let q = if key == PlanetPropertyKey::HazardType {
                props.hazard_type.display().to_string()
            } else {
                qualitative_label(key, value).to_string()
            };
            (q, palette::TEXT)
        }
    };

    let bullet = if matches!(visibility, PropertyVisibility::Hidden) {
        "○ "
    } else {
        "● "
    };

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(space::SM), Val::Px(space::XS)),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{}{}", bullet, key.display_name())),
                TextFont {
                    font_size: font_size::H_SM,
                    ..default()
                },
                TextColor(palette::TEXT),
            ));
            row.spawn((
                Text::new(label_text),
                TextFont {
                    font_size: font_size::H_SM,
                    ..default()
                },
                TextColor(label_color),
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

fn handle_nav(
    btn_q: Query<(&Interaction, &WizardNav), Changed<Interaction>>,
    seed_q: Query<&TextInput, With<WizardSeedInput>>,
    mut draft: ResMut<WizardDraft>,
    mut next_state: ResMut<NextState<GameState>>,
    mut new_run: MessageWriter<NewRunEvent>,
    mut focus: ResMut<FocusedInput>,
    archetypes: Res<PlanetArchetypes>,
) {
    for (interaction, nav) in &btn_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match nav {
            WizardNav::Back => {
                if draft.step == WizardStep::Modifiers
                    && let Ok(input) = seed_q.single()
                {
                    draft.seed_text = input.value.clone();
                }
                match draft.step {
                    WizardStep::Difficulty => next_state.set(GameState::MainMenu),
                    WizardStep::Modifiers => draft.step = WizardStep::Difficulty,
                    WizardStep::Planet => draft.step = WizardStep::Modifiers,
                }
            }
            WizardNav::Next => {
                if draft.step == WizardStep::Modifiers {
                    if let Ok(input) = seed_q.single() {
                        draft.seed_text = input.value.clone();
                    }
                    // Pre-generate planet preview so step 3 can show it.
                    if !archetypes.defs.is_empty() {
                        let hash = hash_text(&draft.seed_text);
                        let seeds = DomainSeeds::from_master(hash);
                        let mut rng = seeds.planet_rng();
                        draft.planet_preview = Some(generate_properties_from(
                            &mut rng,
                            &archetypes.defs,
                            seeds.planet,
                        ));
                    }
                }
                match draft.step {
                    WizardStep::Difficulty => draft.step = WizardStep::Modifiers,
                    WizardStep::Modifiers => draft.step = WizardStep::Planet,
                    WizardStep::Planet => {}
                }
            }
            WizardNav::Land => {
                new_run.write(NewRunEvent {
                    seed_text: draft.seed_text.clone(),
                    test_mode: draft.test_mode,
                });
                focus.0 = None;
            }
            WizardNav::Roll => {
                let seed: String = (0..8)
                    .map(|_| rand::thread_rng().sample(rand::distributions::Alphanumeric) as char)
                    .map(|c| c.to_ascii_uppercase())
                    .collect();
                draft.seed_text = seed;
            }
            WizardNav::ToggleTestMode => {
                draft.test_mode = !draft.test_mode;
            }
            WizardNav::SelectCuratedSeed(seed_text) => {
                draft.seed_text = seed_text.clone();
            }
        }
    }
}

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use crate::{
    GameState,
    save::{
        CheckpointKind, DifficultyTier, LoadRunEvent, RunSaveHeader, RunStatus, SaveRoot,
        list_run_ids, read_run_header,
    },
    ui::{
        theme::{border, font_size, palette, radius, space},
        widgets::{H, UiButton, button_label, caption, divider, heading, hstack, label, vstack},
    },
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct RunList {
    runs: Vec<RunSaveHeader>,
    selected_id: Option<String>,
    filter: RunFilter,
    confirm_delete: bool,
}

#[derive(Default, Clone, PartialEq)]
enum RunFilter {
    #[default]
    All,
    InProgress,
    Completed,
}

impl RunList {
    fn visible_runs(&self) -> Vec<&RunSaveHeader> {
        self.runs
            .iter()
            .filter(|h| match self.filter {
                RunFilter::All => true,
                RunFilter::InProgress => h.status == RunStatus::InProgress,
                RunFilter::Completed => h.status == RunStatus::Completed,
            })
            .collect()
    }

    fn selected(&self) -> Option<&RunSaveHeader> {
        let id = self.selected_id.as_deref()?;
        self.runs.iter().find(|h| h.run_id == id)
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct LoadRunRoot;

#[derive(Component)]
enum Btn {
    Back,
    FilterAll,
    FilterInProgress,
    FilterCompleted,
    Select(String),
    Load,
    Delete,
    ConfirmDelete,
    CancelDelete,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::LoadRun), init_run_list)
        .add_systems(OnExit(GameState::LoadRun), cleanup)
        .add_systems(
            Update,
            (handle_buttons, rebuild_ui)
                .chain()
                .run_if(in_state(GameState::LoadRun)),
        );
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn init_run_list(
    mut commands: Commands,
    save_root: Res<SaveRoot>,
    app_registry: Res<AppTypeRegistry>,
) {
    let runs = scan_runs(&save_root, &app_registry);
    let selected_id = runs.first().map(|h| h.run_id.clone());
    commands.insert_resource(RunList {
        runs,
        selected_id,
        filter: RunFilter::All,
        confirm_delete: false,
    });
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<RunList>();
}

fn scan_runs(save_root: &SaveRoot, app_registry: &AppTypeRegistry) -> Vec<RunSaveHeader> {
    let registry = app_registry.read();
    let ids = list_run_ids(save_root);
    let mut runs: Vec<RunSaveHeader> = ids
        .iter()
        .filter_map(|id| read_run_header(save_root, id, &registry))
        .collect();
    // In-progress first, then newest first within each group
    runs.sort_by(|a, b| {
        let a_ip = a.status == RunStatus::InProgress;
        let b_ip = b.status == RunStatus::InProgress;
        b_ip.cmp(&a_ip)
            .then_with(|| b.start_time_secs.cmp(&a.start_time_secs))
    });
    runs
}

fn handle_buttons(
    btn_q: Query<(&Interaction, &Btn), Changed<Interaction>>,
    mut run_list: ResMut<RunList>,
    mut load_events: MessageWriter<LoadRunEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    save_root: Res<SaveRoot>,
    app_registry: Res<AppTypeRegistry>,
) {
    for (interaction, btn) in &btn_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match btn {
            Btn::Back => next_state.set(GameState::MainMenu),
            Btn::FilterAll => {
                run_list.filter = RunFilter::All;
                run_list.confirm_delete = false;
            }
            Btn::FilterInProgress => {
                run_list.filter = RunFilter::InProgress;
                run_list.confirm_delete = false;
            }
            Btn::FilterCompleted => {
                run_list.filter = RunFilter::Completed;
                run_list.confirm_delete = false;
            }
            Btn::Select(id) => {
                run_list.selected_id = Some(id.clone());
                run_list.confirm_delete = false;
            }
            Btn::Load => {
                if let Some(id) = &run_list.selected_id {
                    load_events.write(LoadRunEvent { run_id: id.clone() });
                }
            }
            Btn::Delete => {
                run_list.confirm_delete = true;
            }
            Btn::ConfirmDelete => {
                if let Some(id) = &run_list.selected_id {
                    let _ = std::fs::remove_dir_all(save_root.run_dir(id));
                }
                let runs = scan_runs(&save_root, &app_registry);
                let selected_id = runs.first().map(|h| h.run_id.clone());
                run_list.runs = runs;
                run_list.selected_id = selected_id;
                run_list.confirm_delete = false;
            }
            Btn::CancelDelete => {
                run_list.confirm_delete = false;
            }
        }
    }
}

fn rebuild_ui(
    mut commands: Commands,
    run_list: Res<RunList>,
    roots: Query<Entity, With<LoadRunRoot>>,
) {
    if !run_list.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    build_ui(&mut commands, &run_list);
}

// ---------------------------------------------------------------------------
// UI builder
// ---------------------------------------------------------------------------

fn build_ui(commands: &mut Commands, run_list: &RunList) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(space::XXXL)),
                row_gap: Val::Px(space::XL),
                ..default()
            },
            BackgroundColor(palette::BG),
            LoadRunRoot,
            DespawnOnExit(GameState::LoadRun),
        ))
        .with_children(|root| {
            spawn_header(root, run_list);
            spawn_body(root, run_list);
        });
}

fn spawn_header(parent: &mut ChildSpawnerCommands, run_list: &RunList) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|hdr| {
            hdr.spawn(hstack(space::LG)).with_children(|left| {
                left.spawn(heading("LOAD RUN", H::Xl));

                for (btn, lbl, active) in [
                    (Btn::FilterAll, "ALL", run_list.filter == RunFilter::All),
                    (
                        Btn::FilterInProgress,
                        "IN PROGRESS",
                        run_list.filter == RunFilter::InProgress,
                    ),
                    (
                        Btn::FilterCompleted,
                        "COMPLETED",
                        run_list.filter == RunFilter::Completed,
                    ),
                ] {
                    left.spawn((UiButton::toggle(active), btn))
                        .with_children(|b| {
                            b.spawn(button_label(lbl));
                        });
                }
            });

            hdr.spawn((UiButton::default(), Btn::Back))
                .with_children(|b| {
                    b.spawn(button_label("BACK"));
                });
        });
}

fn spawn_body(parent: &mut ChildSpawnerCommands, run_list: &RunList) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            column_gap: Val::Px(space::XL),
            min_height: Val::Px(0.0),
            ..default()
        })
        .with_children(|body| {
            // Left: run list
            body.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(space::XS),
                    width: Val::Px(340.0),
                    border: UiRect::all(Val::Px(border::THIN)),
                    padding: UiRect::all(Val::Px(space::XL)),
                    border_radius: BorderRadius::all(Val::Px(radius::LG)),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                BackgroundColor(palette::P1),
                BorderColor::all(palette::BORDER),
            ))
            .with_children(|list| {
                let visible = run_list.visible_runs();
                if visible.is_empty() {
                    list.spawn(caption("No runs found."));
                } else {
                    for h in visible {
                        let selected = run_list.selected_id.as_deref() == Some(h.run_id.as_str());
                        spawn_run_row(list, h, selected);
                    }
                }
            });

            // Right: detail pane
            body.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    border: UiRect::all(Val::Px(border::THIN)),
                    padding: UiRect::all(Val::Px(space::XL)),
                    border_radius: BorderRadius::all(Val::Px(radius::LG)),
                    row_gap: Val::Px(space::MD),
                    ..default()
                },
                BackgroundColor(palette::P1),
                BorderColor::all(palette::BORDER),
            ))
            .with_children(|detail| {
                if let Some(h) = run_list.selected() {
                    spawn_detail(detail, h, run_list.confirm_delete);
                } else {
                    detail.spawn(caption("Select a run to view details."));
                }
            });
        });
}

fn spawn_run_row(parent: &mut ChildSpawnerCommands, h: &RunSaveHeader, selected: bool) {
    let (bg, border_clr) = if selected {
        (Color::srgba(0.541, 0.447, 0.667, 0.12), palette::ACCENT)
    } else {
        (Color::NONE, Color::NONE)
    };

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(space::MD)),
                border: UiRect::all(Val::Px(border::THIN)),
                border_radius: BorderRadius::all(Val::Px(radius::MD)),
                row_gap: Val::Px(space::XS),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border_clr),
            Button,
            Btn::Select(h.run_id.clone()),
        ))
        .with_children(|row| {
            let seed = if h.seed_text.is_empty() {
                "—"
            } else {
                &h.seed_text
            };
            row.spawn(hstack(space::SM)).with_children(|top| {
                top.spawn((
                    Text::new(seed.to_string()),
                    TextFont {
                        font_size: font_size::H_SM,
                        ..default()
                    },
                    TextColor(palette::TEXT),
                ));
                top.spawn(caption(difficulty_label(&h.difficulty)));
            });

            let (status_str, status_color) = match h.status {
                RunStatus::InProgress => ("In Progress", palette::OK),
                RunStatus::Completed => ("Completed", palette::DIM),
            };
            row.spawn(hstack(space::MD)).with_children(|bottom| {
                bottom.spawn((
                    Text::new(status_str),
                    TextFont {
                        font_size: font_size::LABEL_SM,
                        ..default()
                    },
                    TextColor(status_color),
                ));
                bottom.spawn(caption(format_playtime(h.total_playtime_secs)));
            });
        });
}

fn spawn_detail(parent: &mut ChildSpawnerCommands, h: &RunSaveHeader, confirm_delete: bool) {
    let seed = if h.seed_text.is_empty() {
        "—"
    } else {
        &h.seed_text
    };
    parent.spawn(heading(seed, H::Md));

    parent.spawn(vstack(space::XS)).with_children(|meta| {
        meta.spawn(hstack(space::LG)).with_children(|row| {
            row.spawn(caption("DIFFICULTY"));
            row.spawn(label(difficulty_label(&h.difficulty)));
        });
        meta.spawn(hstack(space::LG)).with_children(|row| {
            row.spawn(caption("STATUS"));
            let (status_str, status_color) = match h.status {
                RunStatus::InProgress => ("In Progress", palette::OK),
                RunStatus::Completed => ("Completed", palette::DIM),
            };
            row.spawn((
                Text::new(status_str),
                TextFont {
                    font_size: font_size::LABEL,
                    ..default()
                },
                TextColor(status_color),
            ));
        });
        meta.spawn(hstack(space::LG)).with_children(|row| {
            row.spawn(caption("PLAYTIME"));
            row.spawn(label(format_playtime(h.total_playtime_secs)));
        });
    });

    parent.spawn(divider());

    parent.spawn(caption("CHECKPOINTS"));
    if h.checkpoints.is_empty() {
        parent.spawn(caption("No checkpoints saved."));
    } else {
        for cp in &h.checkpoints {
            parent.spawn(hstack(space::MD)).with_children(|row| {
                let kind = match &cp.kind {
                    CheckpointKind::Manual => "Manual".to_string(),
                    CheckpointKind::TierUnlock(t) => format!("Tier {t} Unlock"),
                    CheckpointKind::EscapeConstructionStart => "Escape Start".to_string(),
                };
                row.spawn(label(kind));
                if !cp.label.is_empty() {
                    row.spawn(caption(format!("\"{}\"", cp.label)));
                }
                row.spawn(caption(format_unix_date(cp.created_at_secs)));
            });
        }
    }

    // Push actions to the bottom
    parent.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });

    parent.spawn(divider());

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(space::MD),
            ..default()
        })
        .with_children(|actions| {
            if confirm_delete {
                actions.spawn((
                    Text::new("Delete this run? This cannot be undone."),
                    TextFont {
                        font_size: font_size::LABEL,
                        ..default()
                    },
                    TextColor(palette::WARN),
                ));
                actions.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                actions
                    .spawn((UiButton::default(), Btn::CancelDelete))
                    .with_children(|b| {
                        b.spawn(button_label("CANCEL"));
                    });
                actions
                    .spawn((UiButton::default(), Btn::ConfirmDelete))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("CONFIRM DELETE"),
                            TextFont {
                                font_size: font_size::BUTTON,
                                ..default()
                            },
                            TextColor(palette::ERR),
                        ));
                    });
            } else {
                actions
                    .spawn((UiButton::ghost(), Btn::Delete))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("DELETE"),
                            TextFont {
                                font_size: font_size::BUTTON,
                                ..default()
                            },
                            TextColor(palette::ERR),
                        ));
                    });
                actions.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                actions
                    .spawn((UiButton::accent(), Btn::Load))
                    .with_children(|b| {
                        b.spawn(button_label("LOAD RUN"));
                    });
            }
        });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn difficulty_label(d: &DifficultyTier) -> &'static str {
    match d {
        DifficultyTier::Initiation => "Initiation",
        DifficultyTier::Standard => "Standard",
        DifficultyTier::Advanced => "Advanced",
        DifficultyTier::Pinnacle => "Pinnacle",
    }
}

fn format_playtime(secs: f64) -> String {
    let s = secs as u64;
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    } else {
        format!("{}m {:02}s", s / 60, s % 60)
    }
}

fn format_unix_date(secs: u64) -> String {
    // Approximate UTC date display (ignores leap years)
    let days = secs / 86400;
    let year = 1970 + days / 365;
    let remaining = days % 365;
    let month = remaining / 30 + 1;
    let day = remaining % 30 + 1;
    format!("{year}-{month:02}-{day:02}")
}

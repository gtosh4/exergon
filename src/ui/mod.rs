use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use crate::{
    content::VeinRegistry,
    seed::{hash_text, DomainSeeds, RunSeed},
    world::LookTarget,
    GameState,
};

use bevy::app::AppExit;
use bevy::ecs::message::MessageWriter;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .init_resource::<MainMenuState>()
            .add_systems(
                EguiPrimaryContextPass,
                (
                    main_menu.run_if(in_state(GameState::MainMenu)),
                    pause_menu.run_if(in_state(GameState::Paused)),
                    look_tooltip.run_if(in_state(GameState::Playing)),
                ),
            );
    }
}

#[derive(Resource, Default)]
struct MainMenuState {
    seed_text: String,
}

fn look_tooltip(
    mut contexts: EguiContexts,
    look_target: Option<Res<LookTarget>>,
    registry: Option<Res<VeinRegistry>>,
) -> Result {
    let Some(target) = look_target else {
        return Ok(());
    };
    let label: std::borrow::Cow<str> = match *target {
        LookTarget::Nothing => return Ok(()),
        LookTarget::Voxel { material } => {
            let name = registry
                .as_ref()
                .and_then(|r| r.material_name(material))
                .unwrap_or("Unknown");
            name.into()
        }
    };
    let ctx = contexts.ctx_mut()?;
    egui::Area::new(egui::Id::new("waila"))
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -40.0])
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_black_alpha(200))
                .show(ui, |ui| {
                    ui.set_max_width(ui.ctx().content_rect().width() * 0.3);
                    ui.colored_label(egui::Color32::WHITE, label);
                });
        });
    Ok(())
}

fn pause_menu(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) -> Result {
    egui::CentralPanel::default().show(contexts.ctx_mut()?, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(200.0);
            ui.heading("Paused");
            ui.add_space(40.0);
            if ui.button("Resume").clicked() {
                next_state.set(GameState::Playing);
            }
            ui.add_space(16.0);
            if ui.button("Quit").clicked() {
                app_exit.write(AppExit::Success);
            }
        });
    });
    Ok(())
}

fn main_menu(
    mut contexts: EguiContexts,
    mut state: ResMut<MainMenuState>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) -> Result {
    egui::CentralPanel::default().show(contexts.ctx_mut()?, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(200.0);
            ui.heading("EXERGON");
            ui.add_space(40.0);
            ui.label("Seed");
            let response = ui.text_edit_singleline(&mut state.seed_text);
            ui.add_space(16.0);
            let start = ui.button("Start Run").clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if start {
                let hash = hash_text(&state.seed_text);
                commands.insert_resource(RunSeed {
                    text: state.seed_text.clone(),
                    hash,
                });
                commands.insert_resource(DomainSeeds::from_master(hash));
                next_state.set(GameState::Loading);
            }
        });
    });
    Ok(())
}

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use crate::{
    inventory::{HotbarSlot, Hotbar, Inventory, InventoryOpen, ItemRegistry},
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
                    (look_tooltip, hotbar_ui, inventory_ui)
                        .run_if(in_state(GameState::Playing)),
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
    item_registry: Option<Res<ItemRegistry>>,
) -> Result {
    let Some(target) = look_target else {
        return Ok(());
    };
    let label: String = match *target {
        LookTarget::Nothing => return Ok(()),
        LookTarget::Voxel { material, .. } => item_registry
            .as_ref()
            .and_then(|r| r.item_for_voxel(material))
            .map(|i| i.name.clone())
            .unwrap_or_else(|| "Unknown".into()),
    };
    let ctx = contexts.ctx_mut()?;
    egui::Area::new(egui::Id::new("waila"))
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -80.0])
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

fn hotbar_ui(
    mut contexts: EguiContexts,
    hotbar: Option<Res<Hotbar>>,
    item_registry: Option<Res<ItemRegistry>>,
) -> Result {
    let Some(hotbar) = hotbar else { return Ok(()) };
    let ctx = contexts.ctx_mut()?;

    egui::Area::new(egui::Id::new("hotbar"))
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for i in 0..9usize {
                    let selected = i == hotbar.selected;
                    let border_color = if selected {
                        egui::Color32::from_rgb(255, 220, 50)
                    } else {
                        egui::Color32::from_gray(120)
                    };
                    let stroke_width = if selected { 2.0 } else { 1.0 };
                    egui::Frame::new()
                        .stroke(egui::Stroke::new(stroke_width, border_color))
                        .fill(egui::Color32::from_black_alpha(180))
                        .inner_margin(egui::Margin::same(4))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::Vec2::new(64.0, 64.0));
                            ui.set_max_size(egui::Vec2::new(64.0, 64.0));
                            match hotbar.slots[i].as_ref() {
                                Some(s) => {
                                    let name = item_registry
                                        .as_ref()
                                        .and_then(|r| r.get(&s.item_id))
                                        .map(|d| d.name.as_str())
                                        .unwrap_or(s.item_id.as_str());
                                    ui.colored_label(egui::Color32::WHITE, name);
                                    ui.colored_label(
                                        egui::Color32::LIGHT_GRAY,
                                        format!("×{}", s.count),
                                    );
                                }
                                None => {
                                    ui.colored_label(egui::Color32::DARK_GRAY, "·");
                                }
                            }
                            ui.colored_label(
                                egui::Color32::from_gray(160),
                                format!("{}", i + 1),
                            );
                        });
                }
            });
        });
    Ok(())
}

fn inventory_ui(
    mut contexts: EguiContexts,
    inv_open: Option<Res<InventoryOpen>>,
    mut inventory: Option<ResMut<Inventory>>,
    mut hotbar: Option<ResMut<Hotbar>>,
    item_registry: Option<Res<ItemRegistry>>,
) -> Result {
    if !inv_open.map(|o| o.0).unwrap_or(false) {
        return Ok(());
    }
    let (Some(inventory), Some(hotbar)) = (inventory.as_mut(), hotbar.as_mut()) else {
        return Ok(());
    };

    let ctx = contexts.ctx_mut()?;
    let mut move_item: Option<String> = None;

    egui::Window::new("Inventory")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("Active slot: {}", hotbar.selected + 1));
            ui.separator();

            if inventory.0.is_empty() {
                ui.colored_label(egui::Color32::DARK_GRAY, "(empty)");
            } else {
                let mut items: Vec<(&String, u32)> =
                    inventory.0.iter().map(|(k, &c)| (k, c)).collect();
                items.sort_by_key(|(k, _)| k.as_str());

                egui::Grid::new("inv_grid")
                    .num_columns(5)
                    .min_col_width(72.0)
                    .spacing([4.0, 4.0])
                    .show(ui, |ui| {
                        for (idx, (item_id, count)) in items.iter().enumerate() {
                            let name = item_registry
                                .as_ref()
                                .and_then(|r| r.get(item_id))
                                .map(|d| d.name.as_str())
                                .unwrap_or(item_id.as_str());
                            let resp = ui.button(format!("{}\n×{}", name, count));
                            if resp.clicked() {
                                move_item = Some(item_id.to_string());
                            }
                            resp.on_hover_text("Move to active hotbar slot");
                            if (idx + 1) % 5 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            }

            ui.separator();
            ui.label("[Tab] or [Esc] to close");
        });

    if let Some(item_id) = move_item {
        if let Some(count) = inventory.0.remove(&item_id) {
            let idx = hotbar.selected;
            let taken = hotbar.slots[idx].take();
            if let Some(current) = taken {
                if current.item_id != item_id {
                    inventory.add(current.item_id.clone(), current.count);
                    hotbar.slots[idx] = Some(HotbarSlot { item_id, count });
                } else {
                    hotbar.slots[idx] =
                        Some(HotbarSlot { item_id, count: count + current.count });
                }
            } else {
                hotbar.slots[idx] = Some(HotbarSlot { item_id, count });
            }
        }
    }

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

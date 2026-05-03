use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use crate::{
    GameState, PlayMode,
    inventory::{Hotbar, HotbarSlot, Inventory, InventoryOpen, ItemRegistry},
    machine::{Machine, MachineActivity, MachineState},
    power::PowerNetwork,
    recipe_graph::RecipeGraph,
    research::{ResearchPool, TechTreeProgress},
    seed::{DomainSeeds, RunSeed, hash_text},
    tech_tree::{NodeEffect, TechTree, UnlockVector},
    world::{LookTarget, MainCamera},
};

use bevy::app::AppExit;
use bevy::ecs::message::MessageWriter;

pub struct UiPlugin;

#[derive(Resource, Default)]
pub struct MachineStatusPanel(pub Option<Entity>);

#[derive(Resource, Default)]
pub struct TechTreePanelOpen(pub bool);

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .init_resource::<MainMenuState>()
            .init_resource::<MachineStatusPanel>()
            .init_resource::<TechTreePanelOpen>()
            .add_systems(
                Update,
                inspect_input
                    .run_if(in_state(PlayMode::Exploring))
                    .run_if(|o: Option<Res<InventoryOpen>>| !o.is_some_and(|r| r.0)),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    main_menu.run_if(in_state(GameState::MainMenu)),
                    pause_menu.run_if(in_state(GameState::Paused)),
                    (
                        crosshair,
                        look_tooltip,
                        hotbar_ui,
                        inventory_ui,
                        machine_status_ui,
                        tech_tree_ui,
                        power_hud_ui,
                    )
                        .run_if(in_state(GameState::Playing)),
                ),
            );
    }
}

#[derive(Resource, Default)]
struct MainMenuState {
    seed_text: String,
}

fn inspect_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    spatial_query: SpatialQuery,
    machine_q: Query<(), With<Machine>>,
    mut panel: ResMut<MachineStatusPanel>,
    mut tech_tree_open: ResMut<TechTreePanelOpen>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        tech_tree_open.0 = !tech_tree_open.0;
    }

    if mouse.just_pressed(MouseButton::Right) {
        let Ok(cam) = camera_q.single() else { return };
        let dir = Dir3::new(*cam.forward()).unwrap_or(Dir3::NEG_Z);
        let hit = spatial_query.cast_ray(cam.translation, dir, 8.0, true, &Default::default());
        panel.0 = hit
            .filter(|h| machine_q.contains(h.entity))
            .map(|h| h.entity);
    }
}

fn machine_status_ui(
    mut contexts: EguiContexts,
    mut panel: ResMut<MachineStatusPanel>,
    machine_q: Query<(&Machine, &MachineState, Option<&MachineActivity>)>,
    recipe_graph: Option<Res<RecipeGraph>>,
) -> Result {
    let Some(entity) = panel.0 else { return Ok(()) };
    let Ok((machine, state, activity)) = machine_q.get(entity) else {
        panel.0 = None;
        return Ok(());
    };

    let ctx = contexts.ctx_mut()?;
    let mut open = true;
    egui::SidePanel::right("machine_status")
        .resizable(false)
        .min_width(220.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&machine.machine_type);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked() {
                        open = false;
                    }
                });
            });
            ui.label(format!("Tier: {}", machine.tier));
            ui.separator();

            let (state_text, state_color) = match state {
                MachineState::Idle => ("Idle", egui::Color32::GRAY),
                MachineState::Running => ("Running", egui::Color32::from_rgb(80, 220, 80)),
            };
            ui.colored_label(state_color, state_text);

            if let Some(act) = activity {
                ui.separator();
                ui.label(format!("Recipe: {}", act.recipe_id));
                let progress_pct = recipe_graph
                    .as_ref()
                    .and_then(|rg| rg.recipes.get(&act.recipe_id))
                    .map(|r| act.progress / r.processing_time)
                    .unwrap_or(0.0);
                ui.add(
                    egui::ProgressBar::new(progress_pct)
                        .text(format!("{:.0}%", progress_pct * 100.0)),
                );
                ui.label(format!("Speed: {:.0}%", act.speed_factor * 100.0));
            }
        });

    if !open {
        panel.0 = None;
    }
    Ok(())
}

fn tech_tree_ui(
    mut contexts: EguiContexts,
    mut open: ResMut<TechTreePanelOpen>,
    tech_tree: Option<Res<TechTree>>,
    progress: Option<Res<TechTreeProgress>>,
    pool: Option<Res<ResearchPool>>,
) -> Result {
    if !open.0 {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;
    let mut is_open = open.0;
    egui::Window::new("Tech Tree")
        .open(&mut is_open)
        .resizable(true)
        .default_size([600.0, 400.0])
        .show(ctx, |ui| {
            if let Some(pool) = &pool {
                ui.label(format!("Research Points: {:.1}", pool.points));
                ui.separator();
            }

            let Some(tree) = &tech_tree else {
                ui.label("(no tech tree loaded)");
                return;
            };
            let empty_progress = TechTreeProgress::default();
            let progress = progress.as_deref().unwrap_or(&empty_progress);

            if tree.tier_order.is_empty() {
                ui.label("(no nodes)");
                return;
            }

            egui::ScrollArea::both().show(ui, |ui| {
                egui::Grid::new("tech_tree_grid")
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        const COLS: usize = 4;
                        let mut col = 0usize;
                        let mut current_tier = 0u8;

                        for node_id in &tree.tier_order {
                            let Some(node) = tree.nodes.get(node_id) else {
                                continue;
                            };
                            if node.tier != current_tier && col != 0 {
                                ui.end_row();
                                col = 0;
                            }
                            current_tier = node.tier;

                            let unlocked = progress.unlocked_nodes.contains(node_id);
                            let resp = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(&node.name).color(if unlocked {
                                        egui::Color32::from_rgb(80, 200, 80)
                                    } else {
                                        egui::Color32::from_gray(160)
                                    }),
                                )
                                .fill(if unlocked {
                                    egui::Color32::from_rgb(30, 60, 30)
                                } else {
                                    egui::Color32::from_gray(40)
                                })
                                .min_size(egui::Vec2::new(120.0, 48.0)),
                            );

                            resp.on_hover_ui(|ui| {
                                ui.label(format!("Tier: {}", node.tier));
                                match &node.primary_unlock {
                                    UnlockVector::ResearchSpend(cost) => {
                                        ui.label(format!("Cost: {} RP", cost));
                                    }
                                    UnlockVector::ExplorationDiscovery(loc) => {
                                        ui.label(format!("Discover: {}", loc));
                                    }
                                    UnlockVector::PrerequisiteChain => {
                                        ui.label("Complete prerequisites");
                                    }
                                    UnlockVector::ProductionMilestone { material, quantity } => {
                                        ui.label(format!("Produce {quantity} {material}"));
                                    }
                                    UnlockVector::Observation(loc) => {
                                        ui.label(format!("Observe: {loc}"));
                                    }
                                }
                                if !node.prerequisites.is_empty() {
                                    ui.label(format!(
                                        "Prereqs: {}",
                                        node.prerequisites.join(", ")
                                    ));
                                }
                                for effect in &node.effects {
                                    match effect {
                                        NodeEffect::UnlockRecipes(recipes) => {
                                            ui.label(format!(
                                                "Unlocks: {}",
                                                recipes.join(", ")
                                            ));
                                        }
                                        NodeEffect::UnlockMachine(m) => {
                                            ui.label(format!("Unlocks machine: {}", m));
                                        }
                                    }
                                }
                            });

                            col += 1;
                            if col >= COLS {
                                ui.end_row();
                                col = 0;
                            }
                        }
                    });
            });
        });

    open.0 = is_open;
    Ok(())
}

fn power_hud_ui(
    mut contexts: EguiContexts,
    net_q: Query<&PowerNetwork>,
    machine_q: Query<(&MachineState, Option<&MachineActivity>)>,
    recipe_graph: Option<Res<RecipeGraph>>,
    inv_open: Option<Res<InventoryOpen>>,
) -> Result {
    if inv_open.is_some_and(|o| o.0) {
        return Ok(());
    }

    let produced: f32 = net_q.iter().map(|n| n.capacity_watts).sum();
    let demanded: f32 = recipe_graph
        .as_ref()
        .map(|rg| {
            machine_q
                .iter()
                .filter_map(|(state, activity)| {
                    if *state != MachineState::Running {
                        return None;
                    }
                    let act = activity?;
                    let recipe = rg.recipes.get(&act.recipe_id)?;
                    Some(recipe.energy_cost / recipe.processing_time)
                })
                .sum()
        })
        .unwrap_or(0.0);

    let label = if demanded > 0.0 {
        let pct = (produced / demanded * 100.0).min(100.0);
        format!("⚡ {produced:.0}W / {demanded:.0}W ({pct:.0}%)")
    } else {
        format!("⚡ {produced:.0}W / 0W")
    };

    let ctx = contexts.ctx_mut()?;
    egui::Area::new(egui::Id::new("power_hud"))
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(160))
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| {
                    ui.colored_label(egui::Color32::from_rgb(255, 220, 50), &label);
                });
        });
    Ok(())
}

fn crosshair(mut contexts: EguiContexts, inv_open: Option<Res<InventoryOpen>>) -> Result {
    if inv_open.is_some_and(|o| o.0) {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let center = ctx.content_rect().center();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("crosshair"),
    ));
    painter.circle_filled(
        center,
        3.0,
        egui::Color32::from_rgba_unmultiplied(180, 180, 180, 120),
    );
    Ok(())
}

fn look_tooltip(mut contexts: EguiContexts, look_target: Option<Res<LookTarget>>) -> Result {
    let Some(target) = look_target else {
        return Ok(());
    };
    let label: String = match *target {
        LookTarget::Nothing => return Ok(()),
        LookTarget::Surface { pos, .. } => {
            let snapped = pos.floor().as_ivec3();
            format!("{}, {}, {}", snapped.x, snapped.y, snapped.z)
        }
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
                            match hotbar.slots.get(i).and_then(|s| s.as_ref()) {
                                Some(s) => {
                                    let name = item_registry
                                        .as_ref()
                                        .and_then(|r| r.get(&s.item_id))
                                        .map_or(s.item_id.as_str(), |d| d.name.as_str());
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
                            ui.colored_label(egui::Color32::from_gray(160), format!("{}", i + 1));
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
    if !inv_open.is_some_and(|o| o.0) {
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
                                .map_or(item_id.as_str(), |d| d.name.as_str());
                            let resp = ui.button(format!("{name}\n×{count}"));
                            if resp.clicked() {
                                move_item = Some((*item_id).clone());
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

    if let Some(item_id) = move_item
        && let Some(count) = inventory.0.remove(&item_id)
    {
        let idx = hotbar.selected;
        if let Some(slot) = hotbar.slots.get_mut(idx) {
            let taken = slot.take();
            if let Some(current) = taken {
                if current.item_id == item_id {
                    *slot = Some(HotbarSlot {
                        item_id,
                        count: count + current.count,
                    });
                } else {
                    inventory.add(current.item_id.clone(), current.count);
                    *slot = Some(HotbarSlot { item_id, count });
                }
            } else {
                *slot = Some(HotbarSlot { item_id, count });
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

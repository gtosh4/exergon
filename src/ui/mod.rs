use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use crate::{
    GameState, PlayMode,
    inventory::{Hotbar, HotbarSlot, Inventory, InventoryOpen, ItemRegistry},
    logistics::StorageUnit,
    machine::{IoPortMarker, Machine, MachineActivity, MachineState},
    power::PowerNetwork,
    recipe_graph::RecipeGraph,
    research::{ResearchPool, TechTreeProgress},
    seed::{DomainSeeds, RunSeed, hash_text},
    tech_tree::{NodeEffect, TechTree, UnlockVector},
    world::{LookTarget, MainCamera, Player},
};

use bevy::app::AppExit;
use bevy::ecs::message::MessageWriter;

pub struct UiPlugin;

#[derive(Resource, Default)]
pub struct MachineStatusPanel {
    pub entity: Option<Entity>,
    pub recipe_filter: String,
}

#[derive(Resource, Default)]
pub struct StorageStatusPanel(pub Option<Entity>);

#[derive(Resource, Default)]
pub struct TechTreePanelOpen {
    pub open: bool,
    pub selected_node: Option<String>,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .init_resource::<MainMenuState>()
            .init_resource::<MachineStatusPanel>()
            .init_resource::<StorageStatusPanel>()
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
                        storage_status_ui,
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
    storage_q: Query<(), With<StorageUnit>>,
    port_q: Query<&IoPortMarker>,
    player_q: Query<Entity, With<Player>>,
    mut panel: ResMut<MachineStatusPanel>,
    mut storage_panel: ResMut<StorageStatusPanel>,
    mut tech_tree_open: ResMut<TechTreePanelOpen>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) || keyboard.just_pressed(KeyCode::F4) {
        tech_tree_open.open = !tech_tree_open.open;
    }

    if mouse.just_pressed(MouseButton::Right) {
        let Ok(cam) = camera_q.single() else { return };
        let dir = Dir3::new(*cam.forward()).unwrap_or(Dir3::NEG_Z);
        let mut filter = SpatialQueryFilter::default();
        if let Ok(player) = player_q.single() {
            filter.excluded_entities.insert(player);
        }
        let hit = spatial_query.cast_ray(cam.translation, dir, 8.0, true, &filter);

        panel.entity = None;
        storage_panel.0 = None;

        if let Some(h) = hit {
            if storage_q.contains(h.entity) {
                storage_panel.0 = Some(h.entity);
            } else if machine_q.contains(h.entity) {
                panel.entity = Some(h.entity);
            } else if let Ok(m) = port_q.get(h.entity) {
                if storage_q.contains(m.owner) {
                    storage_panel.0 = Some(m.owner);
                } else if machine_q.contains(m.owner) {
                    panel.entity = Some(m.owner);
                }
            }
        }
    }
}

fn machine_status_ui(
    mut contexts: EguiContexts,
    mut panel: ResMut<MachineStatusPanel>,
    machine_q: Query<(&Machine, &MachineState, Option<&MachineActivity>)>,
    recipe_graph: Option<Res<RecipeGraph>>,
    item_registry: Option<Res<ItemRegistry>>,
) -> Result {
    let Some(entity) = panel.entity else {
        return Ok(());
    };
    let Ok((machine, state, activity)) = machine_q.get(entity) else {
        panel.entity = None;
        return Ok(());
    };

    let gold = egui::Color32::from_rgb(200, 168, 48);
    let dim = egui::Color32::from_rgb(110, 90, 40);
    let green = egui::Color32::from_rgb(64, 160, 32);

    // Extract before closure to avoid borrow conflicts with panel.recipe_filter
    let machine_type = machine.machine_type.clone();
    let machine_tier = machine.tier;
    let energy_count = machine.energy_ports.len();
    let logistics_count = machine.logistics_ports.len();
    let current_recipe_id = activity.as_ref().map(|a| a.recipe_id.clone());
    let current_progress = activity
        .as_ref()
        .and_then(|a| {
            recipe_graph
                .as_ref()
                .and_then(|rg| rg.recipes.get(&a.recipe_id))
                .map(|r| a.progress / r.processing_time)
        })
        .unwrap_or(0.0);
    let speed_factor = activity.as_ref().map(|a| a.speed_factor).unwrap_or(1.0);
    let state = *state;
    let title = machine_type.to_uppercase().replace('_', " ");

    let ctx = contexts.ctx_mut()?;
    let mut is_open = true;

    egui::Window::new(&title)
        .open(&mut is_open)
        .resizable(true)
        .default_size([560.0, 380.0])
        .show(ctx, |ui| {
            egui::SidePanel::left("machine_info_left")
                .resizable(false)
                .exact_width(200.0)
                .show_inside(ui, |ui| {
                    ui.colored_label(dim, format!("TIER {}", machine_tier));

                    let (state_text, state_color) = match state {
                        MachineState::Idle => ("● IDLE", egui::Color32::GRAY),
                        MachineState::Running => ("● RUNNING", green),
                    };
                    ui.colored_label(state_color, state_text);

                    if current_recipe_id.is_some() {
                        ui.separator();
                        ui.colored_label(
                            gold,
                            current_recipe_id.as_deref().unwrap_or("").replace('_', " "),
                        );
                        ui.add(
                            egui::ProgressBar::new(current_progress)
                                .text(format!("{:.0}%", current_progress * 100.0)),
                        );
                        if speed_factor < 0.99 {
                            ui.colored_label(
                                egui::Color32::YELLOW,
                                format!("Speed: {:.0}%", speed_factor * 100.0),
                            );
                        }
                    }

                    ui.separator();
                    ui.colored_label(dim, format!("⚡ {} power port(s)", energy_count));
                    ui.colored_label(dim, format!("📦 {} logistics port(s)", logistics_count));
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.colored_label(gold, "RECIPES");
                ui.text_edit_singleline(&mut panel.recipe_filter)
                    .on_hover_text("Filter recipes");
                ui.separator();

                let Some(rg) = &recipe_graph else {
                    ui.colored_label(dim, "(no recipes loaded)");
                    return;
                };
                let filter_lower = panel.recipe_filter.to_lowercase();

                let mut recipes: Vec<_> = rg
                    .recipes
                    .values()
                    .filter(|r| r.machine_type == machine_type && r.machine_tier <= machine_tier)
                    .filter(|r| {
                        filter_lower.is_empty() || r.id.to_lowercase().contains(&filter_lower)
                    })
                    .collect();
                recipes.sort_by_key(|r| r.id.as_str());

                egui::ScrollArea::vertical()
                    .id_salt("recipe_scroll")
                    .show(ui, |ui| {
                        if recipes.is_empty() {
                            ui.colored_label(dim, "(no matching recipes)");
                        }
                        for recipe in recipes {
                            let is_active = current_recipe_id
                                .as_deref()
                                .map(|id| id == recipe.id)
                                .unwrap_or(false);
                            let fill = if is_active {
                                egui::Color32::from_rgb(25, 50, 10)
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            egui::Frame::new()
                                .fill(fill)
                                .inner_margin(egui::Margin::same(4))
                                .show(ui, |ui| {
                                    ui.colored_label(
                                        if is_active { green } else { gold },
                                        recipe.id.replace('_', " "),
                                    );
                                    ui.horizontal(|ui| {
                                        let mut first = true;
                                        for inp in &recipe.inputs {
                                            if !first {
                                                ui.colored_label(dim, "+");
                                            }
                                            first = false;
                                            let name = item_registry
                                                .as_ref()
                                                .and_then(|ir| ir.get(&inp.item))
                                                .map_or(inp.item.as_str(), |d| d.name.as_str());
                                            ui.label(format!("{:.0}× {}", inp.quantity, name));
                                        }
                                        ui.colored_label(dim, "→");
                                        for out in &recipe.outputs {
                                            let name = item_registry
                                                .as_ref()
                                                .and_then(|ir| ir.get(&out.item))
                                                .map_or(out.item.as_str(), |d| d.name.as_str());
                                            ui.colored_label(
                                                green,
                                                format!("{:.0}× {}", out.quantity, name),
                                            );
                                        }
                                    });
                                    ui.colored_label(
                                        dim,
                                        format!(
                                            "{:.1}s  |  {:.0} W",
                                            recipe.processing_time,
                                            recipe.energy_cost / recipe.processing_time
                                        ),
                                    );
                                });
                            ui.separator();
                        }
                    });
            });
        });

    if !is_open {
        panel.entity = None;
    }
    Ok(())
}

fn storage_status_ui(
    mut contexts: EguiContexts,
    mut panel: ResMut<StorageStatusPanel>,
    storage_q: Query<&StorageUnit>,
    item_registry: Option<Res<ItemRegistry>>,
) -> Result {
    let Some(entity) = panel.0 else { return Ok(()) };
    let Ok(unit) = storage_q.get(entity) else {
        panel.0 = None;
        return Ok(());
    };

    let ctx = contexts.ctx_mut()?;
    let mut open = true;
    egui::SidePanel::right("storage_status")
        .resizable(false)
        .min_width(220.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Storage Crate");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked() {
                        open = false;
                    }
                });
            });
            ui.separator();

            if unit.items.is_empty() {
                ui.colored_label(egui::Color32::DARK_GRAY, "(empty)");
            } else {
                let mut items: Vec<(&String, u32)> =
                    unit.items.iter().map(|(k, &c)| (k, c)).collect();
                items.sort_by_key(|(k, _)| k.as_str());

                egui::Grid::new("storage_grid")
                    .num_columns(2)
                    .min_col_width(100.0)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        for (item_id, count) in &items {
                            let name = item_registry
                                .as_ref()
                                .and_then(|r| r.get(item_id))
                                .map_or(item_id.as_str(), |d| d.name.as_str());
                            ui.label(name);
                            ui.label(format!("×{count}"));
                            ui.end_row();
                        }
                    });
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
    if !open.open {
        return Ok(());
    }

    let gold = egui::Color32::from_rgb(200, 168, 48);
    let dim = egui::Color32::from_rgb(110, 90, 40);
    let green = egui::Color32::from_rgb(64, 160, 32);
    const NODE_W: f32 = 130.0;
    const NODE_H: f32 = 44.0;
    const TIER_SPACING: f32 = 200.0;
    const ROW_SPACING: f32 = 60.0;
    const HEADER_H: f32 = 20.0;

    let ctx = contexts.ctx_mut()?;
    let mut is_open = open.open;
    let selected_clone = open.selected_node.clone();
    let mut clicked_node: Option<Option<String>> = None;

    egui::Window::new("TECH TREE")
        .open(&mut is_open)
        .resizable(true)
        .default_size([820.0, 520.0])
        .show(ctx, |ui| {
            // Research points header
            egui::TopBottomPanel::top("tech_header").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(gold, "TECH TREE");
                    if let Some(p) = &pool {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.colored_label(gold, format!("{:.0} RP", p.points));
                            ui.colored_label(dim, "Research Points:");
                        });
                    }
                });
            });

            // Selected node detail (right panel — must precede CentralPanel)
            if let Some(ref sel_id) = selected_clone
                && let Some(tree) = &tech_tree
                && let Some(node) = tree.nodes.get(sel_id)
            {
                let empty_prog = TechTreeProgress::default();
                let prog = progress.as_deref().unwrap_or(&empty_prog);
                let unlocked = prog.unlocked_nodes.contains(sel_id);

                egui::SidePanel::right("tech_node_detail")
                    .resizable(false)
                    .exact_width(220.0)
                    .show_inside(ui, |ui| {
                        ui.colored_label(gold, &node.name);
                        ui.colored_label(dim, format!("Tier {}", node.tier));
                        ui.separator();

                        let (status_text, status_color) = if unlocked {
                            ("✓ Unlocked", green)
                        } else {
                            ("Locked", egui::Color32::GRAY)
                        };
                        ui.colored_label(status_color, status_text);
                        ui.separator();

                        ui.colored_label(dim, "Unlock:");
                        match &node.primary_unlock {
                            UnlockVector::ResearchSpend(cost) => {
                                let pts = pool.as_ref().map(|p| p.points).unwrap_or(0.0);
                                let can_afford = pts >= *cost as f32;
                                ui.colored_label(
                                    if can_afford { gold } else { dim },
                                    format!("{} / {} RP", pts as u32, cost),
                                );
                            }
                            UnlockVector::ExplorationDiscovery(loc) => {
                                ui.label(format!("Discover: {}", loc));
                            }
                            UnlockVector::PrerequisiteChain => {
                                ui.label("Complete prerequisites");
                            }
                            UnlockVector::ProductionMilestone { material, quantity } => {
                                ui.label(format!("Produce {:.0}× {}", quantity, material));
                            }
                            UnlockVector::Observation(loc) => {
                                ui.label(format!("Observe: {}", loc));
                            }
                        }

                        if !node.prerequisites.is_empty() {
                            ui.separator();
                            ui.colored_label(dim, "Requires:");
                            for prereq_id in &node.prerequisites {
                                let done = prog.unlocked_nodes.contains(prereq_id);
                                ui.colored_label(
                                    if done { green } else { egui::Color32::GRAY },
                                    format!("  {}", prereq_id.replace('_', " ")),
                                );
                            }
                        }

                        if !node.effects.is_empty() {
                            ui.separator();
                            ui.colored_label(dim, "Effects:");
                            for effect in &node.effects {
                                match effect {
                                    NodeEffect::UnlockRecipes(recipes) => {
                                        for r in recipes {
                                            ui.label(format!("  Recipe: {}", r.replace('_', " ")));
                                        }
                                    }
                                    NodeEffect::UnlockMachine(m) => {
                                        ui.label(format!("  Machine: {}", m.replace('_', " ")));
                                    }
                                }
                            }
                        }
                    });
            }

            // Node graph canvas
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let Some(tree) = &tech_tree else {
                    ui.colored_label(dim, "(no tech tree loaded)");
                    return;
                };
                let empty_prog = TechTreeProgress::default();
                let prog = progress.as_deref().unwrap_or(&empty_prog);

                // Compute stable node positions: tier → column, index-in-tier → row
                let mut positions: std::collections::HashMap<String, (f32, f32)> =
                    Default::default();
                let mut tier_row: std::collections::HashMap<u8, usize> = Default::default();
                for node_id in &tree.tier_order {
                    let Some(node) = tree.nodes.get(node_id) else {
                        continue;
                    };
                    let row = tier_row.entry(node.tier).or_insert(0);
                    let x = (node.tier as f32 - 1.0) * TIER_SPACING;
                    let y = HEADER_H + *row as f32 * ROW_SPACING;
                    positions.insert(node_id.clone(), (x, y));
                    *row += 1;
                }

                let max_tier = tree.nodes.values().map(|n| n.tier).max().unwrap_or(1);
                let max_rows = tier_row.values().max().copied().unwrap_or(1);
                let canvas_size = egui::vec2(
                    max_tier as f32 * TIER_SPACING + NODE_W + 20.0,
                    HEADER_H + max_rows as f32 * ROW_SPACING + NODE_H + 10.0,
                );

                egui::ScrollArea::both()
                    .id_salt("tech_tree_scroll")
                    .show(ui, |ui| {
                        let (response, painter) =
                            ui.allocate_painter(canvas_size, egui::Sense::click());
                        let origin = response.rect.min;

                        // Tier labels
                        let mut labeled: std::collections::HashSet<u8> = Default::default();
                        for node in tree.nodes.values() {
                            if labeled.insert(node.tier) {
                                let lx = (node.tier as f32 - 1.0) * TIER_SPACING + NODE_W / 2.0;
                                painter.text(
                                    origin + egui::vec2(lx, 4.0),
                                    egui::Align2::CENTER_TOP,
                                    format!("TIER {}", node.tier),
                                    egui::FontId::proportional(9.0),
                                    dim,
                                );
                            }
                        }

                        // Edges (prerequisite → node)
                        for node in tree.nodes.values() {
                            let Some(&(nx, ny)) = positions.get(&node.id) else {
                                continue;
                            };
                            for prereq_id in &node.prerequisites {
                                let Some(&(px, py)) = positions.get(prereq_id) else {
                                    continue;
                                };
                                let from = origin + egui::vec2(px + NODE_W, py + NODE_H / 2.0);
                                let to = origin + egui::vec2(nx, ny + NODE_H / 2.0);
                                let prereq_done = prog.unlocked_nodes.contains(prereq_id);
                                let stroke_col = if prereq_done {
                                    dim
                                } else {
                                    egui::Color32::from_rgb(50, 45, 22)
                                };
                                painter
                                    .line_segment([from, to], egui::Stroke::new(1.5, stroke_col));
                            }
                        }

                        // Nodes
                        let hover_pos = response.hover_pos();
                        for node in tree.nodes.values() {
                            let Some(&(nx, ny)) = positions.get(&node.id) else {
                                continue;
                            };
                            let rect = egui::Rect::from_min_size(
                                origin + egui::vec2(nx, ny),
                                egui::vec2(NODE_W, NODE_H),
                            );
                            let unlocked = prog.unlocked_nodes.contains(&node.id);
                            let selected = selected_clone.as_deref() == Some(node.id.as_str());
                            let hovered = hover_pos.map(|p| rect.contains(p)).unwrap_or(false);

                            let fill = if selected {
                                egui::Color32::from_rgb(40, 52, 14)
                            } else if unlocked {
                                egui::Color32::from_rgb(18, 36, 8)
                            } else {
                                egui::Color32::from_rgb(20, 20, 10)
                            };
                            let border_color = if selected {
                                gold
                            } else if unlocked {
                                green
                            } else if hovered {
                                dim
                            } else {
                                egui::Color32::from_rgb(44, 38, 18)
                            };
                            let text_color = if unlocked {
                                egui::Color32::from_rgb(170, 210, 110)
                            } else if hovered {
                                dim
                            } else {
                                egui::Color32::from_rgb(80, 72, 36)
                            };
                            let border_w = if selected { 2.0 } else { 1.0 };

                            painter.rect_filled(rect, 3.0, fill);
                            painter.rect_stroke(
                                rect,
                                3.0,
                                egui::Stroke::new(border_w, border_color),
                                egui::StrokeKind::Middle,
                            );
                            painter.text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                &node.name,
                                egui::FontId::proportional(11.0),
                                text_color,
                            );
                        }

                        // Click detection
                        if response.clicked()
                            && let Some(pos) = response.interact_pointer_pos()
                        {
                            let local = pos - origin;
                            let local_pos = egui::pos2(local.x, local.y);
                            let mut hit = false;
                            for node in tree.nodes.values() {
                                let Some(&(nx, ny)) = positions.get(&node.id) else {
                                    continue;
                                };
                                let node_rect = egui::Rect::from_min_size(
                                    egui::pos2(nx, ny),
                                    egui::vec2(NODE_W, NODE_H),
                                );
                                if node_rect.contains(local_pos) {
                                    clicked_node =
                                        if selected_clone.as_deref() == Some(node.id.as_str()) {
                                            Some(None)
                                        } else {
                                            Some(Some(node.id.clone()))
                                        };
                                    hit = true;
                                    break;
                                }
                            }
                            if !hit {
                                clicked_node = Some(None);
                            }
                        }
                    });
            });
        });

    open.open = is_open;
    if let Some(new_sel) = clicked_node {
        open.selected_node = new_sel;
    }
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
    storage_q: Query<&StorageUnit>,
    mut tab: Local<usize>,
) -> Result {
    if !inv_open.is_some_and(|o| o.0) {
        return Ok(());
    }
    let (Some(inventory), Some(hotbar)) = (inventory.as_mut(), hotbar.as_mut()) else {
        return Ok(());
    };

    let gold = egui::Color32::from_rgb(200, 168, 48);
    let dim = egui::Color32::from_rgb(110, 90, 40);

    // Aggregate network storage before the window closure
    let mut net_items: std::collections::HashMap<String, u32> = Default::default();
    for unit in &storage_q {
        for (id, &count) in &unit.items {
            *net_items.entry(id.clone()).or_insert(0) += count;
        }
    }

    let ctx = contexts.ctx_mut()?;
    let mut move_item: Option<String> = None;
    let current_tab = *tab;
    let mut new_tab = current_tab;

    egui::Window::new("INVENTORY")
        .collapsible(false)
        .resizable(true)
        .default_size([480.0, 360.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Tab bar + close hint
            ui.horizontal(|ui| {
                if ui.selectable_label(new_tab == 0, "Player").clicked() {
                    new_tab = 0;
                }
                if ui
                    .selectable_label(new_tab == 1, format!("Network ({})", net_items.len()))
                    .clicked()
                {
                    new_tab = 1;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(dim, "[Tab/Esc]");
                });
            });
            ui.separator();

            // Active slot indicator
            ui.horizontal(|ui| {
                ui.colored_label(dim, "Slot:");
                ui.colored_label(gold, format!("{}", hotbar.selected + 1));
                match hotbar.slots.get(hotbar.selected).and_then(|s| s.as_ref()) {
                    Some(s) => {
                        let name = item_registry
                            .as_ref()
                            .and_then(|r| r.get(&s.item_id))
                            .map_or(s.item_id.as_str(), |d| d.name.as_str());
                        ui.colored_label(gold, format!("→ {} ×{}", name, s.count));
                    }
                    None => {
                        ui.colored_label(dim, "→ (empty)");
                    }
                }
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("inv_scroll")
                .show(ui, |ui| match new_tab {
                    0 => {
                        if inventory.0.is_empty() {
                            ui.colored_label(dim, "(empty)");
                        } else {
                            let mut items: Vec<(&String, u32)> =
                                inventory.0.iter().map(|(k, &c)| (k, c)).collect();
                            items.sort_by_key(|(k, _)| k.as_str());

                            egui::Grid::new("inv_grid")
                                .num_columns(3)
                                .min_col_width(140.0)
                                .spacing([6.0, 6.0])
                                .show(ui, |ui| {
                                    for (idx, (item_id, count)) in items.iter().enumerate() {
                                        let name = item_registry
                                            .as_ref()
                                            .and_then(|r| r.get(item_id))
                                            .map_or(item_id.as_str(), |d| d.name.as_str());
                                        let resp = ui.add(
                                            egui::Button::new(format!("{}\n×{}", name, count))
                                                .min_size(egui::Vec2::new(130.0, 44.0)),
                                        );
                                        if resp.clicked() {
                                            move_item = Some((*item_id).clone());
                                        }
                                        resp.on_hover_text("Move to active hotbar slot");
                                        if (idx + 1) % 3 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        }
                    }
                    _ => {
                        if net_items.is_empty() {
                            ui.colored_label(dim, "(network empty)");
                        } else {
                            let mut sorted: Vec<(&String, u32)> =
                                net_items.iter().map(|(k, &c)| (k, c)).collect();
                            sorted.sort_by_key(|(k, _)| k.as_str());

                            egui::Grid::new("net_grid")
                                .num_columns(2)
                                .min_col_width(200.0)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    ui.colored_label(dim, "Item");
                                    ui.colored_label(dim, "Qty");
                                    ui.end_row();
                                    for (item_id, count) in &sorted {
                                        let name = item_registry
                                            .as_ref()
                                            .and_then(|r| r.get(item_id.as_str()))
                                            .map_or(item_id.as_str(), |d| d.name.as_str());
                                        ui.label(name);
                                        ui.colored_label(gold, format!("{}", count));
                                        ui.end_row();
                                    }
                                });
                        }
                    }
                });
        });

    *tab = new_tab;

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

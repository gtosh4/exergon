use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use xxhash_rust::xxh64::xxh64;

use crate::{
    content::VeinRegistry,
    inventory::{Hotbar, HotbarSlot, Inventory},
    seed::DomainSeeds,
    GameState,
};

const CHUNK_SIZE: f32 = 32.0;
const CELL_SIZE: f32 = 96.0; // 3 × 32

#[derive(Resource, Default, PartialEq, Clone, Copy)]
enum DebugOverlay {
    #[default]
    None,
    Chunks,
    Veins,
    Biomes,
}

impl DebugOverlay {
    fn cycle(self) -> Self {
        match self {
            Self::None => Self::Chunks,
            Self::Chunks => Self::Veins,
            Self::Veins => Self::Biomes,
            Self::Biomes => Self::None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::None => "Off",
            Self::Chunks => "Chunks",
            Self::Veins => "Veins",
            Self::Biomes => "Biomes",
        }
    }
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        let test_mode = std::env::args().any(|a| a == "--test");
        #[cfg(not(debug_assertions))]
        let test_mode = false;

        app.init_resource::<DebugOverlay>()
            .insert_resource(TestMode(test_mode))
            .add_systems(Update, toggle_overlay)
            .add_systems(OnEnter(GameState::Playing), give_test_blocks.run_if(run_once))
            .add_systems(
                Update,
                draw_gizmos.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                EguiPrimaryContextPass,
                draw_ui.run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Resource)]
struct TestMode(bool);

fn give_test_blocks(
    test_mode: Res<TestMode>,
    mut inventory: ResMut<Inventory>,
    mut hotbar: ResMut<Hotbar>,
) {
    if !test_mode.0 {
        return;
    }
    inventory.add("machine_casing", 128);
    inventory.add("smelter_core", 8);
    inventory.add("assembler_core", 8);
    inventory.add("refinery_core", 8);
    inventory.add("gateway_core", 8);
    inventory.add("logistics_cable", 64);
    inventory.add("power_cable", 64);
    inventory.add("storage_crate", 8);
    inventory.add("generator", 4);
    hotbar.slots[0] = Some(HotbarSlot { item_id: "machine_casing".into(), count: 128 });
    hotbar.slots[1] = Some(HotbarSlot { item_id: "smelter_core".into(), count: 8 });
    hotbar.slots[2] = Some(HotbarSlot { item_id: "assembler_core".into(), count: 8 });
    hotbar.slots[3] = Some(HotbarSlot { item_id: "refinery_core".into(), count: 8 });
    hotbar.slots[4] = Some(HotbarSlot { item_id: "gateway_core".into(), count: 8 });
    hotbar.slots[5] = Some(HotbarSlot { item_id: "logistics_cable".into(), count: 64 });
    hotbar.slots[6] = Some(HotbarSlot { item_id: "power_cable".into(), count: 64 });
    hotbar.slots[7] = Some(HotbarSlot { item_id: "storage_crate".into(), count: 8 });
    hotbar.slots[8] = Some(HotbarSlot { item_id: "generator".into(), count: 4 });
    info!("Test mode: gave machine_casing ×128, machine cores ×8, logistics/power cables ×64, storage ×8, generators ×4");
}

fn toggle_overlay(keyboard: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<DebugOverlay>) {
    if keyboard.just_pressed(KeyCode::F9) {
        *overlay = overlay.cycle();
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    overlay: Res<DebugOverlay>,
    camera_q: Query<&Transform, With<Camera3d>>,
    registry: Option<Res<VeinRegistry>>,
    seeds: Option<Res<DomainSeeds>>,
) {
    if *overlay == DebugOverlay::None {
        return;
    }
    let Ok(cam) = camera_q.single() else { return };
    let pos = cam.translation;
    let cell_y = pos.y.div_euclid(CELL_SIZE) as i32;

    match *overlay {
        DebugOverlay::None => {}
        DebugOverlay::Chunks => {
            draw_grid(&mut gizmos, pos, CHUNK_SIZE, 5, |_, _| {
                Color::srgba(0.8, 0.8, 0.8, 0.5)
            });
        }
        DebugOverlay::Veins => {
            let world_seed = seeds.as_deref().map(|s| s.world).unwrap_or(0);
            draw_grid(&mut gizmos, pos, CELL_SIZE, 3, |cx, cz| {
                vein_cell_color(registry.as_deref(), world_seed, cx, cell_y, cz)
            });
        }
        DebugOverlay::Biomes => {
            draw_grid(&mut gizmos, pos, CELL_SIZE, 3, |_, _| {
                biome_color(registry.as_deref(), cell_y)
            });
        }
    }
}

fn draw_grid(
    gizmos: &mut Gizmos,
    cam_pos: Vec3,
    cell_size: f32,
    radius: i32,
    color_fn: impl Fn(i32, i32) -> Color,
) {
    let origin_cx = cam_pos.x.div_euclid(cell_size) as i32;
    let origin_cy = cam_pos.y.div_euclid(cell_size) as i32;
    let origin_cz = cam_pos.z.div_euclid(cell_size) as i32;
    let y_lo = origin_cy as f32 * cell_size;
    let y_hi = y_lo + cell_size;

    for cx in (origin_cx - radius)..=(origin_cx + radius) {
        for cz in (origin_cz - radius)..=(origin_cz + radius) {
            let x0 = cx as f32 * cell_size;
            let z0 = cz as f32 * cell_size;
            let x1 = x0 + cell_size;
            let z1 = z0 + cell_size;
            let color = color_fn(cx, cz);
            box_xz(gizmos, x0, z0, x1, z1, y_lo, y_hi, color);
        }
    }
}

fn vein_cell_color(
    registry: Option<&VeinRegistry>,
    world_seed: u64,
    cx: i32,
    cy: i32,
    cz: i32,
) -> Color {
    let Some(reg) = registry else {
        return Color::srgba(0.4, 0.4, 0.4, 0.35);
    };
    match reg.cell_vein(world_seed, cx, cy, cz) {
        None => Color::srgba(0.2, 0.2, 0.2, 0.3),
        Some(vein) => {
            let h = (xxh64(vein.id.as_bytes(), 0) % 360) as f32;
            Color::hsla(h, 0.7, 0.55, 0.65)
        }
    }
}

fn biome_color(registry: Option<&VeinRegistry>, cell_y: i32) -> Color {
    let Some(reg) = registry else {
        return Color::srgba(0.4, 0.4, 0.4, 0.35);
    };
    match reg.biome_at_cell_y(cell_y) {
        None => Color::srgba(0.2, 0.2, 0.2, 0.3),
        Some(biome) => {
            let h = (xxh64(biome.id.as_bytes(), 0) % 360) as f32;
            Color::hsla(h, 0.6, 0.45, 0.55)
        }
    }
}

fn box_xz(gizmos: &mut Gizmos, x0: f32, z0: f32, x1: f32, z1: f32, y_lo: f32, y_hi: f32, color: Color) {
    // Top ring
    gizmos.line(Vec3::new(x0, y_hi, z0), Vec3::new(x1, y_hi, z0), color);
    gizmos.line(Vec3::new(x1, y_hi, z0), Vec3::new(x1, y_hi, z1), color);
    gizmos.line(Vec3::new(x1, y_hi, z1), Vec3::new(x0, y_hi, z1), color);
    gizmos.line(Vec3::new(x0, y_hi, z1), Vec3::new(x0, y_hi, z0), color);
    // Bottom ring
    gizmos.line(Vec3::new(x0, y_lo, z0), Vec3::new(x1, y_lo, z0), color);
    gizmos.line(Vec3::new(x1, y_lo, z0), Vec3::new(x1, y_lo, z1), color);
    gizmos.line(Vec3::new(x1, y_lo, z1), Vec3::new(x0, y_lo, z1), color);
    gizmos.line(Vec3::new(x0, y_lo, z1), Vec3::new(x0, y_lo, z0), color);
    // Vertical edges
    gizmos.line(Vec3::new(x0, y_lo, z0), Vec3::new(x0, y_hi, z0), color);
    gizmos.line(Vec3::new(x1, y_lo, z0), Vec3::new(x1, y_hi, z0), color);
    gizmos.line(Vec3::new(x1, y_lo, z1), Vec3::new(x1, y_hi, z1), color);
    gizmos.line(Vec3::new(x0, y_lo, z1), Vec3::new(x0, y_hi, z1), color);
}

fn draw_ui(
    mut contexts: EguiContexts,
    overlay: Res<DebugOverlay>,
    camera_q: Query<&Transform, With<Camera3d>>,
    registry: Option<Res<VeinRegistry>>,
    seeds: Option<Res<DomainSeeds>>,
) -> Result {
    if *overlay == DebugOverlay::None {
        return Ok(());
    }
    let Ok(cam) = camera_q.single() else { return Ok(()) };
    let pos = cam.translation;
    let cell_y = pos.y.div_euclid(CELL_SIZE) as i32;
    let ctx = contexts.ctx_mut()?;

    egui::Area::new(egui::Id::new("debug_hud"))
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_black_alpha(160))
                .show(ui, |ui| {
                    ui.set_max_width(ui.ctx().content_rect().width() * 0.3);
                    ui.colored_label(egui::Color32::YELLOW, format!("[F9] {}", overlay.label()));
                    match *overlay {
                        DebugOverlay::Veins => {
                            let world_seed = seeds.as_deref().map(|s| s.world).unwrap_or(0);
                            let cx = pos.x.div_euclid(CELL_SIZE) as i32;
                            let cz = pos.z.div_euclid(CELL_SIZE) as i32;
                            let label = registry
                                .as_deref()
                                .and_then(|r| r.cell_vein(world_seed, cx, cell_y, cz))
                                .map(|v| v.id.as_ref())
                                .unwrap_or("(empty)");
                            ui.colored_label(
                                egui::Color32::WHITE,
                                format!("Cell [{cx},{cell_y},{cz}]: {label}"),
                            );
                        }
                        DebugOverlay::Biomes => {
                            let text = match registry.as_deref().and_then(|r| r.biome_at_cell_y(cell_y)) {
                                None => format!("Layer [y={cell_y}]: (none)"),
                                Some(info) => format!(
                                    "Layer [y={cell_y}]: {} / {}",
                                    info.layer_name, info.id
                                ),
                            };
                            ui.colored_label(egui::Color32::WHITE, text);
                        }
                        _ => {}
                    }
                });
        });

    Ok(())
}

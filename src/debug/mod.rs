use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use xxhash_rust::xxh64::xxh64;

use crate::{
    GameState, content::VeinRegistry, logistics::LogisticsCableSegment, power::PowerCableSegment,
    seed::DomainSeeds,
};

const CHUNK_SIZE: f32 = 32.0;
const CELL_XZ: f32 = 160.0; // 5 × 32 voxels wide
const CELL_Y: f32 = 64.0; // 2 × 32 voxels tall

#[derive(Resource, Default, PartialEq, Clone, Copy, Debug)]
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

#[derive(Resource, Default, PartialEq, Clone, Copy, Debug)]
enum NetworkOverlay {
    #[default]
    None,
    Power,
    Logistics,
}

impl NetworkOverlay {
    fn cycle(self) -> Self {
        match self {
            Self::None => Self::Power,
            Self::Power => Self::Logistics,
            Self::Logistics => Self::None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::None => "Off",
            Self::Power => "Power",
            Self::Logistics => "Logistics",
        }
    }
}

pub struct DebugPlugin;

#[derive(Component)]
struct DebugHudText;

#[derive(Component)]
struct NetworkHudText;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugOverlay>()
            .init_resource::<NetworkOverlay>()
            .add_systems(OnEnter(GameState::Playing), spawn_debug_huds)
            .add_systems(Update, toggle_overlay)
            .add_systems(Update, toggle_network_overlay)
            .add_systems(Update, screenshot_on_f12)
            .add_systems(Update, draw_gizmos.run_if(in_state(GameState::Playing)))
            .add_systems(
                Update,
                draw_network_gizmos.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    update_debug_hud.run_if(in_state(GameState::Playing)),
                    update_network_hud.run_if(in_state(GameState::Playing)),
                ),
            );
    }
}

fn spawn_debug_huds(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Px(8.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.627)),
        Text::new(""),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Visibility::Hidden,
        DespawnOnExit(GameState::Playing),
        DebugHudText,
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            top: Val::Px(8.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.627)),
        Text::new(""),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Visibility::Hidden,
        DespawnOnExit(GameState::Playing),
        NetworkHudText,
    ));
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
    let cell_y = pos.y.div_euclid(CELL_Y) as i32;

    match *overlay {
        DebugOverlay::None => {}
        DebugOverlay::Chunks => {
            draw_grid(&mut gizmos, pos, CHUNK_SIZE, CHUNK_SIZE, 5, |_, _| {
                Color::srgba(0.8, 0.8, 0.8, 0.5)
            });
        }
        DebugOverlay::Veins => {
            let world_seed = seeds.as_deref().map_or(0, |s| s.world);
            draw_grid(&mut gizmos, pos, CELL_XZ, CELL_Y, 3, |cx, cz| {
                vein_cell_color(registry.as_deref(), world_seed, cx, cell_y, cz)
            });
        }
        DebugOverlay::Biomes => {
            draw_grid(&mut gizmos, pos, CELL_XZ, CELL_Y, 3, |_, _| {
                biome_color(registry.as_deref(), cell_y)
            });
        }
    }
}

fn draw_grid(
    gizmos: &mut Gizmos,
    cam_pos: Vec3,
    cell_xz: f32,
    cell_y: f32,
    radius: i32,
    color_fn: impl Fn(i32, i32) -> Color,
) {
    let origin_cx = cam_pos.x.div_euclid(cell_xz) as i32;
    let origin_cy = cam_pos.y.div_euclid(cell_y) as i32;
    let origin_cz = cam_pos.z.div_euclid(cell_xz) as i32;
    let y_lo = origin_cy as f32 * cell_y;
    let y_hi = y_lo + cell_y;

    for cx in (origin_cx - radius)..=(origin_cx + radius) {
        for cz in (origin_cz - radius)..=(origin_cz + radius) {
            let x0 = cx as f32 * cell_xz;
            let z0 = cz as f32 * cell_xz;
            let x1 = x0 + cell_xz;
            let z1 = z0 + cell_xz;
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

fn box_xz(
    gizmos: &mut Gizmos,
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    y_lo: f32,
    y_hi: f32,
    color: Color,
) {
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

fn update_debug_hud(
    overlay: Res<DebugOverlay>,
    camera_q: Query<&Transform, With<Camera3d>>,
    registry: Option<Res<VeinRegistry>>,
    seeds: Option<Res<DomainSeeds>>,
    mut q: Query<(&mut Visibility, &mut Text), With<DebugHudText>>,
) {
    let Ok((mut vis, mut text)) = q.single_mut() else {
        return;
    };

    if *overlay == DebugOverlay::None {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Inherited;

    let Ok(cam) = camera_q.single() else { return };
    let pos = cam.translation;
    let cell_y = pos.y.div_euclid(CELL_Y) as i32;

    let detail = match *overlay {
        DebugOverlay::Veins => {
            let world_seed = seeds.as_deref().map_or(0, |s| s.world);
            let cx = pos.x.div_euclid(CELL_XZ) as i32;
            let cz = pos.z.div_euclid(CELL_XZ) as i32;
            let label = registry
                .as_deref()
                .and_then(|r| r.cell_vein(world_seed, cx, cell_y, cz))
                .map_or("(empty)", |v| v.id.as_ref());
            format!("\nCell [{cx},{cell_y},{cz}]: {label}")
        }
        DebugOverlay::Biomes => {
            let detail = match registry.as_deref().and_then(|r| r.biome_at_cell_y(cell_y)) {
                None => format!("Layer [y={cell_y}]: (none)"),
                Some(info) => format!("Layer [y={cell_y}]: {} / {}", info.layer_name, info.id),
            };
            format!("\n{detail}")
        }
        _ => String::new(),
    };
    **text = format!("[F9] {}{detail}", overlay.label());
}

fn toggle_network_overlay(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<NetworkOverlay>,
) {
    if keyboard.just_pressed(KeyCode::F10) {
        *overlay = overlay.cycle();
    }
}

fn draw_network_gizmos(
    mut gizmos: Gizmos,
    overlay: Res<NetworkOverlay>,
    power_cables: Query<&PowerCableSegment>,
    logistics_cables: Query<&LogisticsCableSegment>,
) {
    match *overlay {
        NetworkOverlay::None => {}
        NetworkOverlay::Power => {
            for seg in &power_cables {
                gizmos.line(seg.from, seg.to, Color::srgb(1.0, 0.85, 0.0));
            }
        }
        NetworkOverlay::Logistics => {
            for seg in &logistics_cables {
                gizmos.line(seg.from, seg.to, Color::srgb(0.1, 0.9, 0.2));
            }
        }
    }
}

fn update_network_hud(
    overlay: Res<NetworkOverlay>,
    power_cables: Query<&PowerCableSegment>,
    logistics_cables: Query<&LogisticsCableSegment>,
    mut q: Query<(&mut Visibility, &mut Text), With<NetworkHudText>>,
) {
    let Ok((mut vis, mut text)) = q.single_mut() else {
        return;
    };

    if *overlay == NetworkOverlay::None {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Inherited;

    let count = match *overlay {
        NetworkOverlay::Power => power_cables.iter().count(),
        NetworkOverlay::Logistics => logistics_cables.iter().count(),
        NetworkOverlay::None => 0,
    };
    **text = format!(
        "[F10] Network: {}\n{count} {} cable segments",
        overlay.label(),
        overlay.label()
    );
}

fn screenshot_on_f12(input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if input.just_pressed(KeyCode::F12) {
        let ts = chrono::Local::now().to_rfc3339();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("screenshot_{ts}.png")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_visits_all_variants() {
        let sequence = [
            DebugOverlay::None,
            DebugOverlay::Chunks,
            DebugOverlay::Veins,
            DebugOverlay::Biomes,
        ];
        for (i, &v) in sequence.iter().enumerate() {
            let next = sequence[(i + 1) % sequence.len()];
            assert_eq!(v.cycle(), next);
        }
    }

    #[test]
    fn label_matches_variant() {
        assert_eq!(DebugOverlay::None.label(), "Off");
        assert_eq!(DebugOverlay::Chunks.label(), "Chunks");
        assert_eq!(DebugOverlay::Veins.label(), "Veins");
        assert_eq!(DebugOverlay::Biomes.label(), "Biomes");
    }

    #[test]
    fn toggle_overlay_f9_cycles_from_none_to_chunks() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<DebugOverlay>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, toggle_overlay);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F9);
        app.update();

        assert_eq!(
            *app.world().resource::<DebugOverlay>(),
            DebugOverlay::Chunks
        );
    }

    #[test]
    fn biome_color_returns_grey_without_registry() {
        let color = biome_color(None, 0);
        assert_eq!(color, Color::srgba(0.4, 0.4, 0.4, 0.35));
    }

    #[test]
    fn vein_cell_color_returns_grey_without_registry() {
        let color = vein_cell_color(None, 0, 0, 0, 0);
        assert_eq!(color, Color::srgba(0.4, 0.4, 0.4, 0.35));
    }

    fn minimal_registry() -> VeinRegistry {
        use crate::content::{BiomeDef, LayerDef, OreSpec, VeinDef};
        let vein = VeinDef {
            id: "iron_vein".to_string(),
            density: 0.5,
            primary: OreSpec {
                name: "Iron".to_string(),
                material: 2,
                weight: 10,
            },
            secondary: OreSpec {
                name: "Stone".to_string(),
                material: 0,
                weight: 5,
            },
            sporadic: None,
        };
        let layer = LayerDef {
            id: "surface".to_string(),
            name: "Surface".to_string(),
            y_cell_range: (-5, 5),
        };
        let biome = BiomeDef {
            id: "plains".to_string(),
            layer: "surface".to_string(),
            vein_pool: vec![("iron_vein".to_string(), 1)],
        };
        VeinRegistry::new(vec![vein], vec![layer], vec![biome])
    }

    #[test]
    fn biome_color_returns_color_with_matching_biome() {
        let reg = minimal_registry();
        let color = biome_color(Some(&reg), 0); // cell_y=0 is in range [-5,5]
        // Not grey (registry found biome)
        assert_ne!(color, Color::srgba(0.4, 0.4, 0.4, 0.35));
        assert_ne!(color, Color::srgba(0.2, 0.2, 0.2, 0.3));
    }

    #[test]
    fn biome_color_returns_dark_grey_when_no_biome_at_y() {
        let reg = minimal_registry();
        let color = biome_color(Some(&reg), 100); // cell_y=100 outside range
        assert_eq!(color, Color::srgba(0.2, 0.2, 0.2, 0.3));
    }

    #[test]
    fn vein_cell_color_returns_dark_grey_when_no_biome() {
        let reg = minimal_registry();
        let color = vein_cell_color(Some(&reg), 0, 0, 100, 0); // cell_y=100 out of range
        assert_eq!(color, Color::srgba(0.2, 0.2, 0.2, 0.3));
    }

    #[test]
    fn vein_cell_color_returns_hue_when_vein_found() {
        let reg = minimal_registry();
        // Scan positions until cell_vein returns Some (roughly 33% chance each)
        let found = (0i32..200).find_map(|x| {
            let c = vein_cell_color(Some(&reg), 0, x, 0, 0);
            if c != Color::srgba(0.2, 0.2, 0.2, 0.3) {
                Some(c)
            } else {
                None
            }
        });
        assert!(
            found.is_some(),
            "should find a colored cell within 200 tries"
        );
    }

    #[test]
    fn network_overlay_cycle_visits_all() {
        let sequence = [
            NetworkOverlay::None,
            NetworkOverlay::Power,
            NetworkOverlay::Logistics,
        ];
        for (i, &v) in sequence.iter().enumerate() {
            assert_eq!(v.cycle(), sequence[(i + 1) % sequence.len()]);
        }
    }

    #[test]
    fn network_overlay_label_matches_variant() {
        assert_eq!(NetworkOverlay::None.label(), "Off");
        assert_eq!(NetworkOverlay::Power.label(), "Power");
        assert_eq!(NetworkOverlay::Logistics.label(), "Logistics");
    }

    #[test]
    fn toggle_network_overlay_f10_cycles_from_none_to_power() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<NetworkOverlay>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, toggle_network_overlay);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F10);
        app.update();

        assert_eq!(
            *app.world().resource::<NetworkOverlay>(),
            NetworkOverlay::Power
        );
    }
}

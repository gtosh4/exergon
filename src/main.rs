use bevy::ecs::message::MessageWriter;
use bevy::log::{BoxedLayer, LogPlugin};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use clap::Parser;
use tracing_subscriber::Layer;

use exergon::inventory::Hotbar;
use exergon::logistics::StorageUnit;
use exergon::machine::{
    MachineBundle, MachineNetworkChanged, MachineRegistry, MachineVisualAssets,
};
use exergon::research::{ResearchPool, TechTreeProgress};
use exergon::{GameState, GameSystems, PlayMode};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[cfg(debug_assertions)]
    /// Start with test items in inventory
    #[arg(short, long)]
    test: bool,
}

fn file_log_layer(_app: &mut App) -> Option<BoxedLayer> {
    let file = std::fs::File::create("game.log").ok()?;
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(file)
        .boxed();
    Some(layer)
}

fn main() {
    let cli = Cli::parse();

    #[cfg(debug_assertions)]
    let log_plugin = LogPlugin {
        level: bevy::log::Level::DEBUG,
        filter: "info,exergon=debug,wgpu_core=warn,wgpu_hal=warn".into(),
        custom_layer: file_log_layer,
        ..default()
    };
    #[cfg(not(debug_assertions))]
    let log_plugin = LogPlugin {
        level: bevy::log::Level::INFO,
        filter: "info,wgpu_core=warn,wgpu_hal=warn".into(),
        custom_layer: file_log_layer,
        ..default()
    };

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Exergon".into(),
                    resolution: WindowResolution::new(1920, 1080),
                    ..default()
                }),
                ..default()
            })
            .set(log_plugin),
    )
    .init_state::<GameState>()
    .add_sub_state::<PlayMode>()
    .configure_sets(
        Update,
        (
            GameSystems::Input,
            GameSystems::Simulation.after(GameSystems::Input),
            GameSystems::Rendering.after(GameSystems::Simulation),
        ),
    )
    .add_plugins((
        exergon::seed::SeedPlugin,
        exergon::content::ContentPlugin,
        exergon::inventory::InventoryPlugin,
        exergon::world::WorldPlugin,
        exergon::debug::DebugPlugin,
        exergon::recipe_graph::RecipeGraphPlugin,
        exergon::tech_tree::TechTreePlugin,
        exergon::machine::MachinePlugin,
        exergon::logistics::LogisticsPlugin,
        exergon::power::PowerPlugin,
        exergon::drone::DronePlugin,
        exergon::research::ResearchPlugin,
        exergon::reactivity::ReactivityPlugin,
        exergon::meta::MetaPlugin,
        exergon::ui::UiPlugin,
    ));

    #[cfg(debug_assertions)]
    if cli.test {
        app.add_systems(
            OnTransition {
                exited: GameState::Loading,
                entered: GameState::Playing,
            },
            (give_test_items, give_test_research).chain(),
        );
    }
    app.run();
}

#[cfg(debug_assertions)]
fn give_test_items(
    mut _hotbar: ResMut<Hotbar>,
    mut commands: Commands,
    registry: Res<MachineRegistry>,
    visuals: Option<Res<MachineVisualAssets>>,
    mut network_changed: MessageWriter<MachineNetworkChanged>,
) {
    // hotbar.slots[0] = Some(HotbarSlot {
    //     item_id: "smelter".into(),
    // });
    // hotbar.slots[1] = Some(HotbarSlot {
    //     item_id: "assembler".into(),
    // });
    // hotbar.slots[2] = Some(HotbarSlot {
    //     item_id: "analysis_station".into(),
    // });
    // hotbar.slots[3] = Some(HotbarSlot {
    //     item_id: "generator".into(),
    // });
    // hotbar.slots[4] = Some(HotbarSlot {
    //     item_id: "storage_crate".into(),
    // });
    // hotbar.slots[5] = Some(HotbarSlot {
    //     item_id: "logistics_cable".into(),
    // });
    // hotbar.slots[6] = Some(HotbarSlot {
    //     item_id: "power_cable".into(),
    // });
    // hotbar.slots[7] = Some(HotbarSlot {
    //     item_id: "platform".into(),
    // });

    let crate_pos = Vec3::new(5.0, 15.0, 5.0);
    if let Some(def) = registry.machine_def("storage_crate") {
        let tier = def.tiers.iter().map(|t| t.tier).max().unwrap_or(1);
        let bundle = MachineBundle::new(crate_pos, def, tier);
        let crate_e = commands.spawn(bundle).id();
        if let Some(ref v) = visuals
            && let Some(scene) = v.scenes.get(&def.id)
        {
            commands.entity(crate_e).insert(SceneRoot(scene.clone()));
        }
        commands.entity(crate_e).insert(StorageUnit {
            items: [
                ("iron_ore".to_owned(), 20u32),
                ("copper_ore".to_owned(), 20u32),
                ("smelter".to_owned(), 4u32),
                ("assembler".to_owned(), 4u32),
                ("analysis_station".to_owned(), 4u32),
                ("generator".to_owned(), 4u32),
                ("storage_crate".to_owned(), 8u32),
                ("logistics_cable".to_owned(), 64u32),
                ("power_cable".to_owned(), 64u32),
                ("platform".to_owned(), 8u32),
            ]
            .into_iter()
            .collect(),
        });
        network_changed.write(MachineNetworkChanged);
    }
    info!("Test mode: spawned starting crate with all items at {crate_pos}");
}

#[cfg(debug_assertions)]
fn give_test_research(mut pool: ResMut<ResearchPool>, mut progress: ResMut<TechTreeProgress>) {
    pool.points += 50.0;
    progress
        .unlocked_recipes
        .insert("basic_analysis".to_string());
    progress.unlocked_recipes.insert("power_basics".to_string());
    progress
        .unlocked_recipes
        .insert("basic_smelting".to_string());
    info!(
        "Test mode: +50 research points, {0:?} unlocked",
        progress.unlocked_recipes
    );
}

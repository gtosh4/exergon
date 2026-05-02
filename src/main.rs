use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;

use crate::inventory::{Hotbar, HotbarSlot, Inventory};

mod content;
mod debug;
mod drone;
mod inventory;
mod logistics;
mod machine;
mod meta;
mod power;
mod reactivity;
mod recipe_graph;
mod research;
mod seed;
mod tech_tree;
mod textures;
mod ui;
mod world;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[cfg(debug_assertions)]
    /// Start with test items in inventory
    #[arg(short, long)]
    test: bool,
}

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    #[default]
    MainMenu,
    Loading,
    Playing,
    Paused,
}

#[derive(SubStates, Default, Clone, Eq, PartialEq, Debug, Hash)]
#[source(GameState = GameState::Playing)]
pub enum PlayMode {
    #[default]
    Exploring,
    Building,
    DronePilot,
    Research,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystems {
    Input,
    Simulation,
    Rendering,
}

fn main() {
    let cli = Cli::parse();

    let atlas_layers = textures::build_block_atlas();
    #[cfg(debug_assertions)]
    let log_plugin = LogPlugin {
        level: bevy::log::Level::DEBUG,
        filter: "info,exergon=debug,wgpu_core=warn,wgpu_hal=warn".into(),
        ..default()
    };
    #[cfg(not(debug_assertions))]
    let log_plugin = LogPlugin {
        level: bevy::log::Level::INFO,
        filter: "info,wgpu_core=warn,wgpu_hal=warn".into(),
        ..default()
    };

    let mut app = App::new();

    app.insert_resource(textures::BlockAtlasLayers(atlas_layers))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Exergon".into(),
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
            seed::SeedPlugin,
            content::ContentPlugin,
            inventory::InventoryPlugin,
            world::WorldPlugin,
            debug::DebugPlugin,
            recipe_graph::RecipeGraphPlugin,
            tech_tree::TechTreePlugin,
            machine::MachinePlugin,
            logistics::LogisticsPlugin,
            power::PowerPlugin,
            drone::DronePlugin,
            research::ResearchPlugin,
            reactivity::ReactivityPlugin,
            meta::MetaPlugin,
            ui::UiPlugin,
        ));

    #[cfg(debug_assertions)]
    if cli.test {
        app.add_systems(
            OnTransition {
                exited: GameState::Loading,
                entered: GameState::Playing,
            },
            give_test_blocks,
        );
    }
    app.run();
}

fn give_test_blocks(mut inventory: ResMut<Inventory>, mut hotbar: ResMut<Hotbar>) {
    inventory.add("machine_casing", 128);
    inventory.add("smelter_core", 8);
    inventory.add("assembler_core", 8);
    inventory.add("refinery_core", 8);
    inventory.add("gateway_core", 8);
    inventory.add("logistics_cable", 64);
    inventory.add("power_cable", 64);
    inventory.add("storage_crate", 8);
    inventory.add("generator", 4);
    inventory.add("energy_io", 16);
    inventory.add("logistics_io", 16);
    hotbar.slots[0] = Some(HotbarSlot {
        item_id: "machine_casing".into(),
        count: 128,
    });
    hotbar.slots[1] = Some(HotbarSlot {
        item_id: "smelter_core".into(),
        count: 8,
    });
    hotbar.slots[2] = Some(HotbarSlot {
        item_id: "assembler_core".into(),
        count: 8,
    });
    hotbar.slots[3] = Some(HotbarSlot {
        item_id: "refinery_core".into(),
        count: 8,
    });
    hotbar.slots[4] = Some(HotbarSlot {
        item_id: "gateway_core".into(),
        count: 8,
    });
    hotbar.slots[5] = Some(HotbarSlot {
        item_id: "logistics_cable".into(),
        count: 64,
    });
    hotbar.slots[6] = Some(HotbarSlot {
        item_id: "power_cable".into(),
        count: 64,
    });
    hotbar.slots[7] = Some(HotbarSlot {
        item_id: "storage_crate".into(),
        count: 8,
    });
    hotbar.slots[8] = Some(HotbarSlot {
        item_id: "generator".into(),
        count: 4,
    });
    info!(
        "Test mode: gave machine_casing ×128, machine cores ×8, logistics/power cables ×64, storage ×8, generators ×4, IO hatches ×16"
    );
}

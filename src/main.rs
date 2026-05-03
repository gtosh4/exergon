// #![deny(clippy::pedantic)]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;

use crate::inventory::{Hotbar, HotbarSlot, Inventory};
use crate::research::{ResearchPool, TechTreeProgress};

mod content;
mod debug;
mod drone;
mod inventory;
mod logistics;
mod machine;
mod meta;
mod network;
mod power;
mod reactivity;
mod recipe_graph;
mod research;
mod seed;
mod tech_tree;
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

    app.add_plugins(
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
            (give_test_items, give_test_research).chain(),
        );
    }
    app.run();
}

fn give_test_items(mut inventory: ResMut<Inventory>, mut hotbar: ResMut<Hotbar>) {
    inventory.add("smelter", 4);
    inventory.add("assembler", 4);
    inventory.add("analysis_station", 4);
    inventory.add("generator", 4);
    inventory.add("storage_crate", 8);
    inventory.add("logistics_cable", 64);
    inventory.add("power_cable", 64);
    hotbar.slots[0] = Some(HotbarSlot {
        item_id: "smelter".into(),
        count: 4,
    });
    hotbar.slots[1] = Some(HotbarSlot {
        item_id: "assembler".into(),
        count: 4,
    });
    hotbar.slots[2] = Some(HotbarSlot {
        item_id: "analysis_station".into(),
        count: 4,
    });
    hotbar.slots[3] = Some(HotbarSlot {
        item_id: "generator".into(),
        count: 4,
    });
    hotbar.slots[4] = Some(HotbarSlot {
        item_id: "storage_crate".into(),
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
        item_id: "platform".into(),
        count: 8,
    });
    inventory.add("platform", 8);
    inventory.add("iron_ore", 20);
    inventory.add("copper_ore", 20);
    info!("Test mode: gave prefab machines, cables, platforms, and starting ores");
}

#[cfg(debug_assertions)]
fn give_test_research(mut pool: ResMut<ResearchPool>, mut progress: ResMut<TechTreeProgress>) {
    // Pre-unlock basic recipes so test mode has a working factory loop.
    // basic_analysis stays locked — player must earn research points first.
    pool.points += 50.0;
    progress
        .unlocked_recipes
        .insert("basic_analysis".to_string());
    info!("Test mode: +50 research points, basic_analysis unlocked");
}

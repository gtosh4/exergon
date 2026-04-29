use bevy::log::LogPlugin;
use bevy::prelude::*;

mod content;
mod debug;
mod drone;
mod logistics;
mod machine;
mod meta;
mod power;
mod reactivity;
mod recipe_graph;
mod research;
mod seed;
mod tech_tree;
mod ui;
mod world;

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
    #[cfg(debug_assertions)]
    let log_plugin = LogPlugin {
        level: bevy::log::Level::DEBUG,
        filter: "debug,wgpu_core=warn,wgpu_hal=warn,exergon=debug,offset_allocator=warn".into(),
        ..default()
    };
    #[cfg(not(debug_assertions))]
    let log_plugin = LogPlugin {
        level: bevy::log::Level::INFO,
        filter: "info,wgpu_core=warn,wgpu_hal=warn".into(),
        ..default()
    };

    App::new()
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
        ))
        .run();
}

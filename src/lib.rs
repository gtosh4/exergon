// #![deny(clippy::pedantic)]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

pub mod content;
pub mod debug;
pub mod drone;
pub mod inventory;
pub mod logistics;
pub mod machine;
pub mod meta;
pub mod network;
pub mod power;
pub mod reactivity;
pub mod recipe_graph;
pub mod research;
pub mod seed;
pub mod tech_tree;
pub mod ui;
pub mod world;

use bevy::prelude::*;

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

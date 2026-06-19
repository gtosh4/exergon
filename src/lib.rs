// #![deny(clippy::pedantic)]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

pub mod aegis;
pub mod content;
pub mod debug;
pub mod drone;
pub mod escape;
pub mod inventory;
pub mod logistics;
pub mod machine;
pub mod meta;
pub mod network;
pub mod planet;
pub mod pod;
pub mod power;
pub mod reactivity;
pub mod recipe_graph;
pub mod research;
pub mod save;
pub mod seed;
pub mod tech_tree;
#[cfg(debug_assertions)]
pub mod telemetry;
pub mod ui;
pub mod world;

use avian3d::prelude::PhysicsLayer;
use bevy::prelude::*;

/// Physics collision layers used for selective collisions (e.g. aegis boundary).
#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Player,
    AegisBoundary,
}

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    #[default]
    MainMenu,
    NewRunWizard,
    LoadRun,
    Loading,
    Playing,
    Escaped,
}

#[derive(SubStates, Default, Clone, Eq, PartialEq, Debug, Hash)]
#[source(GameState = GameState::Playing)]
pub enum PlayMode {
    #[default]
    Exploring,
    Building,
    DronePilot,
    Research,
    Paused,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystems {
    Input,
    Simulation,
    Rendering,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FixedGameSystems {
    PlayerInput,
    Constraint,
}

use bevy::ecs::message::MessageWriter;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;

use exergon::inventory::{Hotbar, HotbarSlot, Inventory};
use exergon::logistics::StorageUnit;
use exergon::machine::{
    MachineBundle, MachineNetworkChanged, MachineRegistry, MachineVisualAssets, spawn_port_markers,
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

fn give_test_items(
    mut inventory: ResMut<Inventory>,
    mut hotbar: ResMut<Hotbar>,
    mut commands: Commands,
    registry: Res<MachineRegistry>,
    visuals: Option<Res<MachineVisualAssets>>,
    mut network_changed: MessageWriter<MachineNetworkChanged>,
) {
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

    let crate_pos = Vec3::new(5.0, 15.0, 5.0);
    if let Some(def) = registry.machine_def("storage_crate") {
        let tier = def.tiers.iter().map(|t| t.tier).max().unwrap_or(1);
        let bundle = MachineBundle::new(crate_pos, def, tier);
        let energy_ports = bundle.machine.energy_ports.clone();
        let logistics_ports = bundle.machine.logistics_ports.clone();
        let crate_e = commands.spawn(bundle).id();
        commands.entity(crate_e).insert(StorageUnit {
            items: [
                ("iron_ore".to_owned(), 20u32),
                ("copper_ore".to_owned(), 20u32),
            ]
            .into_iter()
            .collect(),
        });
        spawn_port_markers(
            &mut commands,
            crate_e,
            &energy_ports,
            &logistics_ports,
            visuals.as_deref(),
        );
        network_changed.write(MachineNetworkChanged);
    }
    info!(
        "Test mode: gave prefab machines, cables, platforms; spawned starting storage crate at {crate_pos}"
    );
}

#[cfg(debug_assertions)]
fn give_test_research(mut pool: ResMut<ResearchPool>, mut progress: ResMut<TechTreeProgress>) {
    pool.points += 50.0;
    progress
        .unlocked_recipes
        .insert("basic_analysis".to_string());
    info!("Test mode: +50 research points, basic_analysis unlocked");
}

//! Integration test for the lander bootstrap loop's payoff step: an analysis
//! station fed raw stone must *produce research* (add to `ResearchPool`), which is
//! what makes the first tech unlock reachable on a fresh run.
//!
//! Coverage split for the full "landing → producing research" chain:
//! - mining stone from a deposit → `logistics::miner` unit tests
//! - research point spend / node unlock → `research` unit tests
//! - **research station converts stone → research points → `ResearchPool`** ← here

use std::collections::HashMap;

use bevy::prelude::*;

use exergon::logistics::{
    LogisticsNetworkMember, LogisticsSimPlugin, NetworkStorageChanged, StorageUnit,
};
use exergon::machine::{
    LogisticsPortOf, Machine, MachineActivity, MachineState, Mirror, Orientation, Rotation,
};
use exergon::recipe_graph::{ConcreteRecipe, ItemStack, RecipeGraph};
use exergon::research::{ResearchPool, TechTreeProgress};
use exergon::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

/// Mirrors `assets/recipes/basic_analysis.ron`: stone in, research points out, run
/// on an analysis station. Processing time shortened so the test completes in a frame.
fn basic_analysis_recipe() -> ConcreteRecipe {
    ConcreteRecipe {
        id: "basic_analysis".to_string(),
        inputs: vec![ItemStack {
            item: "stone".to_string(),
            quantity: 4.0,
        }],
        outputs: vec![ItemStack {
            item: "research_points".to_string(),
            quantity: 10.0,
        }],
        byproducts: vec![],
        machine_type: "analysis_station".to_string(),
        machine_tier: 1,
        processing_time: 1.0,
        energy_cost: 0.0,
        energy_output: 0.0,
        template_id: None,
    }
}

fn make_graph(recipe: ConcreteRecipe) -> RecipeGraph {
    let id = recipe.id.clone();
    RecipeGraph {
        materials: HashMap::new(),
        form_groups: HashMap::new(),
        templates: HashMap::new(),
        items: HashMap::new(),
        recipes: [(id, recipe)].into_iter().collect(),
        terminal: String::new(),
        producers: HashMap::new(),
        consumers: HashMap::new(),
        template_recipes: HashMap::new(),
    }
}

fn make_machine(machine_type: &str, logistics_ports: Vec<Vec3>) -> Machine {
    Machine {
        machine_type: machine_type.to_string(),
        tier: 1,
        orientation: Orientation {
            rotation: Rotation::North,
            mirror: Mirror::Normal,
        },
        energy_ports: vec![],
        logistics_ports,
    }
}

#[test]
fn analysis_station_converts_stone_into_research_points() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<WorldObjectEvent>()
        .add_message::<CableConnectionEvent>()
        .add_plugins(LogisticsSimPlugin)
        .insert_resource(make_graph(basic_analysis_recipe()))
        .insert_resource(ResearchPool { points: 0.0 })
        // basic_analysis auto-unlocks via science_basics on a real run; unlock it here so
        // the recipe is allowed to run through the tech-gated dispatch path.
        .insert_resource({
            let mut progress = TechTreeProgress::default();
            progress
                .unlocked_recipes
                .insert("basic_analysis".to_string());
            progress
        });

    let storage_port = Vec3::new(1.0, 0.0, 0.0);
    let station_port = Vec3::new(5.0, 0.0, 0.0);

    // Storage crate holding mined stone (the miner's output; mining is unit-tested elsewhere).
    let storage_e = app
        .world_mut()
        .spawn((
            make_machine("storage_crate", vec![storage_port]),
            MachineState::Idle,
            Transform::default(),
        ))
        .id();
    let storage_port_e = app
        .world_mut()
        .spawn((
            LogisticsPortOf(storage_e),
            Transform::from_translation(storage_port),
        ))
        .id();
    let station_e = app
        .world_mut()
        .spawn((
            make_machine("analysis_station", vec![station_port]),
            MachineState::Idle,
        ))
        .id();
    let station_port_e = app
        .world_mut()
        .spawn((
            LogisticsPortOf(station_e),
            Transform::from_translation(station_port),
        ))
        .id();
    app.world_mut().write_message(CableConnectionEvent {
        from: storage_port,
        to: station_port,
        item_id: "logistics_cable".to_string(),
        kind: WorldObjectKind::Placed,
        from_port: None,
        to_port: None,
    });
    app.update();

    let net_e = app
        .world()
        .get::<LogisticsNetworkMember>(station_port_e)
        .expect("station port must join the logistics network")
        .0;
    assert_eq!(
        net_e,
        app.world()
            .get::<LogisticsNetworkMember>(storage_port_e)
            .unwrap()
            .0,
        "station and storage must share a network"
    );

    // Seed mined stone and kick the network so the station picks up the recipe.
    app.world_mut()
        .get_mut::<StorageUnit>(storage_e)
        .unwrap()
        .items
        .insert("stone".to_owned(), 4);
    app.world_mut()
        .write_message(NetworkStorageChanged { network: net_e });
    app.update();

    assert_eq!(
        *app.world().get::<MachineState>(station_e).unwrap(),
        MachineState::Running,
        "station should start analysis once stone is available"
    );

    // Complete the recipe.
    app.world_mut()
        .get_mut::<MachineActivity>(station_e)
        .unwrap()
        .progress = 10.0;
    app.update();

    assert_eq!(
        app.world().resource::<ResearchPool>().points,
        10.0,
        "completed analysis must add research points to the pool"
    );
    assert_eq!(
        app.world()
            .get::<StorageUnit>(storage_e)
            .unwrap()
            .items
            .get("stone")
            .copied()
            .unwrap_or(0),
        0,
        "stone consumed by analysis; research_points route to the pool, not storage"
    );
}

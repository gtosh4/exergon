use std::collections::HashMap;

use bevy::prelude::*;

use exergon::logistics::{
    LogisticsNetworkMember, LogisticsSimPlugin, NetworkStorageChanged, StorageUnit,
};
use exergon::machine::{
    Machine, MachineActivity, MachineNetworkChanged, MachineState, Mirror, Orientation, Rotation,
};
use exergon::recipe_graph::{ConcreteRecipe, ItemStack, RecipeGraph};
use exergon::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

fn make_recipe(
    machine_type: &str,
    inputs: &[(&str, f32)],
    outputs: &[(&str, f32)],
) -> ConcreteRecipe {
    ConcreteRecipe {
        id: "test_recipe".to_string(),
        inputs: inputs
            .iter()
            .map(|(item, qty)| ItemStack {
                item: item.to_string(),
                quantity: *qty,
            })
            .collect(),
        outputs: outputs
            .iter()
            .map(|(item, qty)| ItemStack {
                item: item.to_string(),
                quantity: *qty,
            })
            .collect(),
        byproducts: vec![],
        machine_type: machine_type.to_string(),
        machine_tier: 1,
        processing_time: 1.0,
        energy_cost: 0.0,
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

fn placement_app(rg: RecipeGraph) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<WorldObjectEvent>()
        .add_message::<CableConnectionEvent>()
        .add_message::<MachineNetworkChanged>()
        .add_plugins(LogisticsSimPlugin)
        .insert_resource(rg);
    app
}

#[test]
fn storage_placed_before_cable_joins_network_when_cable_placed_adjacent() {
    let rg = make_graph(make_recipe(
        "smelter",
        &[("iron_ore", 1.0)],
        &[("iron_ingot", 1.0)],
    ));

    let storage_port = Vec3::new(1.0, 0.0, 0.0);
    let smelter_port = Vec3::new(5.0, 0.0, 0.0);

    let mut app = placement_app(rg);

    // Frame 1: spawn storage machine — no cables yet
    let storage_e = app
        .world_mut()
        .spawn((
            make_machine("storage_crate", vec![storage_port]),
            MachineState::Idle,
            Transform::default(),
        ))
        .id();
    app.update();

    assert!(
        app.world()
            .get::<LogisticsNetworkMember>(storage_e)
            .is_none(),
        "no cable yet: storage should not be in any network"
    );

    // Frame 2: place cable adjacent to storage port + smelter with matching port
    app.world_mut().write_message(CableConnectionEvent {
        from: storage_port,
        to: smelter_port,
        item_id: "logistics_cable".to_string(),
        kind: WorldObjectKind::Placed,
    });
    let smelter_e = app
        .world_mut()
        .spawn((
            make_machine("smelter", vec![smelter_port]),
            MachineState::Idle,
        ))
        .id();
    app.world_mut().write_message(MachineNetworkChanged);
    app.update();

    assert!(
        app.world()
            .get::<LogisticsNetworkMember>(storage_e)
            .is_some(),
        "storage placed before cable must join network when cable placed adjacent"
    );
    assert!(
        app.world()
            .get::<LogisticsNetworkMember>(smelter_e)
            .is_some(),
        "smelter must also join same network"
    );
    assert_eq!(
        app.world()
            .get::<LogisticsNetworkMember>(storage_e)
            .unwrap()
            .0,
        app.world()
            .get::<LogisticsNetworkMember>(smelter_e)
            .unwrap()
            .0,
        "smelter and storage must be on the same network"
    );
}

#[test]
fn cable_endpoint_near_port_snaps_to_connect_machine() {
    // Game bug: player clicks near but not exactly on a port marker.
    // key(4.4)=4, key(5.0)=5 — no key match, machine never joins network.
    // Fix: port_near_point with snap radius instead of exact key match.
    let rg = make_graph(make_recipe(
        "smelter",
        &[("iron_ore", 1.0)],
        &[("iron_ingot", 1.0)],
    ));
    let smelter_port = Vec3::new(5.0, 0.0, 0.0);

    let mut app = placement_app(rg);

    // 0.6 units short of the port — rounds to key 4, port is key 5
    let cable_to = Vec3::new(4.4, 0.0, 0.0);
    app.world_mut().write_message(CableConnectionEvent {
        from: Vec3::new(1.0, 0.0, 0.0),
        to: cable_to,
        item_id: "logistics_cable".to_string(),
        kind: WorldObjectKind::Placed,
    });
    // Spawn WITH Transform so cable_placed_system sees it.
    // No MachineNetworkChanged — this test isolates cable_placed_system.
    let smelter_e = app
        .world_mut()
        .spawn((
            make_machine("smelter", vec![smelter_port]),
            MachineState::Idle,
            Transform::default(),
        ))
        .id();

    app.update();

    assert!(
        app.world()
            .get::<LogisticsNetworkMember>(smelter_e)
            .is_some(),
        "machine port within snap radius of cable endpoint must join network"
    );
}

#[test]
fn smelter_with_ore_storage_runs_smelt_recipe_and_outputs_ingot() {
    let rg = make_graph(make_recipe(
        "smelter",
        &[("iron_ore", 1.0)],
        &[("iron_ingot", 1.0)],
    ));

    let storage_port = Vec3::new(1.0, 0.0, 0.0);
    let smelter_port = Vec3::new(5.0, 0.0, 0.0);

    let mut app = placement_app(rg);

    // Place cable, spawn storage and smelter machines with matching ports
    app.world_mut().write_message(CableConnectionEvent {
        from: storage_port,
        to: smelter_port,
        item_id: "logistics_cable".to_string(),
        kind: WorldObjectKind::Placed,
    });
    let storage_e = app
        .world_mut()
        .spawn((
            make_machine("storage_crate", vec![storage_port]),
            MachineState::Idle,
            Transform::default(),
        ))
        .id();
    let smelter_e = app
        .world_mut()
        .spawn((
            make_machine("smelter", vec![smelter_port]),
            MachineState::Idle,
        ))
        .id();
    app.world_mut().write_message(MachineNetworkChanged);

    // Network joins: cable_placed_system assigns smelter and storage
    app.update();

    assert!(
        app.world()
            .get::<LogisticsNetworkMember>(smelter_e)
            .is_some(),
        "smelter should have joined the logistics network"
    );
    assert!(
        app.world().get::<StorageUnit>(storage_e).is_some(),
        "storage machine should have StorageUnit component"
    );
    assert!(
        app.world()
            .get::<LogisticsNetworkMember>(storage_e)
            .is_some(),
        "storage should have joined the logistics network"
    );
    assert_eq!(
        app.world()
            .get::<LogisticsNetworkMember>(smelter_e)
            .unwrap()
            .0,
        app.world()
            .get::<LogisticsNetworkMember>(storage_e)
            .unwrap()
            .0,
        "smelter and storage should be on the same network"
    );

    // Seed ore and trigger recipe start
    app.world_mut()
        .get_mut::<StorageUnit>(storage_e)
        .unwrap()
        .items
        .insert("iron_ore".to_owned(), 5);
    let net_e = app
        .world()
        .get::<LogisticsNetworkMember>(smelter_e)
        .unwrap()
        .0;
    app.world_mut()
        .write_message(NetworkStorageChanged { network: net_e });
    app.update();

    assert_eq!(
        *app.world().get::<MachineState>(smelter_e).unwrap(),
        MachineState::Running,
        "smelter should start after ore available"
    );
    assert_eq!(
        app.world()
            .get::<StorageUnit>(storage_e)
            .unwrap()
            .items
            .get("iron_ore")
            .copied()
            .unwrap_or(0),
        4,
        "one iron_ore consumed on recipe start"
    );

    // Advance progress past processing_time to complete the recipe
    app.world_mut()
        .get_mut::<MachineActivity>(smelter_e)
        .unwrap()
        .progress = 10.0;
    app.update();

    assert_eq!(
        *app.world().get::<MachineState>(smelter_e).unwrap(),
        MachineState::Idle,
        "smelter should be idle after recipe completes"
    );
    assert_eq!(
        app.world()
            .get::<StorageUnit>(storage_e)
            .unwrap()
            .items
            .get("iron_ingot")
            .copied()
            .unwrap_or(0),
        1,
        "one iron_ingot produced"
    );
}

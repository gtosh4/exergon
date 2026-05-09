use std::collections::HashMap;

use bevy::prelude::*;

use exergon::logistics::{
    LogisticsNetworkMember, LogisticsSimPlugin, NetworkStorageChanged, StorageUnit,
};
use exergon::machine::{
    LogisticsPortOf, Machine, MachineActivity, MachineState, Mirror, Orientation, Rotation,
};
use exergon::recipe_graph::{ConcreteRecipe, ItemStack, RecipeGraph};
use exergon::world::{CableConnectionEvent, WorldObjectEvent, WorldObjectKind};

fn make_recipe_tier2(
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
        machine_tier: 2,
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

fn make_assembler_tier2(logistics_ports: Vec<Vec3>) -> Machine {
    Machine {
        machine_type: "assembler".to_string(),
        tier: 2,
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
        .add_plugins(LogisticsSimPlugin)
        .insert_resource(rg);
    app
}

#[test]
fn assembler_tier2_runs_tier2_recipe_and_outputs_gateway_key() {
    let rg = make_graph(make_recipe_tier2(
        "assembler",
        &[("resonite_circuit", 1.0), ("power_cell", 2.0)],
        &[("gateway_key", 1.0)],
    ));

    let storage_port = Vec3::new(1.0, 0.0, 0.0);
    let assembler_port = Vec3::new(5.0, 0.0, 0.0);

    let mut app = placement_app(rg);

    let storage_e = app
        .world_mut()
        .spawn((
            Machine {
                machine_type: "storage_crate".to_string(),
                tier: 1,
                orientation: Orientation {
                    rotation: Rotation::North,
                    mirror: Mirror::Normal,
                },
                energy_ports: vec![],
                logistics_ports: vec![storage_port],
            },
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

    let assembler_e = app
        .world_mut()
        .spawn((
            make_assembler_tier2(vec![assembler_port]),
            MachineState::Idle,
            Transform::default(),
        ))
        .id();
    let assembler_port_e = app
        .world_mut()
        .spawn((
            LogisticsPortOf(assembler_e),
            Transform::from_translation(assembler_port),
        ))
        .id();

    app.world_mut().write_message(CableConnectionEvent {
        from: storage_port,
        to: assembler_port,
        item_id: "logistics_cable".to_string(),
        kind: WorldObjectKind::Placed,
        from_port: None,
        to_port: None,
    });
    app.update();

    assert!(
        app.world()
            .get::<LogisticsNetworkMember>(assembler_port_e)
            .is_some(),
        "assembler port should have joined the logistics network"
    );
    assert!(
        app.world()
            .get::<LogisticsNetworkMember>(storage_port_e)
            .is_some(),
        "storage port should have joined the logistics network"
    );

    // Seed inputs and trigger recipe
    {
        let mut storage = app.world_mut().get_mut::<StorageUnit>(storage_e).unwrap();
        storage.items.insert("resonite_circuit".to_owned(), 1);
        storage.items.insert("power_cell".to_owned(), 2);
    }
    let net_e = app
        .world()
        .get::<LogisticsNetworkMember>(assembler_port_e)
        .unwrap()
        .0;
    app.world_mut()
        .write_message(NetworkStorageChanged { network: net_e });
    app.update();

    assert_eq!(
        *app.world().get::<MachineState>(assembler_e).unwrap(),
        MachineState::Running,
        "assembler tier 2 should start the tier 2 recipe"
    );

    // Advance progress past processing_time
    app.world_mut()
        .get_mut::<MachineActivity>(assembler_e)
        .unwrap()
        .progress = 10.0;
    app.update();

    assert_eq!(
        *app.world().get::<MachineState>(assembler_e).unwrap(),
        MachineState::Idle,
        "assembler should be idle after recipe completes"
    );
    assert_eq!(
        app.world()
            .get::<StorageUnit>(storage_e)
            .unwrap()
            .items
            .get("gateway_key")
            .copied()
            .unwrap_or(0),
        1,
        "one gateway_key produced"
    );
}

#[test]
fn tier1_assembler_cannot_run_tier2_recipe() {
    let rg = make_graph(make_recipe_tier2(
        "assembler",
        &[("resonite_circuit", 1.0)],
        &[("gateway_key", 1.0)],
    ));

    let storage_port = Vec3::new(1.0, 0.0, 0.0);
    let assembler_port = Vec3::new(5.0, 0.0, 0.0);

    let mut app = placement_app(rg);

    let storage_e = app
        .world_mut()
        .spawn((
            Machine {
                machine_type: "storage_crate".to_string(),
                tier: 1,
                orientation: Orientation {
                    rotation: Rotation::North,
                    mirror: Mirror::Normal,
                },
                energy_ports: vec![],
                logistics_ports: vec![storage_port],
            },
            MachineState::Idle,
            Transform::default(),
        ))
        .id();
    app.world_mut().spawn((
        LogisticsPortOf(storage_e),
        Transform::from_translation(storage_port),
    ));

    // Tier 1 assembler — should NOT be able to run a tier 2 recipe
    let assembler_e = app
        .world_mut()
        .spawn((
            Machine {
                machine_type: "assembler".to_string(),
                tier: 1,
                orientation: Orientation {
                    rotation: Rotation::North,
                    mirror: Mirror::Normal,
                },
                energy_ports: vec![],
                logistics_ports: vec![assembler_port],
            },
            MachineState::Idle,
            Transform::default(),
        ))
        .id();
    let assembler_port_e = app
        .world_mut()
        .spawn((
            LogisticsPortOf(assembler_e),
            Transform::from_translation(assembler_port),
        ))
        .id();

    app.world_mut().write_message(CableConnectionEvent {
        from: storage_port,
        to: assembler_port,
        item_id: "logistics_cable".to_string(),
        kind: WorldObjectKind::Placed,
        from_port: None,
        to_port: None,
    });
    app.update();

    {
        let mut storage = app.world_mut().get_mut::<StorageUnit>(storage_e).unwrap();
        storage.items.insert("resonite_circuit".to_owned(), 1);
    }
    let net_e = app
        .world()
        .get::<LogisticsNetworkMember>(assembler_port_e)
        .unwrap()
        .0;
    app.world_mut()
        .write_message(NetworkStorageChanged { network: net_e });
    app.update();

    assert_eq!(
        *app.world().get::<MachineState>(assembler_e).unwrap(),
        MachineState::Idle,
        "tier 1 assembler must not run a tier 2 recipe"
    );
}

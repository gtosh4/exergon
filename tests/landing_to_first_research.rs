//! End-to-end integration test for the lander bootstrap loop, exercised through the
//! real placement + logistics + recipe + research systems (not hand-built entities):
//!
//!   place machines (miner, storage, analysis station) → wire them with cables →
//!   mine stone from a deposit → analyse stone into research points → unlock the
//!   first research node with those points.
//!
//! Machines are "placed" by emitting the same `WorldObjectEvent::Placed` the input
//! layer produces, so `place_machine_system` (port spawning, miner→deposit latching)
//! runs for real. GLTF-derived port layouts are stubbed (headless has no renderer),
//! and storage provisioning is injected — both are data, not the logic under test.

use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy::world_serialization::WorldAsset;

use exergon::GameState;
use exergon::logistics::{
    LogisticsNetworkMember, LogisticsSimPlugin, NetworkStorageChanged, StorageUnit,
};
use exergon::machine::{
    Machine, MachineActivity, MachinePlugin, MachinePortLayout, MachinePortLayouts, MachineState,
    MinerMachine,
};
use exergon::recipe_graph::RecipeGraphPlugin;
use exergon::research::{ResearchPlugin, ResearchPool, TechTreeProgress, UnlockNodeRequest};
use exergon::tech_tree::TechTreePlugin;
use exergon::world::{CableConnectionEvent, OreDeposit, WorldObjectEvent, WorldObjectKind};

const PORT_OFFSET: Vec3 = Vec3::new(1.0, 0.0, 0.0);

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        StatesPlugin,
        ScenePlugin,
    ));
    // Asset stores the machine visual/port-layout startup systems expect (no renderer here).
    app.init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Gltf>()
        .init_asset::<GltfMesh>()
        .init_asset::<GltfNode>()
        .init_asset::<WorldAsset>();
    app.add_message::<WorldObjectEvent>()
        .add_message::<CableConnectionEvent>()
        .init_state::<GameState>()
        .add_plugins((
            RecipeGraphPlugin,
            TechTreePlugin,
            MachinePlugin,
            LogisticsSimPlugin,
            ResearchPlugin,
        ));
    app
}

fn place(app: &mut App, item_id: &str, pos: Vec3) {
    app.world_mut().write_message(WorldObjectEvent {
        transform: Transform::from_translation(pos),
        item_id: item_id.to_string(),
        kind: WorldObjectKind::Placed,
    });
}

fn connect(app: &mut App, from: Vec3, to: Vec3) {
    app.world_mut().write_message(CableConnectionEvent {
        from,
        to,
        item_id: "logistics_cable".to_string(),
        kind: WorldObjectKind::Placed,
        from_port: None,
        to_port: None,
    });
}

fn machine_entity(app: &mut App, machine_type: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Machine)>();
    q.iter(app.world())
        .find(|(_, m)| m.machine_type == machine_type)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("no placed machine of type {machine_type}"))
}

fn research_points(app: &App) -> f32 {
    app.world().resource::<ResearchPool>().points
}

#[test]
fn land_place_wire_mine_and_complete_first_research() {
    let mut app = build_app();

    // Land: enter Playing so placement (gated on GameState::Playing) is live.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    // basic_analysis auto-unlocks via science_basics on a real run; ensure it here too.
    app.world_mut()
        .resource_mut::<TechTreeProgress>()
        .unlocked_recipes
        .insert("basic_analysis".to_string());

    // Stub port layouts for the machines we place: one logistics port each at PORT_OFFSET.
    {
        let mut layouts = app.world_mut().resource_mut::<MachinePortLayouts>();
        for id in ["storage_crate", "miner", "analysis_station"] {
            layouts.by_machine.insert(
                id.to_string(),
                MachinePortLayout {
                    energy: vec![],
                    logistics: vec![PORT_OFFSET],
                },
            );
        }
    }

    // A stone-bearing deposit for the miner to latch onto (public OreDeposit).
    let miner_pos = Vec3::new(10.0, 0.0, 0.0);
    app.world_mut().spawn((
        OreDeposit {
            chunk_pos: IVec2::ZERO,
            ores: vec![("stone".to_string(), 1.0)],
            total_extracted: 0.0,
            depletion_seed: 0,
        },
        Transform::from_translation(miner_pos),
    ));

    // Place the three machines from the starting kit via the real placement path.
    let storage_pos = Vec3::ZERO;
    let station_pos = Vec3::new(20.0, 0.0, 0.0);
    place(&mut app, "storage_crate", storage_pos);
    place(&mut app, "miner", miner_pos);
    place(&mut app, "analysis_station", station_pos);
    app.update();

    // The placed miner must have latched onto the deposit.
    let miner_e = machine_entity(&mut app, "miner");
    assert!(
        app.world().get::<MinerMachine>(miner_e).is_some(),
        "placed miner should latch onto the nearby deposit"
    );

    // Storage machines don't carry a StorageUnit from placement; provision it.
    let storage_e = machine_entity(&mut app, "storage_crate");
    app.world_mut().entity_mut(storage_e).insert(StorageUnit {
        items: Default::default(),
    });

    // Wire everything onto one logistics network: storage↔miner and storage↔station.
    connect(&mut app, storage_pos + PORT_OFFSET, miner_pos + PORT_OFFSET);
    connect(
        &mut app,
        storage_pos + PORT_OFFSET,
        station_pos + PORT_OFFSET,
    );
    app.update();

    // All three machines' ports must share a network.
    let net_of = |app: &mut App, machine: Entity| -> Entity {
        let ports: Vec<Entity> = {
            let mut q = app
                .world_mut()
                .query::<(Entity, &exergon::machine::LogisticsPortOf)>();
            q.iter(app.world())
                .filter(|(_, p)| p.0 == machine)
                .map(|(e, _)| e)
                .collect()
        };
        ports
            .iter()
            .find_map(|&p| app.world().get::<LogisticsNetworkMember>(p).map(|m| m.0))
            .expect("machine port should have joined a network")
    };
    let station_e = machine_entity(&mut app, "analysis_station");
    let net = net_of(&mut app, storage_e);
    assert_eq!(
        net,
        net_of(&mut app, miner_e),
        "miner shares storage network"
    );
    assert_eq!(
        net,
        net_of(&mut app, station_e),
        "station shares storage network"
    );

    // Drive the bootstrap loop: mine stone, run analysis, repeat until enough research
    // to afford the first ResearchSpend node (ore_extraction, cost 30).
    let stone_in_network = |app: &mut App| -> u32 {
        let mut q = app.world_mut().query::<&StorageUnit>();
        q.iter(app.world())
            .map(|s| s.items.get("stone").copied().unwrap_or(0))
            .sum()
    };
    // Test time does not advance under MinimalPlugins, so recipes never progress on their
    // own — force-complete each analysis by driving its progress past the processing time.
    let mut started_at_least_one = false;
    let mut guard = 0;
    while research_points(&app) < 30.0 {
        guard += 1;
        assert!(guard < 100, "bootstrap loop failed to accumulate research");

        // Mine one unit of ore this frame.
        app.world_mut()
            .get_mut::<MinerMachine>(miner_e)
            .unwrap()
            .accumulator = 1.0;
        app.update();

        let state = app.world().get::<MachineState>(station_e).copied();
        match state {
            Some(MachineState::Running) => {
                // Analysis in progress — finish it (routes research_points → ResearchPool).
                app.world_mut()
                    .get_mut::<MachineActivity>(station_e)
                    .unwrap()
                    .progress = 1_000.0;
                app.update();
                started_at_least_one = true;
            }
            _ if stone_in_network(&mut app) >= 4 => {
                // Enough stone: kick the network so the station picks up the recipe.
                app.world_mut()
                    .write_message(NetworkStorageChanged { network: net });
                app.update();
            }
            _ => {}
        }
    }
    assert!(
        started_at_least_one,
        "station must actually run the analysis recipe (not just be granted points)"
    );

    // Complete the first research: spend accumulated points to unlock ore_extraction.
    let points_before = research_points(&app);
    app.world_mut()
        .write_message(UnlockNodeRequest("ore_extraction".into()));
    app.update();

    let progress = app.world().resource::<TechTreeProgress>();
    assert!(
        progress.unlocked_nodes.contains("ore_extraction"),
        "first research node should be unlocked after spending research points"
    );
    assert!(
        research_points(&app) < points_before,
        "unlocking a ResearchSpend node must deduct research points"
    );
}

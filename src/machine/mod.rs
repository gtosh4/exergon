use std::collections::{HashMap, HashSet};

use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use serde::Deserialize;

use crate::content::load_ron_dir;
use crate::inventory::ItemRegistry;
use crate::world::{generation::WorldConfig, BlockChangeKind, BlockChangedEvent};

pub struct MachinePlugin;

impl Plugin for MachinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MachineBlockMap>()
            .init_resource::<UnformedMachineBlockMap>()
            .add_systems(Startup, load_machines)
            .add_systems(
                Update,
                scan_machines
                    .in_set(crate::GameSystems::Simulation)
                    .run_if(resource_exists::<MachineRegistry>)
                    .run_if(in_state(crate::GameState::Playing)),
            );
    }
}

// ---------------------------------------------------------------------------
// Data types (deserialised from RON)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug)]
pub enum CellMatcher {
    BlockMatcher(String),
}

impl CellMatcher {
    fn matches_id(&self, id: &str) -> bool {
        match self {
            CellMatcher::BlockMatcher(bid) => bid == id,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct MachineTierDef {
    pub tier: u8,
    /// pattern[y][z] = row string; each char is a placeholder key into pattern_elements.
    /// Chars absent from pattern_elements must be air.
    pub pattern: Vec<Vec<String>>,
    pub pattern_elements: HashMap<String, CellMatcher>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MachineDef {
    pub id: String,
    pub key_block: String,
    pub tiers: Vec<MachineTierDef>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct MachineRegistry {
    machines: Vec<MachineDef>,
    key_blocks: HashSet<String>,
}

impl MachineRegistry {
    fn new(machines: Vec<MachineDef>) -> Self {
        let key_blocks = machines.iter().map(|m| m.key_block.clone()).collect();
        Self { machines, key_blocks }
    }

    pub fn is_key_block(&self, item_id: &str) -> bool {
        self.key_blocks.contains(item_id)
    }

    fn machines_with_key(&self, item_id: &str) -> impl Iterator<Item = &MachineDef> {
        self.machines.iter().filter(move |m| m.key_block == item_id)
    }

    fn machine_def(&self, id: &str) -> Option<&MachineDef> {
        self.machines.iter().find(|m| m.id == id)
    }
}

// ---------------------------------------------------------------------------
// ECS components & resources
// ---------------------------------------------------------------------------

#[derive(Component, Debug)]
pub struct Machine {
    pub machine_type: String,
    pub tier: u8,
    pub orientation: Orientation,
    pub key_pos: IVec3,
}

#[derive(Component)]
pub struct MachineParts {
    pub positions: Vec<IVec3>,
}

/// Voxel position → formed machine entity.
#[derive(Resource, Default)]
pub struct MachineBlockMap(pub HashMap<IVec3, Entity>);

/// Voxel position → unformed (structure partially intact) machine entity.
#[derive(Resource, Default)]
pub struct UnformedMachineBlockMap(pub HashMap<IVec3, Entity>);

/// Marker: machine entity exists but structure is incomplete.
#[derive(Component)]
pub struct MachineUnformed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirror {
    Normal,
    Mirrored,
}

#[derive(Clone, Copy, Debug)]
pub struct Orientation {
    pub rotation: Rotation,
    pub mirror: Mirror,
}

impl Orientation {
    fn all() -> [Orientation; 8] {
        use Mirror::*;
        use Rotation::*;
        [
            Orientation { rotation: North, mirror: Normal },
            Orientation { rotation: East, mirror: Normal },
            Orientation { rotation: South, mirror: Normal },
            Orientation { rotation: West, mirror: Normal },
            Orientation { rotation: North, mirror: Mirrored },
            Orientation { rotation: East, mirror: Mirrored },
            Orientation { rotation: South, mirror: Mirrored },
            Orientation { rotation: West, mirror: Mirrored },
        ]
    }

    /// Rotate a canonical-space delta into world-space, applying mirror then rotation.
    pub fn transform(&self, delta: IVec3) -> IVec3 {
        let dx = if self.mirror == Mirror::Mirrored { -delta.x } else { delta.x };
        let dy = delta.y;
        let dz = delta.z;
        let (rx, rz) = match self.rotation {
            Rotation::North => (dx, dz),
            Rotation::East => (dz, -dx),
            Rotation::South => (-dx, -dz),
            Rotation::West => (-dz, dx),
        };
        IVec3::new(rx, dy, rz)
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn load_machines(mut commands: Commands) {
    let machines = load_ron_dir::<MachineDef>("assets/machines", "machine");
    info!("Loaded {} machine definitions", machines.len());
    commands.insert_resource(MachineRegistry::new(machines));
}

// ---------------------------------------------------------------------------
// Scanner
// ---------------------------------------------------------------------------

fn scan_machines(
    mut commands: Commands,
    mut events: MessageReader<BlockChangedEvent>,
    mut block_map: ResMut<MachineBlockMap>,
    mut unformed_map: ResMut<UnformedMachineBlockMap>,
    registry: Res<MachineRegistry>,
    item_registry: Option<Res<ItemRegistry>>,
    voxel_world: VoxelWorld<WorldConfig>,
    machine_q: Query<(&Machine, &MachineParts)>,
) {
    let Some(item_registry) = item_registry else {
        return;
    };

    let mut despawned: Vec<Entity> = Vec::new();

    for ev in events.read() {
        let pos = ev.pos;

        // --- UNFORM on remove or replace ---
        if matches!(ev.kind, BlockChangeKind::Removed { .. } | BlockChangeKind::Replaced { .. }) {
            if let Some(&entity) = block_map.0.get(&pos) {
                if !despawned.contains(&entity) {
                    if let Ok((machine, parts)) = machine_q.get(entity) {
                        let still_valid = registry
                            .machine_def(&machine.machine_type)
                            .is_some_and(|def| {
                                try_form_machine(machine.key_pos, def, &item_registry, &voxel_world)
                                    .is_some()
                            });

                        if !still_valid {
                            let solid_count = parts
                                .positions
                                .iter()
                                .filter(|&&p| matches!(voxel_world.get_voxel(p), WorldVoxel::Solid(_)))
                                .count();
                            let total = parts.positions.len();
                            let positions = parts.positions.clone();
                            let machine_type = machine.machine_type.clone();
                            let key_pos = machine.key_pos;

                            for &p in &positions {
                                block_map.0.remove(&p);
                            }

                            if solid_count * 10 < total {
                                despawned.push(entity);
                                commands.entity(entity).despawn();
                                info!("Machine '{}' at {:?} destroyed", machine_type, key_pos);
                            } else {
                                for &p in &positions {
                                    unformed_map.0.insert(p, entity);
                                }
                                commands.entity(entity).insert(MachineUnformed);
                                info!("Machine '{}' at {:?} unformed", machine_type, key_pos);
                            }
                        }
                    }
                }
            }

            // Check if an unformed machine at pos should now be destroyed
            if let Some(&entity) = unformed_map.0.get(&pos) {
                if !despawned.contains(&entity) {
                    if let Ok((machine, parts)) = machine_q.get(entity) {
                        let solid_count = parts
                            .positions
                            .iter()
                            .filter(|&&p| matches!(voxel_world.get_voxel(p), WorldVoxel::Solid(_)))
                            .count();
                        let total = parts.positions.len();
                        if solid_count * 10 < total {
                            let positions = parts.positions.clone();
                            let machine_type = machine.machine_type.clone();
                            let key_pos = machine.key_pos;
                            for &p in &positions {
                                unformed_map.0.remove(&p);
                            }
                            despawned.push(entity);
                            commands.entity(entity).despawn();
                            info!("Machine '{}' at {:?} destroyed", machine_type, key_pos);
                        }
                    }
                }
            }
        }

        // --- FORM on place or replace ---
        if !matches!(ev.kind, BlockChangeKind::Removed { .. }) {
            let new_voxel_id = match ev.kind {
                BlockChangeKind::Placed { voxel_id } => voxel_id,
                BlockChangeKind::Replaced { new_voxel_id, .. } => new_voxel_id,
                BlockChangeKind::Removed { .. } => unreachable!(),
            };

            // If placed block is a key block, try forming machines
            if let Some(item) = item_registry.item_for_voxel(new_voxel_id) {
                if registry.is_key_block(&item.id) {
                    let item_id = item.id.clone();
                    'key_form: for machine_def in registry.machines_with_key(&item_id) {
                        if let Some((tier, orientation, positions)) =
                            try_form_machine(pos, machine_def, &item_registry, &voxel_world)
                        {
                            if positions.iter().any(|p| block_map.0.contains_key(p)) {
                                continue;
                            }
                            // Remove overlapping unformed machines
                            let mut to_remove: Vec<Entity> = Vec::new();
                            for &p in &positions {
                                if let Some(&uf) = unformed_map.0.get(&p) {
                                    if !to_remove.contains(&uf) && !despawned.contains(&uf) {
                                        to_remove.push(uf);
                                    }
                                }
                            }
                            for uf in to_remove {
                                if let Ok((_, parts)) = machine_q.get(uf) {
                                    for &p in &parts.positions {
                                        unformed_map.0.remove(&p);
                                    }
                                }
                                despawned.push(uf);
                                commands.entity(uf).despawn();
                            }

                            let entity = commands
                                .spawn((
                                    Machine {
                                        machine_type: machine_def.id.clone(),
                                        tier,
                                        orientation,
                                        key_pos: pos,
                                    },
                                    MachineParts { positions: positions.clone() },
                                ))
                                .id();
                            for &p in &positions {
                                block_map.0.insert(p, entity);
                            }
                            info!(
                                "Machine '{}' tier {} formed at {:?} ({:?}/{:?})",
                                machine_def.id, tier, pos, orientation.rotation, orientation.mirror
                            );
                            break 'key_form;
                        }
                    }
                }
            }

            // Check if an unformed machine at pos can now re-form
            if let Some(&entity) = unformed_map.0.get(&pos) {
                if !despawned.contains(&entity) {
                    if let Ok((machine, parts)) = machine_q.get(entity) {
                        let machine_type = machine.machine_type.clone();
                        let key_pos = machine.key_pos;
                        let old_positions = parts.positions.clone();

                        if let Some(def) = registry.machine_def(&machine_type) {
                            if let Some((tier, orientation, new_positions)) =
                                try_form_machine(key_pos, def, &item_registry, &voxel_world)
                            {
                                if !new_positions.iter().any(|p| block_map.0.contains_key(p)) {
                                    for &p in &old_positions {
                                        unformed_map.0.remove(&p);
                                    }
                                    for &p in &new_positions {
                                        block_map.0.insert(p, entity);
                                    }
                                    commands
                                        .entity(entity)
                                        .remove::<MachineUnformed>()
                                        .insert(MachineParts { positions: new_positions });
                                    info!(
                                        "Machine '{}' tier {} re-formed at {:?} ({:?}/{:?})",
                                        machine_type, tier, key_pos, orientation.rotation, orientation.mirror
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn try_form_machine(
    key_world: IVec3,
    def: &MachineDef,
    item_registry: &ItemRegistry,
    voxel_world: &VoxelWorld<WorldConfig>,
) -> Option<(u8, Orientation, Vec<IVec3>)> {
    let mut tiers: Vec<&MachineTierDef> = def.tiers.iter().collect();
    tiers.sort_by(|a, b| b.tier.cmp(&a.tier));

    for tier_def in tiers {
        let Some(key_in_pattern) =
            find_key_in_pattern(&tier_def.pattern, &tier_def.pattern_elements, &def.key_block)
        else {
            continue;
        };
        for orientation in Orientation::all() {
            if let Some(positions) = check_pattern(
                key_world,
                key_in_pattern,
                &tier_def.pattern,
                &tier_def.pattern_elements,
                orientation,
                item_registry,
                voxel_world,
            ) {
                return Some((tier_def.tier, orientation, positions));
            }
        }
    }
    None
}

fn find_key_in_pattern(
    pattern: &[Vec<String>],
    elements: &HashMap<String, CellMatcher>,
    key_block: &str,
) -> Option<IVec3> {
    let key_char = elements
        .iter()
        .find_map(|(ch, matcher)| if matcher.matches_id(key_block) { Some(ch.clone()) } else { None })?;

    for (y, layer) in pattern.iter().enumerate() {
        for (z, row) in layer.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch.to_string() == key_char {
                    return Some(IVec3::new(x as i32, y as i32, z as i32));
                }
            }
        }
    }
    None
}

fn check_pattern(
    key_world: IVec3,
    key_in_pattern: IVec3,
    pattern: &[Vec<String>],
    elements: &HashMap<String, CellMatcher>,
    orientation: Orientation,
    item_registry: &ItemRegistry,
    voxel_world: &VoxelWorld<WorldConfig>,
) -> Option<Vec<IVec3>> {
    let mut positions = Vec::new();

    for (y, layer) in pattern.iter().enumerate() {
        for (z, row) in layer.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let pat_pos = IVec3::new(x as i32, y as i32, z as i32);
                let delta = pat_pos - key_in_pattern;
                let world_pos = key_world + orientation.transform(delta);
                let voxel = voxel_world.get_voxel(world_pos);

                let ok = if let Some(matcher) = elements.get(&ch.to_string()) {
                    match matcher {
                        CellMatcher::BlockMatcher(item_id) => {
                            if let WorldVoxel::Solid(vox_id) = voxel {
                                item_registry.voxel_id(item_id) == Some(vox_id)
                            } else {
                                false
                            }
                        }
                    }
                } else {
                    // Char not in elements → must be air
                    matches!(voxel, WorldVoxel::Air | WorldVoxel::Unset)
                };

                if !ok {
                    return None;
                }

                if matches!(voxel, WorldVoxel::Solid(_)) {
                    positions.push(world_pos);
                }
            }
        }
    }

    Some(positions)
}

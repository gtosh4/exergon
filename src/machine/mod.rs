use std::collections::{HashMap, HashSet};

use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use serde::Deserialize;

use crate::content::load_ron_dir;
use crate::inventory::ItemRegistry;
use crate::world::{BlockChangeKind, BlockChangedMessage, generation::WorldConfig};

const ENERGY_IO_ID: &str = "energy_io";
const LOGISTICS_IO_ID: &str = "logistics_io";

/// System set that contains machine scanning. Logistics/power run after this.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MachineScanSet;

/// Emitted whenever machines form, unform, or are destroyed.
/// Listeners (logistics, power) use this to trigger network rebuilds.
#[derive(bevy::ecs::message::Message, Clone, Copy)]
pub struct MachineNetworkChanged;

/// Active recipe processing state on a running machine.
#[derive(Component, Clone)]
pub struct MachineActivity {
    pub recipe_id: String,
    pub progress: f32,
    /// Set by the power brownout system each tick (1.0 = full speed).
    pub speed_factor: f32,
}

pub struct MachinePlugin;

impl Plugin for MachinePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MachineNetworkChanged>()
            .configure_sets(
                Update,
                MachineScanSet
                    .in_set(crate::GameSystems::Simulation)
                    .run_if(resource_exists::<MachineRegistry>)
                    .run_if(in_state(crate::GameState::Playing)),
            )
            .add_systems(Startup, load_machines)
            .add_systems(Update, scan_machines.in_set(MachineScanSet));
    }
}

// ---------------------------------------------------------------------------
// Data types (deserialised from RON)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug)]
pub enum CellMatcher {
    BlockMatcher(String),
    /// Matches any of the listed block IDs. All IDs are registered as scan triggers.
    AnyOf(Vec<String>),
}

impl CellMatcher {
    fn matches_id(&self, id: &str) -> bool {
        match self {
            CellMatcher::BlockMatcher(bid) => bid == id,
            CellMatcher::AnyOf(ids) => ids.iter().any(|x| x == id),
        }
    }

    fn trigger_ids(&self) -> Vec<&str> {
        match self {
            CellMatcher::BlockMatcher(id) => vec![id.as_str()],
            CellMatcher::AnyOf(ids) => ids.iter().map(std::string::String::as_str).collect(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct MachineTierDef {
    pub tier: u8,
    /// pattern[y][z] = row string; each char is a placeholder key into `pattern_elements`.
    /// Chars absent from `pattern_elements` must be air.
    pub pattern: Vec<Vec<String>>,
    pub pattern_elements: HashMap<String, CellMatcher>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MachineDef {
    pub id: String,
    pub tiers: Vec<MachineTierDef>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct MachineRegistry {
    machines: Vec<MachineDef>,
    /// `block_id` → indices into `machines` that use this block in any tier pattern
    trigger_blocks: HashMap<String, Vec<usize>>,
}

impl MachineRegistry {
    fn new(machines: Vec<MachineDef>) -> Self {
        let mut trigger_blocks: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, machine) in machines.iter().enumerate() {
            for tier in &machine.tiers {
                for matcher in tier.pattern_elements.values() {
                    for id in matcher.trigger_ids() {
                        trigger_blocks.entry(id.to_owned()).or_default().push(i);
                    }
                }
            }
        }
        for indices in trigger_blocks.values_mut() {
            indices.sort_unstable();
            indices.dedup();
        }
        Self {
            machines,
            trigger_blocks,
        }
    }

    pub fn is_trigger_block(&self, item_id: &str) -> bool {
        self.trigger_blocks.contains_key(item_id)
    }

    fn machines_using_block(&self, item_id: &str) -> impl Iterator<Item = &MachineDef> {
        self.trigger_blocks
            .get(item_id)
            .into_iter()
            .flat_map(|indices| indices.iter().filter_map(|&i| self.machines.get(i)))
    }

    fn machine_def(&self, id: &str) -> Option<&MachineDef> {
        self.machines.iter().find(|m| m.id == id)
    }
}

// ---------------------------------------------------------------------------
// ECS components
// ---------------------------------------------------------------------------

#[derive(Component, Debug)]
pub struct Machine {
    pub machine_type: String,
    pub tier: u8,
    pub orientation: Orientation,
    /// World position of pattern cell (0,0,0) in canonical orientation.
    pub origin_pos: IVec3,
    /// All voxel positions occupied by this machine.
    pub blocks: HashSet<IVec3>,
    pub energy_io_blocks: HashSet<IVec3>,
    pub logistics_io_blocks: HashSet<IVec3>,
}

/// Marker: machine entity exists but structure is incomplete.
#[derive(Component)]
pub struct MachineUnformed;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineState {
    Idle,
    Running,
}

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
        use Mirror::{Mirrored, Normal};
        use Rotation::{East, North, South, West};
        [
            Orientation {
                rotation: North,
                mirror: Normal,
            },
            Orientation {
                rotation: East,
                mirror: Normal,
            },
            Orientation {
                rotation: South,
                mirror: Normal,
            },
            Orientation {
                rotation: West,
                mirror: Normal,
            },
            Orientation {
                rotation: North,
                mirror: Mirrored,
            },
            Orientation {
                rotation: East,
                mirror: Mirrored,
            },
            Orientation {
                rotation: South,
                mirror: Mirrored,
            },
            Orientation {
                rotation: West,
                mirror: Mirrored,
            },
        ]
    }

    /// Rotate a canonical-space delta into world-space, applying mirror then rotation.
    pub fn transform(&self, delta: IVec3) -> IVec3 {
        let dx = if self.mirror == Mirror::Mirrored {
            -delta.x
        } else {
            delta.x
        };
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
    mut events: MessageReader<BlockChangedMessage>,
    registry: Res<MachineRegistry>,
    item_registry: Option<Res<ItemRegistry>>,
    voxel_world: VoxelWorld<WorldConfig>,
    machine_q: Query<(Entity, &Machine, Option<&MachineUnformed>)>,
    mut network_changed: MessageWriter<MachineNetworkChanged>,
) {
    let Some(item_registry) = item_registry else {
        return;
    };

    let mut despawned: Vec<Entity> = Vec::new();

    for ev in events.read() {
        let pos = ev.pos;

        // --- UNFORM on remove or replace ---
        if matches!(
            ev.kind,
            BlockChangeKind::Removed { .. } | BlockChangeKind::Replaced { .. }
        ) {
            // Check formed machine at pos
            let formed = machine_q.iter().find(|(e, m, uf)| {
                uf.is_none() && m.blocks.contains(&pos) && !despawned.contains(e)
            });

            if let Some((entity, machine, _)) = formed {
                let still_valid = registry
                    .machine_def(&machine.machine_type)
                    .is_some_and(|def| {
                        try_form_machine(machine.origin_pos, def, &item_registry, &voxel_world)
                            .is_some()
                    });

                if !still_valid {
                    let solid_count = machine
                        .blocks
                        .iter()
                        .filter(|&&p| matches!(voxel_world.get_voxel(p), WorldVoxel::Solid(_)))
                        .count();
                    let total = machine.blocks.len();
                    let machine_type = machine.machine_type.clone();
                    let origin_pos = machine.origin_pos;

                    if solid_count * 10 < total {
                        despawned.push(entity);
                        commands.entity(entity).despawn();
                        network_changed.write(MachineNetworkChanged);
                        info!("Machine '{}' at {:?} destroyed", machine_type, origin_pos);
                    } else {
                        commands
                            .entity(entity)
                            .insert(MachineUnformed)
                            .remove::<MachineState>();
                        network_changed.write(MachineNetworkChanged);
                        info!("Machine '{}' at {:?} unformed", machine_type, origin_pos);
                    }
                }
            }

            // Check if an unformed machine at pos should now be destroyed
            let unformed = machine_q.iter().find(|(e, m, uf)| {
                uf.is_some() && m.blocks.contains(&pos) && !despawned.contains(e)
            });

            if let Some((entity, machine, _)) = unformed {
                let solid_count = machine
                    .blocks
                    .iter()
                    .filter(|&&p| matches!(voxel_world.get_voxel(p), WorldVoxel::Solid(_)))
                    .count();
                let total = machine.blocks.len();
                if solid_count * 10 < total {
                    let machine_type = machine.machine_type.clone();
                    let origin_pos = machine.origin_pos;
                    despawned.push(entity);
                    commands.entity(entity).despawn();
                    network_changed.write(MachineNetworkChanged);
                    info!("Machine '{}' at {:?} destroyed", machine_type, origin_pos);
                }
            }
        }

        // --- FORM on place or replace ---
        if !matches!(ev.kind, BlockChangeKind::Removed { .. }) {
            let origin_candidates: Vec<(IVec3, String)> = {
                let mut seen: HashSet<(IVec3, String)> = HashSet::new();
                let mut out = Vec::new();
                if let WorldVoxel::Solid(vox_id) = voxel_world.get_voxel(pos)
                    && let Some(item) = item_registry.item_for_voxel(vox_id)
                    && registry.is_trigger_block(&item.id)
                {
                    for machine_def in registry.machines_using_block(&item.id) {
                        for tier_def in &machine_def.tiers {
                            let pivots = find_pivots_for_block(
                                &tier_def.pattern,
                                &tier_def.pattern_elements,
                                &item.id,
                            );
                            for pivot in pivots {
                                for orientation in Orientation::all() {
                                    let origin = pos - orientation.transform(pivot);
                                    if seen.insert((origin, machine_def.id.clone())) {
                                        out.push((origin, machine_def.id.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
                out
            };

            'origin_candidates: for (origin, machine_id) in origin_candidates {
                if let Some(machine_def) = registry.machine_def(&machine_id)
                    && let Some((tier, orientation, positions)) =
                        try_form_machine(origin, machine_def, &item_registry, &voxel_world)
                {
                    // Skip if any position belongs to a formed machine
                    let has_formed = positions.iter().any(|p| {
                        machine_q
                            .iter()
                            .any(|(_, m, uf)| uf.is_none() && m.blocks.contains(p))
                    });
                    if has_formed {
                        continue;
                    }

                    // Remove overlapping unformed machines
                    let mut to_remove: Vec<Entity> = Vec::new();
                    for &p in &positions {
                        if let Some((uf_entity, _, _)) = machine_q.iter().find(|(e, m, uf)| {
                            uf.is_some()
                                && m.blocks.contains(&p)
                                && !to_remove.contains(e)
                                && !despawned.contains(e)
                        }) {
                            to_remove.push(uf_entity);
                        }
                    }
                    for uf in to_remove {
                        despawned.push(uf);
                        commands.entity(uf).despawn();
                    }

                    let (energy_io_blocks, logistics_io_blocks) =
                        io_blocks_for_positions(&positions, &item_registry, &voxel_world);

                    commands.spawn((
                        Machine {
                            machine_type: machine_def.id.clone(),
                            tier,
                            orientation,
                            origin_pos: origin,
                            blocks: positions.iter().copied().collect(),
                            energy_io_blocks,
                            logistics_io_blocks,
                        },
                        MachineState::Idle,
                    ));
                    network_changed.write(MachineNetworkChanged);
                    info!(
                        "Machine '{}' tier {} formed at origin {:?} ({:?}/{:?})",
                        machine_def.id, tier, origin, orientation.rotation, orientation.mirror
                    );
                    continue 'origin_candidates;
                }
            }

            // Check if an unformed machine at pos can now re-form
            let unformed = machine_q.iter().find(|(e, m, uf)| {
                uf.is_some() && m.blocks.contains(&pos) && !despawned.contains(e)
            });

            if let Some((entity, machine, _)) = unformed {
                let machine_type = machine.machine_type.clone();
                let origin_pos = machine.origin_pos;

                if let Some(def) = registry.machine_def(&machine_type)
                    && let Some((tier, orientation, new_positions)) =
                        try_form_machine(origin_pos, def, &item_registry, &voxel_world)
                {
                    // uf.is_none() already excludes the current unformed entity
                    let no_overlap = !new_positions.iter().any(|p| {
                        machine_q
                            .iter()
                            .any(|(_, m, uf)| uf.is_none() && m.blocks.contains(p))
                    });
                    if no_overlap {
                        let (energy_io_blocks, logistics_io_blocks) =
                            io_blocks_for_positions(&new_positions, &item_registry, &voxel_world);

                        commands
                            .entity(entity)
                            .insert(Machine {
                                machine_type: machine_type.clone(),
                                tier,
                                orientation,
                                origin_pos,
                                blocks: new_positions.iter().copied().collect(),
                                energy_io_blocks,
                                logistics_io_blocks,
                            })
                            .remove::<MachineUnformed>()
                            .insert(MachineState::Idle);
                        network_changed.write(MachineNetworkChanged);
                        info!(
                            "Machine '{}' tier {} re-formed at origin {:?} ({:?}/{:?})",
                            machine_type,
                            tier,
                            origin_pos,
                            orientation.rotation,
                            orientation.mirror
                        );
                    }
                }
            }
        }
    }
}

fn io_blocks_for_positions(
    positions: &[IVec3],
    item_registry: &ItemRegistry,
    voxel_world: &VoxelWorld<WorldConfig>,
) -> (HashSet<IVec3>, HashSet<IVec3>) {
    let mut energy_io = HashSet::new();
    let mut logistics_io = HashSet::new();
    for &p in positions {
        if let WorldVoxel::Solid(vox_id) = voxel_world.get_voxel(p)
            && let Some(item) = item_registry.item_for_voxel(vox_id)
        {
            if item.id == ENERGY_IO_ID {
                energy_io.insert(p);
            } else if item.id == LOGISTICS_IO_ID {
                logistics_io.insert(p);
            }
        }
    }
    (energy_io, logistics_io)
}

fn try_form_machine(
    origin_world: IVec3,
    def: &MachineDef,
    item_registry: &ItemRegistry,
    voxel_world: &VoxelWorld<WorldConfig>,
) -> Option<(u8, Orientation, Vec<IVec3>)> {
    let mut tiers: Vec<&MachineTierDef> = def.tiers.iter().collect();
    tiers.sort_by(|a, b| b.tier.cmp(&a.tier));

    for tier_def in tiers {
        for orientation in Orientation::all() {
            if let Some(positions) = check_pattern(
                origin_world,
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

/// All pattern positions where `block_id` appears in any element.
fn find_pivots_for_block(
    pattern: &[Vec<String>],
    elements: &HashMap<String, CellMatcher>,
    block_id: &str,
) -> Vec<IVec3> {
    let mut pivots = Vec::new();
    for (y, layer) in pattern.iter().enumerate() {
        for (z, row) in layer.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if let Some(matcher) = elements.get(&ch.to_string())
                    && matcher.matches_id(block_id)
                {
                    pivots.push(IVec3::new(x as i32, y as i32, z as i32));
                }
            }
        }
    }
    pivots
}

fn check_pattern(
    origin_world: IVec3,
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
                let world_pos = origin_world + orientation.transform(pat_pos);
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
                        CellMatcher::AnyOf(ids) => {
                            if let WorldVoxel::Solid(vox_id) = voxel {
                                ids.iter()
                                    .any(|id| item_registry.voxel_id(id) == Some(vox_id))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_matcher_block_matches_exact() {
        let m = CellMatcher::BlockMatcher("iron".to_string());
        assert!(m.matches_id("iron"));
        assert!(!m.matches_id("gold"));
        assert_eq!(m.trigger_ids(), vec!["iron"]);
    }

    #[test]
    fn cell_matcher_any_of_matches_member() {
        let m = CellMatcher::AnyOf(vec!["iron".to_string(), "gold".to_string()]);
        assert!(m.matches_id("iron"));
        assert!(m.matches_id("gold"));
        assert!(!m.matches_id("stone"));
        let ids = m.trigger_ids();
        assert!(ids.contains(&"iron"));
        assert!(ids.contains(&"gold"));
    }

    #[test]
    fn orientation_all_returns_8() {
        assert_eq!(Orientation::all().len(), 8);
    }

    #[test]
    fn orientation_north_normal_is_identity() {
        let o = Orientation {
            rotation: Rotation::North,
            mirror: Mirror::Normal,
        };
        assert_eq!(o.transform(IVec3::new(1, 2, 3)), IVec3::new(1, 2, 3));
    }

    #[test]
    fn orientation_east_rotates_correctly() {
        let o = Orientation {
            rotation: Rotation::East,
            mirror: Mirror::Normal,
        };
        // East: (dx, dy, dz) -> (dz, dy, -dx)
        assert_eq!(o.transform(IVec3::new(1, 0, 0)), IVec3::new(0, 0, -1));
        assert_eq!(o.transform(IVec3::new(0, 0, 1)), IVec3::new(1, 0, 0));
    }

    #[test]
    fn orientation_mirror_negates_x() {
        let o = Orientation {
            rotation: Rotation::North,
            mirror: Mirror::Mirrored,
        };
        assert_eq!(o.transform(IVec3::new(2, 1, 3)), IVec3::new(-2, 1, 3));
    }

    fn simple_machine(id: &str, block_id: &str) -> MachineDef {
        let mut elements = HashMap::new();
        elements.insert(
            "A".to_string(),
            CellMatcher::BlockMatcher(block_id.to_string()),
        );
        MachineDef {
            id: id.to_string(),
            tiers: vec![MachineTierDef {
                tier: 1,
                pattern: vec![vec!["A".to_string()]],
                pattern_elements: elements,
            }],
        }
    }

    #[test]
    fn registry_trigger_block_detected() {
        let reg = MachineRegistry::new(vec![simple_machine("smelter", "iron")]);
        assert!(reg.is_trigger_block("iron"));
        assert!(!reg.is_trigger_block("stone"));
    }

    #[test]
    fn registry_machines_using_block() {
        let reg = MachineRegistry::new(vec![simple_machine("smelter", "iron")]);
        let names: Vec<&str> = reg
            .machines_using_block("iron")
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(names, vec!["smelter"]);
        assert_eq!(reg.machines_using_block("stone").count(), 0);
    }

    #[test]
    fn find_pivots_locates_matching_chars() {
        let mut elements = HashMap::new();
        elements.insert(
            "A".to_string(),
            CellMatcher::BlockMatcher("iron".to_string()),
        );
        // pattern[y=0][z=0] = "AB" → A at x=0, B at x=1
        let pattern = vec![vec!["AB".to_string()]];
        let pivots = find_pivots_for_block(&pattern, &elements, "iron");
        assert_eq!(pivots, vec![IVec3::new(0, 0, 0)]);
    }

    #[test]
    fn machine_def_found_and_not_found() {
        let reg = MachineRegistry::new(vec![simple_machine("smelter", "iron")]);
        assert!(reg.machine_def("smelter").is_some());
        assert_eq!(reg.machine_def("smelter").unwrap().id, "smelter");
        assert!(reg.machine_def("unknown").is_none());
    }

    #[test]
    fn orientation_south_normal() {
        let o = Orientation {
            rotation: Rotation::South,
            mirror: Mirror::Normal,
        };
        // South: (-dx, dy, -dz)
        assert_eq!(o.transform(IVec3::new(1, 0, 0)), IVec3::new(-1, 0, 0));
        assert_eq!(o.transform(IVec3::new(0, 0, 1)), IVec3::new(0, 0, -1));
    }

    #[test]
    fn orientation_west_normal() {
        let o = Orientation {
            rotation: Rotation::West,
            mirror: Mirror::Normal,
        };
        // West: (-dz, dy, dx)
        assert_eq!(o.transform(IVec3::new(1, 0, 0)), IVec3::new(0, 0, 1));
        assert_eq!(o.transform(IVec3::new(0, 0, 1)), IVec3::new(-1, 0, 0));
    }

    #[test]
    fn orientation_y_unchanged_all_variants() {
        for o in Orientation::all() {
            assert_eq!(o.transform(IVec3::new(3, 7, 5)).y, 7);
        }
    }

    #[test]
    fn find_pivots_multi_layer() {
        let mut elements = HashMap::new();
        elements.insert(
            "A".to_string(),
            CellMatcher::BlockMatcher("iron".to_string()),
        );
        // Two Y-layers, each with one A at x=0,z=0
        let pattern = vec![vec!["A".to_string()], vec!["A".to_string()]];
        let pivots = find_pivots_for_block(&pattern, &elements, "iron");
        assert!(pivots.contains(&IVec3::new(0, 0, 0)));
        assert!(pivots.contains(&IVec3::new(0, 1, 0)));
        assert_eq!(pivots.len(), 2);
    }
}

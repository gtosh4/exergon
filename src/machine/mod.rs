use std::collections::{HashMap, HashSet};

use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use bevy_voxel_world::prelude::*;
use serde::Deserialize;

use crate::content::load_ron_dir;
use crate::inventory::ItemRegistry;
use crate::world::{generation::WorldConfig, BlockChangedEvent};

pub struct MachinePlugin;

impl Plugin for MachinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MachineBlockMap>()
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
pub struct MachineTierDef {
    pub tier: u8,
    /// pattern[y][z][x]. Cell values: item id, "air"/empty = must be air, "?" = any solid.
    pub pattern: Vec<Vec<Vec<String>>>,
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

/// Voxel position → machine entity. Updated on form/invalidate.
#[derive(Resource, Default)]
pub struct MachineBlockMap(pub HashMap<IVec3, Entity>);

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
    registry: Res<MachineRegistry>,
    item_registry: Option<Res<ItemRegistry>>,
    voxel_world: VoxelWorld<WorldConfig>,
    machine_q: Query<&MachineParts>,
) {
    let Some(item_registry) = item_registry else {
        return;
    };

    // Separate: key positions from invalidated machines (process first = higher priority)
    // vs new key blocks just placed (process second).
    let mut existing_rescans: Vec<IVec3> = Vec::new();
    let mut new_rescans: Vec<IVec3> = Vec::new();
    let mut to_invalidate: Vec<Entity> = Vec::new();

    for ev in events.read() {
        let pos = ev.pos;

        // Collect affected machines from changed pos and 6 face-neighbors
        for candidate in [
            pos,
            pos + IVec3::X,
            pos - IVec3::X,
            pos + IVec3::Y,
            pos - IVec3::Y,
            pos + IVec3::Z,
            pos - IVec3::Z,
        ] {
            if let Some(&entity) = block_map.0.get(&candidate) {
                if !to_invalidate.contains(&entity) {
                    to_invalidate.push(entity);
                }
            }
        }

        // If placed block is a key block, queue for new-formation scan
        if is_key_block_at(pos, &voxel_world, &item_registry, &registry)
            && !new_rescans.contains(&pos)
        {
            new_rescans.push(pos);
        }
    }

    // Invalidate affected machines; collect their key positions for re-scan (priority queue)
    for entity in to_invalidate {
        if let Ok(parts) = machine_q.get(entity) {
            for &part_pos in &parts.positions {
                if is_key_block_at(part_pos, &voxel_world, &item_registry, &registry)
                    && !existing_rescans.contains(&part_pos)
                {
                    existing_rescans.push(part_pos);
                }
                block_map.0.remove(&part_pos);
            }
        }
        commands.entity(entity).despawn();
    }

    // Remove new-key positions that are already in existing_rescans (avoids double-scan)
    new_rescans.retain(|p| !existing_rescans.contains(p));

    // Existing machines first (reclaim their blocks), then new formations
    let rescan_positions = existing_rescans.into_iter().chain(new_rescans);

    // Scan from each key position
    for key_pos in rescan_positions {
        let WorldVoxel::Solid(vox_id) = voxel_world.get_voxel(key_pos) else {
            continue;
        };
        let Some(item) = item_registry.item_for_voxel(vox_id) else {
            continue;
        };
        let item_id = item.id.clone();

        for machine_def in registry.machines_with_key(&item_id) {
            if let Some((tier, orientation, positions)) =
                try_form_machine(key_pos, machine_def, &item_registry, &voxel_world)
            {
                // Reject if any required block is already claimed by another machine
                if positions.iter().any(|p| block_map.0.contains_key(p)) {
                    continue;
                }

                let entity = commands
                    .spawn((
                        Machine {
                            machine_type: machine_def.id.clone(),
                            tier,
                            orientation,
                            key_pos,
                        },
                        MachineParts { positions: positions.clone() },
                    ))
                    .id();

                for pos in &positions {
                    block_map.0.insert(*pos, entity);
                }

                info!(
                    "Machine '{}' tier {} formed at {:?} ({:?}/{:?})",
                    machine_def.id, tier, key_pos, orientation.rotation, orientation.mirror
                );
                break;
            }
        }
    }
}

fn is_key_block_at(
    pos: IVec3,
    voxel_world: &VoxelWorld<WorldConfig>,
    item_registry: &ItemRegistry,
    registry: &MachineRegistry,
) -> bool {
    if let WorldVoxel::Solid(vox_id) = voxel_world.get_voxel(pos) {
        if let Some(item) = item_registry.item_for_voxel(vox_id) {
            return registry.is_key_block(&item.id);
        }
    }
    false
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
        let Some(key_in_pattern) = find_key_in_pattern(&tier_def.pattern, &def.key_block) else {
            continue;
        };
        for orientation in Orientation::all() {
            if let Some(positions) = check_pattern(
                key_world,
                key_in_pattern,
                &tier_def.pattern,
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

fn find_key_in_pattern(pattern: &[Vec<Vec<String>>], key_block: &str) -> Option<IVec3> {
    for (y, layer) in pattern.iter().enumerate() {
        for (z, row) in layer.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if cell == key_block {
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
    pattern: &[Vec<Vec<String>>],
    orientation: Orientation,
    item_registry: &ItemRegistry,
    voxel_world: &VoxelWorld<WorldConfig>,
) -> Option<Vec<IVec3>> {
    let mut positions = Vec::new();

    for (y, layer) in pattern.iter().enumerate() {
        for (z, row) in layer.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let pat_pos = IVec3::new(x as i32, y as i32, z as i32);
                let delta = pat_pos - key_in_pattern;
                let world_pos = key_world + orientation.transform(delta);
                let voxel = voxel_world.get_voxel(world_pos);

                let ok = match cell.as_str() {
                    "" | "air" => matches!(voxel, WorldVoxel::Air | WorldVoxel::Unset),
                    "?" => matches!(voxel, WorldVoxel::Solid(_)),
                    item_id => {
                        if let WorldVoxel::Solid(vox_id) = voxel {
                            item_registry.voxel_id(item_id) == Some(vox_id)
                        } else {
                            false
                        }
                    }
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

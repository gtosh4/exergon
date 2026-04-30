use std::collections::HashMap;

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use serde::Deserialize;
use xxhash_rust::xxh64::xxh64;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_content);
    }
}

// ---------------------------------------------------------------------------
// Data types (deserialised from RON)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug)]
pub struct OreSpec {
    pub name: String,
    pub material: u8,
    pub weight: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct VeinDef {
    pub id: String,
    /// Fraction of blocks within the vein volume that are replaced with ore (0–1).
    pub density: f32,
    pub primary: OreSpec,
    pub secondary: OreSpec,
    pub sporadic: Option<OreSpec>,
}

impl VeinDef {
    pub fn pick_ore<R: Rng>(&self, rng: &mut R) -> &OreSpec {
        let total = self.primary.weight
            + self.secondary.weight
            + self.sporadic.as_ref().map_or(0, |s| s.weight);
        if total == 0 {
            return &self.primary;
        }
        let roll = rng.gen_range(0..total);
        let mut acc = self.primary.weight;
        if roll < acc {
            return &self.primary;
        }
        acc += self.secondary.weight;
        if roll < acc {
            return &self.secondary;
        }
        self.sporadic.as_ref().unwrap_or(&self.primary)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct LayerDef {
    pub id: String,
    pub name: String,
    /// Inclusive cell-Y range this layer covers (1 cell = CHUNK_VOXELS × CELL_CHUNKS voxels tall).
    pub y_cell_range: (i32, i32),
}

#[derive(Deserialize, Clone, Debug)]
pub struct BiomeDef {
    pub id: String,
    /// ID of the layer this biome belongs to.
    pub layer: String,
    /// (vein_id, weight) pairs.
    pub vein_pool: Vec<(String, u32)>,
}

// ---------------------------------------------------------------------------
// Registry — built once from loaded defs, shared via Arc into voxel threads
// ---------------------------------------------------------------------------

pub const CHUNK_VOXELS: i32 = 32;
pub const CELL_CHUNKS: i32 = 3; // 3×3×3 chunks per vein cell

/// Snapshot returned by biome queries — borrows from `VeinRegistry`.
pub struct BiomeInfo<'a> {
    pub id: &'a str,
    pub layer_id: &'a str,
    pub layer_name: &'a str,
}

#[derive(Clone)]
struct ResolvedBiome {
    id: String,
    layer_id: String,
    y_cell_range: (i32, i32),
    pool: Vec<(usize, u32)>,
    pool_total: u32,
}

#[derive(Resource, Clone)]
pub struct VeinRegistry {
    veins: Vec<VeinDef>,
    layers: Vec<LayerDef>,
    biomes: Vec<ResolvedBiome>,
    material_names: HashMap<u8, String>,
}

impl VeinRegistry {
    pub fn new(veins: Vec<VeinDef>, layers: Vec<LayerDef>, biomes: Vec<BiomeDef>) -> Self {
        let mut material_names = HashMap::new();
        material_names.insert(0u8, "Stone".to_string());
        material_names.insert(1u8, "Surface Rock".to_string());

        for vein in &veins {
            for ore in std::iter::once(&vein.primary)
                .chain(std::iter::once(&vein.secondary))
                .chain(vein.sporadic.as_ref())
            {
                material_names.insert(ore.material, ore.name.clone());
            }
        }

        validate_layers(&layers);

        let biomes = biomes
            .into_iter()
            .filter_map(|b| {
                let layer = layers.iter().find(|l| l.id == b.layer);
                if layer.is_none() {
                    error!("Biome '{}' references unknown layer '{}'", b.id, b.layer);
                }
                let layer = layer?;
                let pool: Vec<(usize, u32)> = b
                    .vein_pool
                    .iter()
                    .filter_map(|(id, w)| {
                        veins.iter().position(|v| &v.id == id).map(|idx| (idx, *w))
                    })
                    .collect();
                let pool_total: u32 = pool.iter().map(|(_, w)| w).sum();
                Some(ResolvedBiome {
                    id: b.id,
                    layer_id: layer.id.clone(),
                    y_cell_range: layer.y_cell_range,
                    pool,
                    pool_total,
                })
            })
            .collect();

        Self { veins, layers, biomes, material_names }
    }

    pub fn material_name(&self, material: u8) -> Option<&str> {
        self.material_names.get(&material).map(String::as_str)
    }

    pub fn biome_at_cell_y(&self, cell_y: i32) -> Option<BiomeInfo<'_>> {
        let b = self
            .biomes
            .iter()
            .find(|b| cell_y >= b.y_cell_range.0 && cell_y <= b.y_cell_range.1)?;
        let layer_name = self
            .layers
            .iter()
            .find(|l| l.id == b.layer_id)
            .map(|l| l.name.as_str())
            .unwrap_or("");
        Some(BiomeInfo { id: &b.id, layer_id: &b.layer_id, layer_name })
    }

    /// Returns the vein at the given 3-D cell coordinates, or None if the cell is empty.
    pub fn cell_vein(&self, world_seed: u64, cell_x: i32, cell_y: i32, cell_z: i32) -> Option<&VeinDef> {
        let b = self
            .biomes
            .iter()
            .find(|b| cell_y >= b.y_cell_range.0 && cell_y <= b.y_cell_range.1)?;

        if b.pool.is_empty() || b.pool_total == 0 {
            return None;
        }

        let cell_seed = {
            let mut key = world_seed.to_le_bytes().to_vec();
            key.extend_from_slice(b"vein");
            key.extend_from_slice(&cell_x.to_le_bytes());
            key.extend_from_slice(&cell_y.to_le_bytes());
            key.extend_from_slice(&cell_z.to_le_bytes());
            xxh64(&key, 0)
        };
        let mut rng = Pcg64::seed_from_u64(cell_seed);
        if !rng.gen_bool(0.33) {
            return None;
        }
        let roll = rng.gen_range(0..b.pool_total);
        let mut acc = 0u32;
        b.pool.iter().find_map(|(idx, w)| {
            acc += w;
            if roll < acc { Some(&self.veins[*idx]) } else { None }
        })
    }

    /// Returns the ore material index for a solid stone voxel at (wx, wy, wz),
    /// or None if no vein covers this position.
    pub fn ore_at(&self, world_seed: u64, wx: i32, wy: i32, wz: i32) -> Option<u8> {
        let cell_size = CHUNK_VOXELS * CELL_CHUNKS;
        let cell_x = wx.div_euclid(cell_size);
        let cell_y = wy.div_euclid(cell_size);
        let cell_z = wz.div_euclid(cell_size);
        let vein = self.cell_vein(world_seed, cell_x, cell_y, cell_z)?;

        let pos_seed = {
            let mut key = world_seed.to_le_bytes().to_vec();
            key.extend_from_slice(b"ore");
            key.extend_from_slice(&wx.to_le_bytes());
            key.extend_from_slice(&wy.to_le_bytes());
            key.extend_from_slice(&wz.to_le_bytes());
            xxh64(&key, 0)
        };
        let mut pos_rng = Pcg64::seed_from_u64(pos_seed);

        if !pos_rng.gen_bool(vein.density as f64) {
            return None;
        }

        Some(vein.pick_ore(&mut pos_rng).material)
    }
}

fn validate_layers(layers: &[LayerDef]) {
    let mut sorted: Vec<&LayerDef> = layers.iter().collect();
    sorted.sort_by_key(|l| l.y_cell_range.0);

    for pair in sorted.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if a.y_cell_range.1 >= b.y_cell_range.0 {
            error!(
                "Layer '{}' (y_cell_range {:?}) overlaps layer '{}' (y_cell_range {:?})",
                a.id, a.y_cell_range, b.id, b.y_cell_range
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn load_content(mut commands: Commands) {
    let veins = load_ron_dir::<VeinDef>("assets/deposits", "deposit");
    let layers = load_ron_dir::<LayerDef>("assets/layers", "layer");
    let biomes = load_ron_dir::<BiomeDef>("assets/biomes", "biome");

    info!(
        "Loaded content: {} deposits, {} layers, {} biomes",
        veins.len(),
        layers.len(),
        biomes.len(),
    );

    if veins.is_empty() {
        warn!("No vein definitions found in assets/deposits/");
    }
    if layers.is_empty() {
        warn!("No layer definitions found in assets/layers/");
    }
    if biomes.is_empty() {
        warn!("No biome definitions found in assets/biomes/");
    }

    commands.insert_resource(VeinRegistry::new(veins, layers, biomes));
}

pub(crate) fn load_ron_dir<T: for<'de> Deserialize<'de>>(dir: &str, label: &str) -> Vec<T> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut results = Vec::new();
    for entry in entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "ron"))
    {
        let path = entry.path();
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        match std::fs::read_to_string(&path) {
            Ok(s) => match ron::from_str::<T>(&s) {
                Ok(v) => {
                    debug!("  {label} [{filename}]");
                    results.push(v);
                }
                Err(err) => warn!("Failed to parse {filename}: {err}"),
            },
            Err(err) => warn!("Failed to read {filename}: {err}"),
        }
    }
    results
}

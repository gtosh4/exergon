use std::collections::HashMap;

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use serde::Deserialize;
use xxhash_rust::xxh64::xxh64;

use crate::inventory::{ItemDef, ItemRegistry};

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
    /// Inclusive cell-Y range this layer covers (1 cell = `CHUNK_VOXELS` × `CELL_CHUNKS` voxels tall).
    pub y_cell_range: (i32, i32),
}

#[derive(Deserialize, Clone, Debug)]
pub struct BiomeDef {
    pub id: String,
    /// ID of the layer this biome belongs to.
    pub layer: String,
    /// (`vein_id`, weight) pairs.
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

        Self {
            veins,
            layers,
            biomes,
            material_names,
        }
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
            .map_or("", |l| l.name.as_str());
        Some(BiomeInfo {
            id: &b.id,
            layer_id: &b.layer_id,
            layer_name,
        })
    }

    /// Returns the vein at the given 3-D cell coordinates, or None if the cell is empty.
    pub fn cell_vein(
        &self,
        world_seed: u64,
        cell_x: i32,
        cell_y: i32,
        cell_z: i32,
    ) -> Option<&VeinDef> {
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
            if roll < acc {
                self.veins.get(*idx)
            } else {
                None
            }
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

        if !pos_rng.gen_bool(f64::from(vein.density)) {
            return None;
        }

        Some(vein.pick_ore(&mut pos_rng).material)
    }
}

fn validate_layers(layers: &[LayerDef]) {
    let mut sorted: Vec<&LayerDef> = layers.iter().collect();
    sorted.sort_by_key(|l| l.y_cell_range.0);

    for pair in sorted.windows(2) {
        let [a, b] = pair else { continue };
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
    let item_defs = load_ron_dir::<ItemDef>("assets/items", "item");

    info!(
        "Loaded content: {} deposits, {} layers, {} biomes, {} items",
        veins.len(),
        layers.len(),
        biomes.len(),
        item_defs.len(),
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
    if item_defs.is_empty() {
        warn!("No item definitions found in assets/items/");
    }

    let mut item_registry = ItemRegistry::default();
    for item in item_defs {
        item_registry.register(item);
    }

    commands.insert_resource(item_registry);
    commands.insert_resource(VeinRegistry::new(veins, layers, biomes));
}

pub(crate) fn load_ron_dir<T: for<'de> Deserialize<'de>>(dir: &str, label: &str) -> Vec<T> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut results = Vec::new();
    for entry in entries
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "ron"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_pcg::Pcg64;

    fn ore(name: &str, material: u8, weight: u32) -> OreSpec {
        OreSpec {
            name: name.to_string(),
            material,
            weight,
        }
    }

    fn vein(id: &str, primary_w: u32, secondary_w: u32, sporadic: Option<OreSpec>) -> VeinDef {
        VeinDef {
            id: id.to_string(),
            density: 1.0,
            primary: ore("Primary", 1, primary_w),
            secondary: ore("Secondary", 2, secondary_w),
            sporadic,
        }
    }

    fn layer(id: &str, y_range: (i32, i32)) -> LayerDef {
        LayerDef {
            id: id.to_string(),
            name: format!("Layer {id}"),
            y_cell_range: y_range,
        }
    }

    fn biome(id: &str, layer_id: &str, pool: Vec<(&str, u32)>) -> BiomeDef {
        BiomeDef {
            id: id.to_string(),
            layer: layer_id.to_string(),
            vein_pool: pool.into_iter().map(|(s, w)| (s.to_string(), w)).collect(),
        }
    }

    #[test]
    fn pick_ore_zero_weight_returns_primary() {
        let v = vein("v", 0, 0, None);
        let mut rng = Pcg64::seed_from_u64(0);
        assert_eq!(v.pick_ore(&mut rng).material, 1);
    }

    #[test]
    fn pick_ore_primary_only() {
        let v = vein("v", 100, 0, None);
        let mut rng = Pcg64::seed_from_u64(0);
        for _ in 0..20 {
            assert_eq!(v.pick_ore(&mut rng).material, 1);
        }
    }

    #[test]
    fn pick_ore_sporadic_when_only_weight() {
        let v = vein("v", 0, 0, Some(ore("Sporadic", 3, 100)));
        let mut rng = Pcg64::seed_from_u64(0);
        assert_eq!(v.pick_ore(&mut rng).material, 3);
    }

    #[test]
    fn pick_ore_secondary_reachable() {
        // primary=0, secondary=100 → always secondary
        let v = VeinDef {
            id: "v".into(),
            density: 1.0,
            primary: ore("P", 1, 0),
            secondary: ore("S", 2, 100),
            sporadic: None,
        };
        let mut rng = Pcg64::seed_from_u64(42);
        assert_eq!(v.pick_ore(&mut rng).material, 2);
    }

    #[test]
    fn material_name_defaults_present() {
        let reg = VeinRegistry::new(vec![], vec![], vec![]);
        assert_eq!(reg.material_name(0), Some("Stone"));
        assert_eq!(reg.material_name(1), Some("Surface Rock"));
        assert_eq!(reg.material_name(99), None);
    }

    #[test]
    fn material_name_from_vein() {
        let v = vein("v", 50, 50, Some(ore("Copper", 5, 10)));
        let l = layer("l", (0, 5));
        let b = biome("b", "l", vec![("v", 1)]);
        let reg = VeinRegistry::new(vec![v], vec![l], vec![b]);
        assert_eq!(reg.material_name(1), Some("Primary"));
        assert_eq!(reg.material_name(2), Some("Secondary"));
        assert_eq!(reg.material_name(5), Some("Copper"));
    }

    #[test]
    fn biome_at_cell_y_in_range() {
        let l = layer("deep", (0, 10));
        let v = vein("v", 1, 1, None);
        let b = biome("deep_biome", "deep", vec![("v", 1)]);
        let reg = VeinRegistry::new(vec![v], vec![l], vec![b]);
        let info = reg.biome_at_cell_y(5).unwrap();
        assert_eq!(info.id, "deep_biome");
        assert_eq!(info.layer_id, "deep");
        assert_eq!(info.layer_name, "Layer deep");
    }

    #[test]
    fn biome_at_cell_y_boundary() {
        let l = layer("l", (0, 5));
        let v = vein("v", 1, 1, None);
        let b = biome("b", "l", vec![("v", 1)]);
        let reg = VeinRegistry::new(vec![v], vec![l], vec![b]);
        assert!(reg.biome_at_cell_y(0).is_some());
        assert!(reg.biome_at_cell_y(5).is_some());
        assert!(reg.biome_at_cell_y(-1).is_none());
        assert!(reg.biome_at_cell_y(6).is_none());
    }

    #[test]
    fn biome_at_cell_y_no_biomes_returns_none() {
        let reg = VeinRegistry::new(vec![], vec![], vec![]);
        assert!(reg.biome_at_cell_y(0).is_none());
    }

    #[test]
    fn cell_vein_no_biome_returns_none() {
        let reg = VeinRegistry::new(vec![], vec![], vec![]);
        assert!(reg.cell_vein(0, 0, 99, 0).is_none());
    }

    #[test]
    fn cell_vein_empty_pool_returns_none() {
        let l = layer("l", (0, 5));
        let reg = VeinRegistry::new(vec![], vec![l], vec![biome("b", "l", vec![])]);
        assert!(reg.cell_vein(0, 0, 0, 0).is_none());
    }

    #[test]
    fn cell_vein_deterministic() {
        let l = layer("l", (0, 5));
        let v = vein("iron", 1, 1, None);
        let b = biome("b", "l", vec![("iron", 1)]);
        let reg = VeinRegistry::new(vec![v], vec![l], vec![b]);
        // Same seed + coords must give same result
        let a = reg.cell_vein(12345, 1, 0, 1).map(|v| v.id.clone());
        let b_result = reg.cell_vein(12345, 1, 0, 1).map(|v| v.id.clone());
        assert_eq!(a, b_result);
    }

    #[test]
    fn ore_at_no_biome_returns_none() {
        let reg = VeinRegistry::new(vec![], vec![], vec![]);
        assert!(reg.ore_at(0, 0, 0, 0).is_none());
    }

    #[test]
    fn ore_at_deterministic() {
        let l = layer("l", (0, 5));
        let v = VeinDef {
            id: "iron".into(),
            density: 1.0,
            primary: ore("Iron", 10, 100),
            secondary: ore("Copper", 11, 0),
            sporadic: None,
        };
        let b = biome("b", "l", vec![("iron", 1)]);
        let reg = VeinRegistry::new(vec![v], vec![l], vec![b]);
        let r1 = reg.ore_at(999, 10, 10, 10);
        let r2 = reg.ore_at(999, 10, 10, 10);
        assert_eq!(r1, r2);
    }
}

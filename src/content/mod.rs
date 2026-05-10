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
    /// Fraction of positions within the vein volume that are replaced with ore (0–1).
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
    /// Inclusive cell-Y range this layer covers (1 cell = `CHUNK_SIZE` × `CELL_CHUNKS_Y` units tall).
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
// Registry — built once from loaded defs, shared via Arc into generation threads
// ---------------------------------------------------------------------------

pub const CHUNK_SIZE: i32 = 32;
pub const CELL_CHUNKS_XZ: i32 = 5; // XZ cell span in chunks (5×32 = 160 units wide)
pub const CELL_CHUNKS_Y: i32 = 2; // Y cell span in chunks  (2×32 = 64 units tall)

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

    /// Returns the ore material index for position (wx, wy, wz),
    /// or None if no vein covers this position.
    pub fn ore_at(&self, world_seed: u64, wx: i32, wy: i32, wz: i32) -> Option<u8> {
        let cell_size_xz = CHUNK_SIZE * CELL_CHUNKS_XZ;
        let cell_size_y = CHUNK_SIZE * CELL_CHUNKS_Y;
        let cell_x = wx.div_euclid(cell_size_xz);
        let cell_y = wy.div_euclid(cell_size_y);
        let cell_z = wz.div_euclid(cell_size_xz);
        let vein = self.cell_vein(world_seed, cell_x, cell_y, cell_z)?;

        // Normalized position within the cell in [-1, 1] per axis
        let nx = (wx.rem_euclid(cell_size_xz) as f32 + 0.5) / cell_size_xz as f32 * 2.0 - 1.0;
        let ny = (wy.rem_euclid(cell_size_y) as f32 + 0.5) / cell_size_y as f32 * 2.0 - 1.0;
        let nz = (wz.rem_euclid(cell_size_xz) as f32 + 0.5) / cell_size_xz as f32 * 2.0 - 1.0;
        // Ellipsoidal distance from cell center (0 at center, 1 at surface)
        let dist = (nx * nx + ny * ny + nz * nz).sqrt();

        // Per-position hash jitter gives organic, non-cubic vein boundaries
        let jitter_seed = {
            let mut key = world_seed.to_le_bytes().to_vec();
            key.extend_from_slice(b"jit");
            key.extend_from_slice(&wx.to_le_bytes());
            key.extend_from_slice(&wy.to_le_bytes());
            key.extend_from_slice(&wz.to_le_bytes());
            xxh64(&key, 0)
        };
        let jitter = jitter_seed as f32 / u64::MAX as f32 * 0.5 - 0.25;
        let density_scale = (1.0 - (dist + jitter)).clamp(0.0, 1.0);

        let pos_seed = {
            let mut key = world_seed.to_le_bytes().to_vec();
            key.extend_from_slice(b"ore");
            key.extend_from_slice(&wx.to_le_bytes());
            key.extend_from_slice(&wy.to_le_bytes());
            key.extend_from_slice(&wz.to_le_bytes());
            xxh64(&key, 0)
        };
        let mut pos_rng = Pcg64::seed_from_u64(pos_seed);

        // 1.5× factor so average density across the ellipsoid matches vein.density
        let effective_density = (vein.density * density_scale * 1.5_f32).min(1.0);
        if !pos_rng.gen_bool(f64::from(effective_density)) {
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
// Surface deposit registry
// ---------------------------------------------------------------------------

const DEPOSIT_CELL_SIZE: f32 = 64.0;

/// Surface-level ore deposit definition loaded from `assets/deposits/`.
#[derive(Deserialize, Clone, Debug)]
pub struct DepositDef {
    pub id: String,
    /// (material_id, weight) pairs. Weights are normalised to sum to 1.0 at load.
    pub ores: Vec<(String, f32)>,
}

/// Registry of surface deposit types used to place and query ore deposits.
#[derive(Resource, Clone)]
pub struct DepositRegistry {
    deposits: Vec<DepositDef>,
}

impl DepositRegistry {
    pub fn new(defs: Vec<DepositDef>) -> Self {
        let deposits = defs
            .into_iter()
            .map(|mut d| {
                let total: f32 = d.ores.iter().map(|(_, w)| w).sum();
                if total > 0.0 && (total - 1.0).abs() > f32::EPSILON {
                    for (_, w) in &mut d.ores {
                        *w /= total;
                    }
                }
                d
            })
            .collect();
        Self { deposits }
    }

    /// Returns the weighted ore distribution for the surface deposit at (wx, wz), or None.
    pub fn ore_at(&self, seed: u64, wx: f32, wz: f32) -> Option<Vec<(String, f32)>> {
        if self.deposits.is_empty() {
            return None;
        }
        let cell_x = wx.div_euclid(DEPOSIT_CELL_SIZE) as i64;
        let cell_z = wz.div_euclid(DEPOSIT_CELL_SIZE) as i64;
        let cell_seed = {
            let mut key = seed.to_le_bytes().to_vec();
            key.extend_from_slice(b"dep");
            key.extend_from_slice(&cell_x.to_le_bytes());
            key.extend_from_slice(&cell_z.to_le_bytes());
            xxh64(&key, 0)
        };
        let mut rng = Pcg64::seed_from_u64(cell_seed);
        if !rng.gen_bool(0.33) {
            return None;
        }
        let idx = rng.gen_range(0..self.deposits.len());
        self.deposits.get(idx).map(|d| d.ores.clone())
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn load_content(mut commands: Commands) {
    let veins = load_ron_dir::<VeinDef>("assets/veins", "vein");
    let layers = load_ron_dir::<LayerDef>("assets/layers", "layer");
    let biomes = load_ron_dir::<BiomeDef>("assets/biomes", "biome");
    let deposits = load_ron_dir::<DepositDef>("assets/deposits", "deposit");

    info!(
        "Loaded content: {} veins, {} layers, {} biomes, {} surface deposits",
        veins.len(),
        layers.len(),
        biomes.len(),
        deposits.len(),
    );

    if layers.is_empty() {
        warn!("No layer definitions found in assets/layers/");
    }
    if biomes.is_empty() {
        warn!("No biome definitions found in assets/biomes/");
    }

    commands.insert_resource(DepositRegistry::new(deposits));
    commands.insert_resource(VeinRegistry::new(veins, layers, biomes));
}

pub(crate) fn load_ron_dir<T: for<'de> Deserialize<'de>>(dir: &str, label: &str) -> Vec<T> {
    let mut results = Vec::new();
    collect_ron_dir(dir, label, &mut results);
    results
}

fn collect_ron_dir<T: for<'de> Deserialize<'de>>(dir: &str, label: &str, results: &mut Vec<T>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(std::result::Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if let Some(s) = path.to_str() {
                collect_ron_dir(s, label, results);
            }
        } else if path.extension().is_some_and(|ext| ext == "ron") {
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
    }
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

    #[test]
    fn ore_at_center_of_cell_reliably_present() {
        let l = layer("l", (0, 10));
        let v = VeinDef {
            id: "iron".into(),
            density: 1.0,
            primary: ore("Iron", 10, 100),
            secondary: ore("Copper", 11, 0),
            sporadic: None,
        };
        let b = biome("b", "l", vec![("iron", 1)]);
        let reg = VeinRegistry::new(vec![v], vec![l], vec![b]);
        let cx = CHUNK_SIZE * CELL_CHUNKS_XZ / 2;
        let cy = CHUNK_SIZE * CELL_CHUNKS_Y / 2;
        let cz = CHUNK_SIZE * CELL_CHUNKS_XZ / 2;
        let count = (0u64..50)
            .filter(|&seed| reg.ore_at(seed, cx, cy, cz).is_some())
            .count();
        assert!(
            count > 10,
            "center of vein cell should produce ore for most active-vein seeds, got {count}"
        );
    }

    // DepositRegistry tests

    fn deposit(id: &str, ores: Vec<(&str, f32)>) -> DepositDef {
        DepositDef {
            id: id.to_string(),
            ores: ores.into_iter().map(|(m, w)| (m.to_string(), w)).collect(),
        }
    }

    #[test]
    fn ore_at_no_deposits_returns_none() {
        let reg = DepositRegistry::new(vec![]);
        assert!(reg.ore_at(0, 0.0, 0.0).is_none());
    }

    #[test]
    fn ore_at_is_deterministic() {
        let reg = DepositRegistry::new(vec![deposit("iron", vec![("iron", 1.0)])]);
        let a = reg.ore_at(42, 100.0, 100.0);
        let b = reg.ore_at(42, 100.0, 100.0);
        assert_eq!(a, b);
    }

    #[test]
    fn ore_at_normalizes_weights() {
        let reg =
            DepositRegistry::new(vec![deposit("mix", vec![("iron", 30.0), ("copper", 70.0)])]);
        // Find a position that returns Some
        let result = (0u64..200).find_map(|seed| reg.ore_at(seed, 0.0, 0.0));
        let Some(ores) = result else { return };
        let total: f32 = ores.iter().map(|(_, w)| w).sum();
        assert!(
            (total - 1.0).abs() < 1e-5,
            "weights should sum to 1.0, got {total}"
        );
    }

    #[test]
    fn ore_at_returns_weighted_list() {
        let reg = DepositRegistry::new(vec![deposit("iron", vec![("iron", 0.6), ("copper", 0.4)])]);
        let result = (0u64..200).find_map(|seed| reg.ore_at(seed, 0.0, 0.0));
        let ores = result.expect("should find a deposit in 200 seeds");
        assert_eq!(ores.len(), 2);
        assert!(ores.iter().any(|(m, _)| m == "iron"));
        assert!(ores.iter().any(|(m, _)| m == "copper"));
    }
}

use bevy::prelude::*;

pub struct SeedPlugin;

impl Plugin for SeedPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RunSeed>()
            .register_type::<DomainSeeds>();
    }
}

/// The player-entered seed string for this run.
#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct RunSeed {
    pub text: String,
    pub hash: u64,
}

/// Per-domain sub-seeds derived from the master seed.
#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct DomainSeeds {
    pub world: u64,
    pub tech_tree: u64,
    pub recipes: u64,
    pub power: u64,
    pub reactivity: u64,
    pub biomes: u64,
}

impl DomainSeeds {
    pub fn from_master(master: u64) -> Self {
        Self {
            world: derive(master, "world"),
            tech_tree: derive(master, "tech_tree"),
            recipes: derive(master, "recipes"),
            power: derive(master, "power"),
            reactivity: derive(master, "reactivity"),
            biomes: derive(master, "biomes"),
        }
    }
}

pub fn hash_text(text: &str) -> u64 {
    use xxhash_rust::xxh64::xxh64;
    xxh64(text.as_bytes(), 0)
}

pub fn derive(master: u64, domain: &str) -> u64 {
    use xxhash_rust::xxh64::xxh64;
    let mut buf = master.to_le_bytes().to_vec();
    buf.extend_from_slice(domain.as_bytes());
    xxh64(&buf, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn hash_text_deterministic() {
        assert_eq!(hash_text("hello"), hash_text("hello"));
        assert_ne!(hash_text("hello"), hash_text("world"));
    }

    #[test]
    fn derive_deterministic() {
        assert_eq!(derive(42, "world"), derive(42, "world"));
    }

    #[test]
    fn derive_differs_by_domain() {
        assert_ne!(derive(42, "world"), derive(42, "power"));
    }

    #[test]
    fn domain_seeds_all_distinct() {
        let s = DomainSeeds::from_master(12345);
        let vals: HashSet<u64> = [
            s.world,
            s.tech_tree,
            s.recipes,
            s.power,
            s.reactivity,
            s.biomes,
        ]
        .into_iter()
        .collect();
        assert_eq!(vals.len(), 6);
    }
}

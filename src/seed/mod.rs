use bevy::prelude::*;
use rand::SeedableRng;
use rand_pcg::Pcg64;

pub struct SeedPlugin;

impl Plugin for SeedPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RunSeed>()
            .register_type::<DomainSeeds>();
    }
}

/// The player-entered seed string for this run. Component on the Run entity
/// (see `src/save/mod.rs`).
#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct RunSeed {
    pub text: String,
    pub hash: u64,
}

/// Per-domain sub-seeds derived from the master seed. Component on the Run
/// entity.
#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct DomainSeeds {
    pub world: u64,
    pub planet: u64,
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
            planet: derive(master, "planet"),
            tech_tree: derive(master, "tech_tree"),
            recipes: derive(master, "recipes"),
            power: derive(master, "power"),
            reactivity: derive(master, "reactivity"),
            biomes: derive(master, "biomes"),
        }
    }

    /// Pcg64 RNG seeded from the planet domain. Use for multi-draw planet
    /// generation (archetype axis values, hazard parameters).
    pub fn planet_rng(&self) -> Pcg64 {
        Pcg64::seed_from_u64(self.planet)
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
    use bevy::reflect::{FromReflect, PartialReflect, Reflect};

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
            s.planet,
            s.tech_tree,
            s.recipes,
            s.power,
            s.reactivity,
            s.biomes,
        ]
        .into_iter()
        .collect();
        assert_eq!(vals.len(), 7);
    }

    #[test]
    fn planet_rng_is_deterministic_for_same_master() {
        use rand::RngCore;
        let a = DomainSeeds::from_master(7777).planet_rng();
        let b = DomainSeeds::from_master(7777).planet_rng();
        let mut a = a;
        let mut b = b;
        let a_stream: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let b_stream: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_eq!(a_stream, b_stream);
    }

    #[test]
    fn planet_rng_differs_for_different_master() {
        use rand::RngCore;
        let mut a = DomainSeeds::from_master(1).planet_rng();
        let mut b = DomainSeeds::from_master(2).planet_rng();
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn run_seed_reflect_clone_and_from_reflect() {
        let seed = RunSeed {
            text: "hello".to_string(),
            hash: 99,
        };
        let cloned = seed.reflect_clone().unwrap();
        let back = RunSeed::from_reflect(&*cloned).unwrap();
        assert_eq!(back.text, "hello");
        assert_eq!(back.hash, 99);
    }

    #[test]
    fn run_seed_try_apply_patches_value() {
        let mut seed = RunSeed {
            text: "old".to_string(),
            hash: 0,
        };
        let other = RunSeed {
            text: "new".to_string(),
            hash: 7,
        };
        seed.try_apply(&other).unwrap();
        assert_eq!(seed.text, "new");
        assert_eq!(seed.hash, 7);
    }

    #[test]
    fn run_seed_reflect_set() {
        let mut seed = RunSeed {
            text: "a".to_string(),
            hash: 1,
        };
        let new_seed = Box::new(RunSeed {
            text: "b".to_string(),
            hash: 2,
        });
        seed.set(new_seed).unwrap();
        assert_eq!(seed.text, "b");
        assert_eq!(seed.hash, 2);
    }

    #[test]
    fn domain_seeds_reflect_clone_and_from_reflect() {
        let seeds = DomainSeeds::from_master(42);
        let cloned = seeds.reflect_clone().unwrap();
        let back = DomainSeeds::from_reflect(&*cloned).unwrap();
        assert_eq!(back.world, seeds.world);
        assert_eq!(back.biomes, seeds.biomes);
    }

    #[test]
    fn domain_seeds_try_apply_patches_value() {
        let mut seeds = DomainSeeds::from_master(1);
        let other = DomainSeeds::from_master(2);
        let expected_world = other.world;
        seeds.try_apply(&other).unwrap();
        assert_eq!(seeds.world, expected_world);
    }

    #[test]
    fn domain_seeds_reflect_set() {
        let mut seeds = DomainSeeds::from_master(1);
        let other = DomainSeeds::from_master(99);
        let expected = other.world;
        seeds.set(Box::new(other)).unwrap();
        assert_eq!(seeds.world, expected);
    }
}

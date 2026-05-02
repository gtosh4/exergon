use std::collections::HashMap;

use bevy::prelude::*;

use super::DIRS;

/// BFS from `seed`; removes visited nodes from `unvisited`. Returns the component.
pub fn bfs_component(
    seed: IVec3,
    unvisited: &mut HashMap<IVec3, bevy::prelude::Entity>,
) -> HashMap<IVec3, bevy::prelude::Entity> {
    let Some(seed_entity) = unvisited.remove(&seed) else {
        return HashMap::new();
    };
    let mut component = HashMap::new();
    component.insert(seed, seed_entity);
    let mut queue = vec![seed];

    while let Some(pos) = queue.pop() {
        for &dir in &DIRS {
            let n = pos + dir;
            if let Some(&e) = unvisited.get(&n) {
                unvisited.remove(&n);
                component.insert(n, e);
                queue.push(n);
            }
        }
    }
    component
}

/// Returns connected components of `remaining` after removing `removed_pos`.
pub fn find_components(
    remaining: HashMap<IVec3, bevy::prelude::Entity>,
    removed_pos: IVec3,
) -> Vec<HashMap<IVec3, bevy::prelude::Entity>> {
    let seeds: Vec<IVec3> = DIRS
        .iter()
        .map(|&d| removed_pos + d)
        .filter(|n| remaining.contains_key(n))
        .collect();

    if seeds.is_empty() {
        return vec![remaining];
    }

    let mut unvisited = remaining;
    let mut components = Vec::new();

    for seed in seeds {
        if !unvisited.contains_key(&seed) {
            continue;
        }
        components.push(bfs_component(seed, &mut unvisited));
    }

    if !unvisited.is_empty() {
        components.push(unvisited);
    }

    components
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: u32) -> Entity {
        Entity::from_raw_u32(id).unwrap()
    }

    fn make_map(pairs: &[(IVec3, u32)]) -> HashMap<IVec3, Entity> {
        pairs.iter().map(|&(p, id)| (p, entity(id))).collect()
    }

    #[test]
    fn find_components_single_cable_returns_empty_after_removal() {
        let remaining = HashMap::new();
        let components = find_components(remaining, IVec3::ZERO);
        assert_eq!(components.len(), 1);
        assert!(components[0].is_empty());
    }

    #[test]
    fn find_components_line_removal_splits_into_two() {
        // Line: (0,0,0) - (1,0,0) - (2,0,0); remove middle
        let remaining = make_map(&[(IVec3::new(0, 0, 0), 1), (IVec3::new(2, 0, 0), 2)]);
        let components = find_components(remaining, IVec3::new(1, 0, 0));
        assert_eq!(components.len(), 2);
    }

    #[test]
    fn find_components_no_split_ring() {
        // Square ring: (0,0,0)-(1,0,0)-(1,0,1)-(0,0,1)-(0,0,0); remove (0,0,0)
        let remaining = make_map(&[
            (IVec3::new(1, 0, 0), 1),
            (IVec3::new(1, 0, 1), 2),
            (IVec3::new(0, 0, 1), 3),
        ]);
        let components = find_components(remaining, IVec3::ZERO);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 3);
    }

    #[test]
    fn bfs_component_visits_connected() {
        let mut unvisited = make_map(&[
            (IVec3::new(0, 0, 0), 1),
            (IVec3::new(1, 0, 0), 2),
            (IVec3::new(5, 0, 0), 3),
        ]);
        let comp = bfs_component(IVec3::ZERO, &mut unvisited);
        assert_eq!(comp.len(), 2);
        assert!(comp.contains_key(&IVec3::ZERO));
        assert!(comp.contains_key(&IVec3::new(1, 0, 0)));
        assert_eq!(unvisited.len(), 1);
    }
}

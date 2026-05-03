use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

fn build_endpoint_adjacency(segments: &HashMap<Entity, [IVec3; 2]>) -> HashMap<IVec3, Vec<Entity>> {
    let mut map: HashMap<IVec3, Vec<Entity>> = HashMap::new();
    for (&e, &eps) in segments {
        for ep in eps {
            map.entry(ep).or_default().push(e);
        }
    }
    map
}

fn bfs_component(
    seed: Entity,
    unvisited: &mut HashMap<Entity, [IVec3; 2]>,
    adj: &HashMap<IVec3, Vec<Entity>>,
) -> HashMap<Entity, [IVec3; 2]> {
    let Some(eps) = unvisited.remove(&seed) else {
        return HashMap::new();
    };
    let mut component = HashMap::new();
    component.insert(seed, eps);
    let mut queue = VecDeque::from([eps]);

    while let Some(eps) = queue.pop_front() {
        for ep in eps {
            for &neighbor in adj.get(&ep).into_iter().flatten() {
                if let Some(n_eps) = unvisited.remove(&neighbor) {
                    component.insert(neighbor, n_eps);
                    queue.push_back(n_eps);
                }
            }
        }
    }
    component
}

/// Partitions `remaining` cable segments into connected components.
/// Two segments are connected when they share an endpoint.
pub fn find_segment_components(
    remaining: HashMap<Entity, [IVec3; 2]>,
) -> Vec<HashMap<Entity, [IVec3; 2]>> {
    if remaining.is_empty() {
        return vec![];
    }

    let adj = build_endpoint_adjacency(&remaining);
    let mut unvisited = remaining;
    let mut components = Vec::new();

    while let Some(&seed) = unvisited.keys().next() {
        components.push(bfs_component(seed, &mut unvisited, &adj));
    }

    components
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: u32) -> Entity {
        Entity::from_raw_u32(id).unwrap()
    }

    #[test]
    fn empty_remaining_returns_empty() {
        let components = find_segment_components(HashMap::new());
        assert!(components.is_empty());
    }

    #[test]
    fn single_segment_one_component() {
        let mut map = HashMap::new();
        map.insert(e(1), [IVec3::new(0, 0, 0), IVec3::new(5, 0, 0)]);
        let comps = find_segment_components(map);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 1);
    }

    #[test]
    fn chain_of_three_one_component() {
        // A-B, B-C share endpoint B
        let mut map = HashMap::new();
        map.insert(e(1), [IVec3::new(0, 0, 0), IVec3::new(1, 0, 0)]);
        map.insert(e(2), [IVec3::new(1, 0, 0), IVec3::new(2, 0, 0)]);
        map.insert(e(3), [IVec3::new(2, 0, 0), IVec3::new(3, 0, 0)]);
        let comps = find_segment_components(map);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 3);
    }

    #[test]
    fn two_disconnected_segments_two_components() {
        let mut map = HashMap::new();
        map.insert(e(1), [IVec3::new(0, 0, 0), IVec3::new(1, 0, 0)]);
        map.insert(e(2), [IVec3::new(10, 0, 0), IVec3::new(11, 0, 0)]);
        let comps = find_segment_components(map);
        assert_eq!(comps.len(), 2);
    }

    #[test]
    fn bridge_removed_splits_into_two() {
        // A-B, B-C, C-D: remove B-C → A-B and C-D separate
        let mut map = HashMap::new();
        map.insert(e(1), [IVec3::new(0, 0, 0), IVec3::new(1, 0, 0)]);
        // e(2) = B-C is the "removed" cable — omitted from remaining
        map.insert(e(3), [IVec3::new(2, 0, 0), IVec3::new(3, 0, 0)]);
        let comps = find_segment_components(map);
        assert_eq!(comps.len(), 2);
    }
}

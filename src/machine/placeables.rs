use avian3d::prelude::Collider;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub struct PlaceableDef {
    pub item: ItemSpec,
    pub interaction: InteractionShape,
    pub surface: SurfaceRule,
    pub snap: SnapRule,
    pub orientation: OrientationSupport,
    pub ghost: GhostHint,
    /// Half-extents [x, y, z] used for collision and placement offset.
    /// Must be non-zero in all axes — panics at cache build time otherwise.
    pub footprint: [f32; 3],
    #[serde(default)]
    pub max_reach: Option<f32>,
}

impl PlaceableDef {
    pub fn footprint_vec3(&self) -> Vec3 {
        Vec3::from(self.footprint)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub struct ItemSpec {
    pub id: String,
    pub name: String,
    pub stack_size: u32,
    pub kind: ItemKind,
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub enum ItemKind {
    Machine { tier: u8 },
    Cable { network: String },
    Platform,
    Generator,
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub enum InteractionShape {
    Single,
    TwoEndpoint { item_id: String },
    AreaRect { tile_size: f32 },
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub enum SurfaceRule {
    Ground,
    Anywhere,
    Port,
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub enum SnapRule {
    /// Free placement — no snapping.
    Free,
    Tile(TileSnap),
    PortRaycast,
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub enum TileSnap {
    Horizontal { step: f32 },
    Vertical { step: f32 },
}

/// How player input maps to Build orientation before placement.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, schemars::JsonSchema)]
pub enum OrientationSupport {
    /// Always placed at identity rotation; R-key ignored.
    Fixed,
    /// Continuous Y-axis rotation via R-key (machines).
    AxisY,
    /// Arbitrary Quat rotation via R-drag (platforms, decorations).
    /// VS simplification: treated same as AxisY.
    Free,
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub enum GhostHint {
    Scene,
    TiledScene,
    Routed,
}

// ---------------------------------------------------------------------------
// PlaceableRegistry
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct PlaceableRegistry {
    by_item: HashMap<String, PlaceableDef>,
}

impl PlaceableRegistry {
    pub fn get(&self, id: &str) -> Option<&PlaceableDef> {
        self.by_item.get(id)
    }

    pub fn is_placeable(&self, id: &str) -> bool {
        self.by_item.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &PlaceableDef)> {
        self.by_item.iter()
    }

    pub(super) fn from_defs(defs: Vec<PlaceableDef>) -> Self {
        let mut by_item = HashMap::new();
        for def in defs {
            let id = def.item.id.clone();
            assert!(
                !by_item.contains_key(&id),
                "Duplicate PlaceableDef item id: {id}"
            );
            by_item.insert(id, def);
        }
        Self { by_item }
    }
}

// ---------------------------------------------------------------------------
// PlaceableColliderCache
// ---------------------------------------------------------------------------

pub struct PlaceableCollider {
    pub aabb_half_extents: Vec3,
    pub collider: Collider,
}

#[derive(Resource, Default)]
pub struct PlaceableColliderCache {
    pub by_item: HashMap<String, PlaceableCollider>,
}

#[derive(Resource, Default)]
pub struct PlaceableCacheReady(pub bool);

fn validate_footprint(id: &str, fp: Vec3) {
    assert!(
        fp.x > 0.0 && fp.y > 0.0 && fp.z > 0.0 && fp.is_finite(),
        "PlaceableDef \"{id}\" has invalid footprint {fp:?}"
    );
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

pub(super) fn build_placeable_collider_cache(
    registry: Res<PlaceableRegistry>,
    mut commands: Commands,
) {
    let mut cache = PlaceableColliderCache::default();
    for (id, def) in registry.iter() {
        let fp = def.footprint_vec3();
        validate_footprint(id, fp);
        cache.by_item.insert(
            id.clone(),
            PlaceableCollider {
                aabb_half_extents: fp,
                collider: Collider::cuboid(fp.x, fp.y, fp.z),
            },
        );
    }
    commands.insert_resource(cache);
    commands.insert_resource(PlaceableCacheReady(true));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_def(id: &str) -> PlaceableDef {
        PlaceableDef {
            item: ItemSpec {
                id: id.to_string(),
                name: id.to_string(),
                stack_size: 10,
                kind: ItemKind::Machine { tier: 1 },
            },
            interaction: InteractionShape::Single,
            surface: SurfaceRule::Ground,
            snap: SnapRule::Free,
            orientation: OrientationSupport::AxisY,
            ghost: GhostHint::Scene,
            footprint: [2.0, 2.0, 2.0],
            max_reach: None,
        }
    }

    #[test]
    fn registry_get_returns_def() {
        let reg = PlaceableRegistry::from_defs(vec![make_def("smelter")]);
        assert!(reg.get("smelter").is_some());
        assert_eq!(reg.get("smelter").unwrap().item.id, "smelter");
    }

    #[test]
    fn registry_is_placeable_true_for_known() {
        let reg = PlaceableRegistry::from_defs(vec![make_def("smelter")]);
        assert!(reg.is_placeable("smelter"));
    }

    #[test]
    fn registry_is_placeable_false_for_unknown() {
        let reg = PlaceableRegistry::from_defs(vec![make_def("smelter")]);
        assert!(!reg.is_placeable("furnace"));
    }

    #[test]
    #[should_panic(expected = "Duplicate PlaceableDef item id")]
    fn registry_panics_on_duplicate_id() {
        PlaceableRegistry::from_defs(vec![make_def("smelter"), make_def("smelter")]);
    }

    #[test]
    fn footprint_vec3_converts_correctly() {
        let def = make_def("smelter");
        assert_eq!(def.footprint_vec3(), Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    #[should_panic(expected = "invalid footprint")]
    fn validate_footprint_panics_on_zero() {
        validate_footprint("bad", Vec3::ZERO);
    }

    #[test]
    #[should_panic(expected = "invalid footprint")]
    fn validate_footprint_panics_on_nan() {
        validate_footprint("bad", Vec3::new(f32::NAN, 1.0, 1.0));
    }

    #[test]
    fn validate_footprint_passes_positive() {
        validate_footprint("ok", Vec3::new(2.0, 0.125, 4.0));
    }
}

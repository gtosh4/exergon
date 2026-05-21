use bevy::prelude::*;
use serde::Deserialize;

use super::placeables::{
    GhostHint, InteractionShape, ItemKind, ItemSpec, OrientationSupport, PlaceableDef,
    PlaceableRegistry, SnapRule, SurfaceRule,
};
use crate::content::load_ron_dir;

#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub enum EnvSource {
    #[default]
    Solar,
    Combustion,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GeneratorDef {
    pub watts: f32,
    pub env_source: EnvSource,
    /// Explicit buffer cap in joules. When 0 (default), computed as `watts * 10`.
    #[serde(default)]
    pub max_buffer_joules: f32,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct MachineTierDef {
    pub tier: u8,
    #[serde(default)]
    pub generator: Option<GeneratorDef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MachineDef {
    pub id: String,
    pub tiers: Vec<MachineTierDef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MachineFileDef {
    pub id: String,
    pub name: String,
    pub stack_size: u32,
    pub tiers: Vec<MachineTierDef>,
    pub interaction: InteractionShape,
    pub surface: SurfaceRule,
    pub snap: SnapRule,
    pub orientation: OrientationSupport,
    pub ghost: GhostHint,
    pub footprint: [f32; 3],
    #[serde(default)]
    pub max_reach: Option<f32>,
}

impl MachineFileDef {
    fn to_machine_def(&self) -> MachineDef {
        MachineDef {
            id: self.id.clone(),
            tiers: self.tiers.clone(),
        }
    }

    pub fn to_placeable_def(&self) -> PlaceableDef {
        let kind = if self.tiers.iter().any(|t| t.generator.is_some()) {
            ItemKind::Generator
        } else {
            ItemKind::Machine {
                tier: self.tiers.iter().map(|t| t.tier).max().unwrap_or(1),
            }
        };
        PlaceableDef {
            item: ItemSpec {
                id: self.id.clone(),
                name: self.name.clone(),
                stack_size: self.stack_size,
                kind,
            },
            interaction: self.interaction.clone(),
            surface: self.surface.clone(),
            snap: self.snap.clone(),
            orientation: self.orientation.clone(),
            ghost: self.ghost.clone(),
            footprint: self.footprint,
            max_reach: self.max_reach,
        }
    }
}

#[derive(Resource)]
pub struct MachineRegistry {
    machines: Vec<MachineDef>,
}

impl MachineRegistry {
    pub fn new(machines: Vec<MachineDef>) -> Self {
        Self { machines }
    }

    pub fn machine_def(&self, id: &str) -> Option<&MachineDef> {
        self.machines.iter().find(|m| m.id == id)
    }
}

pub(super) fn load_machines(mut commands: Commands) {
    let file_defs = load_ron_dir::<MachineFileDef>("assets/machines", "machine");
    info!("Loaded {} machine definitions", file_defs.len());

    let machines = file_defs.iter().map(|d| d.to_machine_def()).collect();
    commands.insert_resource(MachineRegistry::new(machines));

    let mut placeable_defs = load_ron_dir::<PlaceableDef>("assets/placeables", "placeable");
    for fd in &file_defs {
        placeable_defs.push(fd.to_placeable_def());
    }
    commands.insert_resource(PlaceableRegistry::from_defs(placeable_defs));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(id: &str) -> MachineDef {
        MachineDef {
            id: id.to_string(),
            tiers: vec![],
        }
    }

    #[test]
    fn machine_def_found() {
        let reg = MachineRegistry::new(vec![def("smelter"), def("furnace")]);
        assert!(reg.machine_def("smelter").is_some());
        assert_eq!(reg.machine_def("smelter").unwrap().id, "smelter");
    }

    #[test]
    fn machine_def_not_found() {
        let reg = MachineRegistry::new(vec![def("smelter")]);
        assert!(reg.machine_def("forge").is_none());
    }

    #[test]
    fn machine_def_empty_registry() {
        let reg = MachineRegistry::new(vec![]);
        assert!(reg.machine_def("anything").is_none());
    }
}

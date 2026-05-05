use bevy::prelude::*;
use serde::Deserialize;

use crate::content::load_ron_dir;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct MachineTierDef {
    pub tier: u8,
    #[serde(default)]
    pub energy_io_offsets: Vec<IVec3>,
    #[serde(default)]
    pub logistics_io_offsets: Vec<IVec3>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MachineDef {
    pub id: String,
    pub tiers: Vec<MachineTierDef>,
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
    let machines = load_ron_dir::<MachineDef>("assets/machines", "machine");
    info!("Loaded {} machine definitions", machines.len());
    commands.insert_resource(MachineRegistry::new(machines));
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

use bevy::prelude::*;

pub mod crosshair;
pub mod field_computer;
pub mod power;
pub mod research;
pub mod tooltip;

pub use field_computer::{FieldComputerLog, FieldComputerMessage, MessageCategory};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        crosshair::plugin(app);
        field_computer::plugin(app);
        power::plugin(app);
        research::plugin(app);
        tooltip::plugin(app);
    }
}

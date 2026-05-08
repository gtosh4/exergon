use bevy::prelude::*;

pub mod crosshair;
pub mod power;
pub mod tooltip;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        crosshair::plugin(app);
        power::plugin(app);
        tooltip::plugin(app);
    }
}

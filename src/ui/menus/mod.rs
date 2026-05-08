use bevy::prelude::*;

pub mod loading;
pub mod main_menu;
pub mod pause;

pub struct MenusPlugin;

impl Plugin for MenusPlugin {
    fn build(&self, app: &mut App) {
        loading::plugin(app);
        main_menu::plugin(app);
        pause::plugin(app);
    }
}

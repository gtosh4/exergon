use bevy::prelude::*;

pub mod load_run;
pub mod loading;
pub mod main_menu;
pub mod new_run_wizard;
pub mod pause;

pub struct MenusPlugin;

impl Plugin for MenusPlugin {
    fn build(&self, app: &mut App) {
        load_run::plugin(app);
        loading::plugin(app);
        main_menu::plugin(app);
        new_run_wizard::plugin(app);
        pause::plugin(app);
    }
}

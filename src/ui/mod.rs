use bevy::prelude::*;
use crosshair::crosshair_plugin;
use loading_screen::loading_screen_plugin;

use crate::ui::{
    main_menu::main_menu_plugin, system_menu::system_menu_plugin, you_win::you_win_plugin,
};

pub mod crosshair;
mod loading_screen;
mod main_menu;
mod system_menu;
pub mod you_win;

pub fn ui_plugins(app: &mut App) {
    app.add_plugins((
        loading_screen_plugin,
        crosshair_plugin,
        main_menu_plugin,
        system_menu_plugin,
        you_win_plugin,
    ));
}

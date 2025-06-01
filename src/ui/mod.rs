use bevy::prelude::*;
use crosshair::crosshair_plugin;
use loading_screen::loading_screen_plugin;

mod loading_screen;
pub mod crosshair;

pub fn ui_plugins(app: &mut App) {
    app.add_plugins((
        loading_screen_plugin,
        crosshair_plugin
    ));
}

use bevy::prelude::*;
use crosshair::crosshair_plugin;
use loading_screen::loading_screen_plugin;

pub mod crosshair;
mod loading_screen;

pub fn ui_plugins(app: &mut App) {
    app.add_plugins((loading_screen_plugin, crosshair_plugin));
}

use asset_loading::assets_plugin;
use asset_tag_components::asset_tag_components_plugin;
use bevy::prelude::*;
use unity::UnityPlugin;

pub mod asset_loading;
pub mod asset_tag_components;
mod unity;

pub fn asset_plugins(app: &mut App) {
    app.add_plugins((
        UnityPlugin::default(),
        assets_plugin,
        asset_tag_components_plugin,
    ));
}

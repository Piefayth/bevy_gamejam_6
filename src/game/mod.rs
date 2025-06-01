use avian3d::{prelude::{Gravity, PhysicsDebugPlugin}, PhysicsPlugins};
use bevy::prelude::*;
use input::input_plugin;
use interaction::interaction_plugin;
use player::player_plugin;

pub mod player;
pub mod input;
pub mod interaction;

pub fn gameplay_plugins(app: &mut App) {
    app.add_plugins((
        PhysicsPlugins::default(),
        PhysicsDebugPlugin::default(),
        player_plugin,
        input_plugin,
        interaction_plugin,
    ))
    .insert_resource(Gravity(Vec3::NEG_Y * 19.6));
}

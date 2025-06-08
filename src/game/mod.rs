use avian3d::{
    prelude::{Collider, Gravity, PhysicsDebugPlugin, PhysicsLayer},
    PhysicsPlugins,
};
use bevy::prelude::*;
use bevy_tween::{bevy_time_runner::TimeRunnerEnded, TweenSystemSet};
use button::button_plugin;
use cube_spitter::cube_spitter_plugin;
use dissolve_gate::dissolve_gate_plugin;
use door::door_plugin;
use inert::inert_plugin;
use input::input_plugin;
use interaction::interaction_plugin;
use player::player_plugin;
use pressure_plate::pressure_plate_plugin;
use signal_spitter::signal_spitter_plugin;
use signals::signals_plugin;
use standing_cube_spitter::standing_cube_spitter_plugin;
use weighted_cube::cube_plugin;

use crate::game::{discharge_gate::discharge_gate_plugin, signal_preview::signal_preview_plugin};

pub mod button;
pub mod cube_spitter;
pub mod discharge_gate;
pub mod dissolve_gate;
pub mod door;
pub mod inert;
pub mod input;
pub mod interaction;
pub mod player;
pub mod pressure_plate;
pub mod signal_spitter;
pub mod signals;
pub mod standing_cube_spitter;
pub mod weighted_cube;
pub mod signal_preview;

pub fn gameplay_plugins(app: &mut App) {
    app.add_plugins((
        PhysicsPlugins::default(),
        //PhysicsDebugPlugin::default(),
        player_plugin,
        input_plugin,
        interaction_plugin,
        signals_plugin,
        pressure_plate_plugin,
        dissolve_gate_plugin,
        door_plugin,
        inert_plugin,
        signal_spitter_plugin,
        cube_spitter_plugin,
        cube_plugin,
        standing_cube_spitter_plugin,
    ))
    .add_plugins((
        button_plugin,
        discharge_gate_plugin,
        signal_preview_plugin
    ))
    .insert_resource(Gravity(Vec3::NEG_Y * 19.6));

    app.add_systems(
        PostUpdate,
        despawn_tween_on_finish.after(TweenSystemSet::ApplyTween),
    );
}

#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Player,
    Signal,
    Device,
    Ignore,
}

#[derive(Component)]
pub struct DespawnOnFinish;

pub fn despawn_tween_on_finish(
    mut time_runner_ended_reader: EventReader<TimeRunnerEnded>,
    q_children: Query<&Children>,
    q_no_collider: Query<(), Without<Collider>>,
    mut commands: Commands,
) {
    for event in time_runner_ended_reader.read() {
        if let Ok(children) = q_children.get(event.time_runner) {
            for child in children.iter() {
                if q_no_collider.contains(child) {
                    commands.entity(child).try_despawn();
                }
            }
        }
    }
}

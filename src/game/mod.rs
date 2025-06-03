use avian3d::{prelude::{Gravity, PhysicsDebugPlugin, PhysicsLayer}, PhysicsPlugins};
use bevy::prelude::*;
use bevy_tween::{bevy_time_runner::{TimeRunner, TimeRunnerEnded, TimeSpan, TimeSpanProgress}, TweenSystemSet};
use dissolve_gate::dissolve_gate_plugin;
use input::input_plugin;
use interaction::interaction_plugin;
use player::player_plugin;
use pressure_plate::pressure_plate_plugin;
use signals::signals_plugin;

pub mod player;
pub mod input;
pub mod interaction;
pub mod signals;
pub mod pressure_plate;
pub mod dissolve_gate;

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
    ))
    .insert_resource(Gravity(Vec3::NEG_Y * 19.6));

    app.add_systems( PostUpdate,
  despawn_tween_on_finish
    .after(TweenSystemSet::ApplyTween)
);
}

#[derive(PhysicsLayer, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Player,
    Signal,
    Device,
    Dissolve,
    Ignore
}

#[derive(Component)]
pub struct DespawnOnFinish;

pub fn despawn_tween_on_finish(
  mut time_runner_ended_reader: EventReader<TimeRunnerEnded>,
  mut commands: Commands,
) {
  for event in time_runner_ended_reader.read() {
    commands.entity(event.time_runner).despawn_related::<Children>();
  }
}

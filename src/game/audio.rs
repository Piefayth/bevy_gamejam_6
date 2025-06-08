use crate::asset_management::asset_loading::GameSounds;
use bevy::{audio::Volume, prelude::*};

#[derive(Resource)]
pub struct AudioSettings {
    pub volume: f32,
    pub volume_step: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume: 0.5,
            volume_step: 0.1,
        }
    }
}

#[derive(Component)]
pub struct BackgroundMusic;

#[derive(Component)]
pub struct VolumeUpButton;

#[derive(Component)]
pub struct VolumeDownButton;

pub fn audio_plugin(app: &mut App) {
    app.init_resource::<AudioSettings>()
        .add_systems(Update, (start_background_music, update_music_volume))
        .add_observer(handle_volume_up)
        .add_observer(handle_volume_down);
}

fn start_background_music(
    mut commands: Commands,
    game_sounds: Res<GameSounds>,
    audio_settings: Res<AudioSettings>,
    music_query: Query<Entity, With<BackgroundMusic>>,
) {
    // Only start music if it's not already playing
    if music_query.is_empty() && !game_sounds.song.is_weak() {
        commands.spawn((
            AudioPlayer::new(game_sounds.song.clone()),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::Linear(audio_settings.volume),
                ..default()
            },
            BackgroundMusic,
        ));
    }
}

pub fn handle_volume_up(
    _trigger: Trigger<Pointer<Click>>,
    mut audio_settings: ResMut<AudioSettings>,
) {
    audio_settings.volume = (audio_settings.volume + audio_settings.volume_step).min(1.0);
}

pub fn handle_volume_down(
    _trigger: Trigger<Pointer<Click>>,
    mut audio_settings: ResMut<AudioSettings>,
) {
    audio_settings.volume = (audio_settings.volume - audio_settings.volume_step).max(0.0);
}

fn update_music_volume(
    audio_settings: Res<AudioSettings>,
    mut music_query: Query<&mut AudioSink, With<BackgroundMusic>>,
) {
    if audio_settings.is_changed() {
        for mut sink in music_query.iter_mut() {
            sink.set_volume(Volume::Linear(audio_settings.volume));
        }
    }
}

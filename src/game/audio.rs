use crate::{
    asset_management::{
        asset_loading::GameSounds,
        asset_tag_components::{Door, PowerButton, PressurePlate},
    },
    game::{
        button::ButtonPressed,
        door::DoorOpened,
        pressure_plate::{PressurePlatePressed, PressurePlateReleased},
    },
};
use bevy::{
    audio::{DefaultSpatialScale, SpatialScale, Volume},
    prelude::*,
};
use std::time::Duration;

#[derive(Resource)]
pub struct AudioSettings {
    pub volume: f32,
    pub volume_step: f32,
    pub spatial_enabled: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume: 0.5,
            volume_step: 0.1,
            spatial_enabled: true,
        }
    }
}

#[derive(Resource)]
pub struct PressurePlateSoundCooldown {
    pub last_down_time: Option<Duration>,
    pub last_up_time: Option<Duration>,
    pub cooldown_duration: Duration,
}

impl Default for PressurePlateSoundCooldown {
    fn default() -> Self {
        Self {
            last_down_time: None,
            last_up_time: None,
            cooldown_duration: Duration::from_millis(100),
        }
    }
}

#[derive(Component)]
pub struct BackgroundMusic;

#[derive(Component)]
pub struct SpatialAudioListener;

#[derive(Component)]
pub struct VolumeUpButton;

#[derive(Component)]
pub struct VolumeDownButton;

pub fn audio_plugin(app: &mut App) {
    app.init_resource::<AudioSettings>()
        .init_resource::<PressurePlateSoundCooldown>()
        .insert_resource::<DefaultSpatialScale>(DefaultSpatialScale(SpatialScale::new(0.1)))
        .add_systems(Startup, setup_spatial_listener)
        .add_systems(
            Update,
            (
                start_background_music,
                update_music_volume,
                update_spatial_listener,
            ),
        )
        .add_observer(handle_volume_up)
        .add_observer(handle_volume_down);
}

fn setup_spatial_listener(mut commands: Commands) {
    commands.spawn((
        SpatialListener::new(2.0),
        SpatialAudioListener,
        Transform::default(),
        Visibility::default(),
    ));
}

fn update_spatial_listener(
    mut listener_query: Query<&mut Transform, (With<SpatialListener>, With<SpatialAudioListener>)>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<SpatialListener>)>,
) {
    // Update listener position to match camera/player position
    if let (Ok(mut listener_transform), Ok(camera_transform)) =
        (listener_query.single_mut(), camera_query.single())
    {
        listener_transform.translation = camera_transform.translation;
        listener_transform.rotation = camera_transform.rotation;
    }
}

fn start_background_music(
    mut commands: Commands,
    game_sounds: Res<GameSounds>,
    audio_settings: Res<AudioSettings>,
    music_query: Query<Entity, With<BackgroundMusic>>,
) {
    // Background music is typically non-spatial (plays everywhere equally)
    if music_query.is_empty() && !game_sounds.song.is_weak() {
        commands.spawn((
            AudioPlayer::new(game_sounds.song.clone()),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::Linear(audio_settings.volume),
                spatial: false, // Keep background music non-spatial
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

// For spatial sounds, we get the position from trigger.target()
pub fn button_pressed_audio(
    trigger: Trigger<ButtonPressed>,
    mut commands: Commands,
    game_sounds: Res<GameSounds>,
    audio_settings: Res<AudioSettings>,
    button_query: Query<&GlobalTransform, With<PowerButton>>,
) {
    if let Ok(button_transform) = button_query.get(trigger.target()) {
        spawn_spatial_sound(
            &mut commands,
            game_sounds.button2.clone(),
            button_transform.translation(),
            audio_settings.volume,
            audio_settings.spatial_enabled,
        );
    }
}

pub fn door_opened_audio(
    trigger: Trigger<DoorOpened>,
    mut commands: Commands,
    game_sounds: Res<GameSounds>,
    audio_settings: Res<AudioSettings>,
    door_query: Query<&GlobalTransform, With<Door>>,
) {
    if let Ok(door_transform) = door_query.get(trigger.target()) {
        spawn_spatial_sound(
            &mut commands,
            game_sounds.door_open.clone(),
            door_transform.translation(),
            audio_settings.volume * 2.0,
            audio_settings.spatial_enabled,
        );
    }
}

pub fn pressure_plate_pressed_audio(
    trigger: Trigger<PressurePlatePressed>,
    mut commands: Commands,
    game_sounds: Res<GameSounds>,
    audio_settings: Res<AudioSettings>,
    mut cooldown: ResMut<PressurePlateSoundCooldown>,
    time: Res<Time>,
    plate_query: Query<&GlobalTransform, With<PressurePlate>>,
) {
    let current_time = time.elapsed();

    let can_play = cooldown
        .last_down_time
        .map(|last_time| current_time.saturating_sub(last_time) >= cooldown.cooldown_duration)
        .unwrap_or(true);

    if can_play {
        if let Ok(plate_transform) = plate_query.get(trigger.target()) {
            spawn_spatial_sound(
                &mut commands,
                game_sounds.pressure_plate_down.clone(),
                plate_transform.translation(),
                audio_settings.volume,
                audio_settings.spatial_enabled,
            );

            cooldown.last_down_time = Some(current_time);
        }
    }
}

pub fn pressure_plate_released_audio(
    trigger: Trigger<PressurePlateReleased>,
    mut commands: Commands,
    game_sounds: Res<GameSounds>,
    audio_settings: Res<AudioSettings>,
    mut cooldown: ResMut<PressurePlateSoundCooldown>,
    time: Res<Time>,
    plate_query: Query<&GlobalTransform, With<PressurePlate>>,
) {
    let current_time = time.elapsed();

    let can_play = cooldown
        .last_up_time
        .map(|last_time| current_time.saturating_sub(last_time) >= cooldown.cooldown_duration)
        .unwrap_or(true);

    if can_play {
        if let Ok(plate_transform) = plate_query.get(trigger.target()) {
            spawn_spatial_sound(
                &mut commands,
                game_sounds.pressure_plate_up.clone(),
                plate_transform.translation(),
                audio_settings.volume,
                audio_settings.spatial_enabled,
            );

            cooldown.last_up_time = Some(current_time);
        }
    }
}

// Helper functions for spawning spatial vs non-spatial sounds
fn spawn_spatial_sound(
    commands: &mut Commands,
    audio_source: Handle<AudioSource>,
    position: Vec3,
    volume: f32,
    spatial_enabled: bool,
) {
    if spatial_enabled {
        commands.spawn((
            AudioPlayer::new(audio_source),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: Volume::Linear(volume),
                spatial: true,
                ..default()
            },
            Transform::from_translation(position),
        ));
    } else {
        spawn_non_spatial_sound(commands, audio_source, volume);
    }
}

fn spawn_non_spatial_sound(
    commands: &mut Commands,
    audio_source: Handle<AudioSource>,
    volume: f32,
) {
    commands.spawn((
        AudioPlayer::new(audio_source),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Despawn,
            volume: Volume::Linear(volume),
            spatial: false,
            ..default()
        },
    ));
}

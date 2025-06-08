use asset_management::asset_plugins;
use avian3d::prelude::{
    Collider, CollisionLayers, PhysicsGizmos, RigidBody, RigidBodyDisabled, RotationInterpolation
};
#[cfg(feature = "dev")]
use bevy::color::palettes::css::GREEN;
#[cfg(feature = "dev")]
use bevy::text::FontSmoothing;
use bevy::{
    color::palettes::{css::MAGENTA, tailwind::CYAN_400},
    core_pipeline::{
        bloom::{Bloom, BloomPrefilter},
        fxaa::Fxaa,
    },
    prelude::*,
};
#[cfg(feature = "dev")]
use bevy_dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
#[cfg(feature = "dev")]
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_tween::DefaultTweenPlugins;
use game::gameplay_plugins;
use rendering::{
    render_plugins, section_color_postprocess::PostProcessSettings,
    section_color_prepass::SectionsPrepass,
};
use ui::ui_plugins;

use crate::game::{dissolve_gate::Dissolveable, player::Player};

mod asset_management;
mod game;
mod rendering;
mod ui;

fn main() -> AppExit {
    App::new()
        .add_plugins((
            DefaultPlugins,
            #[cfg(feature = "dev")]
            EguiPlugin {
                enable_multipass_for_primary_context: true,
            },
            #[cfg(feature = "dev")]
            WorldInspectorPlugin::default(),
            DefaultTweenPlugins,
            #[cfg(feature = "dev")]
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont {
                        // Here we define size of our overlay
                        font_size: 42.0,
                        // If we want, we can use a custom font
                        font: default(),
                        // We could also disable font smoothing,
                        font_smoothing: FontSmoothing::default(),
                        ..default()
                    },
                    // We can also change color of the overlay
                    text_color: GREEN.into(),
                    // We can also set the refresh interval for the FPS counter
                    refresh_interval: core::time::Duration::from_millis(100),
                    enabled: true,
                },
            },
            asset_plugins,
            render_plugins,
            ui_plugins,
            gameplay_plugins,
        ))
        //.insert_resource::<AmbientLight>(AmbientLight { color: WHITE.into(), brightness: 300000., ..default() })
        .init_state::<GameState>()
        .insert_gizmo_config(
            PhysicsGizmos {
                shapecast_color: Some(CYAN_400.into()),
                shapecast_shape_color: Some(MAGENTA.into()),
                ..default()
            },
            GizmoConfig::default(),
        )
        .add_systems(Startup, spawn_main_camera)
        .add_systems(
            FixedPreUpdate,
            (rigid_body_distance_system, collider_distance_system).chain(),
        )
        .init_resource::<RigidBodyDistanceConfig>()
        .init_resource::<ColliderDistanceConfig>()
        .run()
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[states(scoped_entities)]
pub enum GameState {
    #[default]
    Loading,
    MainMenu,
    Playing,
}

#[derive(Component)]
pub struct MainCamera;

fn spawn_main_camera(mut commands: Commands) {
    let mut bloom = Bloom::NATURAL;
    bloom.prefilter = BloomPrefilter {
        threshold: 1.0, // only bloom hdr values
        threshold_softness: 0.0,
    };

    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        SectionsPrepass,
        bloom,
        //VignetteSettings::new(0.5, 1.1, Color::BLACK, 1.0),
        RotationInterpolation,
        Projection::Perspective(PerspectiveProjection {
            fov: 1.396,
            ..default()
        }),
        Msaa::Off,
        Fxaa {
            enabled: true,
            ..default()
        },
        PostProcessSettings {
            stroke_color: Color::BLACK.into(),
            width: 2,
            ..default()
        },
    ));
}

#[derive(Resource)]
pub struct RigidBodyDistanceConfig {
    pub max_distance: f32,
    // Optional: hysteresis to prevent flickering when entities are right at the boundary
    pub hysteresis: f32,
}

impl Default for RigidBodyDistanceConfig {
    fn default() -> Self {
        Self {
            max_distance: 500.0,
            hysteresis: 15.0, // Bodies re-enable at max_distance, disable at max_distance + hysteresis
        }
    }
}

pub fn rigid_body_distance_system(
    mut commands: Commands,
    config: Res<RigidBodyDistanceConfig>,
    player_query: Query<&GlobalTransform, With<Player>>,
    mut rigidbody_query: Query<
        (Entity, &GlobalTransform, Option<&RigidBodyDisabled>),
        (With<RigidBody>, Without<Player>),
    >,
) {
    // Get player position - return early if no player found
    let player_transform = match player_query.single() {
        Ok(transform) => transform,
        Err(_) => return, // No player or multiple players
    };

    let player_pos = player_transform.translation();

    for (entity, transform, disabled_component) in rigidbody_query.iter_mut() {
        let distance = player_pos.distance(transform.translation());
        let is_disabled = disabled_component.is_some();

        match (is_disabled, distance <= config.max_distance) {
            // Currently disabled but should be enabled (within range)
            (true, true) => {
                commands.entity(entity).remove::<RigidBodyDisabled>();
            }
            // Currently enabled but should be disabled (outside range + hysteresis)
            (false, false) if distance > config.max_distance + config.hysteresis => {
                commands.entity(entity).insert(RigidBodyDisabled);
            }
            // No change needed
            _ => {}
        }
    }
}

#[derive(Resource)]
pub struct ColliderDistanceConfig {
    pub max_distance: f32,
    // Optional: hysteresis to prevent flickering when entities are right at the boundary
    pub hysteresis: f32,
}

impl Default for ColliderDistanceConfig {
    fn default() -> Self {
        Self {
            max_distance: 500.0,
            hysteresis: 15.0, // Bodies re-enable at max_distance, disable at max_distance + hysteresis
        }
    }
}

#[derive(Component)]
pub struct DisabledByDistance {
    pub old_layers: CollisionLayers,
}

pub fn collider_distance_system(
    mut commands: Commands,
    config: Res<ColliderDistanceConfig>,
    player_query: Query<&GlobalTransform, With<Player>>,
    mut collider_query: Query<
        (
            Entity,
            &GlobalTransform,
            &mut CollisionLayers,
            Option<&DisabledByDistance>,
        ),
        (With<Collider>, Without<Player>),
    >,
) {
    let player_transform = match player_query.single() {
        Ok(transform) => transform,
        Err(_) => return, // No player found, do nothing.
    };

    let player_pos = player_transform.translation();

    for (entity, transform, mut layers, disabled_marker) in collider_query.iter_mut() {
        let distance = player_pos.distance(transform.translation());
        let is_currently_disabled_by_us = disabled_marker.is_some();

        let should_be_disabled = distance > config.max_distance + config.hysteresis;
        let should_be_enabled = distance <= config.max_distance;

        if should_be_enabled && is_currently_disabled_by_us {
            if let Some(marker) = disabled_marker {
                *layers = marker.old_layers;
                commands.entity(entity).remove::<DisabledByDistance>();
            }
        }
        else if should_be_disabled && !is_currently_disabled_by_us && *layers != CollisionLayers::NONE && *layers != CollisionLayers::DEFAULT {
            commands.entity(entity).insert(DisabledByDistance {
                old_layers: *layers,
            });
            *layers = CollisionLayers::NONE;
        }
    }
}

const DISSOLVE_Y_THRESHOLD: f32 = -50.0;

// System to despawn entities below the threshold
pub fn dissolve_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<Dissolveable>>,
) {
    for (entity, transform) in query.iter() {
        if transform.translation.y < DISSOLVE_Y_THRESHOLD {
            commands.entity(entity).despawn();
        }
    }
}

use asset_management::asset_plugins;
use avian3d::prelude::{PhysicsGizmos, RotationInterpolation};
use bevy::{color::palettes::{css::MAGENTA, tailwind::CYAN_400}, core_pipeline::{bloom::{Bloom, BloomPrefilter}, fxaa::Fxaa, tonemapping::Tonemapping}, pbr::{light_consts::lux, CascadeShadowConfigBuilder}, prelude::*};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_tween::DefaultTweenPlugins;
use game::gameplay_plugins;
use rendering::{render_plugins, section_color_postprocess::PostProcessSettings, section_color_prepass::SectionsPrepass};
use ui::ui_plugins;

mod asset_management;
mod rendering;
mod ui;
mod game;

fn main() -> AppExit {
    App::new()
    .add_plugins((
        DefaultPlugins,
        EguiPlugin {
            enable_multipass_for_primary_context: true,
        },
        WorldInspectorPlugin::default(),
        DefaultTweenPlugins,
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

fn spawn_main_camera(
    mut commands: Commands,
) {
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
        Projection::Perspective(PerspectiveProjection { fov: 1.396, ..default() }),
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

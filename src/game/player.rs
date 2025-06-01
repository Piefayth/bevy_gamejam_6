use std::f32::consts::{self, FRAC_PI_4};

use avian3d::{
    math::PI,
    prelude::{Collider, LockedAxes, PhysicsSet, RigidBody, TransformInterpolation},
};
use bevy::{app::FixedMain, picking::pointer::PointerInteraction, prelude::*};

use bevy_enhanced_input::{
    EnhancedInputSystem,
    events::Completed,
    prelude::{ActionValue, Actions},
};
use bevy_tnua::{builtins::TnuaBuiltinClimb, prelude::*};
use bevy_tnua_avian3d::*;

use crate::{GameState, MainCamera, ui::crosshair::CrosshairState};

use super::input::{FixedInputContext, Jump, Look, Movement, UpdateInputContext, UseInteract};

pub fn player_plugin(app: &mut App) {
    app.add_plugins((
        TnuaControllerPlugin::new(FixedUpdate),
        TnuaAvian3dPlugin::new(FixedUpdate),
    ))
    .add_systems(
        PreUpdate,
        rotate_camera
            .after(EnhancedInputSystem)
            .run_if(in_state(GameState::Playing).and(in_state(CrosshairState::Shown))),
    )
    .add_systems(
        PostUpdate,
        camera_follow_player
            .after(RunFixedMainLoopSystem::AfterFixedMainLoop)
            .before(TransformSystem::TransformPropagate)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        FixedUpdate,
        (move_player, jump).run_if(in_state(GameState::Playing)),
    )
    .add_systems(OnEnter(GameState::Playing), spawn_player)
    .register_type::<PlayerSpawnPoint>();
}

#[derive(Component)]
pub struct Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PlayerSpawnPoint {
    pub unused: bool,
}

fn spawn_player(
    mut commands: Commands,
    spawn_point: Single<&Transform, With<PlayerSpawnPoint>>,
    mut camera: Single<&mut Transform, (With<MainCamera>, Without<PlayerSpawnPoint>)>,
) {
    commands.spawn((
        spawn_point.clone(),
        RigidBody::Dynamic,
        Collider::capsule(1.5, 8.0),
        TnuaController::default(), // todo: what options
        TnuaAvian3dSensorShape(Collider::capsule(1.49, 7.99)),
        LockedAxes::ROTATION_LOCKED,
        Player,
        StateScoped(GameState::Playing),
        TransformInterpolation,
    ));

    // set camera rotation to away from origin.
    **camera = camera.looking_at(Vec3::ZERO, Vec3::Y);
    camera.rotate_y(PI);
}

const PLAYER_VELOCITY: f32 = 30.0;

fn move_player(
    mut controller: Single<&mut TnuaController>,
    input: Single<&Actions<FixedInputContext>>,
    camera: Single<&Transform, With<MainCamera>>,
) {
    if let Ok(action) = input.value::<Movement>() {
        if let ActionValue::Axis2D(movement) = action {
            let camera_forward = camera.forward();
            let camera_right = camera.right();

            let forward_horizontal = Vec3::new(camera_forward.x, 0.0, camera_forward.z).normalize();
            let right_horizontal = Vec3::new(camera_right.x, 0.0, camera_right.z).normalize();

            let direction = forward_horizontal * movement.y + right_horizontal * movement.x;

            controller.basis(TnuaBuiltinWalk {
                desired_velocity: direction * PLAYER_VELOCITY,
                float_height: 2.0,
                max_slope: FRAC_PI_4,
                acceleration: 120.,
                air_acceleration: 120.,
                ..default()
            });
        }
    }
}

fn jump(mut controller: Single<&mut TnuaController>, input: Single<&Actions<FixedInputContext>>) {
    if let Ok(action) = input.value::<Jump>() {
        if let ActionValue::Bool(jump) = action {
            if jump {
                controller.action(TnuaBuiltinJump {
                    height: 8.0,
                    takeoff_extra_gravity: 60.,
                    fall_extra_gravity: 60.,
                    ..default()
                });
            }
        }
    }
}

const MAX_PITCH: f32 = 89.0_f32.to_radians(); // Limit vertical look angle
const SENSITIVITY: f32 = 0.1;

fn rotate_camera(
    input: Single<&Actions<UpdateInputContext>>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    if let Ok(action) = input.value::<Look>() {
        if let ActionValue::Axis2D(look) = action {
            let scaled_sensitivity = SENSITIVITY * time.delta_secs();

            camera.rotate_y(-look.x * scaled_sensitivity);

            let current_pitch = camera.rotation.to_euler(EulerRot::YXZ).1;
            let new_pitch =
                (current_pitch - look.y * scaled_sensitivity).clamp(-MAX_PITCH, MAX_PITCH);
            let pitch_delta = new_pitch - current_pitch;

            camera.rotate_local_x(pitch_delta);
        }
    }
}

const CAMERA_HEIGHT: f32 = 6.0;
fn camera_follow_player(
    maybe_player: Option<Single<&Transform, With<Player>>>,
    mut camera: Single<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    if let Some(player) = maybe_player {
        camera.translation = player
            .translation
            .with_y(player.translation.y + CAMERA_HEIGHT);
    }
}

use std::time::Duration;

use avian3d::prelude::{
    ColliderConstructor, CollisionEventsEnabled, CollisionLayers,
    OnCollisionStart, RigidBody, RigidBodyColliders, Sensor,
};
use bevy::prelude::*;
use bevy_tween::{
    combinator::tween,
    interpolate::translation,
    prelude::{AnimationBuilderExt, EaseKind, Interpolator},
    tween::{AnimationTarget, TargetAsset, TargetComponent},
};

use crate::{
    GameState,
    asset_management::{
        asset_loading::GameAssets,
    },
    rendering::unlit_material::UnlitMaterial,
};

use super::{
    door::PoweredTimer, pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY}, DespawnOnFinish, GameLayer
};

pub fn signals_plugin(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            despawn_after_system,
            signal_after_delay,
        )
            .run_if(in_state(GameState::Playing)),
    );
}

#[derive(Component)]
pub struct Signal;

#[derive(Component)]
pub struct Powered;

#[derive(Component, Default, Deref, DerefMut)]
pub struct OwnedObjects(pub Vec<Entity>);

#[derive(Reflect, Debug)]
pub struct MaterialIntensityInterpolator {
    pub start: f32,
    pub end: f32,
}

impl Interpolator for MaterialIntensityInterpolator {
    type Item = UnlitMaterial;

    fn interpolate(&self, material: &mut Self::Item, progress: f32) {
        material.extension.params.intensity = self.start + (self.end - self.start) * progress;
    }
}


pub fn default_signal_collisions(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_signals: Query<(), With<Signal>>,
    q_powered: Query<(), (With<Powered>, Without<PoweredTimer>)>,
) {
    if let Some(signaled_body) = trigger.body {
        if q_signals.contains(trigger.collider) && !q_powered.contains(trigger.collider) && !q_powered.contains(signaled_body) {
            commands.entity(signaled_body).trigger(DirectSignal);
            commands.entity(trigger.collider).despawn();
        }
    }
}



#[derive(Component)]
pub struct SignalAfterDelay {
    pub delay_ms: u32,
    pub spawn_time: Duration,
    pub signal_size: f32
}


#[derive(Event)]
pub struct DirectSignal;

pub const MAX_SIGNAL_TRAVEL_DIST: f32 = 500.;
pub const MAX_SIGNAL_LIFETIME_SECS: u64 = 10;

fn signal_after_delay(
    mut commands: Commands,
    q_waiting: Query<(Entity, &SignalAfterDelay, &ChildOf)>,
    q_global_transform: Query<&GlobalTransform>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    game_assets: Res<GameAssets>,
) {
    for (entity, signal_delay, child_of) in &q_waiting {
        // Check if the delay time has elapsed
        let elapsed_since_spawn = time.elapsed() - signal_delay.spawn_time;

        if elapsed_since_spawn >= Duration::from_millis(signal_delay.delay_ms as u64) {
            // Delay is complete, spawn the signal
            let y_amount_to_look_good = if signal_delay.signal_size > 10. { // actually depends on where we consider the visual "launch point" on each spitter model to be
                20.
            } else {
                10.
            };

            if let Ok(global_transform) = q_global_transform.get(child_of.0) {
                let spitter_forward = -global_transform.forward();
                let start_loc =
                    global_transform.translation() + Vec3::Y * y_amount_to_look_good + spitter_forward * 10.;

                // Create transform that faces the direction the spitter is pointing
                let signal_transform =
                    Transform::from_translation(start_loc).looking_to(-spitter_forward, Vec3::Y);

                let signal_indicator = commands
                    .spawn((
                        ColliderConstructor::Cuboid {
                            x_length: signal_delay.signal_size,
                            y_length: signal_delay.signal_size,
                            z_length: 2.0,
                        },
                        CollisionLayers::new(GameLayer::Signal, [GameLayer::Device]),
                        Mesh3d(meshes.add(Cuboid::new(signal_delay.signal_size, signal_delay.signal_size, 2.0))),
                        MeshMaterial3d(game_assets.cyan_signal_material.clone()),
                        signal_transform,
                        AnimationTarget,
                        CollisionEventsEnabled,
                        RigidBody::Kinematic,
                        Sensor,
                        Signal,
                        DespawnAfter::new(Duration::from_secs(MAX_SIGNAL_LIFETIME_SECS)), // Despawn after 10 seconds
                    ))
                    .id();
                
                commands.entity(signal_indicator).animation().insert(tween(
                    Duration::from_secs(MAX_SIGNAL_LIFETIME_SECS),
                    EaseKind::Linear,
                    TargetComponent::marker().with(translation(
                        start_loc,
                        start_loc + spitter_forward * MAX_SIGNAL_TRAVEL_DIST,
                    )),
                ));

                // Remove the SignalAfterDelay component since we've spawned the signal
                commands.entity(entity).remove::<SignalAfterDelay>();
            }
        }
    }
}

#[derive(Component)]
pub struct DespawnAfter {
    timer: Timer,
}

impl DespawnAfter {
    pub fn new(duration: Duration) -> Self {
        Self {
            timer: Timer::new(duration, TimerMode::Once),
        }
    }
}

fn despawn_after_system(
    mut commands: Commands,
    mut q_despawn_after: Query<(Entity, &mut DespawnAfter)>,
    time: Res<Time>,
) {
    for (entity, mut despawn_after) in &mut q_despawn_after {
        despawn_after.timer.tick(time.delta());

        if despawn_after.timer.finished() {
            commands.entity(entity).try_despawn();
        }
    }
}

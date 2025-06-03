use std::time::Duration;

use avian3d::prelude::{
    Collider, ColliderConstructor, CollisionEventsEnabled, CollisionLayers, ExternalImpulse,
    OnCollisionStart, RigidBody, RigidBodyColliders, RotationInterpolation, Sensor,
    TransformInterpolation,
};
use bevy::prelude::*;
use bevy_tween::{
    combinator::{sequence, tween},
    interpolate::translation,
    prelude::{AnimationBuilderExt, EaseKind, Interpolator},
    tween::{AnimationTarget, IntoTarget, TargetAsset, TargetComponent},
};

use crate::{
    GameState,
    asset_management::{
        asset_loading::GameAssets,
        asset_tag_components::{CubeSpitter, SignalSpitter, WeightedCube, WeightedCubeColors},
    },
    rendering::unlit_material::UnlitMaterial,
};

use super::{DespawnOnFinish, GameLayer};

pub fn signals_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            despawn_after_system,
            register_cube_spitter_signals,
            register_cube_signals,
            cube_receive_power,
            register_signal_spitter_signals,
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

// This gets added to anything that can produce a signal through physical contact
pub fn cube_spitter_consume_signal(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_signals: Query<(), With<Signal>>,
    mut q_cube_spitters: Query<(&CubeSpitter, &Transform, &mut OwnedObjects)>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    game_assets: Res<GameAssets>,
    q_parents: Query<&ChildOf>,
) {
    if let Some(colliding_body) = trigger.body {
        if let Ok(child_of) = q_parents.get(colliding_body) {
            if q_signals.contains(trigger.collider) {
                if let Ok((spitter, spitter_transform, mut spitter_owned_objects)) =
                    q_cube_spitters.get_mut(child_of.0)
                {
                    // material is on the child / the actual collider
                    if let Ok(spitter_material_handle) = q_unlit_objects.get(trigger.target()) {
                        commands
                            .entity(trigger.target())
                            .animation()
                            .insert(sequence((
                                // First tween: 1.0 → 5.0
                                tween(
                                    Duration::from_secs(1),
                                    EaseKind::CubicOut,
                                    TargetAsset::Asset(spitter_material_handle.clone_weak()).with(
                                        MaterialIntensityInterpolator {
                                            start: 1.0,
                                            end: 10.0,
                                        },
                                    ),
                                ),
                                // Second tween: 5.0 → 1.0
                                tween(
                                    Duration::from_secs(1),
                                    EaseKind::CubicIn,
                                    TargetAsset::Asset(spitter_material_handle.clone_weak()).with(
                                        MaterialIntensityInterpolator {
                                            start: 10.0,
                                            end: 1.0,
                                        },
                                    ),
                                ),
                            )))
                            .insert(DespawnOnFinish);

                        // despawn the old owned objects and clear the list
                        for object in spitter_owned_objects.iter() {
                            commands.entity(*object).despawn();
                        }
                        spitter_owned_objects.clear();

                        let cube_id = commands
                            .spawn((
                                SceneRoot(match spitter.color {
                                    WeightedCubeColors::Cyan => {
                                        game_assets.weighted_cube_cyan.clone()
                                    }
                                }),
                                Transform::from_translation(
                                    spitter_transform.translation + Vec3::Y * 14.5 + -Vec3::X * 20.,
                                ),
                                RigidBody::Dynamic,
                                TransformInterpolation,
                                RotationInterpolation,
                                ExternalImpulse::new(Vec3::new(-5000., 0., 0.)),
                                WeightedCube {
                                    color: WeightedCubeColors::Cyan,
                                },
                            ))
                            .id();

                        // add the new cube to the owned objects
                        spitter_owned_objects.0.push(cube_id);

                        // despawn the signal
                        commands.entity(trigger.collider).despawn();
                    }
                }
            }
        }
    }
}

fn register_cube_spitter_signals(
    mut commands: Commands,
    q_new_spitter: Query<(Entity, &Children), Added<CubeSpitter>>,
) {
    // for static geo like spitters, the tag is on the parent, but the rigid body is on the child
    for (spitter_entity, spitter_children) in &q_new_spitter {
        // warning: we actually expect there to only be ever one spitter child
        // this explodes if not
        commands
            .entity(spitter_entity)
            .insert(OwnedObjects::default());

        for spitter_child in spitter_children.iter() {
            commands
                .entity(spitter_child)
                .insert((
                    CollisionEventsEnabled,
                    CollisionLayers::new(GameLayer::Device, [GameLayer::Signal, GameLayer::Player]),
                    AnimationTarget,
                ))
                .observe(cube_spitter_consume_signal);
        }
    }
}

fn cube_consume_signal(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_signals: Query<(), With<Signal>>,
    q_powered: Query<(), (With<Powered>)>,
) {
    if let Some(device_body) = trigger.body {
        if q_signals.contains(trigger.collider) {
            if !q_powered.contains(device_body) {
                commands.entity(device_body).insert(Powered);
                commands.entity(trigger.collider).despawn();
            }
        }
    }
}

fn cube_receive_power(
    mut commands: Commands,
    q_powered_cube: Query<(Entity, &RigidBodyColliders), (With<WeightedCube>, Added<Powered>)>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for (powered_cube, powered_cube_colliders) in &q_powered_cube {
        for collider_entity in powered_cube_colliders.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                commands
                    .entity(collider_entity)
                    .animation()
                    .insert(tween(
                        Duration::from_secs(1),
                        EaseKind::CubicOut,
                        TargetAsset::Asset(material_handle.clone_weak()).with(
                            MaterialIntensityInterpolator {
                                start: 1.0,
                                end: 5.0,
                            },
                        ),
                    ))
                    .insert(DespawnOnFinish);
            }
        }
    }
}

fn register_cube_signals(
    mut commands: Commands,
    q_new_cube: Query<
        (Entity, &RigidBodyColliders),
        (Added<RigidBodyColliders>, With<WeightedCube>),
    >,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    // probably not the right place, but we need to give each cube a dedicated material if it will be powered individually

    for (cube_entity, cube_children) in &q_new_cube {
        for cube_child in cube_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(cube_child) {
                let old_material = unlit_materials.get(material_handle).unwrap().clone();

                commands
                    .entity(cube_child)
                    .insert((
                        CollisionEventsEnabled,
                        CollisionLayers::new(
                            GameLayer::Device,
                            [GameLayer::Signal, GameLayer::Player, GameLayer::Default],
                        ),
                        AnimationTarget,
                        MeshMaterial3d(unlit_materials.add(old_material)),
                    ))
                    .observe(cube_consume_signal);
            }
        }
    }
}

fn register_signal_spitter_signals(
    mut commands: Commands,
    q_new_signal_spitter: Query<
        (Entity, &RigidBodyColliders),
        (
            Added<RigidBodyColliders>,
            With<SignalSpitter>,
            Without<Collider>,
        ), // there is a rigidbody reassignment to the parent entity in the interaction system for consistency, so we need to filter the "wrong" entity with the collider on it
    >,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for (spitter_entity, spitter_children) in &q_new_signal_spitter {
        for spitter_child in spitter_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(spitter_child) {
                let old_material = unlit_materials.get(material_handle).unwrap().clone();

                commands
                    .entity(spitter_child)
                    .insert((
                        CollisionEventsEnabled,
                        CollisionLayers::new(
                            GameLayer::Device,
                            [GameLayer::Dissolve, GameLayer::Signal, GameLayer::Player, GameLayer::Default],
                        ),
                        AnimationTarget,
                        MeshMaterial3d(unlit_materials.add(old_material)),
                    ))
                    .observe(signal_spitter_consume_signal);
            }
        }
    }
}

const SIGNAL_CHARGE_MS: u32 = 1000;

fn signal_spitter_consume_signal(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_signals: Query<(), With<Signal>>,
    q_spitter: Query<(&RigidBodyColliders), (With<SignalSpitter>)>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    time: Res<Time>,
) {
    if let Some(spitter_body) = trigger.body {
        if q_signals.contains(trigger.collider) {
            // was a signal what hit the spitter?
            if let Ok(spitter_colliders) = q_spitter.get(spitter_body) {
                for collider_entity in spitter_colliders.iter() {
                    if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                        commands.entity(spitter_body).with_child(SignalAfterDelay {
                            delay_ms: SIGNAL_CHARGE_MS,
                            spawn_time: time.elapsed(),
                        });
                        commands
                            .entity(collider_entity)
                            .animation()
                            .insert(sequence((
                                tween(
                                    Duration::from_millis(SIGNAL_CHARGE_MS.into()),
                                    EaseKind::CubicOut,
                                    TargetAsset::Asset(material_handle.clone_weak()).with(
                                        MaterialIntensityInterpolator {
                                            start: 1.0,
                                            end: 20.0,
                                        },
                                    ),
                                ),
                                tween(
                                    Duration::from_millis(SIGNAL_CHARGE_MS.into()),
                                    EaseKind::CubicOut,
                                    TargetAsset::Asset(material_handle.clone_weak()).with(
                                        MaterialIntensityInterpolator {
                                            start: 20.0,
                                            end: 1.0,
                                        },
                                    ),
                                ),
                            )))
                            .insert(DespawnOnFinish);
                    }
                }

                commands.entity(trigger.collider).despawn(); // despawn the signal
            }
        }
    }
}

#[derive(Component)]
pub struct SignalAfterDelay {
    delay_ms: u32,
    spawn_time: Duration,
}

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
            if let Ok(global_transform) = q_global_transform.get(child_of.0) {
                let spitter_forward = -global_transform.forward();
                let start_loc = global_transform.translation() + Vec3::Y * 10. + spitter_forward * 5.;

                // Create transform that faces the direction the spitter is pointing
                let signal_transform =
                    Transform::from_translation(start_loc).looking_to(-spitter_forward, Vec3::Y);

                let signal_indicator = commands
                    .spawn((
                        ColliderConstructor::Cuboid {
                            x_length: 10.,
                            y_length: 10.,
                            z_length: 2.0,
                        },
                        CollisionLayers::new(GameLayer::Signal, [GameLayer::Device]),
                        Mesh3d(meshes.add(Cuboid::new(10., 10., 2.0))),
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
            commands.entity(entity).despawn();
        }
    }
}

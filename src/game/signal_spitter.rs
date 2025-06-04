use std::time::Duration;

use avian3d::prelude::{
    Collider, ColliderConstructor, CollisionEventsEnabled, CollisionLayers, ExternalImpulse,
    OnCollisionStart, RigidBody, RigidBodyColliders, RotationInterpolation, Sensor,
    TransformInterpolation,
};
use bevy::prelude::*;
use bevy_tween::{
    bevy_time_runner::TimeSpan, combinator::{sequence, tween}, interpolate::translation, prelude::{AnimationBuilderExt, EaseKind, Interpolator}, tween::{AnimationTarget, IntoTarget, TargetAsset, TargetComponent}
};

use crate::{
    GameState,
    asset_management::{
        asset_loading::GameAssets,
        asset_tag_components::{CubeSpitter, SignalSpitter, WeightedCube, WeightedCubeColors},
    },
    rendering::unlit_material::UnlitMaterial,
};

use super::{
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY}, signals::{default_signal_collisions, DirectSignal, MaterialIntensityInterpolator, Powered, SignalAfterDelay}, DespawnOnFinish, GameLayer
};

// Component to track continuous emission state
#[derive(Component)]
pub struct ContinuousEmission {
    pub interval_ms: u32,
}

impl Default for ContinuousEmission {
    fn default() -> Self {
        Self {
            interval_ms: 1000, // 1 second default interval
        }
    }
}

pub fn signal_spitter_plugin(app: &mut App) {
    app.add_systems(Update, (register_signal_spitter_signals, handle_continuous_emission));
}

fn signal_spitter_direct_signal(
    trigger: Trigger<DirectSignal>,
    mut commands: Commands,
    q_spitter: Query<(&RigidBodyColliders), (With<SignalSpitter>)>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    time: Res<Time>,
) {
    if let Ok(spitter_colliders) = q_spitter.get(trigger.target()) {
        for collider_entity in spitter_colliders.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                commands
                    .entity(trigger.target())
                    .with_child(SignalAfterDelay {
                        delay_ms: (POWER_ANIMATION_DURATION_SEC * 1000.) as u32,
                        spawn_time: time.elapsed(),
                    });
                commands
                    .entity(collider_entity)
                    .animation()
                    .insert(sequence((
                        tween(
                            Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 1000.) as u64),
                            EaseKind::CubicOut,
                            TargetAsset::Asset(material_handle.clone_weak()).with(
                                MaterialIntensityInterpolator {
                                    start: 1.0,
                                    end: POWER_MATERIAL_INTENSITY,
                                },
                            ),
                        ),
                        tween(
                            Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 1000.) as u64),
                            EaseKind::CubicOut,
                            TargetAsset::Asset(material_handle.clone_weak()).with(
                                MaterialIntensityInterpolator {
                                    start: POWER_MATERIAL_INTENSITY,
                                    end: 1.0,
                                },
                            ),
                        ),
                    )))
                    .insert(DespawnOnFinish);
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
        ),
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
                            [
                                GameLayer::Dissolve,
                                GameLayer::Signal,
                                GameLayer::Player,
                                GameLayer::Default,
                            ],
                        ),
                        AnimationTarget,
                        MeshMaterial3d(unlit_materials.add(old_material)),
                    ))
                    .observe(default_signal_collisions);
            }
        }
        commands
            .entity(spitter_entity)
            .insert(ContinuousEmission::default()) // Add continuous emission component
            .observe(signal_spitter_direct_signal)
            .observe(signal_spitter_receive_power)
            .observe(signal_spitter_lose_power);
    }
}

fn signal_spitter_receive_power(
    trigger: Trigger<OnAdd, Powered>,
    mut commands: Commands, 
    q_signal_spitter: Query<(Entity, &RigidBodyColliders, &ContinuousEmission), With<SignalSpitter>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
    time: Res<Time>,
) {
    if let Ok((signal_spitter, signal_spitter_children, continuous_emission)) = q_signal_spitter.get(trigger.target()) {
        for collider_entity in signal_spitter_children.iter() {
            if let Ok(collider_children) = q_children.get(collider_entity) {
                for child in collider_children.iter() {
                    // the tweens are children of the collider/material entities
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }
            }

            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                if let Some(material) = unlit_materials.get(material_handle) {
                    let current_intensity = material.extension.params.intensity;
                    let intensity_ratio = (POWER_MATERIAL_INTENSITY - current_intensity)
                        / (POWER_MATERIAL_INTENSITY - 1.0);
                    let duration_secs = POWER_ANIMATION_DURATION_SEC * intensity_ratio.max(0.1); // Minimum 0.1 seconds

                    commands
                        .entity(collider_entity)
                        .animation()
                        .insert(tween(
                            Duration::from_secs_f32(duration_secs),
                            EaseKind::CubicOut,
                            TargetAsset::Asset(material_handle.clone_weak()).with(
                                MaterialIntensityInterpolator {
                                    start: current_intensity,
                                    end: POWER_MATERIAL_INTENSITY,
                                },
                            ),
                        ))
                        .insert(DespawnOnFinish);
                }
            }
        }

        // Start continuous emission when powered
        commands
            .entity(signal_spitter)
            .with_child(SignalAfterDelay {
                delay_ms: continuous_emission.interval_ms,
                spawn_time: time.elapsed(),
            });
    }
}

fn signal_spitter_lose_power(
    trigger: Trigger<OnRemove, Powered>,
    mut commands: Commands,
    q_signal_spitter: Query<(Entity, &RigidBodyColliders), With<SignalSpitter>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children>,
    q_signal_after_delay: Query<(), With<SignalAfterDelay>>,
) {
    if let Ok((signal_spitter, signal_spitter_children)) = q_signal_spitter.get(trigger.target()) {
        for collider_entity in signal_spitter_children.iter() {
            if let Ok(collider_children) = q_children.get(collider_entity) {
                for child in collider_children.iter() {
                    // the tweens are children of the collider/material entities
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }
            }

            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                if let Some(material) = unlit_materials.get(material_handle) {
                    let current_intensity = material.extension.params.intensity;
                    let intensity_ratio = (current_intensity - 1.0) / (POWER_MATERIAL_INTENSITY - 1.0);
                    let duration_secs = POWER_ANIMATION_DURATION_SEC * intensity_ratio.max(0.1);

                    commands
                        .entity(collider_entity)
                        .animation()
                        .insert(tween(
                            Duration::from_secs_f32(duration_secs),
                            EaseKind::CubicOut,
                            TargetAsset::Asset(material_handle.clone_weak()).with(
                                MaterialIntensityInterpolator {
                                    start: current_intensity,
                                    end: 1.0,
                                },
                            ),
                        ))
                        .insert(DespawnOnFinish);
                }
            }
        }

        // Remove all SignalAfterDelay children to stop continuous emission
        if let Ok(children) = q_children.get(signal_spitter) {
            for child in children.iter() {
                if q_signal_after_delay.contains(child) {
                    commands.entity(child).despawn();
                }
            }
        }
    }    
}

fn handle_continuous_emission(
    mut commands: Commands,
    q_powered_spitters: Query<(Entity, &ContinuousEmission), (With<SignalSpitter>, With<Powered>)>,
    q_children: Query<&Children>,
    q_signal_after_delay: Query<(), With<SignalAfterDelay>>,
    time: Res<Time>,
) {
    for (spitter_entity, continuous_emission) in &q_powered_spitters {
        // Check if this spitter has any active SignalAfterDelay children
        let mut has_pending_signal = false;
        if let Ok(children) = q_children.get(spitter_entity) {
            for child in children.iter() {
                if q_signal_after_delay.contains(child) {
                    has_pending_signal = true;
                    break;
                }
            }
        }

        // If no pending signal, add a new one to continue the cycle
        if !has_pending_signal {
            commands
                .entity(spitter_entity)
                .with_child(SignalAfterDelay {
                    delay_ms: continuous_emission.interval_ms,
                    spawn_time: time.elapsed(),
                });
        }
    }
}

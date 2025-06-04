use std::time::Duration;

use avian3d::{
    parry::na::Owned,
    prelude::{
        Collider, CollisionEventsEnabled, CollisionLayers, ExternalImpulse, RigidBody,
        RigidBodyColliders, RotationInterpolation, TransformInterpolation,
    },
};
use bevy::prelude::*;
use bevy_tween::{
    bevy_time_runner::TimeSpan,
    combinator::{sequence, tween},
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetAsset},
};

use crate::{
    asset_management::{
        asset_loading::GameAssets,
        asset_tag_components::{
            SignalSpitter, StandingCubeSpitter, WeightedCube, WeightedCubeColors,
        },
    },
    rendering::unlit_material::UnlitMaterial,
};

use super::{
    DespawnOnFinish, GameLayer,
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY},
    signal_spitter::ContinuousEmission,
    signals::{
        DirectSignal, MaterialIntensityInterpolator, OwnedObjects, Powered, SignalAfterDelay,
        default_signal_collisions,
    },
};

pub fn standing_cube_spitter_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            register_standing_cube_spitter_signals,
            handle_continuous_cube_emission,
            cube_after_delay,
        ),
    );
}

fn register_standing_cube_spitter_signals(
    mut commands: Commands,
    q_new_signal_spitter: Query<
        (Entity, &RigidBodyColliders),
        (
            Added<RigidBodyColliders>,
            With<StandingCubeSpitter>,
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
            .insert((
                ContinuousEmission { interval_ms: 3000 },
                OwnedObjects::default(),
            ))
            .observe(cube_spitter_direct_signal)
            .observe(cube_spitter_receive_power)
            .observe(cube_spitter_lose_power);
    }
}

fn handle_continuous_cube_emission(
    mut commands: Commands,
    q_powered_spitters: Query<
        (Entity, &ContinuousEmission),
        (With<StandingCubeSpitter>, With<Powered>),
    >,
    q_children: Query<&Children>,
    q_signal_after_delay: Query<(), With<CubeAfterDelay>>,
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
            commands.entity(spitter_entity).with_child(CubeAfterDelay {
                delay_ms: continuous_emission.interval_ms,
                spawn_time: time.elapsed(),
            });
        }
    }
}

fn cube_spitter_direct_signal(
    trigger: Trigger<DirectSignal>,
    mut commands: Commands,
    mut q_spitter: Query<
        (&RigidBodyColliders, &GlobalTransform, &mut OwnedObjects),
        With<StandingCubeSpitter>,
    >,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    game_assets: Res<GameAssets>,
) {
    if let Ok((spitter_colliders, spitter_transform, mut spitter_owned_objects)) =
        q_spitter.get_mut(trigger.target())
    {
        for collider_entity in spitter_colliders.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
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

        for object in spitter_owned_objects.iter() {
            commands.entity(*object).despawn();
        }
        spitter_owned_objects.clear();

        let cube_id = commands
            .spawn((
                SceneRoot(game_assets.weighted_cube_cyan.clone()),
                Transform::from_translation(
                    spitter_transform.translation()
                        + Vec3::Y * 5.
                        + spitter_transform.forward() * -10.,
                ),
                RigidBody::Dynamic,
                TransformInterpolation,
                RotationInterpolation,
                ExternalImpulse::new(spitter_transform.forward() * -5000.),
                WeightedCube {
                    color: WeightedCubeColors::Cyan,
                },
            ))
            .id();

        // add the new cube to the owned objects
        spitter_owned_objects.0.push(cube_id);
    }
}

fn cube_spitter_receive_power(
    trigger: Trigger<OnAdd, Powered>,
    mut commands: Commands,
    q_spitter: Query<(Entity, &RigidBodyColliders, &ContinuousEmission), With<StandingCubeSpitter>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
    time: Res<Time>,
) {
    if let Ok((spitter_entity, spitter_children, continuous_spawning)) =
        q_spitter.get(trigger.target())
    {
        // Animate material to powered state
        for collider_entity in spitter_children.iter() {
            if let Ok(collider_children) = q_children.get(collider_entity) {
                for child in collider_children.iter() {
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
                                    end: POWER_MATERIAL_INTENSITY,
                                },
                            ),
                        ))
                        .insert(DespawnOnFinish);
                }
            }
        }

        // Start continuous cube spawning when powered
        commands.entity(spitter_entity).with_child(CubeAfterDelay {
            delay_ms: continuous_spawning.interval_ms,
            spawn_time: time.elapsed(),
        });
    }
}

#[derive(Component)]
pub struct CubeAfterDelay {
    pub delay_ms: u32,
    pub spawn_time: Duration,
}

fn cube_spitter_lose_power(
    trigger: Trigger<OnRemove, Powered>,
    mut commands: Commands,
    q_spitter: Query<(Entity, &RigidBodyColliders), With<StandingCubeSpitter>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children>,
    q_cube_after_delay: Query<(), With<CubeAfterDelay>>,
) {
    if let Ok((spitter_entity, spitter_children)) = q_spitter.get(trigger.target()) {
        // Animate material back to unpowered state
        for collider_entity in spitter_children.iter() {
            if let Ok(collider_children) = q_children.get(collider_entity) {
                for child in collider_children.iter() {
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }
            }

            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                if let Some(material) = unlit_materials.get(material_handle) {
                    let current_intensity = material.extension.params.intensity;
                    let intensity_ratio =
                        (current_intensity - 1.0) / (POWER_MATERIAL_INTENSITY - 1.0);
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

        // Stop continuous cube spawning by removing all SignalAfterDelay children
        if let Ok(children) = q_children.get(spitter_entity) {
            for child in children.iter() {
                if q_cube_after_delay.contains(child) {
                    commands.entity(child).despawn();
                }
            }
        }
    }
}

fn cube_after_delay(
    mut commands: Commands,
    q_waiting: Query<(Entity, &CubeAfterDelay, &ChildOf)>, // Removed GlobalTransform and OwnedObjects
    mut q_spitter: Query<(&GlobalTransform, &mut OwnedObjects), With<StandingCubeSpitter>>, // Query parent separately
    time: Res<Time>,
    game_assets: Res<GameAssets>,
) {
    for (entity, signal_delay, child_of) in q_waiting.iter() {
        let elapsed_since_spawn = time.elapsed() - signal_delay.spawn_time;

        if elapsed_since_spawn >= Duration::from_millis(signal_delay.delay_ms as u64) {
            // Get the parent spitter's transform and owned objects
            if let Ok((spitter_transform, mut spitter_owned_objects)) =
                q_spitter.get_mut(child_of.0)
            {
                // Clear existing cubes
                for object in spitter_owned_objects.iter() {
                    commands.entity(*object).despawn();
                }
                spitter_owned_objects.clear();

                // Spawn new cube
                let cube_id = commands
                    .spawn((
                        SceneRoot(game_assets.weighted_cube_cyan.clone()),
                        Transform::from_translation(
                            spitter_transform.translation()
                                + Vec3::Y * 5.
                                + spitter_transform.forward() * -10.,
                        ),
                        RigidBody::Dynamic,
                        TransformInterpolation,
                        RotationInterpolation,
                        ExternalImpulse::new(spitter_transform.forward() * -5000.),
                        WeightedCube {
                            color: WeightedCubeColors::Cyan,
                        },
                    ))
                    .id();

                spitter_owned_objects.0.push(cube_id);

                // Remove the delay component
                commands.entity(entity).remove::<CubeAfterDelay>();
            }
        }
    }
}

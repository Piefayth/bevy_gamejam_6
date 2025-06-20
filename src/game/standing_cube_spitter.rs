use std::time::Duration;

use avian3d::prelude::{
    Collider, CollisionEventsEnabled, CollisionLayers, LinearVelocity, RigidBody,
    RigidBodyColliders, RotationInterpolation, SleepingDisabled, TransformInterpolation,
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
        asset_tag_components::{Immobile, StandingCubeSpitter, WeightedCube, WeightedCubeColors},
    },
    game::signal_spitter::{dont_sink_when_held, sink_when_not_held},
    rendering::unlit_material::UnlitMaterial,
    GameState,
};

use super::{
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY},
    signals::{
        default_signal_collisions, DirectSignal, MaterialIntensityInterpolator, OwnedObjects,
        Powered,
    },
    GameLayer,
};

pub fn standing_cube_spitter_plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, (register_standing_cube_spitter_signals,))
        .add_systems(
            FixedLast,
            (check_and_replace_cubes,).run_if(in_state(GameState::Playing)),
        );
}

fn register_standing_cube_spitter_signals(
    mut commands: Commands,
    q_new_signal_spitter: Query<
        (Entity, &RigidBodyColliders, Has<Immobile>),
        (
            Added<RigidBodyColliders>,
            With<StandingCubeSpitter>,
            Without<Collider>,
        ),
    >,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for (spitter_entity, spitter_children, is_immobile) in &q_new_signal_spitter {
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
                                GameLayer::Device,
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
            .insert((OwnedObjects::default(), SleepingDisabled))
            .observe(cube_spitter_direct_signal)
            .observe(cube_spitter_receive_power)
            .observe(cube_spitter_lose_power);

        if !is_immobile {
            commands
                .entity(spitter_entity)
                .observe(sink_when_not_held)
                .observe(dont_sink_when_held);
        }
    }
}

// New system to check if powered spitters need cube replacement
fn check_and_replace_cubes(
    mut commands: Commands,
    mut q_powered_spitters: Query<
        (&GlobalTransform, &mut OwnedObjects),
        (With<StandingCubeSpitter>, With<Powered>),
    >,
    q_existing_entities: Query<Entity>, // To check if owned entities still exist
    game_assets: Res<GameAssets>,
) {
    for (spitter_transform, mut spitter_owned_objects) in &mut q_powered_spitters {
        // Remove any owned objects that no longer exist
        spitter_owned_objects
            .0
            .retain(|&entity| q_existing_entities.contains(entity));

        // If no cubes exist, spawn a new one immediately
        if spitter_owned_objects.0.is_empty() {
            let cube_id = commands
                .spawn((
                    SceneRoot(game_assets.weighted_cube_cyan.clone()),
                    Transform::from_translation(
                        spitter_transform.translation()
                            + Vec3::Y * 10.
                            + spitter_transform.forward() * -10.,
                    ),
                    RigidBody::Dynamic,
                    TransformInterpolation,
                    RotationInterpolation,
                    LinearVelocity(spitter_transform.forward() * -50. + Vec3::Y * 30.),
                    WeightedCube {
                        color: WeightedCubeColors::Cyan,
                    },
                ))
                .id();

            spitter_owned_objects.0.push(cube_id);
        }
    }
}

// we can't use try_insert with bevy_tween, so we need to mark untweenable objects
#[derive(Component)]
pub struct Tombstone;

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
                    )));
            }
        }

        for object in spitter_owned_objects.iter() {
            if let Ok(mut ec) = commands.get_entity(*object) {
                ec.insert(Tombstone).despawn()
            }
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
                LinearVelocity(spitter_transform.forward() * -50. + Vec3::Y * 30.),
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
    mut q_spitter: Query<
        (&RigidBodyColliders, &GlobalTransform, &mut OwnedObjects),
        With<StandingCubeSpitter>,
    >,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
    game_assets: Res<GameAssets>,
) {
    if let Ok((spitter_children, spitter_transform, mut spitter_owned_objects)) =
        q_spitter.get_mut(trigger.target())
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

                    commands.entity(collider_entity).animation().insert(tween(
                        Duration::from_secs_f32(duration_secs),
                        EaseKind::CubicOut,
                        TargetAsset::Asset(material_handle.clone_weak()).with(
                            MaterialIntensityInterpolator {
                                start: current_intensity,
                                end: POWER_MATERIAL_INTENSITY,
                            },
                        ),
                    ));
                }
            }
        }

        // If no cubes exist when powered, spawn one immediately
        if spitter_owned_objects.0.is_empty() {
            let cube_id = commands
                .spawn((
                    SceneRoot(game_assets.weighted_cube_cyan.clone()),
                    Transform::from_translation(
                        spitter_transform.translation()
                            + Vec3::Y * 10.
                            + spitter_transform.forward() * -10.,
                    ),
                    RigidBody::Dynamic,
                    TransformInterpolation,
                    RotationInterpolation,
                    LinearVelocity(spitter_transform.forward() * -50. + Vec3::Y * 30.),
                    WeightedCube {
                        color: WeightedCubeColors::Cyan,
                    },
                ))
                .id();

            spitter_owned_objects.0.push(cube_id);
        }
    }
}

fn cube_spitter_lose_power(
    trigger: Trigger<OnRemove, Powered>,
    mut commands: Commands,
    q_spitter: Query<&RigidBodyColliders, With<StandingCubeSpitter>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children>,
) {
    if let Ok(spitter_children) = q_spitter.get(trigger.target()) {
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

                    commands.entity(collider_entity).animation().insert(tween(
                        Duration::from_secs_f32(duration_secs),
                        EaseKind::CubicOut,
                        TargetAsset::Asset(material_handle.clone_weak()).with(
                            MaterialIntensityInterpolator {
                                start: current_intensity,
                                end: 1.0,
                            },
                        ),
                    ));
                }
            }
        }
        // No need to remove delay components since we're not using them anymore
    }
}

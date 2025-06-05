use std::time::Duration;

use avian3d::prelude::{
    CollisionEventsEnabled, CollisionLayers, ExternalImpulse, LinearVelocity, RigidBody,
    RigidBodyColliders, RotationInterpolation, TransformInterpolation,
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
        asset_tag_components::{CubeSpitter, PermanentlyPowered, WeightedCube, WeightedCubeColors},
    }, game::standing_cube_spitter::Tombstone, rendering::unlit_material::UnlitMaterial, GameState
};

use super::{
    DespawnOnFinish, GameLayer,
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY},
    signals::{
        DirectSignal, MaterialIntensityInterpolator, OwnedObjects, Powered, default_signal_collisions,
    },
};

pub fn cube_spitter_plugin(app: &mut App) {
    app.add_systems(
        FixedPreUpdate,
        register_cube_spitter_signals,
    )
    .add_systems(
        FixedLast,
        (check_and_replace_wall_cubes,).run_if(in_state(GameState::Playing)),
    );
}

// New system to check if powered wall spitters need cube replacement
fn check_and_replace_wall_cubes(
    mut commands: Commands,
    mut q_powered_spitters: Query<
        (Entity, &CubeSpitter, &Transform, &mut OwnedObjects),
        (With<CubeSpitter>, With<Powered>),
    >,
    q_existing_entities: Query<Entity>, // To check if owned entities still exist
    game_assets: Res<GameAssets>,
) {
    for (spitter_entity, spitter, spitter_transform, mut spitter_owned_objects) in &mut q_powered_spitters {
        // Remove any owned objects that no longer exist
        spitter_owned_objects.0.retain(|&entity| q_existing_entities.contains(entity));
        
        // If no cubes exist, spawn a new one immediately
        if spitter_owned_objects.0.is_empty() {
            let cube_id = commands
                .spawn((
                    SceneRoot(match spitter.color {
                        WeightedCubeColors::Cyan => game_assets.weighted_cube_cyan.clone(),
                    }),
                    Transform::from_translation(
                        spitter_transform.translation
                            + Vec3::Y * 5.
                            + spitter_transform.forward() * -10.,
                    ),
                    RigidBody::Dynamic,
                    TransformInterpolation,
                    RotationInterpolation,
                    LinearVelocity(spitter_transform.forward() * -50.),
                    WeightedCube {
                        color: WeightedCubeColors::Cyan,
                    },
                ))
                .id();

            spitter_owned_objects.0.push(cube_id);
        }
    }
}

pub fn cube_spitter_direct_signal(
    trigger: Trigger<DirectSignal>,
    mut commands: Commands,
    mut q_cube_spitters: Query<(
        &RigidBodyColliders,
        &CubeSpitter,
        &Transform,
        &mut OwnedObjects,
    )>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    game_assets: Res<GameAssets>,
) {
    if let Ok((spitter_colliders, spitter, spitter_transform, mut spitter_owned_objects)) =
        q_cube_spitters.get_mut(trigger.target())
    {
        if let Some(collider_entity) = spitter_colliders.iter().next() {
            if let Ok(spitter_material_handle) = q_unlit_objects.get(collider_entity) {
                commands
                    .entity(collider_entity)
                    .animation()
                    .insert(sequence((
                        tween(
                            Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 1000.) as u64),
                            EaseKind::CubicOut,
                            TargetAsset::Asset(spitter_material_handle.clone_weak()).with(
                                MaterialIntensityInterpolator {
                                    start: 1.0,
                                    end: POWER_MATERIAL_INTENSITY,
                                },
                            ),
                        ),
                        tween(
                            Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 1000.) as u64),
                            EaseKind::CubicIn,
                            TargetAsset::Asset(spitter_material_handle.clone_weak()).with(
                                MaterialIntensityInterpolator {
                                    start: POWER_MATERIAL_INTENSITY,
                                    end: 1.0,
                                },
                            ),
                        ),
                    )))
                    .insert(DespawnOnFinish);

                // despawn the old owned objects and clear the list
                for object in spitter_owned_objects.iter() {
                    commands.entity(*object).insert(Tombstone).despawn();
                }
                spitter_owned_objects.clear();

                let cube_id = commands
                    .spawn((
                        SceneRoot(match spitter.color {
                            WeightedCubeColors::Cyan => game_assets.weighted_cube_cyan.clone(),
                        }),
                        Transform::from_translation(
                            spitter_transform.translation
                                + Vec3::Y * 5.
                                + spitter_transform.forward() * -10.,
                        ),
                        RigidBody::Dynamic,
                        TransformInterpolation,
                        RotationInterpolation,
                        LinearVelocity(spitter_transform.forward() * -50.),
                        WeightedCube {
                            color: WeightedCubeColors::Cyan,
                        },
                    ))
                    .id();

                // add the new cube to the owned objects
                spitter_owned_objects.0.push(cube_id);
            }
        }
    }
}

fn cube_spitter_receive_power(
    trigger: Trigger<OnAdd, Powered>,
    mut commands: Commands,
    mut q_spitter: Query<(&Children, &CubeSpitter, &Transform, &mut OwnedObjects), With<CubeSpitter>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children>,
    game_assets: Res<GameAssets>,
) {
    if let Ok((spitter_children, spitter, spitter_transform, mut spitter_owned_objects)) =
        q_spitter.get_mut(trigger.target())
    {
        println!("SPITTER GOT POWER YEAHHHH");
        // Animate material to powered state for each child
        for spitter_child in spitter_children.iter() {
            if let Ok(child_children) = q_children.get(spitter_child) {
                for child in child_children.iter() {
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }
            }

            if let Ok(material_handle) = q_unlit_objects.get(spitter_child) {
                if let Some(material) = unlit_materials.get(material_handle) {
                    let current_intensity = material.extension.params.intensity;
                    let intensity_ratio = (POWER_MATERIAL_INTENSITY - current_intensity)
                        / (POWER_MATERIAL_INTENSITY - 1.0);
                    let duration_secs = POWER_ANIMATION_DURATION_SEC * intensity_ratio.max(0.1);

                    commands
                        .entity(spitter_child)
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

        // If no cubes exist when powered, spawn one immediately
        if spitter_owned_objects.0.is_empty() {
            let cube_id = commands
                .spawn((
                    SceneRoot(match spitter.color {
                        WeightedCubeColors::Cyan => game_assets.weighted_cube_cyan.clone(),
                    }),
                    Transform::from_translation(
                        spitter_transform.translation
                            + Vec3::Y * 5.
                            + spitter_transform.forward() * -10.,
                    ),
                    RigidBody::Dynamic,
                    TransformInterpolation,
                    RotationInterpolation,
                    LinearVelocity(spitter_transform.forward() * -50.),
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
    q_spitter: Query<&Children, With<CubeSpitter>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children>,
) {
    if let Ok(spitter_children) = q_spitter.get(trigger.target()) {
        // Animate material back to unpowered state for each child
        for spitter_child in spitter_children.iter() {
            if let Ok(child_children) = q_children.get(spitter_child) {
                for child in child_children.iter() {
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }
            }

            if let Ok(material_handle) = q_unlit_objects.get(spitter_child) {
                if let Some(material) = unlit_materials.get(material_handle) {
                    let current_intensity = material.extension.params.intensity;
                    let intensity_ratio =
                        (current_intensity - 1.0) / (POWER_MATERIAL_INTENSITY - 1.0);
                    let duration_secs = POWER_ANIMATION_DURATION_SEC * intensity_ratio.max(0.1);

                    commands
                        .entity(spitter_child)
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
    }
}

fn register_cube_spitter_signals(
    mut commands: Commands,
    q_new_spitter: Query<(Entity, &Children, Has<PermanentlyPowered>), Added<CubeSpitter>>,
) {
    // for static geo like spitters, the tag is on the parent, but the rigid body is on the child
    for (spitter_entity, spitter_children, is_permanently_powered) in &q_new_spitter {
        // warning: we actually expect there to only be ever one spitter child
        // this explodes if not
        commands
            .entity(spitter_entity)
            .insert((OwnedObjects::default(), RigidBody::Static))
            .observe(cube_spitter_direct_signal)
            .observe(cube_spitter_receive_power)
            .observe(cube_spitter_lose_power);

        if is_permanently_powered {
            commands.entity(spitter_entity).insert(Powered).remove::<PermanentlyPowered>();
        }

        for spitter_child in spitter_children.iter() {
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
                        ]
                    ),
                    AnimationTarget,
                ))
                .observe(default_signal_collisions);
        }
    }
}

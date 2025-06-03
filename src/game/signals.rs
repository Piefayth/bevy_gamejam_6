use std::time::Duration;

use avian3d::prelude::{
    CollisionEventsEnabled, CollisionLayers, ExternalImpulse, OnCollisionStart, RigidBody,
    RigidBodyColliders, RotationInterpolation, TransformInterpolation,
};
use bevy::prelude::*;
use bevy_tween::{
    combinator::{sequence, tween},
    prelude::{AnimationBuilderExt, EaseKind, Interpolator},
    tween::{AnimationTarget, IntoTarget, TargetAsset, TargetComponent},
};

use crate::{
    asset_management::{
        asset_loading::GameAssets,
        asset_tag_components::{CubeSpitter, WeightedCube, WeightedCubeColors},
    },
    rendering::unlit_material::UnlitMaterial,
};

use super::{DespawnOnFinish, GameLayer};

pub fn signals_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            register_spitter_signals,
            register_cube_signals,
            cube_receive_power,
        ),
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
        material.extension.intensity = self.start + (self.end - self.start) * progress;
    }
}

// This gets added to anything that can produce a signal through physical contact
pub fn spitter_consume_signal(
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
                                            end: 5.0,
                                        },
                                    ),
                                ),
                                // Second tween: 5.0 → 1.0
                                tween(
                                    Duration::from_secs(1),
                                    EaseKind::CubicIn,
                                    TargetAsset::Asset(spitter_material_handle.clone_weak()).with(
                                        MaterialIntensityInterpolator {
                                            start: 5.0,
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

fn register_spitter_signals(
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
                .observe(spitter_consume_signal);
        }
    }
}

fn cube_consume_signal(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_signals: Query<(), With<Signal>>,
    q_powered_cubes: Query<(), (With<Powered>, With<WeightedCube>)>,
) {
    if let Some(cube_body) = trigger.body {
        if q_signals.contains(trigger.collider) {
            if !q_powered_cubes.contains(cube_body) {
                commands.entity(cube_body).insert(Powered);
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
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>
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

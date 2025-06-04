use std::time::Duration;

use avian3d::prelude::{
    CollisionEventsEnabled, CollisionLayers, ExternalImpulse, RigidBody, RigidBodyColliders, RotationInterpolation,
    TransformInterpolation,
};
use bevy::prelude::*;
use bevy_tween::{
    combinator::{sequence, tween},
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetAsset},
};

use crate::{
    asset_management::{
        asset_loading::GameAssets,
        asset_tag_components::{CubeSpitter, WeightedCube, WeightedCubeColors},
    },
    rendering::unlit_material::UnlitMaterial,
};

use super::{
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY}, signals::{default_signal_collisions, DirectSignal, MaterialIntensityInterpolator, OwnedObjects}, DespawnOnFinish, GameLayer
};

pub fn cube_spitter_plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, register_cube_spitter_signals);
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
                    commands.entity(*object).despawn();
                }
                spitter_owned_objects.clear();

                let cube_id = commands
                    .spawn((
                        SceneRoot(match spitter.color {
                            WeightedCubeColors::Cyan => game_assets.weighted_cube_cyan.clone(),
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
            .insert(OwnedObjects::default())
            .insert(RigidBody::Static)
            .observe(cube_spitter_direct_signal);

        for spitter_child in spitter_children.iter() {
            commands
                .entity(spitter_child)
                .insert((
                    CollisionEventsEnabled,
                    CollisionLayers::new(GameLayer::Device, [GameLayer::Signal, GameLayer::Player]),
                    AnimationTarget,
                ))
                .observe(default_signal_collisions);
        }
    }
}

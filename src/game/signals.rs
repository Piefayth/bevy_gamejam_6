use avian3d::prelude::{CollisionEventsEnabled, CollisionLayers, ExternalImpulse, OnCollisionStart, RigidBody, RotationInterpolation, TransformInterpolation};
use bevy::prelude::*;

use crate::asset_management::{asset_loading::GameAssets, asset_tag_components::{CubeSpitter, WeightedCubeColors}};

use super::GameLayer;

pub fn signals_plugin(app: &mut App) {
    app.add_systems(Update, register_spitter_signals);
}

#[derive(Component)]
pub struct Signal;

#[derive(Component, Default, Deref, DerefMut)]
pub struct OwnedObjects(pub Vec<Entity>);

// This gets added to anything that can produce a signal through physical contact
pub fn spitter_consume_signal(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_signals: Query<(), With<Signal>>,
    mut q_cube_spitters: Query<(&CubeSpitter, &Transform, &mut OwnedObjects)>,
    game_assets: Res<GameAssets>,
    q_parents: Query<&ChildOf>,
) {
    if let Some(colliding_body) = trigger.body {
        if let Ok(child_of) = q_parents.get(colliding_body) {
            if q_signals.contains(trigger.collider) {
                if let Ok((spitter, spitter_transform, mut spitter_owned_objects)) = q_cube_spitters.get_mut(child_of.0) {

                    // despawn the old owned objects and clear the list
                    for object in spitter_owned_objects.iter() {
                        commands.entity(*object).despawn();
                    }
                    spitter_owned_objects.clear();

                    let cube_id = commands.spawn((
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
                    )).id();

                    // add the new cube to the owned objects
                    spitter_owned_objects.0.push(cube_id);

                    // despawn the signal
                    commands.entity(trigger.collider).despawn();
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
        commands.entity(spitter_entity).insert(OwnedObjects::default());

        for spitter_child in spitter_children.iter() {
            commands.entity(spitter_child)
                .insert((
                    CollisionEventsEnabled,
                    CollisionLayers::new(GameLayer::Device, [GameLayer::Signal]),
                ))
                .observe(spitter_consume_signal);
        }
    }

}

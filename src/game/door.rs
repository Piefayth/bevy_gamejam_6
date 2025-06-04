use std::time::Duration;

use avian3d::{
    collision::collider,
    prelude::{CollisionEventsEnabled, CollisionLayers, RigidBody, RigidBodyColliders},
};
use bevy::prelude::*;
use bevy_tween::{
    bevy_time_runner::TimeSpan, combinator::{sequence, tween}, interpolate::translation, prelude::{AnimationBuilderExt, EaseKind}, tween::{AnimationTarget, TargetAsset, TargetComponent, Tween}
};

use crate::{
    asset_management::asset_tag_components::{Door, DoorPole},
    rendering::unlit_material::UnlitMaterial,
};

use super::{
    DespawnOnFinish, GameLayer,
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY, PoweredBy, Powers},
    signals::{DirectSignal, MaterialIntensityInterpolator, default_signal_collisions},
};

pub fn door_plugin(app: &mut App) {
    app.add_systems(Update, register_doors);
}

fn register_doors(
    mut commands: Commands,
    q_new_door: Query<(Entity, &Children, &ChildOf), Added<Door>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_children: Query<&Children>,
    q_pole: Query<Entity, With<DoorPole>>,
) {
    for (door_entity, door_children, door_parent) in &q_new_door {
        // Register the DoorPole
        if let Ok(parent_children) = q_children.get(door_parent.parent()) {
            for sibling in parent_children.iter() {
                if sibling != door_entity {
                    if q_pole.contains(sibling) {
                        commands.entity(door_entity).insert(PoweredBy(sibling));

                        commands
                            .entity(sibling)
                            .insert(RigidBody::Static)
                            .observe(door_pole_direct_signal);

                        if let Ok(pole_children) = q_children.get(sibling) {
                            for pole_child in pole_children.iter() {
                                if let Ok(material_handle) = q_unlit_objects.get(pole_child) {
                                    let old_material =
                                        unlit_materials.get(material_handle).unwrap().clone();

                                    commands
                                        .entity(pole_child)
                                        .insert((
                                            AnimationTarget,
                                            MeshMaterial3d(unlit_materials.add(old_material)),
                                            CollisionLayers::new(
                                                GameLayer::Device,
                                                [
                                                    GameLayer::Device,
                                                    GameLayer::Player,
                                                    GameLayer::Signal,
                                                ],
                                            ),
                                            CollisionEventsEnabled,
                                        ))
                                        .observe(default_signal_collisions);
                                }
                            }
                        }
                        break; // Assuming only one DoorPole sibling
                    }
                }
            }
        }

        commands
            .entity(door_entity)
            .insert((RigidBody::Kinematic, AnimationTarget));
        
    }
}

fn door_pole_direct_signal(
    trigger: Trigger<DirectSignal>,
    mut commands: Commands,
    q_pole: Query<(&RigidBodyColliders, &Powers), (With<DoorPole>)>,
    q_doors: Query<(Entity, &Transform, &Children), With<Door>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    time: Res<Time>,
) {
    if let Ok((pole_colliders, pole_powers)) = q_pole.get(trigger.target()) {
        for powered_entity in pole_powers.iter() {
            if let Ok((door_entity, door_transform, door_children)) = q_doors.get(powered_entity) {
                let current_y = door_transform.translation.y;
                let target_y = 3.0; // Your target height
                let start_y = 0.0; // Your starting height

                // Calculate progress and remaining distance
                let total_distance = target_y - start_y;

                // For the upward movement
                let remaining_up_distance = target_y - current_y;
                let up_progress = if remaining_up_distance > 0.0 {
                    remaining_up_distance / total_distance
                } else {
                    0.0
                };

                // Scale the duration based on remaining distance
                let up_duration = Duration::from_secs_f32(1.0 * up_progress);

                // For the downward movement, we always go full distance from target to start
                let down_duration = Duration::from_secs(1);

                for child in door_children.iter() {
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }
                // Create the sequence based on whether we need the upward movement
                if up_progress > 0.0 {
                    // Door needs to move up first
                    commands
                        .entity(door_entity)
                        .animation()
                        .insert(sequence((
                            tween(
                                up_duration,
                                EaseKind::Linear,
                                TargetComponent::marker().with(translation(
                                    Vec3::new(0.0, current_y, 0.0),
                                    Vec3::new(0.0, target_y, 0.0),
                                )),
                            ),
                            tween(
                                Duration::from_secs(3),
                                EaseKind::Linear,
                                TargetComponent::marker().with(translation(
                                    Vec3::new(0.0, target_y, 0.0),
                                    Vec3::new(0.0, target_y, 0.0), // Same start and end = no movement
                                )),
                            ),
                            tween(
                                down_duration,
                                EaseKind::Linear,
                                TargetComponent::marker().with(translation(
                                    Vec3::new(0.0, target_y, 0.0),
                                    Vec3::new(0.0, start_y, 0.0),
                                )),
                            ),
                        )))
                        .insert(DespawnOnFinish);
                } else {
                    // Door is already at target, skip upward movement
                    commands
                        .entity(door_entity)
                        .animation()
                        .insert(sequence((
                            tween(
                                Duration::from_secs(3),
                                EaseKind::Linear,
                                TargetComponent::marker().with(translation(
                                    Vec3::new(0.0, target_y, 0.0),
                                    Vec3::new(0.0, target_y, 0.0), // Same start and end = no movement
                                )),
                            ),
                            tween(
                                down_duration,
                                EaseKind::Linear,
                                TargetComponent::marker().with(translation(
                                    Vec3::new(0.0, target_y, 0.0),
                                    Vec3::new(0.0, start_y, 0.0),
                                )),
                            ),
                        )))
                        .insert(DespawnOnFinish);
                }
            }
        }

        for collider_entity in pole_colliders.iter() {
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
    }
}

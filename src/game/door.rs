use std::time::Duration;

use avian3d::{
    collision::collider,
    prelude::{CollisionEventsEnabled, CollisionLayers, RigidBody, RigidBodyColliders},
};
use bevy::prelude::*;
use bevy_tween::{
    bevy_time_runner::TimeSpan,
    combinator::{sequence, tween},
    interpolate::translation,
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetAsset, TargetComponent, Tween},
};

use crate::{
    asset_management::asset_tag_components::{Door, DoorPole},
    rendering::{section_color_prepass::DrawSection, unlit_material::UnlitMaterial},
};

use super::{
    pressure_plate::{PoweredBy, Powers, POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY}, signals::{default_signal_collisions, DirectSignal, MaterialIntensityInterpolator, Powered}, DespawnOnFinish, GameLayer
};

pub fn door_plugin(app: &mut App) {
    app.add_systems(Update, (register_doors, update_powered_timers));
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
        for door_child in door_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(door_child) {
                let mut new_material =
                    unlit_materials.get(material_handle).unwrap().clone();

                new_material.base.depth_bias = 100.;

                commands.entity(door_child).insert(MeshMaterial3d(unlit_materials.add(new_material)));
            }
            commands.entity(door_child).remove::<DrawSection>();
        }

        // Register the DoorPole
        if let Ok(parent_children) = q_children.get(door_parent.parent()) {
            for maybe_pole in parent_children.iter() {
                if maybe_pole != door_entity {
                    if q_pole.contains(maybe_pole) {
                        let pole = maybe_pole;
                        commands.entity(door_entity).insert(PoweredBy(pole));

                        commands
                            .entity(pole)
                            .insert(RigidBody::Static)
                            .observe(door_pole_direct_signal)
                            .observe(on_power_added)
                            .observe(on_power_removed);

                        if let Ok(pole_children) = q_children.get(pole) {
                            for pole_child in pole_children.iter() {
                                if let Ok(material_handle) = q_unlit_objects.get(pole_child) {
                                    let new_material =
                                        unlit_materials.get(material_handle).unwrap().clone();

                                    commands
                                        .entity(pole_child)
                                        .insert((
                                            AnimationTarget,
                                            MeshMaterial3d(unlit_materials.add(new_material)),
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

#[derive(Component)]
struct PoweredTimer(Timer);

const DOOR_POLE_POWER_DURATION_SEC: u64 = 5;
fn door_pole_direct_signal(
    trigger: Trigger<DirectSignal>,
    mut commands: Commands,
    q_pole: Query<Entity, With<DoorPole>>,
) {
    if let Ok(pole_entity) = q_pole.get(trigger.target()) {
        commands.entity(pole_entity).insert((
            Powered,
            PoweredTimer(Timer::from_seconds(
                DOOR_POLE_POWER_DURATION_SEC as f32,
                TimerMode::Once,
            )),
        ));
        // All door animation logic moved to power state observers
    }
}

fn on_power_added(
    trigger: Trigger<OnAdd, Powered>,
    mut commands: Commands,
    q_pole: Query<(&RigidBodyColliders, &Powers), With<DoorPole>>,
    q_doors: Query<(Entity, &Transform, &Children), With<Door>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
) {
    let entity = trigger.target();

    // Handle pole material animation
    if let Ok((pole_colliders, pole_powers)) = q_pole.get(entity) {
        // Material animations for pole
        for collider_entity in pole_colliders.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                commands
                    .entity(collider_entity)
                    .animation()
                    .insert(tween(
                        Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 1000.) as u64),
                        EaseKind::CubicOut,
                        TargetAsset::Asset(material_handle.clone_weak()).with(
                            MaterialIntensityInterpolator {
                                start: 1.0,
                                end: POWER_MATERIAL_INTENSITY,
                            },
                        ),
                    ))
                    .insert(DespawnOnFinish);
            }
        }

        // Door animations - move doors UP when powered
        for powered_entity in pole_powers.iter() {
            if let Ok((door_entity, door_transform, door_children)) = q_doors.get(powered_entity) {
                let current_y = door_transform.translation.y;
                let target_y = 3.0;

                // Clear existing tweens
                for child in door_children.iter() {
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }

                // Only animate if door needs to move up
                if current_y < target_y {
                    let remaining_distance = target_y - current_y;
                    let total_distance = target_y; // assuming start_y = 0.0
                    let progress = remaining_distance / total_distance;
                    let duration = Duration::from_secs_f32(1.0 * progress);

                    commands
                        .entity(door_entity)
                        .animation()
                        .insert(tween(
                            duration,
                            EaseKind::Linear,
                            TargetComponent::marker().with(translation(
                                Vec3::new(0.0, current_y, 0.0),
                                Vec3::new(0.0, target_y, 0.0),
                            )),
                        ))
                        .insert(DespawnOnFinish);
                }
            }
        }
    }
}

fn on_power_removed(
    trigger: Trigger<OnRemove, Powered>,
    mut commands: Commands,
    q_pole: Query<(&RigidBodyColliders, &Powers), With<DoorPole>>,
    q_doors: Query<(Entity, &Transform, &Children), With<Door>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
) {
    let entity = trigger.target();

    // Handle pole material animation
    if let Ok((pole_colliders, pole_powers)) = q_pole.get(entity) {
        // Material animations for pole
        for collider_entity in pole_colliders.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                commands
                    .entity(collider_entity)
                    .animation()
                    .insert(tween(
                        Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 1000.) as u64),
                        EaseKind::CubicOut,
                        TargetAsset::Asset(material_handle.clone_weak()).with(
                            MaterialIntensityInterpolator {
                                start: POWER_MATERIAL_INTENSITY,
                                end: 1.0,
                            },
                        ),
                    ))
                    .insert(DespawnOnFinish);
            }
        }

        // Door animations - move doors DOWN when power removed
        for powered_entity in pole_powers.iter() {
            if let Ok((door_entity, door_transform, door_children)) = q_doors.get(powered_entity) {
                let current_y = door_transform.translation.y;
                let start_y = 0.0;

                // Clear existing tweens
                for child in door_children.iter() {
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }

                // Only animate if door needs to move down
                if current_y > start_y {
                    commands
                        .entity(door_entity)
                        .animation()
                        .insert(tween(
                            Duration::from_secs(1),
                            EaseKind::Linear,
                            TargetComponent::marker().with(translation(
                                Vec3::new(0.0, current_y, 0.0),
                                Vec3::new(0.0, start_y, 0.0),
                            )),
                        ))
                        .insert(DespawnOnFinish);
                }
            }
        }
    }
}

fn update_powered_timers(
    mut commands: Commands,
    mut q_powered: Query<(Entity, &mut PoweredTimer)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in q_powered.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            commands
                .entity(entity)
                .remove::<Powered>()
                .remove::<PoweredTimer>();
        }
    }
}

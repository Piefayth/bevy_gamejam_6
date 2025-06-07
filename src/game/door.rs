use std::time::Duration;

use avian3d::prelude::{Collider, CollisionEventsEnabled, CollisionLayers, RigidBody, RigidBodyColliders};
use bevy::prelude::*;
use bevy_tween::{
    bevy_time_runner::TimeSpan,
    combinator::tween,
    interpolate::translation,
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetAsset, TargetComponent},
};

use crate::{
    asset_management::asset_tag_components::{ChargePad, Door, DoorPole, ExtraDoorPowerRequired}, game::pressure_plate::PoweredBy, rendering::{section_color_prepass::DrawSection, unlit_material::UnlitMaterial}
};

use super::{
    DespawnOnFinish, GameLayer,
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY},
    signals::{DirectSignal, MaterialIntensityInterpolator, Powered, default_signal_collisions},
};

pub fn door_plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, register_doors)
        .add_systems(FixedUpdate, update_powered_timers)
        .add_systems(Update, check_door_power_requirements);
}

#[derive(Component)]
pub struct DoorOriginalPosition(pub Vec3);

#[derive(Component)]
pub struct PowersDoor(pub Entity);

fn register_doors(
    mut commands: Commands,
    q_new_door: Query<(Entity, &Children, &ChildOf, &Transform), Added<Door>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_children: Query<&Children>,
    q_pole: Query<Entity, With<DoorPole>>,
) {
    for (door_entity, door_children, door_parent, door_transform) in &q_new_door {
        for door_child in door_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(door_child) {
                let mut new_material = unlit_materials.get(material_handle).unwrap().clone();

                new_material.base.depth_bias = 100.;

                commands
                    .entity(door_child)
                    .insert(MeshMaterial3d(unlit_materials.add(new_material)));
            }
            commands.entity(door_child).remove::<DrawSection>();
        }

        // Register all DoorPole siblings
        if let Ok(parent_children) = q_children.get(door_parent.parent()) {
            for maybe_pole in parent_children.iter() {
                if maybe_pole != door_entity && q_pole.contains(maybe_pole) {
                    let pole = maybe_pole;

                    commands
                        .entity(pole)
                        .insert((
                            RigidBody::Static,
                            PowersDoor(door_entity), // Each pole powers this specific door
                        ))
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
                }
            }
        }

        commands
            .entity(door_entity)
            .insert((
                RigidBody::Kinematic, 
                AnimationTarget,
                DoorOriginalPosition(door_transform.translation)
            ));
    }
}

#[derive(Component)]
pub struct PoweredTimer(Timer);

const DOOR_POLE_POWER_DURATION_SEC: u64 = 2;
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
    }
}

const DOOR_LIFT_HEIGHT: f32 = 20.;

fn count_powered_poles_for_door(
    door_entity: Entity,
    q_poles: &Query<&PowersDoor, (With<DoorPole>, With<Powered>)>
) -> u32 {
    q_poles.iter()
        .filter(|powers_door| powers_door.0 == door_entity)
        .count() as u32
}

fn check_door_power_requirements(
    mut commands: Commands,
    q_doors: Query<(Entity, &Transform, &Children, &DoorOriginalPosition, Option<&ExtraDoorPowerRequired>), With<Door>>,
    q_powered_poles: Query<&PowersDoor, (With<DoorPole>, With<Powered>)>,
    q_tween: Query<(), With<TimeSpan>>,
) {
    for (door_entity, door_transform, door_children, original_pos, extra_power_required) in &q_doors {
        let powered_count = count_powered_poles_for_door(door_entity, &q_powered_poles);
        let required_count = extra_power_required.map(|e| e.amount + 1).unwrap_or(1);
        
        let should_be_open = powered_count >= required_count;
        let current_y = door_transform.translation.y;
        let target_y = original_pos.0.y + DOOR_LIFT_HEIGHT;
        let original_y = original_pos.0.y;
        
        let is_currently_open = current_y > original_y + 1.0;
        
        if should_be_open && !is_currently_open {
            // Door should open
            for child in door_children.iter() {
                if q_tween.contains(child) {
                    commands.entity(child).despawn();
                }
            }
            
            let remaining_distance = target_y - current_y;
            let total_distance = DOOR_LIFT_HEIGHT;
            let progress = remaining_distance / total_distance;
            let duration = Duration::from_secs_f32(1.0 * progress);

            commands
                .entity(door_entity)
                .animation()
                .insert(tween(
                    duration,
                    EaseKind::Linear,
                    TargetComponent::marker().with(translation(
                        door_transform.translation,
                        original_pos.0.with_y(target_y),
                    )),
                ))
                .insert(DespawnOnFinish);
                
        } else if !should_be_open && is_currently_open {
            // Door should close
            for child in door_children.iter() {
                if q_tween.contains(child) {
                    commands.entity(child).despawn();
                }
            }

            commands
                .entity(door_entity)
                .animation()
                .insert(tween(
                    Duration::from_secs(1),
                    EaseKind::Linear,
                    TargetComponent::marker().with(translation(
                        door_transform.translation,
                        original_pos.0,
                    )),
                ))
                .insert(DespawnOnFinish);
        }
    }
}

fn on_power_added(
    trigger: Trigger<OnAdd, Powered>,
    mut commands: Commands,
    q_pole: Query<&RigidBodyColliders, With<DoorPole>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
) {
    let entity = trigger.target();

    // Handle pole material animation only - door logic is handled by check_door_power_requirements
    if let Ok(pole_colliders) = q_pole.get(entity) {
        for collider_entity in pole_colliders.iter() {
            if let Ok(collider_children) = q_children.get(collider_entity) {
                for child in collider_children.iter() {
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }
            }

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
    }
}

fn on_power_removed(
    trigger: Trigger<OnRemove, Powered>,
    mut commands: Commands,
    q_pole: Query<&RigidBodyColliders, With<DoorPole>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
) {
    let entity = trigger.target();

    // Handle pole material animation only - door logic is handled by check_door_power_requirements
    if let Ok(pole_colliders) = q_pole.get(entity) {
        for collider_entity in pole_colliders.iter() {
            if let Ok(collider_children) = q_children.get(collider_entity) {
                for child in collider_children.iter() {
                    if q_tween.contains(child) {
                        commands.entity(child).despawn();
                    }
                }
            }
            
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
    }
}

fn update_powered_timers(
    mut commands: Commands,
    mut q_powered: Query<(Entity, &mut PoweredTimer)>,
    q_powered_by: Query<&PoweredBy>,
    q_charge_pad_powered: Query<&Powered, (With<ChargePad>, Without<PoweredTimer>)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in q_powered.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            // Check if still powered by a ChargePad
            let should_stay_powered = if let Ok(powered_by) = q_powered_by.get(entity) {
                q_charge_pad_powered.contains(powered_by.0)
            } else {
                false
            };
            
            if !should_stay_powered {
                commands.entity(entity).try_remove::<Powered>();
            }
            commands.entity(entity).try_remove::<PoweredTimer>();
        }
    }
}

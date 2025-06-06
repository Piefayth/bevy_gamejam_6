use std::time::Duration;

use avian3d::prelude::{
    Collider, ColliderConstructor, ColliderOf, CollisionEventsEnabled, CollisionLayers,
    ExternalImpulse, OnCollisionStart, RigidBody, RigidBodyColliders, RotationInterpolation,
    Sensor, SpatialQuery, SpatialQueryFilter, TransformInterpolation,
};
use bevy::prelude::*;
use bevy_tween::{
    bevy_time_runner::{TimeRunner, TimeSpan}, combinator::{sequence, tween}, interpolate::translation, prelude::{AnimationBuilderExt, EaseKind, Interpolator}, tween::{AnimationTarget, IntoTarget, TargetAsset, TargetComponent}
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
    door::PoweredTimer, player::Held, pressure_plate::{PoweredBy, POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY}, signals::{DirectSignal, MaterialIntensityInterpolator, Powered, Signal}, standing_cube_spitter::Tombstone, DespawnOnFinish, GameLayer
};

#[derive(Component)]
pub struct CubeDischarge {
    pub timer: Timer,
}

impl CubeDischarge {
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(POWER_ANIMATION_DURATION_SEC, TimerMode::Once),
        }
    }
}

// Constants for cube discharge detection
const CUBE_DISCHARGE_RADIUS: f32 = 20.0;


pub fn cube_plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, (register_cube_signals,))
        .add_systems(
            FixedUpdate,
            (
                cube_receive_power,
                cube_discharge_detection,
                update_cube_discharge_timers,
            )
                .run_if(in_state(GameState::Playing)),
        );
}

fn cube_discharge_detection(
    mut commands: Commands,
    q_cubes: Query<
        (Entity, &GlobalTransform),
        (
            With<WeightedCube>,
            With<Powered>,
            Without<Held>,
        ),
    >,
    spatial_query: SpatialQuery,
    q_collider_of: Query<&ColliderOf>,
    q_discharging: Query<(), With<CubeDischarge>>,
    q_powered: Query<(), With<Powered>>,
    time: Res<Time>,
) {
    for (cube_entity, cube_transform) in q_cubes.iter() {
        // Create spherical detection shape
        let detection_shape = Collider::sphere(CUBE_DISCHARGE_RADIUS);
        let cube_position = cube_transform.translation();

        // Find overlapping entities
        let overlapping = spatial_query.shape_intersections(
            &detection_shape,
            cube_position,
            Quat::IDENTITY,
            &SpatialQueryFilter::from_mask([GameLayer::Device]),
        );

        let mut any_discharged = false;
        // Check each overlapping entity
        for collider_entity in overlapping {
            // Skip self
            if collider_entity == cube_entity {
                continue;
            }

            // Get the rigid body entity if this is a collider
            let target_entity = if let Ok(collider_of) = q_collider_of.get(collider_entity) {
                collider_of.body
            } else {
                collider_entity
            };

            // Skip if it's the same as the cube entity or if it's a discharging cube, or if it's already powered
            if target_entity == cube_entity || q_discharging.contains(target_entity) || q_powered.contains(target_entity) {
                continue;
            }

            any_discharged = true;
            // Trigger DirectSignal on the target entity
            commands.trigger_targets(DirectSignal, target_entity);

            // Depower the cube and add discharge cooldown
            // Note, when we remove break, this needs to happen outside of the loop
        }

        if any_discharged {
            commands
                .entity(cube_entity)
                .remove::<Powered>()
                .try_insert(CubeDischarge::new());
        }
    }
}

fn cube_consume_signal(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_signals: Query<(), With<Signal>>,
    q_powered: Query<(), (With<Powered>, Without<PoweredTimer>)>,
    q_discharging: Query<(), With<CubeDischarge>>, // Check if cube is in cooldown
) {
    if let Some(device_body) = trigger.body {
        if q_signals.contains(trigger.collider) {
            // Don't power cubes that are already powered or in discharge cooldown
            if !q_powered.contains(device_body) && !q_discharging.contains(device_body) {
                commands.entity(device_body).insert(Powered);
                commands.entity(trigger.collider).despawn();
            }
        }
    }
}

/// System to update cube discharge cooldown timers
fn update_cube_discharge_timers(
    mut commands: Commands,
    time: Res<Time>,
    mut q_discharging_cubes: Query<(Entity, &mut CubeDischarge), With<WeightedCube>>,
    q_powered_by: Query<(), With<PoweredBy>>, // Add this query
) {
    for (cube_entity, mut discharge) in q_discharging_cubes.iter_mut() {
        discharge.timer.tick(time.delta());

        if discharge.timer.finished() {
            commands.entity(cube_entity).remove::<CubeDischarge>();
            
            // If this cube has PoweredBy, it should be re-powered immediately
            if q_powered_by.contains(cube_entity) {
                commands.entity(cube_entity).insert(Powered);
            }
        }
    }
}

fn cube_direct_signal(
    trigger: Trigger<DirectSignal>,
    mut commands: Commands,
    q_powered: Query<(), (With<Powered>)>,
    q_discharging: Query<(), With<CubeDischarge>>, // Check if cube is in cooldown
) {
    let target = trigger.target();

    // Don't power cubes that are already powered or in discharge cooldown
    if !q_powered.contains(target) && !q_discharging.contains(target) {
        commands.entity(target).insert(Powered);
    }
}

fn cube_receive_power(
    mut commands: Commands,
    q_powered_cube: Query<(Entity, &RigidBodyColliders), (With<WeightedCube>, Added<Powered>)>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
) {
    for (powered_cube, powered_cube_colliders) in &q_powered_cube {
        for collider_entity in powered_cube_colliders.iter() {
            // Clear existing tweens first
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
    }
}

fn cube_lose_power(
    trigger: Trigger<OnRemove, Powered>,
    mut commands: Commands,
    q_cube: Query<(Entity, &RigidBodyColliders), (With<WeightedCube>, Without<Tombstone>)>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
) {
    if let Ok((cube_entity, cube_colliders)) = q_cube.get(trigger.target()) {
        for collider_entity in cube_colliders.iter() {
            // Clear existing tweens first
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
    }
}

fn register_cube_signals(
    mut commands: Commands,
    q_new_cube: Query<
        (Entity, &RigidBodyColliders),
        (Added<RigidBodyColliders>, With<WeightedCube>),
    >,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    // probably not the right place, but we need to give each cube a dedicated material if it will be powered individually

    for (cube_entity, cube_children) in &q_new_cube {
        commands
            .entity(cube_entity)
            .observe(cube_direct_signal)
            .observe(cube_lose_power)
            ;

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
                    .observe(cube_consume_signal)
                    ;
            }
        }
    }
}

use super::{signals::{MaterialIntensityInterpolator, Powered, DirectSignal}, DespawnOnFinish, GameLayer};
use crate::{
    asset_management::asset_tag_components::{ChargePad, PressurePlate},
    rendering::unlit_material::UnlitMaterial,
};
use avian3d::{parry::bounding_volume::Aabb, prelude::*};
use bevy::{math::VectorSpace, prelude::*};
use bevy_tween::{
    combinator::tween,
    interpolate::translation,
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetAsset, TargetComponent},
};
use std::{collections::HashSet, time::Duration};

/// Component to store pressure plate detection data
#[derive(Component)]
pub struct PressurePlateDetector {
    /// Entities currently overlapping with this pressure plate
    pub overlapping_entities: HashSet<Entity>,
    /// Whether the plate is currently pressed (has any overlapping entities)
    pub is_pressed: bool,
}

impl Default for PressurePlateDetector {
    fn default() -> Self {
        Self {
            overlapping_entities: HashSet::new(),
            is_pressed: false,
        }
    }
}

/// Component for ChargePad signal emission configuration
#[derive(Component)]
pub struct ChargePadSignalEmitter {
    /// Timer for signal emission intervals
    pub timer: Timer,
    /// Size of the detection area above the charge pad
    pub detection_size: Vec3,
    /// Offset from the charge pad center for detection
    pub detection_offset: Vec3,
}

impl Default for ChargePadSignalEmitter {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(2.0, TimerMode::Repeating), // Default 2 seconds
            detection_size: Vec3::new(12.0, 8.0, 12.0), // Slightly larger than pressure plate
            detection_offset: Vec3::new(0.0, 4.0, 0.0), // Above the charge pad
        }
    }
}

/// Observer event fired when a pressure plate is first pressed
#[derive(Event)]
pub struct PressurePlatePressed {
    pub plate_entity: Entity,
    pub triggering_entity: Entity,
}

/// Observer event fired when a pressure plate is released
#[derive(Event)]
pub struct PressurePlateReleased {
    pub plate_entity: Entity,
    pub last_entity: Option<Entity>,
}

// Constants for detection box
const DETECTION_SIZE: Vec3 = Vec3::new(5.0, 9.0, 5.0);
const DETECTION_OFFSET: Vec3 = Vec3::new(0.0, 5.0, 0.0);

pub fn pressure_plate_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            register_pressure_plates,
            register_charge_pads,
            update_pressure_plate_overlaps,
            update_charge_pad_signal_emission,
            //debug_draw_pressure_plate_detection,
        )
            .chain(),
    );
}

fn debug_draw_pressure_plate_detection(
    mut gizmos: Gizmos,
    q_plates: Query<(&GlobalTransform, &PressurePlateDetector), With<PressurePlate>>,
    q_charge_pads: Query<(&GlobalTransform, &ChargePadSignalEmitter), With<ChargePad>>,
) {
    for (plate_transform, detector) in q_plates.iter() {
        let detection_center = plate_transform.translation() + DETECTION_OFFSET;
        let color = if detector.is_pressed {
            Color::srgb(1.0, 0.0, 0.0)
        } else {
            Color::srgb(0.0, 1.0, 0.0)
        };

        gizmos.cuboid(
            Transform::from_translation(detection_center).with_scale(DETECTION_SIZE),
            color,
        );
    }

    // Debug draw charge pad detection areas
    for (charge_pad_transform, emitter) in q_charge_pads.iter() {
        let detection_center = charge_pad_transform.translation() + emitter.detection_offset;
        let color = Color::srgb(0.0, 0.0, 1.0); // Blue for charge pads

        gizmos.cuboid(
            Transform::from_translation(detection_center).with_scale(emitter.detection_size),
            color,
        );
    }
}

#[derive(Component, Debug)]
#[relationship_target(relationship = PoweredBy)]
pub struct Powers(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target = Powers)]
pub struct PoweredBy(pub Entity);

fn register_pressure_plates(
    mut commands: Commands,
    q_new_plate: Query<(Entity, &Children, &ChildOf), Added<PressurePlate>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_children: Query<&Children>,
    q_charge_pad: Query<Entity, With<ChargePad>>,
) {
    for (plate_entity, plate_children, plate_parent) in &q_new_plate {
        // Add detector to the main plate entity
        commands
            .entity(plate_entity)
            .insert(PressurePlateDetector::default())
            .observe(on_pressure_plate_pressed)
            .observe(on_pressure_plate_released);

        // Find the sibling ChargePad entity
        if let Ok(parent_children) = q_children.get(plate_parent.parent()) {
            for sibling in parent_children.iter() {
                // Skip the pressure plate itself
                if sibling != plate_entity {
                    // Check if this sibling is a ChargePad
                    if q_charge_pad.contains(sibling) {
                        // Set up the relationship: PressurePlate Powers ChargePad
                        commands.entity(sibling)
                            .insert(PoweredBy(plate_entity))
                            .observe(charge_pad_receive_power)
                            .observe(charge_pad_lose_power);

                        if let Ok(charge_pad_children) = q_children.get(sibling) {
                            for charge_pad_child in charge_pad_children.iter() {
                                if let Ok(material_handle) = q_unlit_objects.get(charge_pad_child) {
                                    let old_material =
                                        unlit_materials.get(material_handle).unwrap().clone();

                                    commands.entity(charge_pad_child).insert((
                                        AnimationTarget,
                                        MeshMaterial3d(unlit_materials.add(old_material)),
                                    ));
                                }
                            }
                        }
                        break; // Assuming only one ChargePad sibling
                    }
                }
            }
        }

        for plate_child in plate_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(plate_child) {
                let old_material = unlit_materials.get(material_handle).unwrap().clone();
                commands.entity(plate_child).insert((
                    AnimationTarget,
                    MeshMaterial3d(unlit_materials.add(old_material)),
                    RigidBody::Kinematic,
                ));
            }
        }
    }
}

fn register_charge_pads(
    mut commands: Commands,
    q_new_charge_pad: Query<Entity, Added<ChargePad>>,
) {
    for charge_pad_entity in &q_new_charge_pad {
        // Add the signal emitter component with default settings
        commands
            .entity(charge_pad_entity)
            .insert(ChargePadSignalEmitter::default());
    }
}

fn update_charge_pad_signal_emission(
    mut commands: Commands,
    mut q_charge_pads: Query<
        (Entity, &GlobalTransform, &mut ChargePadSignalEmitter),
        (With<ChargePad>, With<Powered>), // Only emit signals when powered
    >,
    spatial_query: SpatialQuery,
    time: Res<Time>,
    q_collider_of: Query<&ColliderOf>, // To check if entity has a rigid body
) {
    for (charge_pad_entity, charge_pad_transform, mut emitter) in q_charge_pads.iter_mut() {
        // Update the timer
        emitter.timer.tick(time.delta());
        // Check if it's time to emit a signal
        if emitter.timer.just_finished() {
            // Calculate detection box center
            let detection_center = charge_pad_transform.translation() + emitter.detection_offset;

            // Create detection shape
            let detection_shape = Collider::cuboid(
                emitter.detection_size.x * 0.5,
                emitter.detection_size.y * 0.5,
                emitter.detection_size.z * 0.5,
            );

            // Find overlapping entities
            let overlapping = spatial_query.shape_intersections(
                &detection_shape,
                detection_center,
                Quat::IDENTITY,
                &SpatialQueryFilter::from_mask([
                    GameLayer::Player,
                    GameLayer::Device,
                ]),
            );

            // Fire DirectSignal on bodies of overlapping entities
            for entity in overlapping {
                // Skip the charge pad itself
                if entity != charge_pad_entity {
                    // Send the event to the body
                    if let Ok(collider_of) = q_collider_of.get(entity) {
                        commands.entity(collider_of.body).trigger(DirectSignal);
                    }
                }
            }
        }
    }
}

fn update_pressure_plate_overlaps(
    mut commands: Commands,
    mut q_plates: Query<
        (
            Entity,
            &GlobalTransform,
            &mut PressurePlateDetector,
            &Children,
        ),
        With<PressurePlate>,
    >,
    spatial_query: SpatialQuery,
) {
    for (plate_entity, plate_transform, mut detector, plate_children) in q_plates.iter_mut() {
        let mut current_overlaps = HashSet::new();

        // Calculate detection box center
        let detection_center = plate_transform.translation() + DETECTION_OFFSET;

        // Use spatial query to find overlapping entities
        let detection_shape = Collider::cuboid(
            DETECTION_SIZE.x * 0.5,
            DETECTION_SIZE.y * 0.5,
            DETECTION_SIZE.z * 0.5,
        );
        let overlapping = spatial_query.shape_intersections(
            &detection_shape,
            detection_center,
            Quat::IDENTITY,
            &SpatialQueryFilter::from_mask([GameLayer::Player, GameLayer::Device]),
        );

        for entity in overlapping {
            // Skip the plate itself and its children
            if entity != plate_entity && !plate_children.contains(&entity) {
                current_overlaps.insert(entity);
            }
        }

        // Detect new overlaps (entities that just entered)
        for &entity in &current_overlaps {
            if !detector.overlapping_entities.contains(&entity) {
                // New entity entered
                if !detector.is_pressed {
                    // Plate was not pressed, now it is
                    detector.is_pressed = true;
                    commands.trigger_targets(
                        PressurePlatePressed {
                            plate_entity,
                            triggering_entity: entity,
                        },
                        plate_entity,
                    );
                }
            }
        }

        // Detect entities that left
        let entities_that_left: Vec<Entity> = detector
            .overlapping_entities
            .difference(&current_overlaps)
            .copied()
            .collect();

        // Update the overlapping entities
        detector.overlapping_entities = current_overlaps;

        // Check if plate should be released
        if detector.is_pressed && detector.overlapping_entities.is_empty() {
            detector.is_pressed = false;
            commands.trigger_targets(
                PressurePlateReleased {
                    plate_entity,
                    last_entity: entities_that_left.first().copied(),
                },
                plate_entity,
            );
        }
    }
}

fn on_pressure_plate_pressed(
    trigger: Trigger<PressurePlatePressed>,
    mut commands: Commands,
    q_plate_children: Query<(&Children, &Powers), With<PressurePlate>>,
) {
    let plate_entity = trigger.event().plate_entity;
    if let Ok((plate_children, power_targets)) = q_plate_children.get(plate_entity) {
        for child in plate_children {
            commands.entity(*child).animation().insert(tween(
                Duration::from_millis(500),
                EaseKind::CubicOut,
                TargetComponent::marker().with(translation(Vec3::ZERO, -Vec3::Y * 1.0)),
            ));
        }

        for target in power_targets.iter() {
            commands.entity(target).insert(Powered);
        }
    }
}

fn on_pressure_plate_released(
    trigger: Trigger<PressurePlateReleased>,
    mut commands: Commands,
    q_plate_children: Query<(&Children, &Powers), With<PressurePlate>>,
) {
    let plate_entity = trigger.event().plate_entity;
    if let Ok((plate_children, power_targets)) = q_plate_children.get(plate_entity) {
        for child in plate_children {
            commands.entity(*child).animation().insert(tween(
                Duration::from_millis(500),
                EaseKind::CubicOut,
                TargetComponent::marker().with(translation(-Vec3::Y * 1.0, Vec3::ZERO)),
            ));
        }
        
        for target in power_targets.iter() {
            commands.entity(target).remove::<Powered>();
        }
    }
}

pub const POWER_MATERIAL_INTENSITY: f32 = 20.0;
pub const POWER_ANIMATION_DURATION_SEC: f32 = 1.0;

fn charge_pad_receive_power(
    trigger: Trigger<OnAdd, Powered>,
    mut commands: Commands,
    q_charge_pad: Query<(Entity, &Children), With<ChargePad>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
) {
    if let Ok((charge_pad, charge_pad_children)) = q_charge_pad.get(trigger.target()) {
        for collider_entity in charge_pad_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                if let Some(material) = unlit_materials.get(material_handle) {
                    let current_intensity = material.extension.params.intensity;
                    let intensity_ratio = (POWER_MATERIAL_INTENSITY - current_intensity) / (POWER_MATERIAL_INTENSITY - 1.0);
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

fn charge_pad_lose_power(
    trigger: Trigger<OnRemove, Powered>,
    mut commands: Commands,
    q_charge_pad: Query<(Entity, &Children), With<ChargePad>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
) {
    if let Ok((charge_pad, charge_pad_children)) = q_charge_pad.get(trigger.target()) {
        for collider_entity in charge_pad_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                if let Some(material) = unlit_materials.get(material_handle) {
                    let current_intensity = material.extension.params.intensity;

                    let intensity_ratio = (current_intensity - 1.0) / (POWER_MATERIAL_INTENSITY - 1.0);
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

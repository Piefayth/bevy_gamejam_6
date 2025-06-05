use super::{
    DespawnOnFinish, GameLayer,
    signals::{MaterialIntensityInterpolator, Powered},
};
use crate::{
    asset_management::asset_tag_components::{ChargePad, PressurePlate, WeightedCube},
    rendering::unlit_material::UnlitMaterial, GameState,
};
use avian3d::prelude::*;
use bevy::{math::VectorSpace, prelude::*};
use bevy_tween::{
    bevy_time_runner::TimeSpan, combinator::tween, interpolate::translation, prelude::{AnimationBuilderExt, EaseKind}, tween::{AnimationTarget, TargetAsset, TargetComponent}
};
use std::{collections::HashSet, time::Duration};

/// Component to store pressure plate detection data
#[derive(Component, Default)]
pub struct PressurePlateDetector {
    /// Entities currently overlapping with this pressure plate
    pub overlapping_entities: HashSet<Entity>,
    /// Whether the plate is currently pressed (has any overlapping entities)
    pub is_pressed: bool,
}

/// Component for ChargePad detection configuration
#[derive(Component)]
pub struct ChargePadDetector {
    /// Size of the detection area above the charge pad
    pub detection_size: Vec3,
    /// Offset from the charge pad center for detection
    pub detection_offset: Vec3,
    /// Currently charged entity (only one at a time)
    pub charged_entity: Option<Entity>,
    /// Entities currently overlapping with this charge pad
    pub overlapping_entities: HashSet<Entity>,
}

impl Default for ChargePadDetector {
    fn default() -> Self {
        Self {
            detection_size: Vec3::new(15.0, 8.0, 15.0), // Slightly larger than pressure plate
            detection_offset: Vec3::new(0.0, 4.0, 0.0), // Above the charge pad
            charged_entity: None,
            overlapping_entities: HashSet::new(),
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

/// Observer event fired when an entity enters a charge pad
#[derive(Event)]
pub struct ChargePadEntityEntered {
    pub charge_pad_entity: Entity,
    pub entity: Entity,
}

/// Observer event fired when an entity leaves a charge pad
#[derive(Event)]
pub struct ChargePadEntityLeft {
    pub charge_pad_entity: Entity,
    pub entity: Entity,
}

// Constants for detection box
const DETECTION_SIZE: Vec3 = Vec3::new(5.0, 9.0, 5.0);
const DETECTION_OFFSET: Vec3 = Vec3::new(0.0, 5.0, 0.0);

pub fn pressure_plate_plugin(app: &mut App) {
    app.add_systems(
        FixedPreUpdate,
        (register_pressure_plates, register_charge_pads),
    )
    .add_systems(
        FixedUpdate,
        (update_pressure_plate_overlaps, update_charge_pad_overlaps).run_if(in_state(GameState::Playing)),
    );
}

fn debug_draw_pressure_plate_detection(
    mut gizmos: Gizmos,
    q_plates: Query<(&GlobalTransform, &PressurePlateDetector), With<PressurePlate>>,
    q_charge_pads: Query<(&GlobalTransform, &ChargePadDetector), With<ChargePad>>,
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
    for (charge_pad_transform, detector) in q_charge_pads.iter() {
        let detection_center = charge_pad_transform.translation() + detector.detection_offset;
        let color = if detector.charged_entity.is_some() {
            Color::srgb(1.0, 1.0, 0.0) // Yellow when charging something
        } else {
            Color::srgb(0.0, 0.0, 1.0) // Blue when idle
        };

        gizmos.cuboid(
            Transform::from_translation(detection_center).with_scale(detector.detection_size),
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
                        commands
                            .entity(sibling)
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

fn register_charge_pads(mut commands: Commands, q_new_charge_pad: Query<Entity, Added<ChargePad>>) {
    for charge_pad_entity in &q_new_charge_pad {
        // Add the detector component with default settings
        commands
            .entity(charge_pad_entity)
            .insert(ChargePadDetector::default())
            .observe(on_charge_pad_entity_entered)
            .observe(on_charge_pad_entity_left);
    }
}

fn update_charge_pad_overlaps(
    mut commands: Commands,
    mut q_charge_pads: Query<
        (Entity, &GlobalTransform, &mut ChargePadDetector, &Children, Option<&Powered>),
        (With<ChargePad>),
    >,
    spatial_query: SpatialQuery,
    q_collider_of: Query<&ColliderOf>, // To check if entity has a rigid body
) {
    for (charge_pad_entity, charge_pad_transform, mut detector, charge_pad_children, maybe_powered) in
        q_charge_pads.iter_mut()
    {
        let mut current_overlaps = HashSet::new();

        // Calculate detection box center
        let detection_center = charge_pad_transform.translation() + detector.detection_offset;

        // Create detection shape
        let detection_shape = Collider::cuboid(
            detector.detection_size.x * 0.5,
            detector.detection_size.y * 0.5,
            detector.detection_size.z * 0.5,
        );

        // Find overlapping entities
        let overlapping = spatial_query.shape_intersections(
            &detection_shape,
            detection_center,
            Quat::IDENTITY,
            &SpatialQueryFilter::from_mask([GameLayer::Device]),
        );

        // Convert collider entities to their bodies and filter out the charge pad itself
        for entity in overlapping {
            if entity != charge_pad_entity && !charge_pad_children.contains(&entity) {
                if let Ok(collider_of) = q_collider_of.get(entity) {
                    current_overlaps.insert(collider_of.body);
                }
            }
        }

        // Detect new overlaps (entities that just entered)
        for &entity in &current_overlaps {
            if !detector.overlapping_entities.contains(&entity) {
                // New entity entered
                commands.trigger_targets(
                    ChargePadEntityEntered {
                        charge_pad_entity,
                        entity,
                    },
                    charge_pad_entity,
                );
            }
        }

        // Detect entities that left
        let entities_that_left: Vec<Entity> = detector
            .overlapping_entities
            .difference(&current_overlaps)
            .copied()
            .collect();

        for entity in entities_that_left {
            commands.trigger_targets(
                ChargePadEntityLeft {
                    charge_pad_entity,
                    entity,
                },
                charge_pad_entity,
            );
        }

        // Update the overlapping entities
        detector.overlapping_entities = current_overlaps;
    }
}

fn on_charge_pad_entity_entered(
    trigger: Trigger<ChargePadEntityEntered>,
    mut commands: Commands,
    mut q_charge_pad: Query<(&mut ChargePadDetector, Option<&Powered>), With<ChargePad>>,
) {
    let event = trigger.event();
    let charge_pad_entity = event.charge_pad_entity;
    let entering_entity = event.entity;

    if let Ok((mut detector, maybe_powered)) = q_charge_pad.get_mut(charge_pad_entity) {
        // If no entity is currently being charged, charge this one
        if detector.charged_entity.is_none() {
            detector.charged_entity = Some(entering_entity);

            // Add Powered component and PoweredBy relationship
            if maybe_powered.is_some() {
                commands
                    .entity(entering_entity)
                    .insert(Powered)
                    .insert(PoweredBy(charge_pad_entity));
            }

        }
    }
}

fn on_charge_pad_entity_left(
    trigger: Trigger<ChargePadEntityLeft>,
    mut commands: Commands,
    mut q_charge_pad: Query<&mut ChargePadDetector, With<ChargePad>>,
    q_powered_by: Query<&PoweredBy>,
    q_cubes: Query<&WeightedCube>,
) {
    let event = trigger.event();
    let charge_pad_entity = event.charge_pad_entity;
    let leaving_entity = event.entity;

    if let Ok(mut detector) = q_charge_pad.get_mut(charge_pad_entity) {
        // If this is the entity we're currently charging, stop charging it
        if detector.charged_entity == Some(leaving_entity) {
            detector.charged_entity = None;

            // Remove Powered component only if it's powered by this charge pad
            if let Ok(powered_by) = q_powered_by.get(leaving_entity) {
                if powered_by.0 == charge_pad_entity {
                    if !q_cubes.contains(leaving_entity) {
                        commands
                            .entity(leaving_entity)
                            .remove::<Powered>()
                            .remove::<PoweredBy>();
                    } else {
                        // cubes RETAIN power
                        commands.entity(leaving_entity).remove::<PoweredBy>();
                    }
                }
            }

            // Check if there are other entities we can start charging
            // Priority: charge the first available entity in the overlapping set
            if let Some(&next_entity) = detector.overlapping_entities.iter().next() {
                if next_entity != leaving_entity {
                    detector.charged_entity = Some(next_entity);
                    commands
                        .entity(next_entity)
                        .insert(Powered)
                        .insert(PoweredBy(charge_pad_entity));
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
    q_charge_pad: Query<(Entity, &Children, &ChargePadDetector), With<ChargePad>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
) {
    if let Ok((charge_pad, charge_pad_children, detector)) = q_charge_pad.get(trigger.target()) {
        if let Some(charged_entity) = detector.charged_entity {
            // Verify the entity is actually powered by this charge pad
            commands
                .entity(charged_entity)
                .try_insert((Powered, PoweredBy(charge_pad)));
        }

        for collider_entity in charge_pad_children.iter() {
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

fn charge_pad_lose_power(
    trigger: Trigger<OnRemove, Powered>,
    mut commands: Commands,
    q_charge_pad: Query<(Entity, &Children, &ChargePadDetector), With<ChargePad>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    unlit_materials: Res<Assets<UnlitMaterial>>,
    q_powered_by: Query<&PoweredBy>,
    q_cubes: Query<&WeightedCube>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children, With<Collider>>,
) {
    if let Ok((charge_pad, charge_pad_children, detector)) = q_charge_pad.get(trigger.target()) {
        // Remove power from any entity this charge pad is currently charging
        if let Some(charged_entity) = detector.charged_entity {
            // Verify the entity is actually powered by this charge pad
            if let Ok(powered_by) = q_powered_by.get(charged_entity) {
                if powered_by.0 == charge_pad {
                    if !q_cubes.contains(charged_entity) {
                        commands
                            .entity(charged_entity)
                            .remove::<Powered>()
                            .remove::<PoweredBy>();
                    } else {
                        // cubes RETAIN power
                        commands.entity(charged_entity).remove::<PoweredBy>();
                    }
                }
            }
        }

        // Animate the charge pad's visual feedback
        for collider_entity in charge_pad_children.iter() {
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

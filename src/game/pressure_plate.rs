use super::GameLayer;
use crate::{
    asset_management::asset_tag_components::PressurePlate, rendering::unlit_material::UnlitMaterial,
};
use avian3d::{parry::bounding_volume::Aabb, prelude::*};
use bevy::{math::VectorSpace, prelude::*};
use bevy_tween::{
    combinator::tween,
    interpolate::translation,
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetComponent},
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
            update_pressure_plate_overlaps,
            //debug_draw_pressure_plate_detection,
        )
            .chain(),
    );
}

fn debug_draw_pressure_plate_detection(
    mut gizmos: Gizmos,
    q_plates: Query<(&GlobalTransform, &PressurePlateDetector), With<PressurePlate>>,
) {
    for (plate_transform, detector) in q_plates.iter() {
        let detection_center = plate_transform.translation() + DETECTION_OFFSET;
        let color = if detector.is_pressed {
            Color::srgb(1.0, 0.0, 0.0)
        } else {
            Color::srgb(0.0, 1.0, 0.0)
        };

        // Fix: Use the same half-extents as the spatial query
        gizmos.cuboid(
            Transform::from_translation(detection_center)
                .with_scale(DETECTION_SIZE), // This should match your collider size
            color,
        );
    }
}

fn register_pressure_plates(
    mut commands: Commands,
    q_new_plate: Query<(Entity, &Children), Added<PressurePlate>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for (plate_entity, plate_children) in &q_new_plate {
        // Add detector to the main plate entity
        commands
            .entity(plate_entity)
            .insert(PressurePlateDetector::default())
            .observe(on_pressure_plate_pressed)
            .observe(on_pressure_plate_released);

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

                // TODO: If the new entity that entered was powered, we should power the pressure plate
                // Which is a different event than pressing it?
            }
        }

        // Detect entities that left
        let entities_that_left: Vec<Entity> = detector
            .overlapping_entities
            .difference(&current_overlaps)
            .copied()
            .collect();

        if entities_that_left.len() > 0 {
                println!("exit");
        }
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
    q_plate_children: Query<&Children, With<PressurePlate>>,
) {
    let plate_entity = trigger.event().plate_entity;
    if let Ok(plate_children) = q_plate_children.get(plate_entity) {
        for child in plate_children {
            commands.entity(*child).animation().insert(tween(
                Duration::from_millis(500),
                EaseKind::CubicOut,
                TargetComponent::marker().with(translation(Vec3::ZERO, -Vec3::Y * 1.0)),
            ));
        }
    }
}

fn on_pressure_plate_released(
    trigger: Trigger<PressurePlateReleased>,
    mut commands: Commands,
    q_plate_children: Query<&Children, With<PressurePlate>>,
) {
    let plate_entity = trigger.event().plate_entity;
    if let Ok(plate_children) = q_plate_children.get(plate_entity) {
        for child in plate_children {
            commands.entity(*child).animation().insert(tween(
                Duration::from_millis(500),
                EaseKind::CubicOut,
                TargetComponent::marker().with(translation(-Vec3::Y * 1.0, Vec3::ZERO)),
            ));
        }
    }
}

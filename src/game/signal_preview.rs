use std::collections::HashSet;
use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use crate::{
    asset_management::asset_tag_components::{Immobile, SignalSpitter},
    game::{
        player::{Held, RightHand},
        signals::{MAX_SIGNAL_LIFETIME_SECS, MAX_SIGNAL_TRAVEL_DIST}, GameLayer,
    },
    rendering::unlit_material::{MaterialColorOverrideInterpolator, UnlitMaterial},
};
use bevy_tween::{
    combinator::tween,
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetAsset},
};
use std::time::Duration;

const IMMOBILE_SPIT_SIZE: f32 = 30.;
const STANDARD_SPIT_SIZE: f32 = 10.;
const SIGNAL_SHAPE_DEPTH: f32 = 2.0;

#[derive(Component, Default)]
pub struct SignalPreview {
    pub highlighted_entities: HashSet<Entity>,
}

pub fn signal_preview_plugin(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            update_signal_preview,
            cleanup_signal_preview_on_drop,
            cleanup_signal_preview_on_invalid_placement,
            initialize_signal_preview
        ),
    );
}

fn update_signal_preview(
    spatial_query: SpatialQuery,
    mut q_held_spitters: Query<
        (Entity, &mut SignalPreview, &GlobalTransform, Has<Immobile>),
        (With<SignalSpitter>, With<Held>)
    >,
    q_unlit_materials: Query<&MeshMaterial3d<UnlitMaterial>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    right_hand: Single<&RightHand>,
    //mut gizmos: Gizmos,
) {
    for (spitter_entity, mut preview, spitter_transform, is_immobile) in &mut q_held_spitters {
        // Check if this spitter is actually being held by the player
        if right_hand.held_object != Some(spitter_entity) {
            continue;
        }

        // Determine signal size based on spitter type
        let signal_size = if is_immobile {
            IMMOBILE_SPIT_SIZE
        } else {
            STANDARD_SPIT_SIZE
        };

        // Calculate signal spawn position and direction
        let y_offset = if signal_size > 10. { 20. } else { 10. };
        let spitter_forward = -spitter_transform.forward();
        let signal_start = spitter_transform.translation() 
            + Vec3::Y * y_offset 
            + spitter_forward * 10.;

        // Create shape for the signal path
        let signal_shape = Collider::cuboid(signal_size, signal_size, SIGNAL_SHAPE_DEPTH);
        
        // Calculate how far the signal travels over its lifetime
        let total_distance = MAX_SIGNAL_TRAVEL_DIST;
        
        // Draw visualization of the cast
        let cast_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, spitter_forward.into());
        
        // // Draw the starting position of the signal
        // gizmos.cuboid(
        //     Transform::from_translation(signal_start)
        //         .with_rotation(cast_rotation)
        //         .with_scale(Vec3::new(signal_size, signal_size, SIGNAL_SHAPE_DEPTH)),
        //     Color::srgba(0.0, 1.0, 1.0, 0.3), // Cyan, semi-transparent
        // );
        
        // // Draw the cast direction as a line
        // gizmos.line(
        //     signal_start,
        //     signal_start + spitter_forward * total_distance,
        //     Color::srgb(0.0, 1.0, 1.0), // Cyan
        // );

        // Perform single shapecast along the signal path
        let mut new_highlighted = HashSet::new();
        
        // Cast the shape along the path to find the first hit
        if let Some(hit_info) = spatial_query.cast_shape(
            &signal_shape,
            signal_start,
            cast_rotation,
            Dir3::new(spitter_forward.into()).unwrap(),
            &ShapeCastConfig::default(),
            &SpatialQueryFilter::default().with_mask([GameLayer::Device])
        ) {
    let hit_distance = hit_info.distance;
    let hit_position = signal_start + spitter_forward * hit_distance;
    
    // Always include the entity that was actually hit by the cast
    new_highlighted.insert(hit_info.entity);
    
    // Make the intersection query more robust - try multiple approaches:
    
    // Approach 1: Slightly larger shape for intersection
    let expanded_shape = Collider::cuboid(
        signal_size + 0.1, 
        signal_size + 0.1, 
        SIGNAL_SHAPE_DEPTH + 0.1
    );
    
    let entities_at_hit = spatial_query.shape_intersections(
        &expanded_shape,  // Slightly larger shape
        hit_position,
        cast_rotation,
        &SpatialQueryFilter::default().with_mask([GameLayer::Device])
    );
    
    for entity in entities_at_hit {
        new_highlighted.insert(entity);
    }
    
    // Approach 2: Also check a small area around the hit point
    let nearby_entities = spatial_query.shape_intersections(
        &signal_shape,
        hit_position + Vec3::new(0.1, 0.0, 0.0), // Slight offset
        cast_rotation,
        &SpatialQueryFilter::default().with_mask([GameLayer::Device])
    );
    
    for entity in nearby_entities {
        new_highlighted.insert(entity);
    }
        } else {
            // No hit - draw the full path in a different color
            // gizmos.cuboid(
            //     Transform::from_translation(signal_start + spitter_forward * total_distance)
            //         .with_rotation(cast_rotation)
            //         .with_scale(Vec3::new(signal_size, signal_size, SIGNAL_SHAPE_DEPTH)),
            //     Color::srgba(0.0, 1.0, 0.0, 0.3), // Green, semi-transparent for no hit
            // );
        }

        // Remove highlighting from entities no longer in the path
        for &entity in &preview.highlighted_entities {
            if !new_highlighted.contains(&entity) {
                if let Ok(material_handle) = q_unlit_materials.get(entity) {
                    // Return to original state (white blend_color, 0 blend_factor)
                    if let Some(material) = unlit_materials.get_mut(material_handle) {
                        material.extension.params.blend_color = LinearRgba::WHITE;
                        material.extension.params.blend_factor = 0.0;
                    }
                }
            }
        }

        // Add highlighting to new entities in the path
        for &entity in &new_highlighted {
            if !preview.highlighted_entities.contains(&entity) {
                if let Ok(material_handle) = q_unlit_materials.get(entity) {
                    // Set to green highlighting
                    if let Some(material) = unlit_materials.get_mut(material_handle) {
                        material.extension.params.blend_color = LinearRgba::rgb(0.0, 1.0, 0.0);
                        material.extension.params.blend_factor = 1.0;
                    }
                }
            }
        }

        // Update the stored set
        preview.highlighted_entities = new_highlighted;
    }
}

fn cleanup_signal_preview_on_drop(
    mut q_spitters_losing_held: RemovedComponents<Held>,
    mut q_spitter_preview: Query<&mut SignalPreview, With<SignalSpitter>>,
    q_unlit_materials: Query<&MeshMaterial3d<UnlitMaterial>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
) {
    for entity in q_spitters_losing_held.read() {
        if let Ok(mut preview) = q_spitter_preview.get_mut(entity) {
            // Clear all highlighting
            for &highlighted_entity in &preview.highlighted_entities {
                if let Ok(material_handle) = q_unlit_materials.get(highlighted_entity) {
                    if let Some(material) = unlit_materials.get_mut(material_handle) {
                        material.extension.params.blend_color = LinearRgba::WHITE;
                        material.extension.params.blend_factor = 0.0;
                    }
                }
            }
            preview.highlighted_entities.clear();
        }
    }
}

fn cleanup_signal_preview_on_invalid_placement(
    mut q_held_spitters: Query<(&mut SignalPreview, &Held), (With<SignalSpitter>, With<Held>)>,
    q_unlit_materials: Query<&MeshMaterial3d<UnlitMaterial>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
) {
    for (mut preview, held) in &mut q_held_spitters {
        // If placement is invalid (can't release), clear highlighting
        if !held.can_release {
            for &highlighted_entity in &preview.highlighted_entities {
                if let Ok(material_handle) = q_unlit_materials.get(highlighted_entity) {
                    if let Some(material) = unlit_materials.get_mut(material_handle) {
                        material.extension.params.blend_color = LinearRgba::WHITE;
                        material.extension.params.blend_factor = 0.0;
                    }
                }
            }
            preview.highlighted_entities.clear();
        }
    }
}

// System to add SignalPreview component to new signal spitters
pub fn initialize_signal_preview(
    mut commands: Commands,
    q_new_spitters: Query<Entity, (Added<SignalSpitter>, Without<SignalPreview>)>,
) {
    for entity in &q_new_spitters {
        commands.entity(entity).insert(SignalPreview::default());
    }
}

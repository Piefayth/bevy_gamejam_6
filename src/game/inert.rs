use std::time::Duration;

use avian3d::prelude::{CollisionEventsEnabled, CollisionLayers, RigidBody};
use bevy::prelude::*;
use bevy_tween::{
    combinator::{parallel, tween},
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetAsset},
};

use crate::{
    asset_management::asset_tag_components::Inert,
    rendering::unlit_material::{MaterialColorOverrideInterpolator, UnlitMaterial},
};

use super::{
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY},
    signals::{default_signal_collisions, DirectSignal, MaterialIntensityInterpolator},
    GameLayer,
};

pub fn inert_plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, register_inert);
}

fn register_inert(
    mut commands: Commands,
    q_new_inert: Query<(Entity, &Children), Added<Inert>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for (inert_entity, inert_children) in &q_new_inert {
        commands
            .entity(inert_entity)
            .insert(RigidBody::Static)
            .observe(inert_direct_signal);

        for inert_child in inert_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(inert_child) {
                if let Some(old_material) = unlit_materials.get(material_handle) {
                    let new_material = old_material.clone();
                    commands
                        .entity(inert_child)
                        .insert((
                            AnimationTarget,
                            MeshMaterial3d(unlit_materials.add(new_material)),
                            CollisionLayers::new(
                                GameLayer::Device,
                                [GameLayer::Player, GameLayer::Signal, GameLayer::Device],
                            ),
                            CollisionEventsEnabled,
                        ))
                        .observe(default_signal_collisions);
                }
            }
        }
    }
}

fn inert_direct_signal(
    trigger: Trigger<DirectSignal>,
    mut commands: Commands,
    q_inert: Query<&Children, With<Inert>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
) {
    let inert_entity = trigger.target();

    if let Ok(inert_children) = q_inert.get(inert_entity) {
        for child in inert_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(child) {
                // Instantly snap to max brightness
                if let Some(material) = unlit_materials.get_mut(material_handle) {
                    material.extension.params.intensity = POWER_MATERIAL_INTENSITY;
                }

                // Then tween down to dim
                commands.entity(child).animation().insert(parallel((
                    tween(
                        Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 1000.) as u64),
                        EaseKind::CubicOut,
                        TargetAsset::Asset(material_handle.clone_weak()).with(
                            MaterialIntensityInterpolator {
                                start: POWER_MATERIAL_INTENSITY,
                                end: 1.0, // Normal intensity
                            },
                        ),
                    ),
                    tween(
                        Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 1000.) as u64),
                        EaseKind::CubicOut,
                        TargetAsset::Asset(material_handle.clone_weak()).with(
                            MaterialColorOverrideInterpolator {
                                target_color: LinearRgba::new(
                                    2. / 255.,
                                    76. / 255.,
                                    128. / 255.,
                                    1.0,
                                ),
                            },
                        ),
                    ),
                )));
            }
        }
    }
}

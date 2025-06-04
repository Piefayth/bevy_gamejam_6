use std::time::Duration;

use avian3d::prelude::{
    Collider, ColliderConstructor, CollisionEventsEnabled, CollisionLayers, ExternalImpulse,
    OnCollisionStart, RigidBody, RigidBodyColliders, RotationInterpolation, Sensor,
    TransformInterpolation,
};
use bevy::prelude::*;
use bevy_tween::{
    combinator::{sequence, tween},
    interpolate::translation,
    prelude::{AnimationBuilderExt, EaseKind, Interpolator},
    tween::{AnimationTarget, IntoTarget, TargetAsset, TargetComponent},
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
    pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY}, signals::{default_signal_collisions, DirectSignal, MaterialIntensityInterpolator, SignalAfterDelay}, DespawnOnFinish, GameLayer
};

pub fn signal_spitter_plugin(app: &mut App) {
    app.add_systems(Update, register_signal_spitter_signals);
}

fn signal_spitter_direct_signal(
    trigger: Trigger<DirectSignal>,
    mut commands: Commands,
    q_spitter: Query<(&RigidBodyColliders), (With<SignalSpitter>)>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    time: Res<Time>,
) {
    if let Ok(spitter_colliders) = q_spitter.get(trigger.target()) {
        for collider_entity in spitter_colliders.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(collider_entity) {
                commands
                    .entity(trigger.target())
                    .with_child(SignalAfterDelay {
                        delay_ms: (POWER_ANIMATION_DURATION_SEC * 1000.) as u32,
                        spawn_time: time.elapsed(),
                    });
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

fn register_signal_spitter_signals(
    mut commands: Commands,
    q_new_signal_spitter: Query<
        (Entity, &RigidBodyColliders),
        (
            Added<RigidBodyColliders>,
            With<SignalSpitter>,
            Without<Collider>,
        ),
    >,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for (spitter_entity, spitter_children) in &q_new_signal_spitter {
        for spitter_child in spitter_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(spitter_child) {
                let old_material = unlit_materials.get(material_handle).unwrap().clone();

                commands
                    .entity(spitter_child)
                    .insert((
                        CollisionEventsEnabled,
                        CollisionLayers::new(
                            GameLayer::Device,
                            [
                                GameLayer::Dissolve,
                                GameLayer::Signal,
                                GameLayer::Player,
                                GameLayer::Default,
                            ],
                        ),
                        AnimationTarget,
                        MeshMaterial3d(unlit_materials.add(old_material)),
                    ))
                    .observe(default_signal_collisions);
            }
        }
        commands
            .entity(spitter_entity)
            .observe(signal_spitter_direct_signal);
    }
}

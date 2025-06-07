use std::f32::consts::FRAC_PI_4;

use avian3d::prelude::{
    ColliderOf, CollisionEventsEnabled, CollisionLayers, OnCollisionStart, Sensor,
};
use bevy::{color::palettes::tailwind::PURPLE_300, prelude::*};

use crate::{
    asset_management::asset_tag_components::DissolveGate,
    game::{player::Held, standing_cube_spitter::Tombstone},
    rendering::{
        test_material::{TestMaterial, TestMaterialExtension, TestMaterialParams},
        unlit_material::UnlitMaterial,
    },
};

use super::{
    player::{Player, RightHand},
    GameLayer,
};

pub fn dissolve_gate_plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, (register_dissolve_gates,));
}

// Indicates a device can be dissolved

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Dissolveable {
    pub respawn_transform: Option<Transform>,
}

fn register_dissolve_gates(
    mut commands: Commands,
    q_new_gate: Query<&Children, Added<DissolveGate>>,
    unlit_materials: ResMut<Assets<UnlitMaterial>>,
    mut test_materials: ResMut<Assets<TestMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for gate_children in &q_new_gate {
        for gate_child in gate_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(gate_child) {
                let mut old_material = unlit_materials.get(material_handle).unwrap().clone();
                old_material.base.alpha_mode = AlphaMode::Blend;
                // old_material.extension.params.alpha = 0.5;
                // old_material.extension.params.blend_color = RED.into();
                // old_material.extension.params.blend_factor = 1.0;

                let test_material = test_materials.add(TestMaterial {
                    base: old_material.base,
                    extension: TestMaterialExtension {
                        params: TestMaterialParams {
                            stripe_color: PURPLE_300.into(),
                            stripe_frequency: 20.0,
                            stripe_angle: FRAC_PI_4,
                            stripe_thickness: 0.95,
                            scroll_speed: 0.05,
                        },
                    },
                });
                commands
                    .entity(gate_child)
                    .remove::<MeshMaterial3d<UnlitMaterial>>()
                    .insert((
                        MeshMaterial3d(test_material),
                        CollisionEventsEnabled,
                        CollisionLayers::new(
                            GameLayer::Default,
                            [GameLayer::Device, GameLayer::Player],
                        ),
                        Sensor,
                    ))
                    .observe(handle_dissolve_collisions);
            }
        }
    }
}

pub fn handle_dissolve_collisions(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_dissolveable: Query<&Dissolveable>,
    q_player: Query<&RightHand, With<Player>>,
    q_collider_of: Query<&ColliderOf>,
    q_dissolve_gates: Query<(Entity, &DissolveGate)>,
    q_child_of: Query<&ChildOf>,
) {
    let device_or_player_collider_entity = trigger.collider;
    let maybe_dissolve_gate = trigger.target();

    if let Ok(maybe_dissolve_gate_parent) = q_child_of.get(maybe_dissolve_gate) {
        if !q_dissolve_gates.contains(maybe_dissolve_gate_parent.0) {
            return;
        }
    } else {
        return;
    }

    if let Ok(targeted_body) = q_collider_of.get(device_or_player_collider_entity) {
        if let Ok(dissolveable) = q_dissolveable.get(targeted_body.body) {
            match &dissolveable.respawn_transform {
                Some(respawn_transform) => {
                    // Respawn the entity at the specified transform
                    commands
                        .entity(targeted_body.body)
                        .insert(*respawn_transform);
                    info!(
                        "Dissolved entity {:?} respawned at {:?}",
                        targeted_body.body, respawn_transform
                    );
                }
                None => {
                    // Despawn the entity
                    if let Ok(mut ec) = commands.get_entity(targeted_body.body) {
                        ec.insert(Tombstone).despawn();

                        info!("Dissolved entity {:?} despawned", targeted_body.body);
                    }
                }
            }
            return;
        }

        // Check if the colliding entity is a player with a held object
        if let Ok(right_hand) = q_player.get(targeted_body.body) {
            if let Some(held_entity) = right_hand.held_object {
                if let Ok(dissolveable) = q_dissolveable.get(held_entity) {
                    match &dissolveable.respawn_transform {
                        Some(respawn_transform) => {
                            // Respawn the held entity at the specified transform and remove Held component
                            commands
                                .entity(held_entity)
                                .insert(*respawn_transform)
                                .remove::<Held>();
                            info!(
                                "Dissolved held entity {:?} respawned at {:?}",
                                held_entity, respawn_transform
                            );
                        }
                        None => {
                            if let Ok(mut ec) = commands.get_entity(held_entity) {
                                ec.insert(Tombstone).despawn();

                                info!("Dissolved entity {:?} despawned", held_entity);
                            }
                        }
                    }
                }
            }
        }
    }
}

use std::f32::consts::FRAC_PI_4;

use avian3d::prelude::{
    ColliderOf, CollisionEventsEnabled, CollisionLayers, OnCollisionStart, Sensor,
};
use bevy::{color::palettes::tailwind::ORANGE_300, prelude::*, render::view::NoFrustumCulling};

use crate::{
    asset_management::asset_tag_components::DischargeGate,
    game::signals::Powered,
    rendering::{
        test_material::{TestMaterial, TestMaterialExtension, TestMaterialParams},
        unlit_material::UnlitMaterial,
    },
};

use super::{
    player::{Player, RightHand},
    GameLayer,
};

pub fn discharge_gate_plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, (register_discharge_gates,));
}

fn register_discharge_gates(
    mut commands: Commands,
    q_new_gate: Query<&Children, Added<DischargeGate>>,
    unlit_materials: ResMut<Assets<UnlitMaterial>>,
    mut test_materials: ResMut<Assets<TestMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for gate_children in &q_new_gate {
        for gate_child in gate_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(gate_child) {
                let mut old_material = unlit_materials.get(material_handle).unwrap().clone();
                old_material.base.alpha_mode = AlphaMode::Blend;

                let test_material = test_materials.add(TestMaterial {
                    base: old_material.base,
                    extension: TestMaterialExtension {
                        params: TestMaterialParams {
                            stripe_color: ORANGE_300.into(),
                            stripe_frequency: 15.0,
                            stripe_angle: -FRAC_PI_4, // Opposite angle to distinguish from dissolve gate
                            stripe_thickness: 0.9,
                            scroll_speed: -0.03, // Opposite direction scroll
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
                        NoFrustumCulling,
                    ))
                    .observe(handle_discharge_collisions);
            }
        }
    }
}

pub fn handle_discharge_collisions(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    q_powered: Query<&Powered>,
    q_player: Query<&RightHand, With<Player>>,
    q_collider_of: Query<&ColliderOf>,
    q_discharge_gates: Query<(Entity, &DischargeGate)>,
    q_child_of: Query<&ChildOf>,
) {
    let device_or_player_collider_entity = trigger.collider;
    let maybe_discharge_gate = trigger.target();

    // Verify this is actually a discharge gate
    if let Ok(maybe_discharge_gate_parent) = q_child_of.get(maybe_discharge_gate) {
        if !q_discharge_gates.contains(maybe_discharge_gate_parent.0) {
            return;
        }
    } else {
        return;
    }

    if let Ok(targeted_body) = q_collider_of.get(device_or_player_collider_entity) {
        // Check if the colliding entity itself is powered
        if q_powered.contains(targeted_body.body) {
            commands.entity(targeted_body.body).remove::<Powered>();
            info!(
                "Discharged entity {:?} - removed Powered component",
                targeted_body.body
            );
            return;
        }

        // Check if the colliding entity is a player with a held powered object
        if let Ok(right_hand) = q_player.get(targeted_body.body) {
            if let Some(held_entity) = right_hand.held_object {
                if q_powered.contains(held_entity) {
                    commands.entity(held_entity).remove::<Powered>();
                    info!(
                        "Discharged held entity {:?} - removed Powered component",
                        held_entity
                    );
                }
            }
        }
    }
}

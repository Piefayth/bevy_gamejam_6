use avian3d::prelude::{
    ColliderOf, CollisionEventsEnabled, CollisionLayers, OnCollisionStart, RigidBody, Sensor,
};
use bevy::{color::palettes::css::RED, prelude::*};

use crate::{
    asset_management::asset_tag_components::DissolveGate, game::player::Held,
    rendering::unlit_material::UnlitMaterial,
};

use super::{
    GameLayer,
    player::{Player, RightHand},
};

pub fn dissolve_gate_plugin(app: &mut App) {
    app.add_systems(Update, (register_dissolve_gates,));
}

// Indicates a device can be dissolved

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Dissolveable {
    pub respawn_transform: Option<Transform>,
}

fn register_dissolve_gates(
    mut commands: Commands,
    q_new_gate: Query<(Entity, &Children), Added<DissolveGate>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
) {
    for (gate_entity, gate_children) in &q_new_gate {
        for gate_child in gate_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(gate_child) {
                let mut old_material = unlit_materials.get(material_handle).unwrap().clone();
                old_material.base.alpha_mode = AlphaMode::Blend;
                old_material.extension.alpha = 0.5;
                old_material.extension.blend_color = RED.into();
                old_material.extension.blend_factor = 1.0;

                commands
                    .entity(gate_child)
                    .insert((
                        MeshMaterial3d(unlit_materials.add(old_material)),
                        CollisionEventsEnabled,
                        CollisionLayers::new(GameLayer::Default, [GameLayer::Device, GameLayer::Player]),
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
    q_dissolve_gates: Query<&DissolveGate>,
    q_child_of: Query<&ChildOf>,
) {
    let device_or_player_collider_entity = trigger.collider;
    let maybe_dissolve_gate = trigger.target();

    // If the event target wasn't a dissolve gate, skip it
    // This can happen if, say, the player touches a dissolvable device
    if let Ok(maybe_dissolve_gate_parent) = q_child_of.get(maybe_dissolve_gate) {
        if !q_dissolve_gates.contains(maybe_dissolve_gate_parent.0) {
            return;
        }
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
                    commands.entity(targeted_body.body).despawn();
                    info!(
                        "Dissolved entity {:?} despawned",
                        targeted_body.body
                    );
                }
            }
            return;
        }

        // Check if the colliding entity is a player with a held object
        if let Ok(right_hand) = q_player.get(targeted_body.body) {
            println!("hit player");
            if let Some(held_entity) = right_hand.held_object {
                println!("player was holding");
                if let Ok(dissolveable) = q_dissolveable.get(held_entity) {
                    println!("hold was dissolavabelasdf");
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
                            // Despawn the held entity
                            commands.entity(held_entity).despawn();
                            info!("Dissolved held entity {:?} despawned", held_entity);
                        }
                    }
                }
            }
        }
    }
}

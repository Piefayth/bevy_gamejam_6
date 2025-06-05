use std::time::Duration;

use avian3d::prelude::{ColliderOf, RigidBody};
use bevy::prelude::*;
use bevy_tween::{bevy_time_runner::TimeSpan, combinator::{sequence, tween}, prelude::{AnimationBuilderExt, EaseKind}, tween::{AnimationTarget, TargetAsset}};

use crate::{
    asset_management::asset_tag_components::{ChargePad, Door, PowerButton, PressurePlate},
    game::signals::DirectSignal,
    rendering::unlit_material::UnlitMaterial,
};

use super::{interaction::Interacted, pressure_plate::{POWER_ANIMATION_DURATION_SEC, POWER_MATERIAL_INTENSITY}, signals::MaterialIntensityInterpolator};

pub fn button_plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, register_buttons)
        .add_systems(Update, update_delayed_signals);
}

#[derive(Component)]
pub struct ButtonTargets(pub Vec<Entity>);

#[derive(Component)]
struct DelayedSignalTimer {
    timer: Timer,
    target: Entity,
}

fn register_buttons(
    mut commands: Commands,
    q_new_button: Query<(Entity, &Children, &ChildOf), Added<PowerButton>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_children: Query<&Children>,
    q_doors: Query<&Door>, 
) {
    for (button_entity, button_children, button_child_of) in &q_new_button {
        if let Ok(parent_children) = q_children.get(button_child_of.parent()) {
            let mut button_targets: Vec<Entity> = vec![];

            for sibling in parent_children.iter() {
                // buttons can't power doors directly
                if sibling != button_entity && !q_doors.contains(sibling) {
                    button_targets.push(sibling);
                }
            }

            commands
                .entity(button_entity)
                .insert((ButtonTargets(button_targets), RigidBody::Static))
                .observe(button_pressed);
        }

        for button_child in button_children.iter() {
            if let Ok(material_handle) = q_unlit_objects.get(button_child) {
                let old_material = unlit_materials.get(material_handle).unwrap().clone();

                commands.entity(button_child).insert((
                    AnimationTarget,
                    MeshMaterial3d(unlit_materials.add(old_material)),
                ));
            }
        }
    }
}

pub fn button_pressed(
    trigger: Trigger<Interacted>,
    mut commands: Commands,
    q_button: Query<(&ButtonTargets, &Children)>,
    q_collider_of: Query<&ColliderOf>,
    q_unlit_objects: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_tween: Query<(), With<TimeSpan>>,
    q_children: Query<&Children>,
) {
    if let Ok(collider_of) = q_collider_of.get(trigger.target()) {
        if let Ok((button_targets, button_children)) = q_button.get(collider_of.body) {
            // Animate the button's material when pressed
            for button_child in button_children.iter() {
                // Clear any existing animations
                if let Ok(child_children) = q_children.get(button_child) {
                    for child in child_children.iter() {
                        if q_tween.contains(child) {
                            commands.entity(child).try_despawn();
                        }
                    }
                }

                // Add the button press animation
                if let Ok(material_handle) = q_unlit_objects.get(button_child) {
                    commands
                        .entity(button_child)
                        .animation()
                        .insert(sequence((
                            // Flash bright when pressed
                            tween(
                                Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 500.) as u64),
                                EaseKind::CubicOut,
                                TargetAsset::Asset(material_handle.clone_weak()).with(
                                    MaterialIntensityInterpolator {
                                        start: 1.0,
                                        end: POWER_MATERIAL_INTENSITY,
                                    },
                                ),
                            ),
                            // Return to normal
                            tween(
                                Duration::from_millis((POWER_ANIMATION_DURATION_SEC * 500.) as u64),
                                EaseKind::CubicOut,
                                TargetAsset::Asset(material_handle.clone_weak()).with(
                                    MaterialIntensityInterpolator {
                                        start: POWER_MATERIAL_INTENSITY,
                                        end: 1.0,
                                    },
                                ),
                            ),
                        )));
                }
            }

            // Send signals to targets with delay
            for target in &button_targets.0 {
                commands.spawn(DelayedSignalTimer {
                    timer: Timer::from_seconds(0.5, TimerMode::Once),
                    target: *target,
                });
            }
        }
    }
}

fn update_delayed_signals(
    mut commands: Commands,
    mut q_delayed_signals: Query<(Entity, &mut DelayedSignalTimer)>,
    time: Res<Time>,
) {
    for (timer_entity, mut delayed_signal) in &mut q_delayed_signals {
        delayed_signal.timer.tick(time.delta());
        
        if delayed_signal.timer.finished() {
            // Send the DirectSignal
            commands.entity(delayed_signal.target).trigger(DirectSignal);
            
            // Remove the timer entity
            commands.entity(timer_entity).try_despawn();
        }
    }
}

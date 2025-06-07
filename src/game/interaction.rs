use std::time::Duration;

use avian3d::prelude::{
    ColliderConstructor, ColliderOf, CollisionEventsEnabled, CollisionLayers, LockedAxes, RigidBody, RigidBodyColliders, Sensor, SpatialQuery, SpatialQueryFilter
};
use bevy::prelude::*;
use bevy_enhanced_input::events::Completed;
use bevy_tween::{
    combinator::{sequence, tween},
    interpolate::translation,
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{AnimationTarget, TargetComponent},
};

use crate::asset_management::{
        asset_loading::GameAssets,
        asset_tag_components::{
            BigRedButton, CubeSpitter, ExitDoorShutter, Immobile, PowerButton, SignalSpitter,
            StandingCubeSpitter, WeightedCube,
        },
    };

use super::{
    button::button_pressed, dissolve_gate::Dissolveable, input::UseInteract, player::{Held, RightHand}, signals::{Signal, MAX_SIGNAL_TRAVEL_DIST}, GameLayer
};

pub fn interaction_plugin(app: &mut App) {
    app.add_observer(interact).add_systems(
        FixedPreUpdate,
        (
            register_big_red_button_interaction,
            register_power_button_interaction,
            register_weighted_cube_interaction,
            register_signal_spitter_interaction,
            register_standing_cube_spitter_interaction,
        ),
    );
}

pub const INTERACTION_DISTANCE: f32 = 30.;

fn interact(
    _trigger: Trigger<Completed<UseInteract>>,
    mut commands: Commands,
    spatial_query: SpatialQuery,
    camera_query: Query<&GlobalTransform, With<Camera>>,
    interactables: Query<&Interactable, Without<InteractionsDisabled>>,
    right_hand: Single<&mut RightHand>,
    q_held: Query<&Held>,
) {
    let mut found_hit: bool = false;

    // Get camera transform and window for raycast
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    // Cast ray from camera center forward
    let ray_origin = camera_transform.translation();
    let ray_direction = camera_transform.forward();

    // Perform raycast
    if let Some(hit) = spatial_query.cast_ray(
        ray_origin,
        ray_direction,
        INTERACTION_DISTANCE,
        true, // solid hits only
        &SpatialQueryFilter::default().with_mask([GameLayer::Default, GameLayer::Device]),
    ) {
        let hit_entity = hit.entity;

        // Check if the hit entity is interactable
        if let Ok(interactable) = interactables.get(hit_entity) {
            // Check if we can interact (don't pick up if already holding something)
            let can_interact = !(right_hand.held_object.is_some()
                && matches!(interactable.primary_action, Interactions::PickUp));

            if can_interact {
                commands.entity(hit_entity).trigger(Interacted);
                found_hit = true;
            }
        }
    }

    // If no interaction found, try to release held object
    if !found_hit {
        if let Some(held_entity) = right_hand.held_object {
            if let Ok(held) = q_held.get(held_entity) {
                if held.can_release {
                    commands.entity(held_entity).remove::<Held>();
                }
            }
        }
    }
}

#[derive(Event)]
pub struct Interacted;

#[derive(Component)]
pub struct Interactable {
    pub primary_action: Interactions,
}

#[derive(Component)]
pub struct InteractionsDisabled;

impl Interactable {
    fn new(primary_action: Interactions) -> Interactable {
        Interactable { primary_action }
    }
}

pub enum Interactions {
    Press,
    PickUp,
}

// Rest of your existing functions remain the same...
fn big_red_button_interaction(
    trigger: Trigger<Interacted>,
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    q_collider_of: Query<&ColliderOf>,
    q_body_transforms: Query<&GlobalTransform, (With<RigidBody>, Without<CubeSpitter>)>,
    exit_door_shutter: Single<Entity, With<ExitDoorShutter>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let button_collider_of = q_collider_of.get(trigger.target()).unwrap();
    let target_location = q_body_transforms.get(button_collider_of.body).unwrap();

    let start_loc = target_location.translation() + Vec3::Y * 10.;
    let signal_indicator = commands
        .spawn((
            ColliderConstructor::Cuboid {
                x_length: 100.,
                y_length: 100.,
                z_length: 0.1,
            },
            CollisionLayers::new(GameLayer::Signal, [GameLayer::Device]),
            Mesh3d(meshes.add(Plane3d::new(-Vec3::Z, Vec2::splat(100.)))),
            MeshMaterial3d(game_assets.cyan_signal_material.clone()),
            Transform::from_translation(start_loc),
            AnimationTarget,
            CollisionEventsEnabled,
            RigidBody::Kinematic,
            Sensor,
            Signal,
        ))
        .id();

    commands.entity(signal_indicator).animation().insert(tween(
        Duration::from_secs(10),
        EaseKind::Linear,
        TargetComponent::marker().with(translation(
            start_loc,
            start_loc + Vec3::Z * MAX_SIGNAL_TRAVEL_DIST,
        )),
    ));

    let target = TargetComponent::marker();
    commands.entity(*exit_door_shutter).insert(AnimationTarget);

    commands
        .entity(*exit_door_shutter)
        .animation()
        .insert(sequence((
            tween(
                Duration::from_secs(3),
                EaseKind::Linear,
                target.with(translation(Vec3::ZERO, Vec3::Y * 3.)),
            ),
            tween(
                Duration::from_secs(4),
                EaseKind::Linear,
                target.with(translation(Vec3::Y * 3., Vec3::Y * 3.)),
            ),
            tween(
                Duration::from_secs(3),
                EaseKind::ExponentialOut,
                target.with(translation(Vec3::Y * 3., Vec3::ZERO)),
            ),
        )));
}

fn register_big_red_button_interaction(
    mut commands: Commands,
    q_new_buttons: Query<&Children, Added<BigRedButton>>,
    q_mesh: Query<Entity, With<Mesh3d>>,
) {
    for children in &q_new_buttons {
        if let Some(found_child) = children.iter().find(|&child| q_mesh.contains(child)) {
            commands
                .entity(found_child)
                .observe(big_red_button_interaction)
                .insert(Interactable::new(Interactions::Press));
        }
    }
}

fn register_power_button_interaction(
    mut commands: Commands,
    q_new_buttons: Query<&Children, Added<PowerButton>>,
    q_mesh: Query<Entity, With<Mesh3d>>,
) {
    for children in &q_new_buttons {
        if let Some(found_child) = children.iter().find(|&child| q_mesh.contains(child)) {
            commands
                .entity(found_child)
                .observe(button_pressed)
                .insert(Interactable::new(Interactions::Press));
        }
    }
}

fn pick_up(
    trigger: Trigger<Interacted>,
    mut commands: Commands,
    mut right_hand: Single<&mut RightHand>,
    q_collider_of: Query<&ColliderOf>,
) {
    if let Ok(collider_of) = q_collider_of.get(trigger.target()) {
        if right_hand.held_object.is_none() {
            right_hand.held_object = Some(collider_of.body);
            commands.entity(collider_of.body).insert(Held::default());
        }
    }
}

fn register_weighted_cube_interaction(
    mut commands: Commands,
    q_new_cubes: Query<
        (Entity, &RigidBodyColliders),
        (Added<RigidBodyColliders>, With<WeightedCube>),
    >,
    q_mesh: Query<Entity, With<Mesh3d>>,
) {
    for (cube_entity, children) in &q_new_cubes {
        if let Some(found_child) = children.iter().find(|&child| q_mesh.contains(child)) {
            commands
                .entity(found_child)
                .observe(pick_up)
                .insert(Interactable::new(Interactions::PickUp));
        }
        commands.entity(cube_entity).insert(Dissolveable {
            respawn_transform: None,
        });
    }
}

fn register_signal_spitter_interaction(
    mut commands: Commands,
    q_new_spitters: Query<(Entity, &Children, &Transform, Has<Immobile>), Added<SignalSpitter>>,
    q_mesh: Query<Entity, With<Mesh3d>>,
) {
    for (new_spitter, children, transform, is_immobile) in &q_new_spitters {
        if children.len() > 1 {
            warn!("spitter cannot have more than one child");
            continue;
        }

        if let Some(found_child) = children.iter().find(|&child| q_mesh.contains(child)) {
            if is_immobile {
                commands.entity(new_spitter).insert(RigidBody::Static);
            } else {
                commands
                    .entity(found_child)
                    .observe(pick_up)
                    .insert((Interactable::new(Interactions::PickUp),));

                commands.entity(new_spitter).insert((
                    RigidBody::Dynamic,
                    LockedAxes::ALL_LOCKED.unlock_translation_y(),
                    Dissolveable {
                        respawn_transform: Some(*transform),
                    },
                ));
            }
        }
    }
}

fn register_standing_cube_spitter_interaction(
    mut commands: Commands,
    q_new_spitters: Query<
        (Entity, &Children, &Transform, Has<Immobile>),
        Added<StandingCubeSpitter>
    >,
    q_mesh: Query<Entity, With<Mesh3d>>,
) {
    for (new_spitter, children, transform, is_immobile) in &q_new_spitters {
        if children.len() > 1 {
            warn!("spitter cannot have more than one child");
            continue;
        }

        if let Some(found_child) = children.iter().find(|&child| q_mesh.contains(child)) {
            if is_immobile {
                commands.entity(new_spitter).insert(RigidBody::Static);
            } else {
                commands
                    .entity(found_child)
                    .observe(pick_up)
                    .insert((Interactable::new(Interactions::PickUp),));

                commands.entity(new_spitter).insert((
                    RigidBody::Dynamic,
                    LockedAxes::ALL_LOCKED.unlock_translation_y(),
                    Dissolveable {
                        respawn_transform: Some(*transform),
                    },
                ));
            }
        }
    }
}

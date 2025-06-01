use std::time::Duration;

use avian3d::prelude::{
    ColliderDensity, ColliderOf, ComputedMass, ExternalForce, ExternalImpulse, Mass, RigidBody, RotationInterpolation, SleepingDisabled, TransformInterpolation
};
use bevy::{picking::pointer::PointerInteraction, prelude::*};
use bevy_enhanced_input::events::Completed;
use bevy_tween::{
    combinator::{sequence, tween},
    interpolate::{Scale, scale_to, translation},
    prelude::{AnimationBuilderExt, EaseKind},
    tween::{self, AnimationTarget, IntoTarget, TargetComponent},
};

use crate::asset_management::{
    asset_loading::GameAssets,
    asset_tag_components::{
        BigRedButton, CubeSpitter, ExitDoorShutter, WeightedCube, WeightedCubeColors,
    },
};

use super::{
    input::UseInteract,
    player::{Held, RightHand},
};

pub fn interaction_plugin(app: &mut App) {
    app.add_observer(interact).add_systems(
        Update,
        (
            register_big_red_button_interaction,
            register_weighted_cube_interaction,
        ),
    );
}

pub const INTERACTION_DISTANCE: f32 = 20.;

fn interact(
    _trigger: Trigger<Completed<UseInteract>>,
    mut commands: Commands,
    pointers: Query<&PointerInteraction>,
    interactables: Query<(), (With<Interactable>, Without<InteractionsDisabled>)>,
    mut right_hand: Single<&mut RightHand>,
    q_held: Query<&Held>,
) {
    let mut found_hit: bool = false;

    for (entity, _hit) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter(|(entity, hit)| {
            hit.depth <= INTERACTION_DISTANCE && interactables.contains(*entity)
        })
    {
        commands.entity(*entity).trigger(Interacted);
        found_hit = true;
    }

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
    primary_action: Interactions,
}

#[derive(Component)]
pub struct InteractionsDisabled;

impl Interactable {
    fn new(primary_action: Interactions) -> Interactable {
        return Interactable { primary_action };
    }
}

pub enum Interactions {
    Press,
    PickUp,
}

fn big_red_button_interaction(
    _trigger: Trigger<Interacted>,
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    q_cube_spitter: Query<(&Transform, &CubeSpitter)>,
    exit_door_shutter: Single<Entity, With<ExitDoorShutter>>,
) {
    for (spitter_transform, spitter) in &q_cube_spitter {
        commands.spawn((
            SceneRoot(match spitter.color {
                WeightedCubeColors::Cyan => game_assets.weighted_cube_cyan.clone(),
            }),
            Transform::from_translation(
                spitter_transform.translation + Vec3::Y * 14.5 + -Vec3::X * 20.,
            ),
            RigidBody::Dynamic,
            TransformInterpolation,
            RotationInterpolation,
            ExternalImpulse::new(Vec3::new(-5000., 0., 0.)),
        ));
    }

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

fn weighted_cube_interaction(
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
    q_new_buttons: Query<&Children, Added<WeightedCube>>,
    q_mesh: Query<Entity, With<Mesh3d>>,
) {
    for children in &q_new_buttons {
        if let Some(found_child) = children.iter().find(|&child| q_mesh.contains(child)) {
            commands
                .entity(found_child)
                .observe(weighted_cube_interaction)
                .insert(Interactable::new(Interactions::PickUp));
        }
    }
}

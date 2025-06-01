use avian3d::prelude::{ComputedMass, ExternalForce, ExternalImpulse, Mass, RigidBody, RotationInterpolation, SleepingDisabled, TransformInterpolation};
use bevy::{picking::pointer::PointerInteraction, prelude::*};
use bevy_enhanced_input::events::Completed;

use crate::{asset_management::{asset_loading::GameAssets, asset_tag_components::{BigRedButton, CubeSpitter, WeightedCubeColors}}};

use super::input::UseInteract;

pub fn interaction_plugin(app: &mut App) {
    app
        .add_observer(interact)
        .add_systems(Update, register_big_red_button_interaction);
}

pub const INTERACTION_DISTANCE: f32 = 10.;

fn interact(
    _trigger: Trigger<Completed<UseInteract>>,
    mut commands: Commands,
    pointers: Query<&PointerInteraction>,
) {
    for entity in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(entity, _hit)| Some(entity))
    {
        commands.entity(*entity).trigger(Interacted);
    }
}

#[derive(Event)]
pub struct Interacted;

#[derive(Component)]
pub struct Interactable;

fn big_red_button_interaction(
    _trigger: Trigger<Interacted>,
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    q_cube_spitter: Query<(&Transform, &CubeSpitter)>,
) {
    for (spitter_transform, spitter) in &q_cube_spitter {
        println!("spawn");
        commands.spawn((
            SceneRoot(match spitter.color {
                WeightedCubeColors::Cyan => game_assets.weighted_cube_cyan.clone(),
            }),
            Transform::from_translation(spitter_transform.translation + Vec3::Y * 14.5 + -Vec3::X * 20.), // something about the scale makes the spitter y translation goofy
            RigidBody::Dynamic,
            //ExternalImpulse::new(Vec3::new(-50.0, 0.0, 00.0)),
            TransformInterpolation,
            RotationInterpolation,
        ));
    }

}

fn register_big_red_button_interaction(
    mut commands: Commands,
    q_new_buttons: Query<&Children, Added<BigRedButton>>,
    q_mesh: Query<Entity, With<Mesh3d>>, 
) {
    for children in &q_new_buttons {
        if let Some(found_child) = children
            .iter()
            .find(|&child| q_mesh.contains(child))
        {
            commands
                .entity(found_child)
                .observe(big_red_button_interaction)
                .insert(Interactable);
        }
    }
}

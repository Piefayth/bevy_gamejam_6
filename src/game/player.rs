use std::f32::consts::{self, FRAC_PI_2, FRAC_PI_4};

use avian3d::{
    math::PI,
    prelude::{
        Collider, ColliderDisabled, ColliderOf, CollisionEventsEnabled, CollisionLayers, LockedAxes, Physics, PhysicsSet, RigidBody, RigidBodyColliders, RigidBodyDisabled, ShapeCastConfig, ShapeCaster, ShapeHits, SpatialQuery, SpatialQueryFilter, TransformInterpolation
    },
};
use bevy::{
    app::FixedMain,
    color::palettes::css::{RED, WHITE},
    picking::pointer::PointerInteraction,
    prelude::*,
};

use bevy_enhanced_input::{
    EnhancedInputSystem,
    events::Completed,
    prelude::{ActionValue, Actions},
};
use bevy_tnua::{builtins::TnuaBuiltinClimb, prelude::*};
use bevy_tnua_avian3d::*;

use crate::{
    GameState, MainCamera,
    rendering::{section_color_prepass::DrawSection, unlit_material::UnlitMaterial},
    ui::crosshair::CrosshairState,
};

use super::{
    input::{FixedInputContext, Jump, Look, Movement, UpdateInputContext, UseInteract}, interaction::{Interactable, InteractionsDisabled}, GameLayer
};

pub fn player_plugin(app: &mut App) {
    app.add_plugins((
        TnuaControllerPlugin::new(FixedUpdate),
        TnuaAvian3dPlugin::new(FixedUpdate),
    ))
    .add_systems(
        PreUpdate,
        rotate_camera
            .after(EnhancedInputSystem)
            .run_if(in_state(GameState::Playing).and(in_state(CrosshairState::Shown))),
    )
    .add_systems(
        PostUpdate,
        camera_follow_player
            .after(RunFixedMainLoopSystem::AfterFixedMainLoop)
            .before(TransformSystem::TransformPropagate)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        FixedUpdate,
        (move_player, jump).run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        PreUpdate, // this is on its own because we are basically guessing where to put it atm
        project_held_placable_item.run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        (picked_up_item).run_if(in_state(GameState::Playing)),
    )
    .add_systems(OnEnter(GameState::Playing), spawn_player)
    .add_observer(released_item)
    .register_type::<PlayerSpawnPoint>()
    .register_type::<RightHand>();
}

#[derive(Component)]
pub struct Player;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct RightHand {
    pub held_object: Option<Entity>,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PlayerSpawnPoint {
    pub unused: bool,
}

fn spawn_player(
    mut commands: Commands,
    spawn_point: Single<&Transform, With<PlayerSpawnPoint>>,
    mut camera: Single<&mut Transform, (With<MainCamera>, Without<PlayerSpawnPoint>)>,
) {
    commands.spawn((
        spawn_point.clone(),
        RigidBody::Dynamic,
        Collider::capsule(1.5, 8.0),
        TnuaController::default(), // todo: what options
        TnuaAvian3dSensorShape(Collider::capsule(1.49, 7.99)),
        LockedAxes::ROTATION_LOCKED,
        Player,
        RightHand::default(),
        StateScoped(GameState::Playing),
        TransformInterpolation,
        CollisionLayers::new(GameLayer::Player, [GameLayer::Default, GameLayer::Device]),
        CollisionEventsEnabled
    ));

    // set camera rotation to away from origin.
    **camera = camera.looking_at(Vec3::ZERO, Vec3::Y);
    camera.rotate_y(PI);
}

const PLAYER_VELOCITY: f32 = 30.0;

fn move_player(
    mut controller: Single<&mut TnuaController>,
    input: Single<&Actions<FixedInputContext>>,
    camera: Single<&Transform, With<MainCamera>>,
) {
    if let Ok(action) = input.value::<Movement>() {
        if let ActionValue::Axis2D(movement) = action {
            let camera_forward = camera.forward();
            let camera_right = camera.right();

            let forward_horizontal = Vec3::new(camera_forward.x, 0.0, camera_forward.z).normalize();
            let right_horizontal = Vec3::new(camera_right.x, 0.0, camera_right.z).normalize();

            let direction = forward_horizontal * movement.y + right_horizontal * movement.x;

            controller.basis(TnuaBuiltinWalk {
                desired_velocity: direction * PLAYER_VELOCITY,
                float_height: 4.0,
                max_slope: FRAC_PI_2,
                acceleration: 120.,
                air_acceleration: 120.,
                free_fall_extra_gravity: 100.,
                ..default()
            });
        }
    }
}

fn jump(mut controller: Single<&mut TnuaController>, input: Single<&Actions<FixedInputContext>>) {
    if let Ok(action) = input.value::<Jump>() {
        if let ActionValue::Bool(jump) = action {
            if jump {
                controller.action(TnuaBuiltinJump {
                    height: 8.0,
                    takeoff_extra_gravity: 120.,
                    fall_extra_gravity: 60.,
                    shorten_extra_gravity: 0.0,
                    ..default()
                });
            }
        }
    }
}

const MAX_PITCH: f32 = 89.0_f32.to_radians(); // Limit vertical look angle
const SENSITIVITY: f32 = 0.1;

fn rotate_camera(
    input: Single<&Actions<UpdateInputContext>>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    if let Ok(action) = input.value::<Look>() {
        if let ActionValue::Axis2D(look) = action {
            let scaled_sensitivity = SENSITIVITY * time.delta_secs();

            camera.rotate_y(-look.x * scaled_sensitivity);

            let current_pitch = camera.rotation.to_euler(EulerRot::YXZ).1;
            let new_pitch =
                (current_pitch - look.y * scaled_sensitivity).clamp(-MAX_PITCH, MAX_PITCH);
            let pitch_delta = new_pitch - current_pitch;

            camera.rotate_local_x(pitch_delta);
        }
    }
}

const CAMERA_HEIGHT: f32 = 4.0;
fn camera_follow_player(
    maybe_player: Option<Single<&Transform, With<Player>>>,
    mut camera: Single<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    if let Some(player) = maybe_player {
        camera.translation = player
            .translation
            .with_y(player.translation.y + CAMERA_HEIGHT);
    }
}

#[derive(Component, Default)]
pub struct Held {
    pub can_release: bool,
}

fn picked_up_item(
    mut commands: Commands,
    q_picked_up: Query<(Entity, &RigidBodyColliders), Added<Held>>,
    mut q_collider_materials: Query<(
        Entity,
        &MeshMaterial3d<UnlitMaterial>,
        &Collider,
    )>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    mut transforms: Query<&mut Transform>,
    mut player: Single<(Entity, &mut RightHand), With<Player>>,
) {
    for (picked_up_body, picked_up_colliders) in &q_picked_up {
        let mut last_collider: Collider = Collider::sphere(1.0);

        for collider_entity in picked_up_colliders.iter() {
            if let Ok((picked_up_collider, material, collider)) =
                q_collider_materials.get_mut(collider_entity)
            {
                let material_to_update = unlit_materials.get_mut(material).unwrap();
                material_to_update.extension.alpha = 0.75;
                material_to_update.extension.blend_color = RED.into();
                material_to_update.extension.blend_factor = 0.8;
                material_to_update.base.alpha_mode = AlphaMode::Opaque;

                commands
                    .entity(picked_up_collider)
                    .remove::<DrawSection>()
                    .insert(CollisionLayers::new(GameLayer::Ignore, [GameLayer::Default]))
                    .insert(InteractionsDisabled)
                    .insert(Pickable::IGNORE);

                last_collider = collider.clone();
            }
        }

        let mut excluded_entities: Vec<Entity> = vec![];

        for thing in picked_up_colliders.iter() {
            excluded_entities.push(thing.clone());
        }

        commands.entity(picked_up_body).insert(RigidBodyDisabled);

        if let Ok(mut body_transform) = transforms.get_mut(picked_up_body) {
            body_transform.rotation = Quat::IDENTITY;
        }

        player.1.held_object = Some(picked_up_body);
        commands.entity(player.0).insert(
            ShapeCaster::new(
                last_collider,
                Vec3::ZERO,     // Will be updated each frame
                Quat::IDENTITY, // We force shape to identity rot above
                Dir3::X,        // Will be updated each frame
            )
            .with_max_distance(50.0)
            .with_query_filter(
                SpatialQueryFilter::default()
                    .with_mask([GameLayer::Default, GameLayer::Device, GameLayer::Player])
                    .with_excluded_entities(excluded_entities),
            ).with_max_hits(1),
        );
    }
}

fn released_item(
    trigger: Trigger<OnRemove, Held>,
    mut commands: Commands,
    q_releasables: Query<(Entity, &RigidBodyColliders)>,
    q_collider_materials: Query<(
        Entity,
        &MeshMaterial3d<UnlitMaterial>,
    )>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    mut player: Single<(Entity, &mut RightHand),  With<Player>>,
) {
    if let Ok((releasable_entity, releasable_colliders)) = q_releasables.get(trigger.target()) {
        for collider_entity in releasable_colliders.iter() {
            if let Ok((collider_entity, material)) = q_collider_materials.get(collider_entity) {
                let material_to_update = unlit_materials.get_mut(material).unwrap();
                material_to_update.extension.alpha = 1.0;
                material_to_update.extension.blend_color = WHITE.into();
                material_to_update.extension.blend_factor = 0.0;
                material_to_update.base.alpha_mode = AlphaMode::Opaque;

                commands
                    .entity(collider_entity)
                    .insert(DrawSection)
                    // TODO: These layers might not be the same for every item we can hold?
                    .insert(CollisionLayers::new(GameLayer::Device, [GameLayer::Default, GameLayer::Player, GameLayer::Signal, GameLayer::Device]))
                    .remove::<InteractionsDisabled>()
                    .insert(Pickable::default());
            }
        }

        player.1.held_object = None;
        commands.entity(player.0).remove::<ShapeCaster>().remove::<ShapeHits>();
        commands.entity(releasable_entity).remove::<RigidBodyDisabled>();
    }

}

fn project_held_placable_item(
    camera: Single<&GlobalTransform, With<MainCamera>>,
    player: Single<(Entity, &RightHand, &Transform), With<Player>>,
    mut transforms: Query<&mut Transform, (Without<MainCamera>, Without<Player>)>,
    mut shape_casters: Query<(&mut ShapeCaster, &ShapeHits), With<Player>>,
    q_material_handles: Query<&MeshMaterial3d<UnlitMaterial>>,
    q_rigid_body_colliders: Query<&RigidBodyColliders>,
    mut q_held: Query<&mut Held>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
) {
    if let Some(held_entity) = player.1.held_object {
        if let Ok((mut shape_caster, shape_hits)) = shape_casters.get_mut(player.0) {
            let camera_pos = camera.translation();
            let camera_forward = camera.forward();

            shape_caster.origin = Vec3::Y * CAMERA_HEIGHT;
            shape_caster.direction = camera_forward;

            // Use the first hit from the shape caster
            if let Some(hit) = shape_hits.iter().min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap()) {
                if let Ok(mut held_transform) = transforms.get_mut(held_entity) {
                    let camera_pos = camera.translation();
                    let camera_forward = camera.forward();

                    held_transform.translation = camera_pos + hit.distance * camera_forward;

                    if let Ok(rigid_body_colliders) = q_rigid_body_colliders.get(held_entity) {
                        for collider_entity in rigid_body_colliders.iter() {
                            if let Ok(handle) = q_material_handles.get(collider_entity) {
                                if let Some(unlit_material) = unlit_materials.get_mut(handle) {
                                    unlit_material.extension.blend_color = WHITE.into();
                                    unlit_material.extension.blend_factor = 0.0;
                                }
                            }
                        }
                    }

                    // update the Held component
                    if let Ok(mut held) = q_held.get_mut(held_entity) {
                        held.can_release = true;
                    }
                }
            } else {
                // No hit found, place at default distance from camera
                if let Ok(mut held_transform) = transforms.get_mut(held_entity) {
                    let default_distance = 20.0;
                    held_transform.translation = camera_pos + camera_forward * default_distance;
                }

                if let Ok(rigid_body_colliders) = q_rigid_body_colliders.get(held_entity) {
                    for collider_entity in rigid_body_colliders.iter() {
                        if let Ok(handle) = q_material_handles.get(collider_entity) {
                            if let Some(unlit_material) = unlit_materials.get_mut(handle) {
                                unlit_material.extension.blend_color = RED.into();
                                unlit_material.extension.blend_factor = 0.8;
                            }
                        }
                    }
                }

                if let Ok(mut held) = q_held.get_mut(held_entity) {
                    held.can_release = false;
                }
            }
        }
    }
}

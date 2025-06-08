use bevy::prelude::*;

use crate::{
    asset_management::asset_loading::GameAssets,
    game::{
        audio::{handle_volume_down, handle_volume_up},
        dissolve_gate::Dissolveable,
        player::{Held, Player, PlayerSpawnPoint, RightHand},
        standing_cube_spitter::Tombstone,
    },
    ui::crosshair::CrosshairState,
};

pub fn system_menu_plugin(app: &mut App) {
    app.add_systems(OnEnter(CrosshairState::Hidden), spawn_system_menu);
}

fn spawn_system_menu(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                ..default()
            },
            StateScoped(CrosshairState::Hidden),
        ))
        .with_children(|child_spawner| {
            child_spawner
                .spawn((
                    Node {
                        min_width: Val::Px(300.),
                        min_height: Val::Px(400.),
                        height: Val::Auto,
                        justify_content: JustifyContent::Start,
                        align_items: AlignItems::Start,
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(3.)),
                        padding: UiRect::all(Val::Px(5.)).with_left(Val::Px(15.)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.75, 0.75, 0.75)),
                    BorderColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .with_children(|child_spawner| {
                    child_spawner.spawn((
                        Text::new("in the menu"),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..default()
                        },
                        TextFont {
                            font: game_assets.font.clone(),
                            font_size: 48.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.1, 0.1, 0.1)),
                        Node {
                            margin: UiRect::bottom(Val::Percent(30.)),
                            ..default()
                        },
                    ));

                    let text_entity = child_spawner
                        .spawn((
                            Text::new("Volume Up"),
                            TextFont {
                                font: game_assets.font.clone(),
                                font_size: 33.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.1, 0.1, 0.1)),
                        ))
                        .id();

                    child_spawner
                        .commands()
                        .entity(text_entity)
                        .observe(handle_volume_up)
                        .observe(
                            move |_trigger: Trigger<Pointer<Over>>,
                                  mut text_query: Query<&mut Text>| {
                                if let Ok(mut text) = text_query.get_mut(text_entity) {
                                    **text = "Volume Up ◀".into();
                                }
                            },
                        )
                        .observe(
                            move |_trigger: Trigger<Pointer<Out>>,
                                  mut text_query: Query<&mut Text>| {
                                if let Ok(mut text) = text_query.get_mut(text_entity) {
                                    **text = "Volume Up".into();
                                }
                            },
                        );
                    let text_entity = child_spawner
                        .spawn((
                            Text::new("Volume Down"),
                            TextFont {
                                font: game_assets.font.clone(),
                                font_size: 33.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.1, 0.1, 0.1)),
                        ))
                        .id();

                    child_spawner
                        .commands()
                        .entity(text_entity)
                        .observe(handle_volume_down)
                        .observe(
                            move |_trigger: Trigger<Pointer<Over>>,
                                  mut text_query: Query<&mut Text>| {
                                if let Ok(mut text) = text_query.get_mut(text_entity) {
                                    **text = "Volume Down ◀".into();
                                }
                            },
                        )
                        .observe(
                            move |_trigger: Trigger<Pointer<Out>>,
                                  mut text_query: Query<&mut Text>| {
                                if let Ok(mut text) = text_query.get_mut(text_entity) {
                                    **text = "Volume Down".into();
                                }
                            },
                        );

                    let text_entity = child_spawner
                        .spawn((
                            Text::new("Respawn"),
                            TextFont {
                                font: game_assets.font.clone(),
                                font_size: 33.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.1, 0.1, 0.1)),
                        ))
                        .id();

                    child_spawner
                        .commands()
                        .entity(text_entity)
                        .observe(respawn_player)
                        .observe(
                            move |_trigger: Trigger<Pointer<Over>>,
                                  mut text_query: Query<&mut Text>| {
                                if let Ok(mut text) = text_query.get_mut(text_entity) {
                                    **text = "Respawn ◀".into();
                                }
                            },
                        )
                        .observe(
                            move |_trigger: Trigger<Pointer<Out>>,
                                  mut text_query: Query<&mut Text>| {
                                if let Ok(mut text) = text_query.get_mut(text_entity) {
                                    **text = "Respawn".into();
                                }
                            },
                        );

                    let text_entity = child_spawner
                        .spawn((
                            Text::new("Reset All Objects"),
                            TextFont {
                                font: game_assets.font.clone(),
                                font_size: 33.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.1, 0.1, 0.1)),
                        ))
                        .id();

                    child_spawner
                        .commands()
                        .entity(text_entity)
                        .observe(reset_all_objects)
                        .observe(
                            move |_trigger: Trigger<Pointer<Over>>,
                                  mut text_query: Query<&mut Text>| {
                                if let Ok(mut text) = text_query.get_mut(text_entity) {
                                    **text = "Reset All Objects ◀".into();
                                }
                            },
                        )
                        .observe(
                            move |_trigger: Trigger<Pointer<Out>>,
                                  mut text_query: Query<&mut Text>| {
                                if let Ok(mut text) = text_query.get_mut(text_entity) {
                                    **text = "Reset All Objects".into();
                                }
                            },
                        );
                });
        });
}

fn respawn_player(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    respawn: Single<&Transform, With<PlayerSpawnPoint>>,
) {
    commands
        .entity(*player)
        .insert(Transform::from_translation(respawn.translation));
}

fn reset_all_objects(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    q_dissolveable: Query<(Entity, &Dissolveable)>,
    q_player: Query<&RightHand, With<Player>>,
) {
    // Reset all dissolveable objects in the world
    for (entity, dissolveable) in &q_dissolveable {
        match &dissolveable.respawn_transform {
            Some(respawn_transform) => {
                // Respawn the entity at the specified transform
                commands.entity(entity).try_insert(*respawn_transform);
            }
            None => {
                // If no respawn transform, despawn the entity
                if let Ok(mut ec) = commands.get_entity(entity) {
                    ec.try_insert(Tombstone).despawn();
                }
            }
        }
    }

    // Also reset any held objects that are dissolveable
    for right_hand in &q_player {
        if let Some(held_entity) = right_hand.held_object {
            if let Ok((_, dissolveable)) = q_dissolveable.get(held_entity) {
                match &dissolveable.respawn_transform {
                    Some(respawn_transform) => {
                        // Respawn the held entity at the specified transform and remove Held component
                        commands
                            .entity(held_entity)
                            .try_insert(*respawn_transform)
                            .remove::<Held>();
                    }
                    None => {
                        if let Ok(mut ec) = commands.get_entity(held_entity) {
                            ec.try_insert(Tombstone).despawn();
                        }
                    }
                }
            }
        }
    }
}

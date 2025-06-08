use avian3d::prelude::RigidBodyDisabled;
use bevy::prelude::*;

use crate::{asset_management::asset_loading::GameAssets, game::player::Player, GameState};

pub fn main_menu_plugin(app: &mut App) {
    app.add_sub_state::<MainMenuState>()
        .add_systems(OnEnter(MainMenuState::Shown), spawn_main_menu);
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Playing)]
#[states(scoped_entities)]
pub enum MainMenuState {
    #[default]
    Shown,
    Hidden,
}

fn spawn_main_menu(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    player: Single<Entity, With<Player>>,
) {
    commands.entity(*player).insert(RigidBodyDisabled);
    let player_id = *player;
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Start,
                align_items: AlignItems::FlexStart,
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                padding: UiRect::left(Val::Percent(8.33)).with_top(Val::Percent(8.33)),
                ..default()
            },
            StateScoped(MainMenuState::Shown),
        ))
        .with_children(|child_spawner| {
            child_spawner.spawn((
                Text::new("at the end of the hall"),
                TextFont {
                    font: game_assets.font.clone(),
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(0.1, 0.1, 0.1)),
                Node {
                    margin: UiRect::bottom(Val::Percent(12.)),
                    ..default()
                },
            ));

            let text_entity = child_spawner
                .spawn((
                    Text::new("Play"),
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
                .observe(
                    move |_trigger: Trigger<Pointer<Click>>, mut commands: Commands| {
                        commands.set_state(MainMenuState::Hidden);
                        commands.entity(player_id).remove::<RigidBodyDisabled>();
                    },
                )
                .observe(
                    move |_trigger: Trigger<Pointer<Over>>, mut text_query: Query<&mut Text>| {
                        if let Ok(mut text) = text_query.get_mut(text_entity) {
                            **text = "Play ◀".into();
                        }
                    },
                )
                .observe(
                    move |_trigger: Trigger<Pointer<Out>>, mut text_query: Query<&mut Text>| {
                        if let Ok(mut text) = text_query.get_mut(text_entity) {
                            **text = "Play".into();
                        }
                    },
                );
        });
}

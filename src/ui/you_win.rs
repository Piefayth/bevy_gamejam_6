use crate::{asset_management::asset_loading::GameAssets, GameState};
use bevy::prelude::*;

#[derive(Component)]
struct FadeInBackground {
    timer: Timer,
}

pub fn you_win_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Win), win)
        .add_systems(Update, fade_in_background.run_if(in_state(GameState::Win)));
}

fn win(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        FadeInBackground {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
        },
        StateScoped(GameState::Win),
        children![
            (
                Text::new("congratulations."),
                TextFont {
                    font: game_assets.font.clone(),
                    font_size: 33.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
            ),
            (
                Text::new("and thank you for playing."),
                TextFont {
                    font: game_assets.font.clone(),
                    font_size: 33.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
            )
        ],
    ));
}

fn fade_in_background(
    time: Res<Time>,
    mut query: Query<(&mut FadeInBackground, &mut BackgroundColor)>,
) {
    for (mut fade, mut bg_color) in query.iter_mut() {
        fade.timer.tick(time.delta());

        // Calculate fade progress (0.0 to 1.0)
        let progress = fade.timer.elapsed_secs() / fade.timer.duration().as_secs_f32();
        let alpha = progress.clamp(0.0, 1.0);

        // Update background color alpha
        *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, alpha));
    }
}

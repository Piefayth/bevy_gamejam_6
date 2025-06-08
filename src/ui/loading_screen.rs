use bevy::{color::palettes::css::BLACK, prelude::*};

use crate::GameState;

pub fn loading_screen_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), spawn_loading_screen);
}

fn spawn_loading_screen(mut commands: Commands) {
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
        BackgroundColor(BLACK.into()),
        StateScoped(GameState::Loading),
        children![(
            Text::new("Loading"),
            TextFont {
                //font: game_assets.font.clone(),
                font_size: 33.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )],
    ));
}

use bevy::{color::palettes::css::{BLACK, ORANGE}, picking::pointer::PointerInteraction, prelude::*, window::CursorGrabMode};
use bevy_enhanced_input::events::Completed;

use crate::{
    game::{
        input::SystemMenuOrCancel,
        interaction::{Interactable, Interactions, InteractionsDisabled, INTERACTION_DISTANCE}, player::Held,
    }, GameState
};

pub fn crosshair_plugin(app: &mut App) {
    app.add_sub_state::<CrosshairState>()
        .add_systems(OnEnter(CrosshairState::Shown), enable_crosshair)
        .add_systems(OnEnter(CrosshairState::Hidden), disable_crosshair)
        .add_observer(toggle_aim_state)
        .add_systems(Update, display_interaction_state);
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Playing)]
#[states(scoped_entities)]
pub enum CrosshairState {
    #[default]
    Shown,
    Hidden,
}

#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct CrosshairReticle;

#[derive(Component)]
pub struct LeftCrosshairText;

#[derive(Component)]
pub struct RightCrosshairText;

fn enable_crosshair(mut commands: Commands, mut primary_window: Single<&mut Window>) {
    primary_window.cursor_options.grab_mode = CursorGrabMode::Confined;
    primary_window.cursor_options.visible = false;

commands
    .spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Grid,
            grid_template_columns: vec![
                GridTrack::flex::<RepeatedGridTrack>(1.0),      // Left area
                GridTrack::auto::<RepeatedGridTrack>(),         // Center reticle (auto-sized)
                GridTrack::flex::<RepeatedGridTrack>(1.0),      // Right area
            ],
            align_items: AlignItems::Center,
            justify_items: JustifyItems::Center,
            position_type: PositionType::Absolute,
            ..default()
        },
        Crosshair,
        Pickable::IGNORE,
        StateScoped(CrosshairState::Shown),
    ))
    .with_children(|child_builder| {
        // Left text (placed in left column, right-aligned)
        child_builder.spawn((
            Node {
                grid_column: GridPlacement::start(1),
                justify_self: JustifySelf::End,
                padding: UiRect::right(Val::Px(10.0)),
                ..default()
            },
            Text::new(""),
            TextShadow {
                offset: Vec2::new(1., 1.),
                color: BLACK.into(),
            },
            TextFont {
                font_size: 14.,
                ..default()
            },
            Pickable::IGNORE,
            LeftCrosshairText,
        ));
        
        // Center reticle (placed in center column)
        child_builder.spawn((
            Node {
                width: Val::Px(10.0),
                height: Val::Px(10.0),
                border: UiRect::all(Val::Px(2.0)),
                grid_column: GridPlacement::start(2),
                ..default()
            },
            CrosshairReticle,
            BorderRadius::all(Val::Percent(50.0)),
            BackgroundColor(Color::Srgba(Srgba::new(1., 1., 1., 0.25))),
            BorderColor(Color::Srgba(Srgba::new(0., 0., 0., 0.25))),
            Pickable::IGNORE,
        ));
        
        // Right text (placed in right column, left-aligned)
        child_builder.spawn((
            Node {
                grid_column: GridPlacement::start(3),
                justify_self: JustifySelf::Start,
                padding: UiRect::left(Val::Px(10.0)),
                ..default()
            },
            Text::new(""),
            TextShadow {
                offset: Vec2::new(1., 1.),
                color: BLACK.into(),
            },
            TextFont {
                font_size: 14.,
                ..default()
            },
            Pickable::IGNORE,
            RightCrosshairText,
        ));
    });
}

fn disable_crosshair(mut primary_window: Single<&mut Window>) {
    primary_window.cursor_options.grab_mode = CursorGrabMode::None;
    primary_window.cursor_options.visible = true;
}

fn toggle_aim_state(
    _trigger: Trigger<Completed<SystemMenuOrCancel>>,
    mut commands: Commands,
    crosshair_state: Option<Res<State<CrosshairState>>>,
) {
    if let Some(crosshair_state) = crosshair_state {
        if matches!(**crosshair_state, CrosshairState::Shown) {
            commands.set_state(CrosshairState::Hidden);
        } else {
            commands.set_state(CrosshairState::Shown);
        }
    }
}

fn display_interaction_state(
    mut commands: Commands,
    pointers: Query<&PointerInteraction>,
    q_interactable: Query<&Interactable, (Without<InteractionsDisabled>)>,
    q_crosshair_reticle: Query<Entity, With<CrosshairReticle>>,
    crosshair_state: Option<Res<State<CrosshairState>>>,
    maybe_left_text: Option<Single<&mut Text, With<LeftCrosshairText>>>,
    maybe_held_object: Option<Single<&Held>>,
) {
    if let Some(crosshair_state) = crosshair_state {
        if matches!(**crosshair_state, CrosshairState::Shown) {
            if let Ok(reticle_entity) = q_crosshair_reticle.single() {
                // Get the interactable entity if one is hit
                let hit_interactable = pointers
                    .iter()
                    .filter_map(|interaction| interaction.get_nearest_hit())
                    .find_map(|(entity, hit)| {
                        if hit.depth <= INTERACTION_DISTANCE {
                            q_interactable.get(*entity).ok()
                        } else {
                            None
                        }
                    });
                
                let (border_color, background_color) = if hit_interactable.is_some() {
                    (
                        Color::Srgba(Srgba::new(1.0, 0.5, 0.0, 1.0)),
                        Color::Srgba(Srgba::new(1.0, 1.0, 1.0, 1.0)),
                    )
                } else {
                    (
                        Color::Srgba(Srgba::new(0.0, 0.0, 0.0, 0.25)),
                        Color::Srgba(Srgba::new(1.0, 1.0, 1.0, 0.25)),
                    )
                };
                
                if let Some(mut left_text) = maybe_left_text {
                    if let Some(interactable) = hit_interactable {
                        left_text.0 = match interactable.primary_action {
                            Interactions::Press => String::from("Press"),
                            Interactions::PickUp => String::from("Pick Up"),
                        };
                    } else if let Some(held_object) = maybe_held_object {
                        if held_object.can_release {
                            left_text.0 = String::from("Release");
                        } else {
                            left_text.0 = String::from("");
                        }
                    }
                    else {
                        left_text.0 = String::from("");
                    }
                }

                
                commands
                    .entity(reticle_entity)
                    .insert(BorderColor(border_color))
                    .insert(BackgroundColor(background_color));
            }
        }
    }
}

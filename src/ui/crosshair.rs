use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::{color::palettes::css::BLACK, prelude::*, window::CursorGrabMode};
use bevy_enhanced_input::events::Completed;

use crate::{
    game::{
        input::SystemMenuOrCancel,
        interaction::{Interactable, Interactions, InteractionsDisabled, INTERACTION_DISTANCE},
        player::Held,
        GameLayer,
    },
    ui::main_menu::MainMenuState,
};

pub fn crosshair_plugin(app: &mut App) {
    app.add_sub_state::<CrosshairState>()
        .add_systems(OnEnter(CrosshairState::Shown), enable_crosshair)
        .add_systems(OnEnter(CrosshairState::Hidden), disable_crosshair)
        .add_systems(
            Update,
            (display_interaction_state).run_if(in_state(CrosshairState::Shown)),
        )
        //.add_systems(PreUpdate, override_pointer_to_center.before(PickSet::Backend).after(PickSet::ProcessInput))
        .add_observer(toggle_aim_state);
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(MainMenuState = MainMenuState::Hidden)]
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
                    GridTrack::flex::<RepeatedGridTrack>(1.0), // Left area
                    GridTrack::auto::<RepeatedGridTrack>(),    // Center reticle (auto-sized)
                    GridTrack::flex::<RepeatedGridTrack>(1.0), // Right area
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

// pub fn override_pointer_to_center(
//     mut pointers: Query<&mut PointerLocation>,
//     primary_window: Single<Entity, With<PrimaryWindow>>,
//     windows: Query<&Window>,
//     crosshair_state: Option<Res<State<CrosshairState>>>,
// ) {
//     // Only override when crosshair is shown (cursor is grabbed)
//     if let Some(crosshair_state) = crosshair_state {
//         if matches!(**crosshair_state, CrosshairState::Shown) {
//             if let Ok(window) = windows.get(primary_window.entity()) {
//                 let window_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);

//                 // Create the center location for the primary window
//                 let primary_window_target = NormalizedRenderTarget::Window(
//                     WindowRef::Primary.normalize(Some(primary_window.entity())).unwrap()
//                 );

//                 let center_location = Location {
//                     target: primary_window_target.clone(),
//                     position: window_center,
//                 };

//                 // Only update pointers that are targeting the primary window
//                 for mut pointer_location in &mut pointers {
//                     if let Some(current_location) = &pointer_location.location {
//                         // Check if this pointer is targeting the primary window
//                         if current_location.target == primary_window_target {
//                             pointer_location.location = Some(center_location.clone());
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

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
    spatial_query: SpatialQuery,
    camera_query: Query<&GlobalTransform, With<Camera>>,
    q_interactable: Query<&Interactable, Without<InteractionsDisabled>>,
    q_crosshair_reticle: Query<Entity, With<CrosshairReticle>>,
    crosshair_state: Option<Res<State<CrosshairState>>>,
    maybe_left_text: Option<Single<&mut Text, With<LeftCrosshairText>>>,
    maybe_held_object: Option<Single<&Held>>,
) {
    if let Some(crosshair_state) = crosshair_state {
        if matches!(**crosshair_state, CrosshairState::Shown) {
            if let Ok(reticle_entity) = q_crosshair_reticle.single() {
                // Get camera transform for raycast
                let Ok(camera_transform) = camera_query.single() else {
                    return;
                };

                // Cast ray from camera forward
                let ray_origin = camera_transform.translation();
                let ray_direction = camera_transform.forward();

                // Get the interactable entity if one is hit
                let hit_interactable = if let Some(hit) = spatial_query.cast_ray(
                    ray_origin,
                    ray_direction,
                    INTERACTION_DISTANCE,
                    true,
                    &SpatialQueryFilter::default()
                        .with_mask([GameLayer::Default, GameLayer::Device]),
                ) {
                    let hit_entity = hit.entity;
                    if q_interactable.contains(hit_entity)
                        && !(maybe_held_object.is_some()
                            && q_interactable
                                .get(hit_entity)
                                .is_ok_and(|i| matches!(i.primary_action, Interactions::PickUp)))
                    {
                        q_interactable.get(hit_entity).ok()
                    } else {
                        None
                    }
                } else {
                    None
                };

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
                    } else {
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

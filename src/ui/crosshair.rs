use bevy::{picking::pointer::PointerInteraction, prelude::*, window::CursorGrabMode};
use bevy_enhanced_input::events::Completed;

use crate::{
    game::{input::SystemMenuOrCancel, interaction::{Interactable, InteractionsDisabled, INTERACTION_DISTANCE}}, GameState
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

fn enable_crosshair(mut commands: Commands, mut primary_window: Single<&mut Window>) {
    primary_window.cursor_options.grab_mode = CursorGrabMode::Confined;
    primary_window.cursor_options.visible = false;

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
            Crosshair,
            Pickable::IGNORE,
            StateScoped(CrosshairState::Shown),
        ))
        .with_children(|child_builder| {
            child_builder.spawn((
                Node {
                    width: Val::Px(10.0),
                    height: Val::Px(10.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                CrosshairReticle,
                BorderRadius::all(Val::Percent(50.0)),
                BackgroundColor(Color::Srgba(Srgba::new(1., 1., 1., 0.25))),
                BorderColor(Color::Srgba(Srgba::new(0., 0., 0., 0.25))),
                Pickable::IGNORE,
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
   q_interactable: Query<Entity,  (With<Interactable>, Without<InteractionsDisabled>)>,
   q_crosshair_reticle: Query<Entity, With<CrosshairReticle>>,
   crosshair_state: Option<Res<State<CrosshairState>>>,
) {
   if let Some(crosshair_state) = crosshair_state {
       if matches!(**crosshair_state, CrosshairState::Shown) {
           if let Ok(reticle_entity) = q_crosshair_reticle.single() {
               let hit_interactable = pointers
                   .iter()
                   .filter_map(|interaction| interaction.get_nearest_hit())
                   .any(|(entity, hit)| hit.depth <= INTERACTION_DISTANCE && q_interactable.contains(*entity));
              
               let (border_color, background_color) = if hit_interactable {
                   (
                       Color::Srgba(Srgba::new(1.0, 0.5, 0.0, 1.0)),
                       Color::Srgba(Srgba::new(1.0, 1.0, 1.0, 1.0))
                   )
               } else {
                   (
                       Color::Srgba(Srgba::new(0.0, 0.0, 0.0, 0.25)),
                       Color::Srgba(Srgba::new(1.0, 1.0, 1.0, 0.25))
                   )
               };
              
               commands.entity(reticle_entity)
                   .insert(BorderColor(border_color))
                   .insert(BackgroundColor(background_color));
           }
       }
   }
}

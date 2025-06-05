use bevy::prelude::*;
use bevy_enhanced_input::{input::Input, prelude::{Actions, Binding, InputAction, InputContext, InputContextAppExt}, preset::Cardinal, EnhancedInputPlugin};

pub struct UpdateInputContext;
impl InputContext for UpdateInputContext {
    type Schedule = PreUpdate;
}

pub struct FixedInputContext;
impl InputContext for FixedInputContext {
    type Schedule = FixedPreUpdate;
}

pub fn input_plugin(app: &mut App) {
    app
        .add_plugins(EnhancedInputPlugin)
        // .add_plugins(MeshPickingPlugin)
        // .insert_resource(MeshPickingSettings {
        //     require_markers: true,
        //     ..default()
        // })
        .add_input_context::<UpdateInputContext>()
        .add_input_context::<FixedInputContext>()
        .add_observer(update_input_binding)
        .add_observer(fixed_update_input_binding)
        .add_systems(Startup, spawn_input_manager);
}


#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct SystemMenuOrCancel;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct UseInteract;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Jump;

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
pub struct Look;

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
pub struct Movement;

fn spawn_input_manager(mut commands: Commands) {
    // i don't think there's any reason to put these on the player itself in a single player game?
    commands.spawn((
        Actions::<UpdateInputContext>::default(),
        Actions::<FixedInputContext>::default(),
    ));
}

fn update_input_binding(
    trigger: Trigger<Binding<UpdateInputContext>>,
    mut q_update_input_manager: Query<&mut Actions<UpdateInputContext>>,
) {
    if let Ok(mut actions) = q_update_input_manager.get_mut(trigger.target()) {
        actions.bind::<SystemMenuOrCancel>().to(KeyCode::Escape);

        actions.bind::<Look>()
            .to(Input::mouse_motion());
    }
}

fn fixed_update_input_binding(
    trigger: Trigger<Binding<FixedInputContext>>,
    mut q_fixed_input_manager: Query<&mut Actions<FixedInputContext>>,
) {
    if let Ok(mut actions) = q_fixed_input_manager.get_mut(trigger.target()) {
        actions.bind::<Movement>()
            .to(Cardinal::wasd_keys());
        
        actions.bind::<Jump>()
            .to(KeyCode::Space);

        actions.bind::<UseInteract>().to(MouseButton::Left);
    }
}

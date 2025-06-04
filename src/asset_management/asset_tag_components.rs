use avian3d::prelude::RigidBody;
use bevy::prelude::*;

use crate::game::dissolve_gate::Dissolveable;

// Thanks to a bug with Bevity, we need to make these tag components with any random field
// whoops

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct RoomWall {
    pub unused: bool,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct BigRedButton {
    pub unused: bool,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct WeightedCube {
    pub color: WeightedCubeColors,
}

#[derive(Reflect)]
pub enum WeightedCubeColors {
    Cyan
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CubeSpitter {
    pub color: WeightedCubeColors,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SignalSpitter {
    pub unused: bool,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct NeedsRigidBody {
    pub kind: RigidBody,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ExitDoorShutter {
    pub unused: bool,
}

// Note: This is the actual part that moves; we don't need a reference to the other bit
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PressurePlate {
    pub unused: bool,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ChargePad {
    pub unused: bool,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct DissolveGate {
    pub unused: bool,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Door {
    pub unused: bool,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct DoorPole {
    pub unused: bool,
}


#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Inert {
    pub unused: bool,
}

pub fn asset_tag_components_plugin(app: &mut App) {
    app.register_type::<RoomWall>()
        .register_type::<BigRedButton>()
        .register_type::<WeightedCube>()
        .register_type::<WeightedCubeColors>()
        .register_type::<CubeSpitter>()
        .register_type::<SignalSpitter>()
        .register_type::<NeedsRigidBody>()
        .register_type::<ExitDoorShutter>()
        .register_type::<PressurePlate>()
        .register_type::<DissolveGate>()
        .register_type::<Dissolveable>()
        .register_type::<ChargePad>()
        .register_type::<Door>()
        .register_type::<DoorPole>()
        .register_type::<Inert>()
        ;
}

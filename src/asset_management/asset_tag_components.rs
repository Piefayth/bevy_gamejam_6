use avian3d::prelude::RigidBody;
use bevy::prelude::*;

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
pub struct NeedsRigidBody {
    pub kind: RigidBody,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ExitDoorShutter {
    pub unused: bool,
}

pub fn asset_tag_components_plugin(app: &mut App) {
    app.register_type::<RoomWall>()
        .register_type::<BigRedButton>()
        .register_type::<WeightedCube>()
        .register_type::<WeightedCubeColors>()
        .register_type::<CubeSpitter>()
        .register_type::<NeedsRigidBody>()
        .register_type::<ExitDoorShutter>();
}

use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
};
use bevy_tween::tween::TargetAsset;

pub fn test_material_plugin(app: &mut App) {
    app
        .add_plugins(MaterialPlugin::<TestMaterial>::default())
        .register_type::<TestMaterial>()
        .register_type::<TargetAsset<TestMaterial>>()
        .register_asset_reflect::<TestMaterial>();
}

pub type TestMaterial = ExtendedMaterial<StandardMaterial, TestMaterialExtension>;

#[derive(Asset, AsBindGroup, Reflect, Default, Debug, Clone)]
#[reflect(Default)]
pub struct TestMaterialExtension {
    #[uniform(100)]
    pub params: TestMaterialParams,
}

#[derive(Reflect, ShaderType, Default, Debug, Clone)]
pub struct TestMaterialParams {
    pub stripe_color: LinearRgba,
    pub stripe_frequency: f32,
    pub stripe_angle: f32,
    pub stripe_thickness: f32,
    pub scroll_speed: f32,
}

impl MaterialExtension for TestMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/test_material.wgsl".into()
    }
}

use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

pub fn unlit_material_plugin(app: &mut App) {
    app
        .add_plugins(MaterialPlugin::<UnlitMaterial>::default())
        .register_type::<UnlitMaterial>()
        .register_asset_reflect::<UnlitMaterial>(); 
}

pub type UnlitMaterial = ExtendedMaterial<StandardMaterial, UnlitMaterialExtension>;

#[derive(Asset, AsBindGroup, Reflect, Default, Debug, Clone)]
#[reflect(Default)]
pub struct UnlitMaterialExtension {
    #[uniform(100)]
    pub intensity: f32,
    #[uniform(101)]
    pub alpha: f32,
    #[uniform(102)]
    pub blend_color: LinearRgba,
    #[uniform(103)]
    pub blend_factor: f32,

}

impl MaterialExtension for UnlitMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/unlit.wgsl".into()
    }
}

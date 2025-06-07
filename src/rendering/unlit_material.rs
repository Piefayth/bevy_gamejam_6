use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MeshPipelineKey},
    prelude::*,
    render::{mesh::MeshVertexBufferLayoutRef, render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef, ShaderType, SpecializedMeshPipelineError}},
};
use bevy_tween::{asset_tween_system, prelude::Interpolator, tween::TargetAsset, BevyTweenRegisterSystems};
use crate::game::signals::MaterialIntensityInterpolator;

pub fn unlit_material_plugin(app: &mut App) {
    app
        .add_plugins(MaterialPlugin::<UnlitMaterial>::default())
        .register_type::<UnlitMaterial>()
        .register_type::<TargetAsset<UnlitMaterial>>()
        .register_asset_reflect::<UnlitMaterial>()
        .add_tween_systems(asset_tween_system::<MaterialIntensityInterpolator>())
        .add_tween_systems(asset_tween_system::<MaterialColorOverrideInterpolator>());
}

pub type UnlitMaterial = ExtendedMaterial<StandardMaterial, UnlitMaterialExtension>;

#[derive(Asset, AsBindGroup, Reflect, Default, Debug, Clone)]
#[reflect(Default)]
pub struct UnlitMaterialExtension {
    #[uniform(100)]
    pub params: UnlitParams,
}

#[derive(Reflect, ShaderType, Default, Debug, Clone)]
pub struct UnlitParams {
    pub intensity: f32,
    pub alpha: f32,
    pub blend_color: LinearRgba,
    pub blend_factor: f32,
    pub grey_threshold: f32,
}

impl MaterialExtension for UnlitMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/unlit.wgsl".into()
    }
}

#[derive(Reflect, Debug)]
pub struct MaterialColorOverrideInterpolator {
    pub target_color: LinearRgba,
}

impl Interpolator for MaterialColorOverrideInterpolator {
    type Item = UnlitMaterial;
    
    fn interpolate(&self, material: &mut Self::Item, progress: f32) {
        let invert_progress = 1.0 - progress;
        material.extension.params.blend_color = self.target_color;
        material.extension.params.grey_threshold = 0.0;
        material.extension.params.blend_factor = invert_progress;
    }
}

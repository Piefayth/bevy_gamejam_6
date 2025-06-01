#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
}

@group(2) @binding(100) var<uniform> intensity: f32;
@group(2) @binding(101) var<uniform> alpha: f32;
@group(2) @binding(102) var<uniform> blend_color: vec4<f32>;
@group(2) @binding(103) var<uniform> blend_factor: f32;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let material_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
   
    // Blend between material color and blend_color using blend_factor
    let blended_color = mix(material_color, blend_color, blend_factor);
   
    var out: FragmentOutput;
    out.color = vec4<f32>(blended_color.rgb * intensity, material_color.a * alpha);
   
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    mesh_view_bindings::globals,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
}

struct StripedMaskParams {
    stripe_color: vec4<f32>,
    stripe_frequency: f32,
    stripe_angle: f32,
    stripe_thickness: f32,
    scroll_speed: f32,
}

@group(2) @binding(100) var<uniform> params: StripedMaskParams;

fn rotate_uv(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let cos_angle = cos(angle);
    let sin_angle = sin(angle);
    let rotation_matrix = mat2x2<f32>(
        cos_angle, -sin_angle,
        sin_angle, cos_angle
    );
    return rotation_matrix * uv;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let material_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
   
    // Rotate UV coordinates based on stripe angle
    let rotated_uv = rotate_uv(in.uv, params.stripe_angle);
   
    // Add continuous scrolling animation - move along the rotated Y axis
    let animated_position = rotated_uv.y + (globals.time * params.scroll_speed);
   
    // Create stripe pattern using sine wave
    let stripe_position = animated_position * params.stripe_frequency;
    let stripe_wave = sin(stripe_position * 3.14159265359 * 2.0);
   
    // Convert wave to binary mask based on thickness
    let stripe_mask = step(params.stripe_thickness, stripe_wave);
   
    var out: FragmentOutput;
    // Use the material color's alpha (which handles AlphaMode::Mask) and apply stripe mask
    out.color = vec4<f32>(params.stripe_color.rgb, stripe_mask);
   
    //out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

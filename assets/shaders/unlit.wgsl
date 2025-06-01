#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
}

@group(2) @binding(1) var base_color_texture: texture_2d<f32>;
@group(2) @binding(2) var base_color_sampler: sampler;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    let base_color = textureSample(base_color_texture, base_color_sampler, in.uv);
    
    var out: FragmentOutput;
    out.color = base_color;
    return out;
}

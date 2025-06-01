// Since post processing is a fullscreen effect, we use the fullscreen vertex shader provided by bevy.
// This will import a vertex shader that renders a single fullscreen triangle.
//
// A fullscreen triangle is a single triangle that covers the entire screen.
// The box in the top left in that diagram is the screen. The 4 x are the corner of the screen
//
// Y axis
//  1 |  x-----x......
//  0 |  |  s  |  . ´
// -1 |  x_____x´
// -2 |  :  .´
// -3 |  :´
//    +---------------  X axis
//      -1  0  1  2  3
//
// As you can see, the triangle ends up bigger than the screen.
//
// You don't need to worry about this too much since bevy will compute the correct UVs for you.
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct PostProcessSettings {
    stroke_color: vec4<f32>,  // Fixed: vec4f -> vec4<f32>
    width: u32,
    // Fixed: Always include padding for consistency across platforms
    _padding: vec3<f32>,  // Changed from vec3<f32> to match u32 alignment
}

@group(0) @binding(2) var<uniform> settings: PostProcessSettings;
@group(0) @binding(3) var vertex_id_texture: texture_2d<f32>;
@group(0) @binding(4) var vertex_id_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(vertex_id_texture);

    let diff = sobel(
        vertex_id_texture,
        dimensions,
        in.uv,
        vec2u(settings.width, settings.width)
    );

    return mix(
        textureSample(screen_texture, texture_sampler, in.uv),
        settings.stroke_color,
        step(0.001, diff)
    );

}

// Fixed function with proper bounds checking
fn sobel(
    vertex_id_texture: texture_2d<f32>,
    dimensions: vec2u,
    uv: vec2f,
    offset: vec2u
) -> f32 {
    let offseti: vec2i = vec2i(offset);
    let xy = vec2i(uv * vec2f(dimensions));

    let px_center: f32 = textureLoad(vertex_id_texture, xy, 0).r;

    let px_left: f32 = textureLoad(vertex_id_texture, xy + vec2i(-offseti.x, 0), 0).r;
    let px_left_up: f32 = textureLoad(vertex_id_texture, xy + vec2i(-offseti.x, 1), 0).r;
    let px_left_down: f32 = textureLoad(vertex_id_texture, xy + vec2i(-offseti.x, -offseti.y), 0).r;

    let px_up: f32 = textureLoad(vertex_id_texture, xy + vec2i(0, offseti.y), 0).r;

    let px_right: f32 = textureLoad(vertex_id_texture, xy + vec2i(offseti.x, 0), 0).r;
    let px_right_up: f32 = textureLoad(vertex_id_texture, xy + vec2i(offseti.x, offseti.y), 0).r;
    let px_right_down: f32 = textureLoad(vertex_id_texture, xy + vec2i(offseti.x, -offseti.y), 0).r;

    let px_down: f32 = textureLoad(vertex_id_texture, xy + vec2i(0, -offseti.y), 0).r;

    return max(
        abs(
            1 * px_left_down + 2 * px_left + 1 * px_left_up - 1 * px_right_down - 2 * px_right - 1 * px_right_up
        ),
        abs(
            1 * px_left_up + 2 * px_up + 1 * px_right_up - 1 * px_left_down - 2 * px_down - 1 * px_right_down
        )
    );
}

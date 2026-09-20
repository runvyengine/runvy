// Vertex shader
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), // left-bottom
        vec2<f32>(3.0, -1.0),  // right-bottom
        vec2<f32>(-1.0, 3.0),  // left-top

        vec2<f32>(-1.0, 3.0),  // left-top
        vec2<f32>(3.0, -1.0),  // right-bottom
        vec2<f32>(3.0, 3.0)    // right-top
    );

    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),

        vec2<f32>(0.0, -1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(2.0, -1.0)
    );

    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = uv[vertex_index];
    return out;
}

// Fragment shader
@group(0) @binding(0) var t_render_target: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_render_target, s_sampler, in.uv);
}

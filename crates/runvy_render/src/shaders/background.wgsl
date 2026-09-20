struct BackgroundUniforms {
    inverse_view_proj: mat4x4<f32>,
    mode: vec4<u32>,
    background_params: vec4<f32>,
    solid_color: vec4<f32>,
    zenith_color: vec4<f32>,
    horizon_color: vec4<f32>,
    ground_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> background: BackgroundUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) ndc: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );

    let position = positions[vertex_index];

    var out: VertexOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = position * 0.5 + vec2<f32>(0.5);
    out.ndc = position;
    return out;
}

fn gradient_color(t: f32) -> vec3<f32> {
    let horizon = clamp(background.zenith_color.w, 0.001, 0.999);
    let smoothness = max(background.horizon_color.w, 0.001);
    let lower_width = max(horizon * smoothness, 0.001);
    let upper_width = max((1.0 - horizon) * smoothness, 0.001);

    let lower = smoothstep(0.0, horizon + lower_width, t);
    var color = mix(background.ground_color.rgb, background.horizon_color.rgb, lower);

    let upper = smoothstep(horizon - upper_width, 1.0, t);
    color = mix(color, background.zenith_color.rgb, upper);
    return color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if background.mode.x == 0u {
        return vec4<f32>(background.solid_color.rgb * background.background_params.x, 1.0);
    }

    // Sky mode is reserved; until a sky renderer exists it uses the gradient fallback.
    let near_world_h = background.inverse_view_proj * vec4<f32>(in.ndc, 0.0, 1.0);
    let far_world_h = background.inverse_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let near_world = near_world_h.xyz / near_world_h.w;
    let far_world = far_world_h.xyz / far_world_h.w;
    let world_ray = normalize(far_world - near_world);
    let world_t = clamp(world_ray.y * 0.5 + 0.5, 0.0, 1.0);

    return vec4<f32>(gradient_color(world_t) * background.background_params.x, 1.0);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
        vec2<f32>(-1.0, 3.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(3.0, 3.0)
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

struct PostProcessParams {
    fade_color: vec4<f32>,
    vignette_strength: f32,
    vignette_radius: f32,
    vignette_softness: f32,
    rgb_shift: vec2<f32>,
    tint_color: vec4<f32>,
    brightness: f32,
    contrast: f32,
    flags: u32,
    _pad: vec3<u32>,
}

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;
@group(0) @binding(2) var<uniform> params: PostProcessParams;

fn unpack_flags(flags: u32) -> vec4<bool> {
    return vec4<bool>(
        (flags & 1u) != 0u,
        (flags & 2u) != 0u,
        (flags & 4u) != 0u,
        (flags & 8u) != 0u,
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let flags = unpack_flags(params.flags);
    var color = textureSample(t_scene, s_sampler, in.uv);

    if flags.y {
        let dist = distance(in.uv, vec2<f32>(0.5, 0.5));
        let normalized = dist / params.vignette_radius;
        let vignette = 1.0 - smoothstep(params.vignette_radius - params.vignette_softness, params.vignette_radius + params.vignette_softness, dist);
        color = mix(color, vec4<f32>(0.0, 0.0, 0.0, 1.0), params.vignette_strength * (1.0 - vignette));
    }

    if flags.z {
        let r = textureSample(t_scene, s_sampler, in.uv + vec2<f32>(params.rgb_shift.x, 0.0)).r;
        let g = textureSample(t_scene, s_sampler, in.uv + vec2<f32>(0.0, 0.0)).g;
        let b = textureSample(t_scene, s_sampler, in.uv + vec2<f32>(0.0, params.rgb_shift.y)).b;
        color = vec4<f32>(r, g, b, color.a);
    }

    if flags.w {
        color = color * params.tint_color;
    }

    if (flags.x) {
        color = mix(color, params.fade_color, params.fade_color.a);
    }

    return color;
}

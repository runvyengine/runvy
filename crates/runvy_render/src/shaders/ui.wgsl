// UI Shader for rendering debug rectangles and text
// Uses screen-space coordinates (0,0 = top-left)

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct TexturedVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct TexturedVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct UIUniforms {
    screen_width: f32,
    screen_height: f32,
};

@group(0) @binding(0) var<uniform> ui: UIUniforms;
@group(0) @binding(1) var t_diffuse: texture_2d<f32>;
@group(0) @binding(2) var s_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Convert from screen space (0..width, 0..height) to clip space (-1..1)
    // Y is flipped: screen Y=0 is top, clip Y=-1 is bottom
    let clip_x = (vertex.position.x / ui.screen_width) * 2.0 - 1.0;
    let clip_y = 1.0 - (vertex.position.y / ui.screen_height) * 2.0;

    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.color = vertex.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

// Textured vertex shader for text rendering
@vertex
fn vs_textured_main(vertex: TexturedVertexInput) -> TexturedVertexOutput {
    var out: TexturedVertexOutput;

    // Convert from screen space (0..width, 0..height) to clip space (-1..1)
    let clip_x = (vertex.position.x / ui.screen_width) * 2.0 - 1.0;
    let clip_y = 1.0 - (vertex.position.y / ui.screen_height) * 2.0;

    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.tex_coords = vertex.tex_coords;
    out.color = vertex.color;

    return out;
}

@fragment
fn fs_textured_main(in: TexturedVertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(t_diffuse, s_sampler, in.tex_coords);

    // Heuristic: if texture has meaningful alpha, treat as regular RGBA image.
    // Otherwise fall back to using red channel as mask (font atlas grayscale).
    let alpha_from_tex = texture_color.a;
    let mask = texture_color.r;

    var out_rgb: vec3<f32>;
    var out_a: f32;

    if (alpha_from_tex > 0.001) {
        // Regular textured image: modulate RGB and multiply alpha
        out_rgb = texture_color.rgb * in.color.rgb;
        out_a = alpha_from_tex * in.color.a;
    } else {
        // Likely a font atlas stored in single channel: use mask as alpha and vertex color as rgb
        out_rgb = in.color.rgb;
        out_a = mask * in.color.a;
    }

    return vec4<f32>(out_rgb, out_a);
}

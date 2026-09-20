struct VertexInput {
  @location(0) position: vec3<f32>,   // local quad vertex position (-0.5..0.5)
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) replace_color: f32,
};

struct InstanceData {
  @location(2) position: vec3<f32>,   // sprite world position
    @location(3) rotation: f32,         // Z-axis rotation in radians
    @location(4) scale: vec3<f32>,      // scale along X, Y, Z
    @location(5) color: vec4<f32>,
    @location(6) uv_offset: vec2<f32>,
    @location(7) uv_size: vec2<f32>,
    @location(8) flip: u32,
};

struct Globals {
    view_proj: mat4x4<f32>,
    aspect: f32,
    _padding: vec3<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var t_diffuse: texture_2d<f32>;
@group(0) @binding(2) var s_sampler: sampler;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceData,
) -> VertexOutput {
    var out: VertexOutput;

    // === 1. Apply 2D rotation around the Z axis ===
    let cos_a = cos(instance.rotation);
    let sin_a = sin(instance.rotation);

    let rotated_x = vertex.position.x * cos_a - vertex.position.y * sin_a;
    let rotated_y = vertex.position.x * sin_a + vertex.position.y * cos_a;

    // === 2. Apply scale ===
    let scaled_x = rotated_x * instance.scale.x;
    let scaled_y = rotated_y * instance.scale.y;
    let scaled_z = vertex.position.z * instance.scale.z; // Usually 0.0 for sprites

    // === 3. Add the instance world position ===
    let world_x = scaled_x + instance.position.x;
    let world_y = scaled_y + instance.position.y;
    let world_z = scaled_z + instance.position.z;

    // === 4. Convert to clip space ===
    out.clip_position = globals.view_proj * vec4<f32>(world_x, world_y, world_z, 1.0);

    // Base UVs from the vertex (0..1)
    var uv = vertex.tex_coords;

    // Flipping
    if (instance.flip & 1u) != 0u { uv.x = 1.0 - uv.x; }
    if (instance.flip & 2u) != 0u { uv.y = 1.0 - uv.y; }

    // Key line: apply the UV rect
    let final_uv = instance.uv_offset + uv * instance.uv_size;

    out.tex_coords = final_uv;  // Pass to the fragment shader
    out.color = instance.color;
    out.replace_color = select(0.0, 1.0, (instance.flip & 4u) != 0u);

    // ... remaining position logic ...
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_sampler, in.tex_coords);
    let color = select(tex * in.color, vec4<f32>(in.color.rgb * tex.a, tex.a), in.replace_color != 0.0);
    if (color.a <= 0.001) {
        discard;
    }
    return color;
}

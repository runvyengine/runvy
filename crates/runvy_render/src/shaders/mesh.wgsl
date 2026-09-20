const MAX_POINT_LIGHTS: u32 = 16u;

struct PointLight {
    position_radius: vec4<f32>,
    color_intensity: vec4<f32>,
    params: vec4<f32>,
};

struct MeshUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    base_color: vec4<f32>,
    emission: vec4<f32>,
    directional_direction: vec4<f32>,
    directional_color_intensity: vec4<f32>,
    ambient_color_intensity: vec4<f32>,
    flags: vec4<u32>,
    point_lights: array<PointLight, 16>,
};

@group(0) @binding(0) var<uniform> globals: MeshUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) vertex_color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let world = globals.model * vec4<f32>(in.position, 1.0);
    let normal = normalize((globals.model * vec4<f32>(in.normal, 0.0)).xyz);

    var out: VertexOutput;
    out.position = globals.view_proj * world;
    out.uv = in.uv;
    out.normal = normal;
    out.world_pos = world.xyz;
    out.vertex_color = in.color;
    return out;
}

fn directional_light(normal: vec3<f32>) -> vec3<f32> {
    let direction = normalize(-globals.directional_direction.xyz);
    let ndotl = max(dot(normal, direction), 0.0);
    return globals.directional_color_intensity.rgb
        * globals.directional_color_intensity.a
        * ndotl;
}

fn point_light(light: PointLight, world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let to_light = light.position_radius.xyz - world_pos;
    let distance = length(to_light);
    if distance > light.position_radius.w {
        return vec3<f32>(0.0);
    }
    if distance <= 0.0001 {
        return light.color_intensity.rgb * light.color_intensity.a;
    }

    let light_dir = normalize(to_light);
    let ndotl = max(dot(normal, light_dir), 0.0);
    let attenuation = 1.0 / (1.0 + light.params.x * distance * distance);
    return light.color_intensity.rgb * light.color_intensity.a * ndotl * attenuation;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var base = globals.base_color;
    if globals.flags.y != 0u {
        base = base * in.vertex_color;
    }

    let ambient = globals.ambient_color_intensity.rgb * globals.ambient_color_intensity.a;
    var lighting = ambient;
    if globals.flags.x != 0u {
        if globals.flags.w != 0u {
            lighting = lighting + directional_light(normalize(in.normal));
        }
        let point_count = min(globals.flags.z, MAX_POINT_LIGHTS);
        for (var i = 0u; i < point_count; i = i + 1u) {
            lighting = lighting + point_light(globals.point_lights[i], in.world_pos, normalize(in.normal));
        }
    }

    let rgb = base.rgb * lighting + globals.emission.rgb;
    return vec4<f32>(rgb, base.a);
}

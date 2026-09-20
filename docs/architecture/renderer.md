<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Renderer Architecture

Runvy currently uses a small forward renderer. The runtime world owns objects and
components, builds a `RenderQueue` each frame, and the renderer consumes that
queue without keeping a separate editor/runtime scene copy.

## World Atmosphere

`WorldAtmosphere` is a global `World` resource, not an object component. It owns
the world background and the ambient term used by mesh lighting.

```rust
pub struct WorldAtmosphere {
    pub ambient_color: Vec3,
    pub ambient_intensity: f32,
    pub background_intensity: f32,
    pub background: BackgroundMode,
}
```

Supported background modes:

- `SolidColor`: fills the frame with one color.
- `VerticalGradient`: blends ground, horizon, and zenith colors in screen space.
- `Sky`: reserved for future skybox, skysphere, or HDR environment rendering.

Render pass order:

1. Background pass: fullscreen triangle, no depth, fills the color buffer.
2. Geometry pass: meshes, sprites, tiles, and debug geometry.
3. Lighting evaluation: currently forward in the mesh shader.
4. UI pass: editor/runtime UI over the scene.

The background shader receives a compact uniform block with mode, background
brightness, solid color, and gradient colors. `horizon_height` is packed into
`zenith_color.w`; `smoothness` is packed into `horizon_color.w`.

The vertical gradient is evaluated in world space. The background pass
reconstructs the view ray from the inverse view-projection matrix and uses the
ray's world `Y` component for ground/horizon/zenith blending. This keeps the
atmosphere aligned to world height instead of screen `uv.y`.

`ambient_intensity` affects indirect mesh lighting. `background_intensity`
affects only the visible background brightness.

`WorldAtmosphere` is serialized into world files with a `version` field on the
world asset. This keeps the scene format ready for future `.runvy3d` world-level
environment data.

## Lighting MVP

Lighting is intentionally implemented as a forward pass for now. This keeps the
pipeline simple, while preserving a clear upgrade path to a dedicated light
buffer or deferred renderer later.

Runtime light components:

- `DirectionalLight`: global sunlight-style light. It stores direction, color,
  and intensity.
- `PointLight`: local light. Its position is taken from the owning object's
  `Transform`; the component stores color, intensity, radius, and falloff.

Frame flow:

1. `World::render` scans runtime objects.
2. Light components are converted into render API light data.
3. Mesh render commands carry material data and per-vertex color.
4. The mesh shader evaluates simple Lambert lighting.

Scene fallback rule with atmosphere:

- If the world has no direct lights, meshes use ambient lighting from
  `WorldAtmosphere`.
- If the world has direct lights, final lighting is `ambient + direct_lights`.

The current shader supports one directional light and up to 16 point lights.
This cap is a renderer constant and can later move into renderer settings.

## Material Data

`MeshRenderer` exposes a minimal `Material` shape prepared for `.runvy3d`:

```rust
pub struct Material {
    pub base_color: [f32; 4],
    pub use_vertex_color: bool,
    pub emission: [f32; 3],
}
```

The legacy `MeshRenderer::color` field is still used as the effective base color
for compatibility with existing worlds and editor UI. Future material assets
should write into `Material` directly.

Shader-side color model:

```text
surface_color = base_color * vertex_color
lighting = atmosphere_ambient + direct_lights
final_color = surface_color * lighting + emission
```

`vertex_color` is only applied when `use_vertex_color` is enabled.

## Future Extension Points

- SkySphere / Skybox: implement `BackgroundMode::Sky` as a separate sky pass.
- HDR environment map: add an environment texture slot to `WorldAtmosphere`.
- Fog: add distance/height fog uniforms after geometry depth is available.
- Day/night cycle: animate `WorldAtmosphere` and light components from runtime
  systems.
- Exposure / tonemapping: add post-processing after geometry and before UI.
- Normal maps: extend material textures and fragment input with tangent-space
  normal sampling.
- Shadows: add shadow map passes per supported light type before the mesh pass.
- PBR: replace the Lambert fragment function with a BRDF and expand `Material`
  with metallic, roughness, normal, occlusion, and emissive texture slots.
- `.runvy3d`: import mesh vertex colors, material parameters, textures, and
  optional light nodes into the same runtime components and material structure.


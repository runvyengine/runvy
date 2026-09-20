<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
UI Feature Implementation Notes
================================

Technical details and rationale
--------------------------------

1) Renderer grouping and textures
- Textured UI vertices are grouped by a texture key (pointer as usize) so the renderer can create a single GpuTexture and bind group per texture and draw batches efficiently.
- A textures_cache (Arc<TextureAsset>) stores TextureAsset handles; a textures map holds Arc<GpuTexture> GPU resources created on demand via GpuTexture::from_asset.

2) UV handling
- Callers sometimes provide pixel-based UV (x,y,w,h in pixels). The renderer now normalizes these to [0..1] by dividing by the texture width/height when any uv component is > 1.0.
- For typical image assets the UV origin and V orientation are now preserved (no forced V flip), avoiding inverted images. Font atlases (GPU textures) are handled separately and use their own atlas texture coordinates.

3) Shader (ui.wgsl)
- The textured fragment shader inspects sampled alpha; if alpha > 0.001, the texture is treated as a regular RGBA image (rgb * vertex_color, alpha * vertex_alpha).
- If alpha is near zero, assume a grayscale atlas (font) and use the red channel as a mask for alpha while using vertex color for rgb. This prevents dark rims around glyphs.

4) UI layout
- ui_renderer::layout(viewport_size) implements a minimal layout: images get a default height (20% of viewport height) and width computed from aspect ratio; text width/height are estimated from font_size.
- The engine's world update obtains viewport_size from the active camera and calls layout automatically when canvas.dirty_layout is true.

5) Example adjustments
- examples/sandbox_ui now uses RunvyApp::run_default(world_rc) so app/default config is used (no manual RunvyWindowConfig in user main.rs).
- Example uses auto-layout anchors and positions rather than manually setting computed rects.

Known limitations and next steps
--------------------------------
- The layout implementation is a lightweight placeholder (heuristics). Replace with a full measure/arrange engine for robust UI (text wrapping, anchoring, stretch behaviors).
- Consider splitting font rendering and image rendering into separate pipelines if more control is needed (e.g., handling subpixel AA for fonts vs linear sampling for images).
- The shader heuristic (alpha check) is pragmatic but not perfect; explicit metadata for texture type (font atlas vs image) would be cleaner.

(This document captures implementation details for the current UI feature set. A more complete layout system or egui_dock integration may be explored separately.)


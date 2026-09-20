<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
UI Feature Changes
===================

Summary
-------
This document describes recent UI-related changes introduced to the codebase. The goal of the changes was to add runtime UI support for images and rects, improve UI layout ergonomics, and make the sandbox UI example show a tangible result.

Files changed (new behavior)
----------------------------
- crates/runvy_render/src/renderer.rs
- crates/runvy_render/src/shaders/ui.wgsl
- crates/runvy_core/src/components/ui/ui_renderer.rs
- examples/sandbox_ui/src/main.rs

Short description of changes
----------------------------
- Added support for textured UI quads (UiImage) and solid rectangles (UiRect) in the renderer.
- Grouped textured UI vertices by texture key to create per-texture bind groups and avoid binding mistakes.
- Implemented UV normalization: if UiImage uv values are pixel-based (>1.0), they are normalized by texture width/height automatically.
- Adjusted shader (ui.wgsl) to handle both regular RGBA textures and grayscale font atlases. If the sampled texture has meaningful alpha, it's treated as an RGBA image; otherwise the red channel is used as a mask (font atlas).
- Added a heuristic to avoid alpha-edge artifacts for text by choosing appropriate alpha source in shader.
- Implemented a simple auto-layout in ui_renderer::layout(viewport_size) so nodes obtain sizes and positions automatically (images sized by viewport percent, text sized by font_size heuristics).
- Updated the sandbox_ui example to rely on engine defaults (RunvyApp::run_default) and use automatic layout, simplifying the user's main.rs.

How to test
-----------
1. Build and run the sandbox example:
   cd examples/sandbox_ui
   cargo run

2. Expect to see text and the image (Charactert.png) visible. If not, set the test UiImage's uv to [0,0,1,1] and tint to [1,1,1,1] to rule out UV issues.

Reverting these changes
-----------------------
If you need to revert the functional changes, revert the files listed above. If using git:

    git checkout -- crates/runvy_render/src/renderer.rs \
                   crates/runvy_render/src/shaders/ui.wgsl \
                   crates/runvy_core/src/components/ui/ui_renderer.rs \
                   examples/sandbox_ui/src/main.rs

Alternatively, review the docs in this folder for details before rolling back.


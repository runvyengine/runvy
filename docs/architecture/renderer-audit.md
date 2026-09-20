<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Renderer Audit Notes

Current renderer risks and optimization targets for the next pass:

- ~~Per-mesh uniform and GPU buffers are created every frame.~~ **Fixed in v0.6:** mesh GPU cache + persistent uniform buffers.
- ~~Tilemap rendering emits one draw command per painted tile.~~ **Fixed in v0.6:** texture-batched TileBatch with instancing.
- ~~Background pass creates a uniform buffer every frame.~~ **Fixed in v0.6:** persistent `background_uniform_buffer` with `write_buffer`.
- Mesh lighting is forward-only and sends the full point light array per mesh. This is fine for the MVP, but clustered/tiled light assignment or per-frame light buffers will scale better.
- Mesh lighting is forward-only and sends the full point light array per mesh. This is fine for the MVP, but clustered/tiled light assignment or per-frame light buffers will scale better.
- Render sorting currently mixes 2D and 3D with simple order/depth rules. This is practical, but transparent 3D, particle systems, and UI-world overlays will need explicit render layers.
- World transform matrices are computed recursively during render. Cache dirty world matrices in `World` once hierarchy editing and runtime parenting become more common.
- Sprite and tile paths are still unlit. Decide deliberately whether 2D should stay unlit by default or receive a separate 2D lighting model.
- Several renderer match arms intentionally ignore UI/debug fields and currently produce warnings. Clean these up before enforcing warning-free CI.


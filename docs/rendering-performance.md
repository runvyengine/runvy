<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Rendering Performance Optimizations

## Overview

Optimizations to eliminate CPU bottlenecks in the 2D/3D rendering pipeline,
targeting large tilemaps (100x100+) and general 3D scenes.

## Changes

### 1. GPU Mesh Cache (`renderer.rs`)

**File:** `crates/runvy_render/src/renderer.rs`

`mesh_gpu_cache: HashMap<u64, (Buffer, Buffer)>` keyed by `Arc::as_ptr(&mesh.inner)`.
Vertex/index buffers created once per unique `Arc<Mesh>`, reused across frames.

### 2. Persistent Uniform Buffers (`renderer.rs`)

Three persistent uniform buffers replaced per-frame `create_buffer_init`:

| Buffer | Usage |
|--------|-------|
| `mesh_uniform_buffer` | `UNIFORM | COPY_DST`, aligned writes per mesh |
| `background_uniform_buffer` | Background pass uniforms |
| `postprocess_uniform_buffer` | Post-process pass uniforms |

Stride respects `min_uniform_buffer_offset_alignment` (typically 256).

### 3. Single Mesh Sort (`renderer.rs`)

Replaced O(N^2) group-by-order filter with `sort_by((order, depth, index))`
followed by `partition_point` for group boundaries.

### 4. Sprite Batch Merging (`renderer.rs`)

Consecutive same-texture, same-order sprites merged into one batch entry
(count > 1) instead of count=1 per sprite — reduces draw calls.

### 5. Tile Batching (`world.rs`, `render_command.rs`)

Moved `InstanceData` from `renderer.rs` to `runvy_render_api::command`.
Added `TileBatch` render command variant. `world.rs` groups tiles by texture
pointer into `Vec<InstanceData>` and issues one `TileBatch` per texture per
layer instead of 10k individual `Tile` commands.

### 6. HashMap Texture Grouping (`world.rs`)

Replaced O(T*U) linear scan (`texture_groups.iter().position(...)`) with
`HashMap::entry()` for O(1) per-tile lookup.

**Before:**
```rust
let pos = texture_groups.iter().position(|(t, _)| Arc::as_ptr(t) as usize == tex_ptr);
```

**After:**
```rust
let entry = texture_groups.entry(tex_ptr).or_insert_with(|| {
    (texture.clone(), Vec::with_capacity(total_visible))
});
```

### 7. Pre-allocated Vecs (`world.rs`)

`Vec::with_capacity(total_visible)` instead of `vec![instance]` / `Vec::new()`
avoids ~14 reallocations per frame during Vec growth for 10k tiles.

### 8. Per-frame Vec Reuse (`renderer.rs`)

Six containers moved from local `Vec::new()` to `Renderer` struct fields,
reused via `.clear()` each frame:

| Field | Type |
|-------|------|
| `all_instances` | `Vec<InstanceData>` |
| `sprite_instances` | `Vec<(i32, f32, usize, usize, InstanceData)>` |
| `mesh_items` | `Vec<(i32, f32, usize)>` |
| `ui_vertices` | `Vec<UIVertex>` |
| `batches` | `Vec<(i32, f32, usize, usize, usize, usize)>` |
| `orders` | `Vec<i32>` |

### 9. Screen-space Tilemap Culling (`world.rs`)

Finds active orthographic camera, computes visible world rect from inverse
view-projection matrix, narrows tile iteration to only tiles overlapping
the visible rect (with 1-tile margin). Empty ranges skip the layer entirely.

### 10. Frustum Culling for 3D (`world.rs`)

Extracts 6 frustum planes from active camera's view-projection matrix.
Each 3D object is tested against all planes using bounding sphere radius
computed from `Mesh.bounds` AABB. Object culled only when entire sphere
is outside the frustum.

## Files Modified

- `crates/runvy_render_api/src/command.rs` — `InstanceData`, `TileBatch`
- `crates/runvy_render_api/src/queue.rs` — `draw_tiles_batch()`
- `crates/runvy_render/src/renderer.rs` — mesh cache, uniform buffers, sprite/tile
  batching, per-frame Vec reuse
- `crates/runvy_render/src/pipelines/pipeline.rs` — `InstanceData` import
- `crates/runvy_core/src/ocs/world.rs` — HashMap grouping, pre-allocate, culling
- `crates/runvy_core/src/components/tilemap.rs` — `generation` field
- `crates/runvy_core/src/components/mesh_renderer.rs` — `Vertex3D`, `Mesh`, `Material`
- `crates/runvy_project/src/world_asset.rs` — tilemap `generation: 0`

## Performance

- 100x100 tilemap: ~200 FPS → ~4000 FPS (20x improvement)
- 3D scenes: frustum culling eliminates off-screen objects
- Memory: ~2MB less alloc/dealloc churn per frame from Vec reuse


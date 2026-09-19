# Tilemap System

Tilemaps are runtime data components used to build 2D levels. Tiles are painted programmatically through the `Tilemap` API.

## Runtime Composition

```rust
use runa_engine::asset::load_image;
use runa_engine::ecs::World;
use runa_engine::prelude::{Transform, Tilemap, TilemapLayer, TilemapRenderer};
use runa_engine::core::glam::USizeVec2;

fn spawn_level(world: &mut World) -> u64 {
    let mut tilemap = Tilemap::builder(10, 10)
        .centered()
        .tile_size(USizeVec2::new(32, 32))
        .pixels_per_unit(32.0)
        .atlas(
            load_image!("assets/tiles/atlas.png"),
            Some("assets/tiles/atlas.png".to_string()),
            8,
            8,
        )
        .layer(TilemapLayer::new("Ground".into(), 10, 10))
        .build();

    // paint tile at layer 0, world cell (0, 0), atlas frame 0
    tilemap.paint_tile(0, 0, 0, 0);

    world.spawn((Transform::default(), tilemap, TilemapRenderer::new()))
}
```

`TilemapRenderer` is intentionally separate from `Tilemap`: `Tilemap` stores level data, while `TilemapRenderer` makes it visible.

## Tile Ids

Cells are stored as `TileId` (a `u32`). The value `EMPTY_TILE` (`0`) means the cell is empty. The first atlas frame maps to id `1` (see `id_for_frame` / `frame_for_id`), so `EMPTY_TILE` never collides with a real tile.

## Atlas And Scale

The atlas is a regular texture split by `columns` and `rows`. `pixels_per_unit` controls how large each tile is in world space.

- `32x32` tiles at `32 PPU` become `1.0 x 1.0` world units.
- `16x16` tiles at `16 PPU` become `1.0 x 1.0` world units.
- `32x32` tiles at `16 PPU` become `2.0 x 2.0` world units.

## Editor Painting

In the inspector, assign an atlas and choose the atlas slicing mode through columns/rows or tile pixel size. Then use:

- `Open Palette` to show the separate Tile Palette window.
- `Paint` mode to paint the selected 16x16 preview tile under the viewport cursor.
- `Erase` mode to clear the tile under the viewport cursor.
- `None` mode to disable tile editing and return to normal object selection/gizmo interaction.

Painting writes directly into the selected object's runtime `Tilemap` component. The editor does not keep a separate tilemap copy.

## Notes

- Use layers for visual organization; layers render back-to-front via `layer.order`.
- Keep tile painting on objects that have both `Tilemap` and `TilemapRenderer`.
- For large maps, prefer atlas-based tiles over separate textures per cell.

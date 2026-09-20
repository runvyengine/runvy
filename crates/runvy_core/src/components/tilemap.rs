use glam::{IVec2, USizeVec2, Vec2, Vec3};
use runvy_asset::Handle;
use runvy_asset::TextureAsset;
use runvy_macros::Scriptable;
use std::ops::{Index, IndexMut};
use std::sync::Arc;

/// Identifies a tile inside a [`TilemapLayer`].
///
/// Values are dense atlas-frame ids plus `1` (see
/// [`Tilemap::id_for_frame`]), so `0` is always reserved as the empty tile.
pub type TileId = u32;

/// Sentinel for an empty cell: holds no tile and renders nothing.
pub const EMPTY_TILE: TileId = 0;

/// Rectangle used for UV coordinates inside an atlas.
#[derive(Clone, Copy, Debug, PartialEq, Scriptable)]
#[script(crate = "::runvy_script_api", not_addable, builtin)]
pub struct UvRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UvRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// A single tilemap layer: a dense, rectangular grid of [`TileId`]s.
#[derive(Clone, Scriptable)]
#[script(crate = "::runvy_script_api", not_addable, builtin)]
pub struct TilemapLayer {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// `width * height` elements, row-major. `EMPTY_TILE` (0) is empty.
    pub tiles: Vec<TileId>,
    pub visible: bool,
    pub opacity: f32,
    /// Draw order relative to other layers and world objects.
    pub order: i32,
}

impl TilemapLayer {
    pub fn new(name: String, width: u32, height: u32) -> Self {
        Self {
            name,
            width,
            height,
            tiles: vec![EMPTY_TILE; (width * height) as usize],
            visible: true,
            opacity: 1.0,
            order: 0,
        }
    }

    /// Returns the cell content, or [`EMPTY_TILE`] when out of bounds.
    pub fn get(&self, x: u32, y: u32) -> TileId {
        if x >= self.width || y >= self.height {
            return EMPTY_TILE;
        }
        self.tiles[(y * self.width + x) as usize]
    }

    /// Sets a cell. Returns `false` when the cell is out of bounds.
    pub fn set(&mut self, x: u32, y: u32, tile: TileId) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.tiles[(y * self.width + x) as usize] = tile;
        true
    }

    /// Fills a rectangular region. Out-of-bounds cells are ignored.
    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, tile: TileId) {
        for cy in y..y.saturating_add(height) {
            for cx in x..x.saturating_add(width) {
                self.set(cx, cy, tile);
            }
        }
    }

    /// Clears every cell of this layer.
    pub fn clear(&mut self) {
        self.tiles.fill(EMPTY_TILE);
    }

    /// Iterates every cell in row-major order: `(x, y, tile_id)`.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, TileId)> + '_ {
        (0..self.height).flat_map(move |y| (0..self.width).map(move |x| (x, y, self.get(x, y))))
    }
}

impl Index<(u32, u32)> for TilemapLayer {
    type Output = TileId;

    fn index(&self, (x, y): (u32, u32)) -> &TileId {
        debug_assert!(x < self.width && y < self.height, "tile out of bounds");
        &self.tiles[(y * self.width + x) as usize]
    }
}

impl IndexMut<(u32, u32)> for TilemapLayer {
    fn index_mut(&mut self, (x, y): (u32, u32)) -> &mut TileId {
        debug_assert!(x < self.width && y < self.height, "tile out of bounds");
        &mut self.tiles[(y * self.width + x) as usize]
    }
}

/// Fluent builder for [`Tilemap`].
#[derive(Clone)]
pub struct TilemapBuilder {
    width: u32,
    height: u32,
    tile_size: USizeVec2,
    offset: IVec2,
    pixels_per_unit: f32,
    layers: Vec<TilemapLayer>,
    atlas: Option<TilemapAtlas>,
    selected_tile: u32,
}

impl TilemapBuilder {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            tile_size: USizeVec2::new(16, 16),
            offset: IVec2::ZERO,
            pixels_per_unit: 16.0,
            layers: Vec::new(),
            atlas: None,
            selected_tile: 0,
        }
    }

    /// Centers the map around the world origin.
    pub fn centered(mut self) -> Self {
        self.offset = IVec2::new(-(self.width as i32) / 2, -(self.height as i32) / 2);
        self
    }

    pub fn offset(mut self, offset: IVec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn tile_size(mut self, tile_size: USizeVec2) -> Self {
        self.tile_size = tile_size;
        self
    }

    pub fn pixels_per_unit(mut self, pixels_per_unit: f32) -> Self {
        self.pixels_per_unit = pixels_per_unit.max(f32::EPSILON);
        self
    }

    pub fn atlas(
        mut self,
        texture: Handle<TextureAsset>,
        texture_path: Option<String>,
        columns: u32,
        rows: u32,
    ) -> Self {
        self.atlas = Some(TilemapAtlas {
            texture: texture.inner,
            texture_path,
            columns: columns.max(1),
            rows: rows.max(1),
        });
        self
    }

    pub fn layer(mut self, layer: TilemapLayer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn add_layer(&mut self, layer: TilemapLayer) {
        self.layers.push(layer);
    }

    pub fn build(self) -> Tilemap {
        Tilemap {
            width: self.width,
            height: self.height,
            tile_size: self.tile_size,
            offset: self.offset,
            layers: self.layers,
            atlas: self.atlas,
            selected_tile: self.selected_tile,
            pixels_per_unit: self.pixels_per_unit,
            generation: 0,
        }
    }
}

/// Tilemap data component.
#[derive(Clone, Scriptable)]
#[script(crate = "::runvy_script_api", not_addable, builtin)]
pub struct Tilemap {
    /// Map size in tiles.
    pub width: u32,
    pub height: u32,

    /// Tile size in pixels. `tile_size / pixels_per_unit` is the world size.
    #[script(skip)]
    pub tile_size: USizeVec2,
    #[script(skip)]
    pub offset: IVec2,

    /// Layers ordered from back to front. Painting expands the map
    /// automatically, growing `width`/`height`/`offset` as needed.
    pub layers: Vec<TilemapLayer>,

    #[script(skip)]
    pub atlas: Option<TilemapAtlas>,
    pub selected_tile: u32,
    pub pixels_per_unit: f32,

    /// Incremented on every tile mutation for dirty tracking.
    #[script(skip)]
    pub generation: u64,
}

impl Tilemap {
    /// Creates a map whose corner sits at the world origin.
    pub fn new(width: u32, height: u32, tile_size: USizeVec2) -> Self {
        Self::builder(width, height).tile_size(tile_size).build()
    }

    /// Creates a map centered at the world origin.
    pub fn centered(width: u32, height: u32, tile_size: USizeVec2) -> Self {
        Self::builder(width, height)
            .centered()
            .tile_size(tile_size)
            .build()
    }

    /// Starts a fluent [`TilemapBuilder`].
    pub fn builder(width: u32, height: u32) -> TilemapBuilder {
        TilemapBuilder::new(width, height)
    }

    pub fn add_layer(&mut self, layer: TilemapLayer) {
        self.layers.push(layer);
    }

    pub fn layer(&self, index: usize) -> Option<&TilemapLayer> {
        self.layers.get(index)
    }

    pub fn layer_mut(&mut self, index: usize) -> Option<&mut TilemapLayer> {
        self.layers.get_mut(index)
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Replaces the atlas texture and grid. `selected_tile` is clamped.
    pub fn set_atlas(
        &mut self,
        texture: Option<Handle<TextureAsset>>,
        texture_path: Option<String>,
        columns: u32,
        rows: u32,
    ) {
        self.atlas = texture.map(|texture| TilemapAtlas {
            texture: texture.inner,
            texture_path,
            columns: columns.max(1),
            rows: rows.max(1),
        });
        self.selected_tile = self
            .selected_tile
            .min(self.atlas_frame_count().saturating_sub(1));
    }

    pub fn atlas_frame_count(&self) -> u32 {
        self.atlas
            .as_ref()
            .map(TilemapAtlas::frame_count)
            .unwrap_or(1)
    }

    /// UV rect of an atlas frame (clamped to the atlas grid), if one is set.
    pub fn atlas_uv(&self, frame: u32) -> Option<UvRect> {
        self.atlas
            .as_ref()
            .map(|atlas| atlas.uv_rect_for_frame(frame))
    }

    /// Converts an atlas frame index to a storable [`TileId`]. Frame 0 maps
    /// to id 1 so the empty sentinel `0` never collides with a real tile.
    pub fn id_for_frame(&self, frame: u32) -> TileId {
        frame.saturating_add(1)
    }

    /// Converts a stored [`TileId`] back to an atlas frame index.
    pub fn frame_for_id(&self, tile: TileId) -> u32 {
        tile.saturating_sub(1)
    }

    /// Paints an atlas frame at world tile coords, expanding the map if the
    /// cell lies outside its current bounds. Returns `false` when there is no
    /// atlas or the layer does not exist.
    pub fn paint_tile(&mut self, layer_index: usize, tile_x: i32, tile_y: i32, frame: u32) -> bool {
        let frame_count = self.atlas_frame_count();
        if layer_index >= self.layers.len() || frame >= frame_count {
            return false;
        }
        self.ensure_tile_position(tile_x, tile_y);
        let array_x = (tile_x - self.offset.x) as u32;
        let array_y = (tile_y - self.offset.y) as u32;
        self.layers[layer_index].set(array_x, array_y, frame.saturating_add(1));
        self.generation += 1;
        true
    }

    /// Clears a cell at world tile coords. Cells outside the map are ignored.
    pub fn erase_tile(&mut self, layer_index: usize, tile_x: i32, tile_y: i32) -> bool {
        if !self.contains(tile_x, tile_y) {
            return false;
        }
        let Some(layer) = self.layers.get_mut(layer_index) else {
            return false;
        };
        let array_x = (tile_x - self.offset.x) as u32;
        let array_y = (tile_y - self.offset.y) as u32;
        layer.set(array_x, array_y, EMPTY_TILE);
        self.generation += 1;
        true
    }

    /// Sets an arbitrary stored [`TileId`] at world tile coords.
    pub fn set_tile(
        &mut self,
        layer_index: usize,
        world_x: i32,
        world_y: i32,
        tile: TileId,
    ) -> bool {
        if !self.contains(world_x, world_y) {
            return false;
        }
        let Some(layer) = self.layers.get_mut(layer_index) else {
            return false;
        };
        let array_x = (world_x - self.offset.x) as u32;
        let array_y = (world_y - self.offset.y) as u32;
        layer.set(array_x, array_y, tile);
        self.generation += 1;
        true
    }

    /// Reads a cell content at world tile coords, or [`EMPTY_TILE`] when
    /// outside the map or in a missing layer.
    pub fn get_tile(&self, layer_index: usize, world_x: i32, world_y: i32) -> TileId {
        if !self.contains(world_x, world_y) {
            return EMPTY_TILE;
        }
        let array_x = (world_x - self.offset.x) as u32;
        let array_y = (world_y - self.offset.y) as u32;
        self.layers
            .get(layer_index)
            .map(|layer| layer.get(array_x, array_y))
            .unwrap_or(EMPTY_TILE)
    }

    /// Paints a filled rectangle of atlas frames (world tile coords).
    pub fn fill_rect(
        &mut self,
        layer_index: usize,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        frame: u32,
    ) {
        for cy in y..y.saturating_add(height as i32) {
            for cx in x..x.saturating_add(width as i32) {
                self.paint_tile(layer_index, cx, cy, frame);
            }
        }
    }

    /// Paints a Bresenham line between two world tile coords.
    pub fn line(&mut self, layer_index: usize, x0: i32, y0: i32, x1: i32, y1: i32, frame: u32) {
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.paint_tile(layer_index, x, y, frame);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Paints an 8-way symmetric circle outline around `(cx, cy)`.
    pub fn circle(&mut self, layer_index: usize, cx: i32, cy: i32, radius: i32, frame: u32) {
        let mut x = radius.max(0);
        let mut y = 0;
        let mut err = 1 - x;
        while x >= y {
            let points = [
                (x, y),
                (y, x),
                (-x, -y),
                (-y, -x),
                (x, -y),
                (y, -x),
                (-x, y),
                (-y, x),
            ];
            for (dx, dy) in points {
                self.paint_tile(layer_index, cx + dx, cy + dy, frame);
            }
            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
    }

    /// Calls `f(x, y, &mut tile_id)` for every cell of a layer.
    pub fn for_each(&mut self, layer_index: usize, mut f: impl FnMut(u32, u32, &mut TileId)) {
        let Some(layer) = self.layers.get_mut(layer_index) else {
            return;
        };
        for y in 0..layer.height {
            for x in 0..layer.width {
                let index = (y * layer.width + x) as usize;
                f(x, y, &mut layer.tiles[index]);
            }
        }
        self.generation += 1;
    }

    /// Iterates every non-empty tile of visible layers in world tile coords:
    /// `(x, y, tile_id)`.
    pub fn iter(&self) -> impl Iterator<Item = (i32, i32, TileId)> + '_ {
        let offset = self.offset;
        self.layers
            .iter()
            .filter(|layer| layer.visible)
            .flat_map(move |layer| {
                layer.iter().filter_map(move |(x, y, tile)| {
                    (tile != EMPTY_TILE).then_some((x as i32 + offset.x, y as i32 + offset.y, tile))
                })
            })
    }

    /// Whether a world tile coord lies inside the map bounds.
    pub fn contains(&self, tile_x: i32, tile_y: i32) -> bool {
        tile_x >= self.offset.x
            && tile_y >= self.offset.y
            && tile_x < self.offset.x + self.width as i32
            && tile_y < self.offset.y + self.height as i32
    }

    /// Inclusive map bounds as `(min_x, min_y, max_x, max_y)` in tile coords.
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        (
            self.offset.x,
            self.offset.y,
            self.offset.x + self.width as i32 - 1,
            self.offset.y + self.height as i32 - 1,
        )
    }

    /// Converts world coordinates to tile coordinates.
    pub fn world_to_tile(&self, world_pos: Vec3) -> (i32, i32) {
        let tile_size = self.world_tile_size();
        (
            (world_pos.x / tile_size.x).floor() as i32,
            (world_pos.y / tile_size.y).floor() as i32,
        )
    }

    /// Converts tile coordinates to the tile's minimum-corner world position.
    pub fn tile_to_world(&self, tile_x: i32, tile_y: i32) -> Vec3 {
        let tile_size = self.world_tile_size();
        Vec3::new(
            tile_x as f32 * tile_size.x,
            tile_y as f32 * tile_size.y,
            0.0,
        )
    }

    /// World position of a tile's center.
    pub fn tile_center_world(&self, tile_x: i32, tile_y: i32) -> Vec3 {
        let tile_size = self.world_tile_size();
        Vec3::new(
            (tile_x as f32 + 0.5) * tile_size.x,
            (tile_y as f32 + 0.5) * tile_size.y,
            0.0,
        )
    }

    /// World-space size of a single tile.
    pub fn world_tile_size(&self) -> Vec2 {
        let ppu = self.pixels_per_unit.max(f32::EPSILON);
        Vec2::new(self.tile_size.x as f32 / ppu, self.tile_size.y as f32 / ppu)
    }

    /// Grows the map so `(tile_x, tile_y)` fits inside, keeping existing tiles.
    fn ensure_tile_position(&mut self, tile_x: i32, tile_y: i32) {
        let min_x = self.offset.x.min(tile_x);
        let min_y = self.offset.y.min(tile_y);
        let max_x = (self.offset.x + self.width as i32 - 1).max(tile_x);
        let max_y = (self.offset.y + self.height as i32 - 1).max(tile_y);
        let new_width = (max_x - min_x + 1).max(1) as u32;
        let new_height = (max_y - min_y + 1).max(1) as u32;
        if new_width == self.width
            && new_height == self.height
            && min_x == self.offset.x
            && min_y == self.offset.y
        {
            return;
        }

        let old_offset = self.offset;
        let old_width = self.width;
        self.offset = IVec2::new(min_x, min_y);
        self.width = new_width;
        self.height = new_height;
        for layer in &mut self.layers {
            let mut resized = vec![EMPTY_TILE; (new_width * new_height) as usize];
            for y in 0..layer.height {
                for x in 0..layer.width {
                    let old_index = (y * old_width + x) as usize;
                    let new_x = old_offset.x + x as i32 - min_x;
                    let new_y = old_offset.y + y as i32 - min_y;
                    let new_index = (new_y as u32 * new_width + new_x as u32) as usize;
                    if let Some(target) = resized.get_mut(new_index) {
                        *target = layer.tiles[old_index];
                    }
                }
            }
            layer.width = new_width;
            layer.height = new_height;
            layer.tiles = resized;
        }
        self.generation += 1;
    }
}

#[derive(Clone)]
pub struct TilemapAtlas {
    pub texture: Arc<TextureAsset>,
    pub texture_path: Option<String>,
    pub columns: u32,
    pub rows: u32,
}

impl TilemapAtlas {
    pub fn frame_count(&self) -> u32 {
        self.columns.saturating_mul(self.rows).max(1)
    }

    pub fn uv_rect_for_frame(&self, frame: u32) -> UvRect {
        let columns = self.columns.max(1);
        let rows = self.rows.max(1);
        let frame = frame.min(self.frame_count().saturating_sub(1));
        let col = frame % columns;
        let row = frame / columns;
        let width = 1.0 / columns as f32;
        let height = 1.0 / rows as f32;
        UvRect::new(col as f32 * width, row as f32 * height, width, height)
    }

    pub fn tile_index_for_uv(&self, uv: UvRect) -> Option<u32> {
        let col = (uv.x * self.columns as f32).round() as u32;
        let row = (uv.y * self.rows as f32).round() as u32;
        let index = row.saturating_mul(self.columns).saturating_add(col);
        (index < self.frame_count()).then_some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(w: u32, h: u32) -> Tilemap {
        Tilemap::new(w, h, USizeVec2::new(16, 16))
    }

    fn layer(w: u32, h: u32) -> TilemapLayer {
        TilemapLayer::new("base".into(), w, h)
    }

    #[test]
    fn id_frame_roundtrip() {
        let tm = map(2, 2);
        assert_eq!(tm.id_for_frame(0), 1);
        assert_eq!(tm.id_for_frame(7), 8);
        assert_eq!(tm.frame_for_id(0), 0);
        assert_eq!(tm.frame_for_id(tm.id_for_frame(3)), 3);
    }

    #[test]
    fn layer_get_set_index() {
        let mut l = layer(2, 2);
        assert_eq!(l.get(0, 0), EMPTY_TILE);
        assert_eq!(l.get(5, 5), EMPTY_TILE);
        assert!(l.set(1, 1, 42));
        assert!(!l.set(9, 9, 42));
        l[(0, 1)] = 7;
        assert_eq!(l.get(1, 1), 42);
        assert_eq!(l.get(0, 1), 7);
        assert_eq!(l[(1, 1)], 42);
        l.clear();
        assert_eq!(l.get(1, 1), EMPTY_TILE);
    }

    #[test]
    fn set_get_tile_bounds() {
        let mut tm = map(4, 4);
        tm.add_layer(layer(4, 4));
        assert!(tm.set_tile(0, 2, 3, 9));
        assert!(!tm.set_tile(0, 9, 0, 9));
        assert_eq!(tm.get_tile(0, 2, 3), 9);
        assert_eq!(tm.get_tile(0, 2, 4), EMPTY_TILE);
        assert_eq!(tm.get_tile(7, 0, 0), EMPTY_TILE);
        assert_eq!(tm.get_tile(3, -1, -1), EMPTY_TILE);
    }

    #[test]
    fn paint_erase() {
        let mut tm = map(4, 4);
        tm.add_layer(layer(4, 4));
        assert!(tm.paint_tile(0, 1, 2, 0));
        assert_eq!(tm.get_tile(0, 1, 2), 1);
        assert!(tm.erase_tile(0, 1, 2));
        assert_eq!(tm.get_tile(0, 1, 2), EMPTY_TILE);
        assert!(!tm.erase_tile(0, 100, 100));
    }

    #[test]
    fn paint_expands_negative_cells() {
        let mut tm = map(2, 2); // offset (0,0), cells (0,0)-(1,1)
        tm.add_layer(layer(2, 2));
        tm.paint_tile(0, -3, 4, 0);
        let (min_x, min_y, max_x, max_y) = tm.bounds();
        assert_eq!((min_x, min_y), (-3, 0));
        assert_eq!((max_x, max_y), (1, 4));
        assert_eq!(tm.get_tile(0, -3, 4), 1);
        assert_eq!(tm.get_tile(0, 0, 0), EMPTY_TILE);
        tm.paint_tile(0, 1, 1, 0);
        assert_eq!(tm.get_tile(0, 1, 1), 1);
    }

    #[test]
    fn fill_rect_line_circle() {
        let mut tm = map(8, 8);
        tm.add_layer(layer(8, 8));
        tm.fill_rect(0, 1, 1, 3, 2, 0);
        assert_eq!(tm.get_tile(0, 1, 1), 1);
        assert_eq!(tm.get_tile(0, 3, 2), 1);
        assert_eq!(tm.get_tile(0, 4, 2), EMPTY_TILE);

        tm.line(0, 0, 0, 2, 2, 0);
        assert_eq!(tm.get_tile(0, 0, 0), 1);
        assert_eq!(tm.get_tile(0, 1, 1), 1);
        assert_eq!(tm.get_tile(0, 2, 2), 1);
        assert_eq!(tm.get_tile(0, 0, 1), EMPTY_TILE);

        tm.circle(0, 4, 4, 1, 0);
        assert_eq!(tm.get_tile(0, 4, 5), 1);
        assert_eq!(tm.get_tile(0, 3, 4), 1);
        assert_eq!(tm.get_tile(0, 3, 3), EMPTY_TILE);
    }

    #[test]
    fn for_each_mutates_layer() {
        let mut tm = map(2, 2); // offset 0, so cell == world
        tm.add_layer(layer(2, 2));
        tm.for_each(0, |x, _y, tile| *tile = x + 1);
        assert_eq!(tm.get_tile(0, 0, 0), 1);
        assert_eq!(tm.get_tile(0, 1, 0), 2);
        assert_eq!(tm.get_tile(0, 0, 1), 1);
        assert_eq!(tm.get_tile(0, 1, 1), 2);
    }

    #[test]
    fn iter_yields_non_empty_visible_cells() {
        let mut tm = map(3, 3);
        tm.add_layer(layer(3, 3));
        tm.add_layer(layer(3, 3));
        tm.paint_tile(0, 0, 0, 0);
        tm.paint_tile(0, 2, 1, 0);
        tm.paint_tile(1, 1, 1, 0);
        let cells: Vec<(i32, i32, TileId)> = tm.iter().collect();
        assert_eq!(cells.len(), 3);
        assert!(cells.contains(&(0, 0, 1)));
        assert!(cells.contains(&(2, 1, 1)));
        assert!(cells.contains(&(1, 1, 1)));

        tm.layers[0].visible = false;
        let cells: Vec<(i32, i32, TileId)> = tm.iter().collect();
        assert_eq!(cells, vec![(1, 1, 1)]);
    }

    #[test]
    fn coordinate_conversion_roundtrip() {
        let tm = map(4, 4); // ppu default 16, tile 16px -> 1 world unit
        let (tx, ty) = tm.world_to_tile(Vec3::new(2.4, 3.9, 0.0));
        assert_eq!((tx, ty), (2, 3));
        assert_eq!(tm.tile_to_world(2, 3), Vec3::new(2.0, 3.0, 0.0));
        assert_eq!(tm.tile_center_world(2, 3), Vec3::new(2.5, 3.5, 0.0));
        assert_eq!(tm.world_tile_size(), Vec2::new(1.0, 1.0));
    }
}

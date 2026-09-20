use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};

use crate::resources::texture::GpuTexture;
use runvy_asset::TextureAsset;
use runvy_render_api::FontId;
use rusttype::{point, Font, Scale};
use wgpu::{Device, Queue};

/// UV coordinates for a character in the atlas
#[derive(Clone, Copy, Debug)]
pub struct CharUV {
    pub u: f32,
    pub v: f32,
    pub u_width: f32,
    pub v_height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphInfo {
    pub uv: CharUV,
    pub advance: f32,
}

pub struct FontManager {
    fonts: Vec<FontData>,
}

struct FontData {
    atlas_texture: Arc<GpuTexture>,
    glyphs: HashMap<char, GlyphInfo>,
    cell_width: f32,
    cell_height: f32,
    line_height: f32,
    ascent: f32,
    descent: f32,
    base_font_size: f32,
}

impl FontManager {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        let mut manager = Self { fonts: Vec::new() };

        // Always load the built-in pixel font as default
        let default = manager.load_builtin_pixel_font(device, queue);
        manager.fonts.push(default);

        // Try to load a system TTF for smoother text
        if let Some(bytes) = Self::load_default_system_font_bytes() {
            if let Some(ttf_font) = Self::new_ttf(device, queue, bytes, 32.0) {
                manager.fonts.push(ttf_font);
            }
        }

        manager
    }

    /// Load a TTF font from raw bytes, returns its FontId
    pub fn load_ttf(
        &mut self,
        device: &Device,
        queue: &Queue,
        font_bytes: Vec<u8>,
        font_size: f32,
    ) -> Option<FontId> {
        let data = Self::new_ttf(device, queue, font_bytes, font_size)?;
        let id = FontId(self.fonts.len());
        self.fonts.push(data);
        Some(id)
    }

    fn new_ttf(
        device: &Device,
        queue: &Queue,
        font_bytes: Vec<u8>,
        font_size: f32,
    ) -> Option<FontData> {
        let font = Font::try_from_vec(font_bytes)?;
        let scale = Scale::uniform(font_size);
        let v_metrics = font.v_metrics(scale);
        let ascent = v_metrics.ascent;
        let descent = v_metrics.descent.abs();
        let line_height = (ascent + descent).ceil() as u32;
        let chars: Vec<char> = (32u8..127u8).map(|c| c as char).collect();

        let mut max_advance: f32 = 0.0;
        for ch in chars.iter() {
            let glyph = font.glyph(*ch).scaled(scale);
            max_advance = max_advance.max(glyph.h_metrics().advance_width);
        }

        let padding = 2usize;
        let cell_width = max_advance.ceil() as usize + padding * 2;
        let cell_height = line_height as usize + padding * 2;
        let cols = 16usize;
        let rows = chars.len().div_ceil(cols);
        let atlas_width = cols * cell_width;
        let atlas_height = rows * cell_height;

        let pixel_count = atlas_width
            .checked_mul(atlas_height)
            .and_then(|count| count.checked_mul(4))
            .expect("Font atlas size overflowed");
        let mut pixels = vec![0u8; pixel_count];
        let mut glyphs = HashMap::new();

        for (index, ch) in chars.iter().enumerate() {
            let col = index % cols;
            let row = index / cols;
            let cell_x = col * cell_width;
            let cell_y = row * cell_height;

            let glyph = font.glyph(*ch).scaled(scale).positioned(point(0.0, ascent));
            let advance = glyph.unpositioned().h_metrics().advance_width;

            let uv = if let Some(bb) = glyph.pixel_bounding_box() {
                let atlas_x = cell_x as i32 + padding as i32;
                let atlas_y = cell_y as i32 + padding as i32;

                glyph.draw(|x, y, v| {
                    let px_i32 = atlas_x + x as i32;
                    let py_i32 = atlas_y + y as i32;
                    if px_i32 >= 0 && py_i32 >= 0 {
                        let px = px_i32 as usize;
                        let py = py_i32 as usize;
                        if px < atlas_width && py < atlas_height {
                            let idx = (py * atlas_width + px) * 4;
                            let intensity = (v * 255.0).round() as u8;
                            pixels[idx] = intensity;
                            pixels[idx + 1] = intensity;
                            pixels[idx + 2] = intensity;
                            pixels[idx + 3] = intensity;
                        }
                    }
                });

                CharUV {
                    u: atlas_x as f32 / atlas_width as f32,
                    v: atlas_y as f32 / atlas_height as f32,
                    u_width: bb.width() as f32 / atlas_width as f32,
                    v_height: bb.height() as f32 / atlas_height as f32,
                    bearing_x: padding as f32 + bb.min.x as f32,
                    bearing_y: padding as f32 + bb.min.y as f32,
                    width: bb.width() as f32,
                    height: bb.height() as f32,
                }
            } else {
                CharUV {
                    u: cell_x as f32 / atlas_width as f32,
                    v: cell_y as f32 / atlas_height as f32,
                    u_width: 0.0,
                    v_height: 0.0,
                    bearing_x: 0.0,
                    bearing_y: 0.0,
                    width: 0.0,
                    height: 0.0,
                }
            };

            glyphs.insert(
                *ch,
                GlyphInfo {
                    uv,
                    advance: if advance > 0.0 {
                        advance
                    } else {
                        cell_width as f32 * 0.5
                    },
                },
            );
        }

        let temp_asset = TextureAsset {
            width: atlas_width
                .try_into()
                .expect("Font atlas width exceeds u32"),
            height: atlas_height
                .try_into()
                .expect("Font atlas height exceeds u32"),
            pixels,
            path: PathBuf::new(),
        };

        Some(FontData {
            atlas_texture: Arc::new(GpuTexture::from_asset(device, queue, &temp_asset)),
            glyphs,
            cell_width: cell_width as f32,
            cell_height: cell_height as f32,
            line_height: line_height as f32,
            ascent,
            descent,
            base_font_size: font_size,
        })
    }

    /// Built-in pixel font (8x16) with a classic bitmap look.
    /// Always available as FontId::DEFAULT.
    fn load_builtin_pixel_font(&mut self, device: &Device, queue: &Queue) -> FontData {
        let char_width = 8u32;
        let char_height = 16u32;
        let cols = 16u32;
        let rows = 6u32;
        let atlas_width = cols * char_width;
        let atlas_height = rows * char_height;

        let mut pixels = vec![0u8; (atlas_width * atlas_height * 4) as usize];
        let mut glyphs = HashMap::new();

        // 8x16 pixel font bitmaps (96 chars, from space to ~)
        let char_bitmaps: &[(u8, &[u8])] = &[
            (
                b' ',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'!',
                &[
                    0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'"',
                &[
                    0x00, 0x00, 0x24, 0x24, 0x24, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'#',
                &[
                    0x00, 0x00, 0x24, 0x24, 0x24, 0x7E, 0x24, 0x24, 0x7E, 0x24, 0x24, 0x24, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'$',
                &[
                    0x00, 0x08, 0x3E, 0x49, 0x48, 0x38, 0x0E, 0x09, 0x49, 0x3E, 0x08, 0x08, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'%',
                &[
                    0x00, 0x00, 0x60, 0x92, 0x94, 0x68, 0x08, 0x10, 0x2C, 0x52, 0x92, 0x0C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'&',
                &[
                    0x00, 0x00, 0x38, 0x44, 0x44, 0x48, 0x30, 0x50, 0x4C, 0x44, 0x44, 0x3A, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'\'',
                &[
                    0x00, 0x00, 0x08, 0x08, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'(',
                &[
                    0x00, 0x04, 0x08, 0x10, 0x10, 0x20, 0x20, 0x20, 0x20, 0x10, 0x10, 0x08, 0x04,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b')',
                &[
                    0x00, 0x20, 0x10, 0x08, 0x08, 0x04, 0x04, 0x04, 0x04, 0x08, 0x08, 0x10, 0x20,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'*',
                &[
                    0x00, 0x00, 0x00, 0x08, 0x2A, 0x1C, 0x3E, 0x1C, 0x2A, 0x08, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'+',
                &[
                    0x00, 0x00, 0x00, 0x08, 0x08, 0x08, 0x7F, 0x08, 0x08, 0x08, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b',',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x08, 0x10,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'-',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'.',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'/',
                &[
                    0x00, 0x00, 0x02, 0x04, 0x04, 0x08, 0x08, 0x10, 0x10, 0x20, 0x20, 0x40, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'0',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x46, 0x4A, 0x52, 0x62, 0x42, 0x42, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'1',
                &[
                    0x00, 0x00, 0x08, 0x18, 0x28, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'2',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x42, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'3',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x02, 0x02, 0x1C, 0x02, 0x02, 0x02, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'4',
                &[
                    0x00, 0x00, 0x04, 0x0C, 0x14, 0x24, 0x44, 0x7E, 0x04, 0x04, 0x04, 0x04, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'5',
                &[
                    0x00, 0x00, 0x7E, 0x40, 0x40, 0x7C, 0x42, 0x02, 0x02, 0x02, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'6',
                &[
                    0x00, 0x00, 0x1C, 0x20, 0x40, 0x40, 0x7C, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'7',
                &[
                    0x00, 0x00, 0x7E, 0x02, 0x04, 0x08, 0x08, 0x10, 0x10, 0x20, 0x20, 0x20, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'8',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x42, 0x42, 0x3C, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'9',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x42, 0x42, 0x3E, 0x02, 0x02, 0x02, 0x04, 0x38, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b':',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b';',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x08, 0x10,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'<',
                &[
                    0x00, 0x00, 0x04, 0x08, 0x10, 0x20, 0x40, 0x20, 0x10, 0x08, 0x04, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'=',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'>',
                &[
                    0x00, 0x00, 0x40, 0x20, 0x10, 0x08, 0x04, 0x08, 0x10, 0x20, 0x40, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'?',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x42, 0x02, 0x04, 0x08, 0x08, 0x00, 0x08, 0x08, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'@',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x42, 0x4E, 0x52, 0x56, 0x4A, 0x40, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'A',
                &[
                    0x00, 0x00, 0x18, 0x24, 0x42, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'B',
                &[
                    0x00, 0x00, 0x7C, 0x42, 0x42, 0x42, 0x7C, 0x42, 0x42, 0x42, 0x42, 0x7C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'C',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'D',
                &[
                    0x00, 0x00, 0x78, 0x44, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x44, 0x78, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'E',
                &[
                    0x00, 0x00, 0x7E, 0x40, 0x40, 0x40, 0x7C, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'F',
                &[
                    0x00, 0x00, 0x7E, 0x40, 0x40, 0x40, 0x7C, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'G',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x40, 0x40, 0x4E, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'H',
                &[
                    0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'I',
                &[
                    0x00, 0x00, 0x3E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'J',
                &[
                    0x00, 0x00, 0x3E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x48, 0x30, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'K',
                &[
                    0x00, 0x00, 0x42, 0x44, 0x48, 0x50, 0x60, 0x50, 0x48, 0x44, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'L',
                &[
                    0x00, 0x00, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'M',
                &[
                    0x00, 0x00, 0x42, 0x66, 0x5A, 0x5A, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'N',
                &[
                    0x00, 0x00, 0x42, 0x62, 0x52, 0x4A, 0x46, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'O',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'P',
                &[
                    0x00, 0x00, 0x7C, 0x42, 0x42, 0x42, 0x7C, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'Q',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x52, 0x4A, 0x44, 0x3A, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'R',
                &[
                    0x00, 0x00, 0x7C, 0x42, 0x42, 0x42, 0x7C, 0x48, 0x44, 0x42, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'S',
                &[
                    0x00, 0x00, 0x3C, 0x42, 0x40, 0x30, 0x0C, 0x02, 0x02, 0x02, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'T',
                &[
                    0x00, 0x00, 0x7F, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'U',
                &[
                    0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'V',
                &[
                    0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0x18, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'W',
                &[
                    0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x42, 0x5A, 0x5A, 0x66, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'X',
                &[
                    0x00, 0x00, 0x42, 0x42, 0x24, 0x24, 0x18, 0x18, 0x24, 0x24, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'Y',
                &[
                    0x00, 0x00, 0x41, 0x41, 0x22, 0x22, 0x14, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'Z',
                &[
                    0x00, 0x00, 0x7E, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x40, 0x40, 0x7E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'[',
                &[
                    0x00, 0x1E, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1E,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'\\',
                &[
                    0x00, 0x00, 0x40, 0x20, 0x20, 0x10, 0x10, 0x08, 0x08, 0x04, 0x04, 0x02, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b']',
                &[
                    0x00, 0x78, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x78,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'^',
                &[
                    0x00, 0x00, 0x08, 0x14, 0x22, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'_',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x7F, 0x00, 0x00,
                ],
            ),
            (
                b'`',
                &[
                    0x00, 0x00, 0x10, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'a',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x3C, 0x02, 0x3E, 0x42, 0x42, 0x42, 0x46, 0x3A, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'b',
                &[
                    0x00, 0x00, 0x40, 0x40, 0x5C, 0x62, 0x42, 0x42, 0x42, 0x42, 0x62, 0x5C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'c',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x3C, 0x42, 0x40, 0x40, 0x40, 0x40, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'd',
                &[
                    0x00, 0x00, 0x02, 0x02, 0x3A, 0x46, 0x42, 0x42, 0x42, 0x42, 0x46, 0x3A, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'e',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x3C, 0x42, 0x42, 0x7E, 0x40, 0x40, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'f',
                &[
                    0x00, 0x00, 0x0C, 0x12, 0x10, 0x10, 0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'g',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x3A, 0x46, 0x42, 0x42, 0x42, 0x46, 0x3A, 0x02, 0x02,
                    0x3C, 0x00, 0x00,
                ],
            ),
            (
                b'h',
                &[
                    0x00, 0x00, 0x40, 0x40, 0x5C, 0x62, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'i',
                &[
                    0x00, 0x00, 0x08, 0x08, 0x00, 0x38, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'j',
                &[
                    0x00, 0x00, 0x04, 0x04, 0x00, 0x1C, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x44,
                    0x38, 0x00, 0x00,
                ],
            ),
            (
                b'k',
                &[
                    0x00, 0x00, 0x40, 0x40, 0x44, 0x48, 0x50, 0x60, 0x50, 0x48, 0x44, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'l',
                &[
                    0x00, 0x00, 0x38, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'm',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x76, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'n',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x5C, 0x62, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'o',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'p',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x5C, 0x62, 0x42, 0x42, 0x42, 0x62, 0x5C, 0x40, 0x40,
                    0x40, 0x00, 0x00,
                ],
            ),
            (
                b'q',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x3A, 0x46, 0x42, 0x42, 0x42, 0x46, 0x3A, 0x02, 0x02,
                    0x02, 0x00, 0x00,
                ],
            ),
            (
                b'r',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x5C, 0x62, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b's',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x3E, 0x40, 0x40, 0x3C, 0x02, 0x02, 0x02, 0x7C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b't',
                &[
                    0x00, 0x00, 0x10, 0x10, 0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x12, 0x0C, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'u',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x46, 0x3A, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'v',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0x18, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'w',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x41, 0x41, 0x49, 0x49, 0x49, 0x49, 0x49, 0x36, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'x',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x42, 0x42, 0x24, 0x18, 0x18, 0x24, 0x42, 0x42, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'y',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x42, 0x46, 0x3A, 0x02, 0x02,
                    0x3C, 0x00, 0x00,
                ],
            ),
            (
                b'z',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x7E, 0x04, 0x08, 0x10, 0x10, 0x20, 0x40, 0x7E, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'{',
                &[
                    0x00, 0x06, 0x08, 0x08, 0x08, 0x08, 0x30, 0x08, 0x08, 0x08, 0x08, 0x08, 0x06,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'|',
                &[
                    0x00, 0x00, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'}',
                &[
                    0x00, 0x60, 0x10, 0x10, 0x10, 0x10, 0x0C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x60,
                    0x00, 0x00, 0x00,
                ],
            ),
            (
                b'~',
                &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x62, 0x92, 0x8C, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ),
        ];

        for c in 32u8..127 {
            let ch = c as char;
            let char_idx = (c - 32) as usize;
            let col = (char_idx % cols as usize) as u32;
            let row = (char_idx / cols as usize) as u32;
            let u = (col * char_width) as f32 / atlas_width as f32;
            let v = (row * char_height) as f32 / atlas_height as f32;
            let u_width = char_width as f32 / atlas_width as f32;
            let v_height = char_height as f32 / atlas_height as f32;
            glyphs.insert(
                ch,
                GlyphInfo {
                    uv: CharUV {
                        u,
                        v,
                        u_width,
                        v_height,
                        bearing_x: 0.0,
                        bearing_y: 0.0,
                        width: char_width as f32,
                        height: char_height as f32,
                    },
                    advance: char_width as f32,
                },
            );

            let bitmap = char_bitmaps
                .iter()
                .find(|(code, _)| *code == c)
                .map(|(_, bmp)| *bmp)
                .unwrap_or(&[0x00; 16]);

            for y in 0..char_height {
                for x in 0..char_width {
                    let atlas_x = col * char_width + x;
                    let atlas_y = row * char_height + y;
                    let idx = ((atlas_y * atlas_width + atlas_x) * 4) as usize;
                    let bit = 7 - (x % 8);
                    let pixel_on = (bitmap[y as usize] >> bit) & 1 != 0;
                    let v = if pixel_on { 255u8 } else { 0u8 };
                    pixels[idx] = v;
                    pixels[idx + 1] = v;
                    pixels[idx + 2] = v;
                    pixels[idx + 3] = v;
                }
            }
        }

        let temp_asset = TextureAsset {
            width: atlas_width,
            height: atlas_height,
            pixels,
            path: PathBuf::new(),
        };

        FontData {
            atlas_texture: Arc::new(GpuTexture::from_asset(device, queue, &temp_asset)),
            glyphs,
            cell_width: char_width as f32,
            cell_height: char_height as f32,
            line_height: char_height as f32,
            ascent: char_height as f32,
            descent: 0.0,
            base_font_size: char_height as f32,
        }
    }

    fn load_default_system_font_bytes() -> Option<Vec<u8>> {
        const WINDOWS_FONTS: &[&str] = &[
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\calibri.ttf",
        ];
        const MACOS_FONTS: &[&str] = &["/Library/Fonts/Arial.ttf", "/Library/Fonts/Helvetica.ttf"];
        const LINUX_FONTS: &[&str] = &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];

        let font_paths = if cfg!(target_os = "windows") {
            WINDOWS_FONTS
        } else if cfg!(target_os = "macos") {
            MACOS_FONTS
        } else if cfg!(target_os = "linux") {
            LINUX_FONTS
        } else {
            &[]
        };

        for path in font_paths {
            if let Ok(bytes) = std::fs::read(path) {
                return Some(bytes);
            }
        }
        None
    }

    pub fn get_atlas_texture(&self) -> Option<&Arc<GpuTexture>> {
        self.fonts.first().map(|f| &f.atlas_texture)
    }

    pub fn get_atlas_texture_for(&self, font_id: FontId) -> Option<&Arc<GpuTexture>> {
        self.fonts.get(font_id.0).map(|f| &f.atlas_texture)
    }

    pub fn get_char_uv(&self, ch: char) -> Option<CharUV> {
        self.fonts
            .first()
            .and_then(|f| f.glyphs.get(&ch).map(|info| info.uv))
    }

    pub fn get_char_uv_for(&self, font_id: FontId, ch: char) -> Option<CharUV> {
        self.fonts
            .get(font_id.0)
            .and_then(|f| f.glyphs.get(&ch).map(|info| info.uv))
    }

    pub fn get_glyph_info(&self, ch: char) -> Option<&GlyphInfo> {
        self.fonts.first().and_then(|f| f.glyphs.get(&ch))
    }

    pub fn get_glyph_info_for(&self, font_id: FontId, ch: char) -> Option<&GlyphInfo> {
        self.fonts.get(font_id.0).and_then(|f| f.glyphs.get(&ch))
    }

    pub fn get_char_advance(&self, ch: char) -> Option<f32> {
        self.fonts
            .first()
            .and_then(|f| f.glyphs.get(&ch).map(|info| info.advance))
    }

    pub fn get_char_advance_for(&self, font_id: FontId, ch: char) -> Option<f32> {
        self.fonts
            .get(font_id.0)
            .and_then(|f| f.glyphs.get(&ch).map(|info| info.advance))
    }

    pub fn char_size(&self) -> (u32, u32) {
        self.fonts
            .first()
            .map(|f| (f.cell_width as u32, f.cell_height as u32))
            .unwrap_or((8, 16))
    }

    pub fn char_size_for(&self, font_id: FontId) -> (u32, u32) {
        self.fonts
            .get(font_id.0)
            .map(|f| (f.cell_width as u32, f.cell_height as u32))
            .unwrap_or((8, 16))
    }

    pub fn line_height(&self) -> f32 {
        self.fonts.first().map(|f| f.line_height).unwrap_or(16.0)
    }

    pub fn line_height_for(&self, font_id: FontId) -> f32 {
        self.fonts
            .get(font_id.0)
            .map(|f| f.line_height)
            .unwrap_or(16.0)
    }

    pub fn ascent(&self) -> f32 {
        self.fonts.first().map(|f| f.ascent).unwrap_or(16.0)
    }

    pub fn descent(&self) -> f32 {
        self.fonts.first().map(|f| f.descent).unwrap_or(0.0)
    }

    pub fn base_font_size(&self) -> f32 {
        self.fonts.first().map(|f| f.base_font_size).unwrap_or(16.0)
    }

    pub fn base_font_size_for(&self, font_id: FontId) -> f32 {
        self.fonts
            .get(font_id.0)
            .map(|f| f.base_font_size)
            .unwrap_or(16.0)
    }

    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }
}

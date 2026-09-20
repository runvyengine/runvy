use std::fs;
use std::path::Path;

use egui::{ColorImage, Id, TextureHandle};
use resvg::{tiny_skia, usvg};

const EDITOR_ICON_RASTER_SIZE: u32 = 64;
const COMPONENT_ICON_RASTER_SIZE: u32 = 96;

struct EmbeddedIcon {
    bytes: &'static [u8],
    extension: &'static str,
}

pub fn load_editor_icon(ctx: &egui::Context, texture_name: &str, icon_name: &str) -> TextureHandle {
    let icon = embedded_editor_icon(icon_name)
        .unwrap_or_else(|| panic!("failed to find embedded editor icon `{icon_name}`"));

    load_cached_embedded_texture(
        ctx,
        texture_name,
        icon_name,
        icon,
        Some(EDITOR_ICON_RASTER_SIZE),
    )
    .unwrap_or_else(|error| panic!("failed to load editor icon `{icon_name}`: {error}"))
}

pub fn load_component_icon(
    ctx: &egui::Context,
    texture_name: &str,
    component_icon_name: &str,
) -> TextureHandle {
    let (resolved_name, icon) = embedded_component_icon(component_icon_name)
        .map(|icon| (component_icon_name, icon))
        .or_else(|| embedded_component_icon("c-Object").map(|icon| ("c-Object", icon)))
        .unwrap_or_else(|| {
            panic!("failed to find embedded component icon `{component_icon_name}` or fallback `c-Object`")
        });

    load_cached_embedded_texture(
        ctx,
        texture_name,
        resolved_name,
        icon,
        Some(COMPONENT_ICON_RASTER_SIZE),
    )
    .unwrap_or_else(|error| {
        panic!("failed to load component icon `{component_icon_name}`: {error}")
    })
}

pub fn load_texture_from_path(
    ctx: &egui::Context,
    texture_name: &str,
    path: &Path,
    raster_size: Option<u32>,
) -> Result<TextureHandle, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "svg" => load_svg_texture(
            ctx,
            texture_name,
            &bytes,
            raster_size.unwrap_or(EDITOR_ICON_RASTER_SIZE),
        ),
        _ => load_raster_texture(ctx, texture_name, &bytes),
    }
}

fn load_cached_embedded_texture(
    ctx: &egui::Context,
    texture_name: &str,
    icon_name: &str,
    icon: EmbeddedIcon,
    raster_size: Option<u32>,
) -> Result<TextureHandle, String> {
    let cache_id = Id::new(("embedded_editor_texture_cache", texture_name, icon_name));
    if let Some(texture) = ctx.data_mut(|data| data.get_temp::<TextureHandle>(cache_id)) {
        return Ok(texture);
    }

    let texture = match icon.extension {
        "svg" => load_svg_texture(
            ctx,
            texture_name,
            icon.bytes,
            raster_size.unwrap_or(EDITOR_ICON_RASTER_SIZE),
        ),
        _ => load_raster_texture(ctx, texture_name, icon.bytes),
    }?;
    ctx.data_mut(|data| data.insert_temp(cache_id, texture.clone()));
    Ok(texture)
}

fn embedded_editor_icon(icon_name: &str) -> Option<EmbeddedIcon> {
    let icon = match icon_name {
        "audio" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/audio.svg"),
            extension: "svg",
        },
        "camera" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/camera.svg"),
            extension: "svg",
        },
        "cross-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/cross-icon.svg"),
            extension: "svg",
        },
        "edit-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/edit-icon.svg"),
            extension: "svg",
        },
        "file" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/file.svg"),
            extension: "svg",
        },
        "folder-empty" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/folder-empty.svg"),
            extension: "svg",
        },
        "folder-open" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/folder-open.svg"),
            extension: "svg",
        },
        "folder" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/folder.svg"),
            extension: "svg",
        },
        "image" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/image.svg"),
            extension: "svg",
        },
        "Play" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/Play.svg"),
            extension: "svg",
        },
        "position-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/position-icon.svg"),
            extension: "svg",
        },
        "Pause" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/Pause.svg"),
            extension: "svg",
        },
        "rotation-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/rotation-icon.svg"),
            extension: "svg",
        },
        "scale-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/scale-icon.svg"),
            extension: "svg",
        },
        "Stop" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/Stop.svg"),
            extension: "svg",
        },
        "question-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/question-icon.svg"),
            extension: "svg",
        },
        "rust-file" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/rust-file.svg"),
            extension: "svg",
        },
        "wgsl" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/wgsl.svg"),
            extension: "svg",
        },
        "world" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/world.svg"),
            extension: "svg",
        },
        "r3m" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/runvy3dm.svg"),
            extension: "svg",
        },
        _ => return None,
    };
    Some(icon)
}

fn embedded_component_icon(icon_name: &str) -> Option<EmbeddedIcon> {
    let icon = match icon_name {
        "c-ActiveCamera" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-ActiveCamera.png"),
            extension: "png",
        },
        "c-AudioListener" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-AudioListener.png"),
            extension: "png",
        },
        "c-AudioSource" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-AudioSource.png"),
            extension: "png",
        },
        "c-Camera" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-Camera.png"),
            extension: "png",
        },
        "c-Canvas" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-Canvas.png"),
            extension: "png",
        },
        "c-Collider" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-Collider.png"),
            extension: "png",
        },
        "c-Collider2D" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-Collider2D.png"),
            extension: "png",
        },
        "c-CursorInteractable" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-CursorInteractable.png"),
            extension: "png",
        },
        "c-DirectionalLight" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-DirectionalLight.png"),
            extension: "png",
        },
        "c-MeshRenderer" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-MeshRenderer.png"),
            extension: "png",
        },
        "c-Object" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-Object.png"),
            extension: "png",
        },
        "c-PhysicsCollision" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-PhysicsCollision.png"),
            extension: "png",
        },
        "c-PointLight" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-PointLight.png"),
            extension: "png",
        },
        "c-Script" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-Script.png"),
            extension: "png",
        },
        "c-Sorting" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-Sorting.png"),
            extension: "png",
        },
        "c-SpriteAnimator" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-SpriteAnimator.png"),
            extension: "png",
        },
        "c-SpriteRenderer" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-SpriteRenderer.png"),
            extension: "png",
        },
        "c-TilemapRenderer" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-TilemapRenderer.png"),
            extension: "png",
        },
        "c-Transform" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-Transform.png"),
            extension: "png",
        },
        "c-UiImage" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-UiImage.png"),
            extension: "png",
        },
        "c-UiText" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/c-UiText.png"),
            extension: "png",
        },
        "cross-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/cross-icon.png"),
            extension: "png",
        },
        "edit-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/edit-icon.png"),
            extension: "png",
        },
        "question-icon" => EmbeddedIcon {
            bytes: include_bytes!("../assets/icons/components/question-icon.png"),
            extension: "png",
        },
        _ => return None,
    };
    Some(icon)
}

fn load_raster_texture(
    ctx: &egui::Context,
    texture_name: &str,
    bytes: &[u8],
) -> Result<TextureHandle, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| format!("failed to decode raster image: {error}"))?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();
    let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
    Ok(ctx.load_texture(texture_name, color_image, egui::TextureOptions::LINEAR))
}

fn load_svg_texture(
    ctx: &egui::Context,
    texture_name: &str,
    bytes: &[u8],
    raster_size: u32,
) -> Result<TextureHandle, String> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(bytes, &options)
        .map_err(|error| format!("failed to parse SVG: {error}"))?;

    let svg_size = tree.size();
    let largest_axis = svg_size.width().max(svg_size.height());
    if largest_axis <= 0.0 {
        return Err("SVG has invalid size".to_string());
    }

    // Rasterize into a predictable square budget so icons stay crisp in egui.
    let scale = raster_size as f32 / largest_axis;
    let width = (svg_size.width() * scale).round().max(1.0) as u32;
    let height = (svg_size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "failed to allocate SVG pixmap".to_string())?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let color_image =
        ColorImage::from_rgba_unmultiplied([width as usize, height as usize], pixmap.data());
    Ok(ctx.load_texture(texture_name, color_image, egui::TextureOptions::LINEAR))
}

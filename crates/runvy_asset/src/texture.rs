use std::path::PathBuf;

#[derive(Debug)]
pub struct TextureAsset {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA8
    pub path: PathBuf,
}

impl TextureAsset {
    pub fn load(path: &PathBuf) -> Result<Self, image::ImageError> {
        // println!("📂 Loading from: {:?}", path);
        let img = image::open(path)?.to_rgba8();

        let (width, height) = img.dimensions();

        Ok(Self {
            width,
            height,
            pixels: img.into_raw(),
            path: path.clone(),
        })
    }

    pub fn from_rgba8(path: PathBuf, width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
            path,
        }
    }
}

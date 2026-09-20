use std::sync::Arc;

/// A font asset that can be loaded from a TTF file.
pub struct FontAsset {
    pub name: String,
    pub data: Arc<Vec<u8>>,
}

impl FontAsset {
    pub fn load_from_bytes(data: Vec<u8>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Arc::new(data),
        }
    }

    pub fn load_from_ttf(path: impl AsRef<std::path::Path>) -> Result<Self, std::io::Error> {
        let data = std::fs::read(path.as_ref())?;
        let name = path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("font")
            .to_string();
        Ok(Self {
            name,
            data: Arc::new(data),
        })
    }
}

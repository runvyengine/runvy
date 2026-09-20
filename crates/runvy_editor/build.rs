#[cfg(windows)]
fn main() {
    use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
    use std::path::PathBuf;

    let icon_path = PathBuf::from("assets").join("big_icon.png");
    println!("cargo:rerun-if-changed={}", icon_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        PathBuf::from("assets").join("icon.png").display()
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must exist"));
    let ico_path = out_dir.join("runvy_editor.ico");

    let source = image::open(&icon_path).expect("failed to load editor icon");
    let mut icon_dir = IconDir::new(ResourceType::Icon);

    // Windows shell is more reliable when the PE resource contains several standard sizes.
    for size in [16, 32, 48, 64, 128, 256] {
        let resized = source.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let rgba = resized.to_rgba8();
        let icon_image = IconImage::from_rgba_data(size, size, rgba.into_raw());
        let entry = IconDirEntry::encode(&icon_image).expect("failed to encode editor icon frame");
        icon_dir.add_entry(entry);
    }

    let mut file = std::fs::File::create(&ico_path).expect("failed to create editor .ico");
    icon_dir
        .write(&mut file)
        .expect("failed to write editor .ico");

    winres::WindowsResource::new()
        .set_icon(ico_path.to_string_lossy().as_ref())
        .compile()
        .expect("failed to embed windows icon resource");
}

#[cfg(not(windows))]
fn main() {}

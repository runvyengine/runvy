use std::fs;
use std::path::{Path, PathBuf};

use runvy_core::components::ui::UiAssetFile;

use crate::project::ProjectPaths;

pub fn save_ui_asset(project: &ProjectPaths, ui: &UiAssetFile) -> Result<(), String> {
    let path = ui_asset_path(project, "");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content =
        ron::ser::to_string_pretty(ui, ron::ser::PrettyConfig::default()).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_ui_asset(path: &Path) -> Result<UiAssetFile, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    ron::from_str(&content).map_err(|e| e.to_string())
}

pub fn ui_asset_path(project: &ProjectPaths, name: &str) -> PathBuf {
    let name = if name.is_empty() {
        "ui/NewUI.ui.ron".to_string()
    } else if name.ends_with(".ui.ron") {
        name.to_string()
    } else {
        format!("{name}.ui.ron")
    };
    project.root_dir.join("assets").join(name)
}

pub fn find_existing_ui_asset_path(project: &ProjectPaths, relative_path: &str) -> Option<PathBuf> {
    let path = project.root_dir.join(relative_path);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub fn collect_ui_asset_paths(project: &ProjectPaths) -> Vec<PathBuf> {
    let ui_dir = project.root_dir.join("assets").join("ui");
    if !ui_dir.is_dir() {
        return Vec::new();
    }
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(&ui_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron")
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".ui.ron"))
                    .unwrap_or(false)
            {
                results.push(path);
            }
        }
    }
    results
}

pub fn collect_ui_asset_relative_paths(project: &ProjectPaths) -> Vec<String> {
    collect_ui_asset_paths(project)
        .into_iter()
        .filter_map(|path| {
            path.strip_prefix(&project.root_dir)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
        })
        .collect()
}

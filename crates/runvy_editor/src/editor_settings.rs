use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const SETTINGS_PATH: &str = ".runvy_editor/settings.ron";
const MAX_RECENT_PROJECTS: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentProjectEntry {
    pub manifest_path: PathBuf,
    #[serde(default)]
    pub project_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub external_editor_executable: String,
    pub external_editor_args: String,
    #[serde(default = "default_external_editor_project_args")]
    pub external_editor_project_args: String,
    pub ui_scale: f32,
    pub show_hidden_files: bool,
    pub content_icon_size: f32,
    #[serde(default)]
    pub recent_projects: Vec<RecentProjectEntry>,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            external_editor_executable: "zed".to_string(),
            external_editor_args: "{file}".to_string(),
            external_editor_project_args: "{project}".to_string(),
            ui_scale: 1.0,
            show_hidden_files: false,
            content_icon_size: 48.0,
            recent_projects: Vec::new(),
        }
    }
}

impl EditorSettings {
    pub fn load() -> Self {
        let path = settings_path();
        let Ok(content) = fs::read_to_string(path) else {
            return Self::default();
        };
        ron::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = settings_path();
        let Some(parent) = path.parent() else {
            return Err("Invalid settings path".to_string());
        };
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let content = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|error| error.to_string())
    }

    pub fn remember_project(&mut self, manifest_path: PathBuf, project_name: impl Into<String>) {
        let manifest_path = normalize_path(manifest_path);
        let project_name = project_name.into();
        self.recent_projects
            .retain(|entry| normalize_path(entry.manifest_path.clone()) != manifest_path);
        self.recent_projects.insert(
            0,
            RecentProjectEntry {
                manifest_path,
                project_name,
            },
        );
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }

    pub fn prune_missing_recent_projects(&mut self) -> bool {
        let before = self.recent_projects.len();
        self.recent_projects
            .retain(|entry| entry.manifest_path.is_file());
        before != self.recent_projects.len()
    }

    pub fn remove_recent_project(&mut self, manifest_path: &std::path::Path) {
        let manifest_path = normalize_path(manifest_path.to_path_buf());
        self.recent_projects
            .retain(|entry| normalize_path(entry.manifest_path.clone()) != manifest_path);
    }
}

fn settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join(SETTINGS_PATH)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn default_external_editor_project_args() -> String {
    "{project}".to_string()
}

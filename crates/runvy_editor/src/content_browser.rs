use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use egui::{Color32, RichText, TextureHandle, Ui, Vec2};
use rfd::FileDialog;

use crate::editor_settings::EditorSettings;
use crate::editor_textures::{load_editor_icon, load_texture_from_path};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AssetKind {
    Directory,
    GenericFile,
    RustFile,
    WorldFile,
    ImageFile,
    AudioFile,
    ShaderFile,
    PrefabFile,
    Runvy3DModelFile,
    UiFile,
}

#[derive(Clone)]
pub struct ContentEntry {
    pub name: String,
    pub relative_path: String,
    pub full_path: PathBuf,
    kind: AssetKind,
}

impl ContentEntry {
    fn is_dir(&self) -> bool {
        self.kind == AssetKind::Directory
    }
}

struct ContentBrowserIcons {
    folder: TextureHandle,
    folder_empty: TextureHandle,
    folder_open: TextureHandle,
    file: TextureHandle,
    rust_file: TextureHandle,
    world_file: TextureHandle,
    image_file: TextureHandle,
    audio_file: TextureHandle,
    shader_file: TextureHandle,
    r3m_file: TextureHandle,
}

struct RenameState {
    path: PathBuf,
    buffer: String,
    request_focus: bool,
}

#[derive(Clone)]
struct ClipboardEntry {
    path: PathBuf,
    cut: bool,
}

pub struct ContentBrowserState {
    project_root: PathBuf,
    current_dir: PathBuf,
    entries: Vec<ContentEntry>,
    sidebar_width: f32,
    icons: Option<ContentBrowserIcons>,
    image_previews: HashMap<PathBuf, TextureHandle>,
    selected_path: Option<PathBuf>,
    dragging_asset_path: Option<PathBuf>,
    rename_state: Option<RenameState>,
    clipboard: Option<ClipboardEntry>,
    pending_open_dir: Option<PathBuf>,
    pending_open_world: Option<PathBuf>,
    last_message: Option<String>,
    expanded_dirs: HashSet<PathBuf>,
}

impl ContentBrowserState {
    pub fn new(project_root: PathBuf) -> Self {
        let entries = collect_directory_entries(&project_root, &project_root, false);
        let mut expanded_dirs = HashSet::new();
        expanded_dirs.insert(project_root.clone());
        Self {
            current_dir: project_root.clone(),
            project_root,
            entries,
            sidebar_width: 220.0,
            icons: None,
            image_previews: HashMap::new(),
            selected_path: None,
            dragging_asset_path: None,
            rename_state: None,
            clipboard: None,
            pending_open_dir: None,
            pending_open_world: None,
            last_message: None,
            expanded_dirs,
        }
    }

    pub fn open_dir(&mut self, dir: PathBuf, settings: &EditorSettings) {
        self.ensure_directory_expanded(&dir);
        self.current_dir = dir;
        self.entries = collect_directory_entries(
            &self.project_root,
            &self.current_dir,
            settings.show_hidden_files,
        );
        self.selected_path = None;
        self.dragging_asset_path = None;
        self.rename_state = None;
        self.pending_open_world = None;
    }

    pub fn refresh(&mut self, settings: &EditorSettings) {
        self.entries = collect_directory_entries(
            &self.project_root,
            &self.current_dir,
            settings.show_hidden_files,
        );
        self.image_previews
            .retain(|path, _| path.exists() && path.starts_with(&self.project_root));
    }

    pub fn set_project_root(&mut self, project_root: PathBuf, settings: &EditorSettings) {
        self.project_root = project_root.clone();
        self.current_dir = project_root.clone();
        self.selected_path = None;
        self.dragging_asset_path = None;
        self.rename_state = None;
        self.clipboard = None;
        self.pending_open_world = None;
        self.image_previews.clear();
        self.expanded_dirs.clear();
        self.expanded_dirs.insert(project_root);
        self.refresh(settings);
    }

    pub fn take_message(&mut self) -> Option<String> {
        self.last_message.take()
    }

    pub fn take_pending_world_open(&mut self) -> Option<PathBuf> {
        self.pending_open_world.take()
    }

    pub fn current_dir_display(&self) -> String {
        self.current_dir.display().to_string()
    }

    pub fn import_into_current_dir(&mut self, settings: &EditorSettings) {
        let target_dir = self.current_dir.clone();
        self.import_assets_into(&target_dir, settings);
    }

    pub fn ui(&mut self, ui: &mut Ui, settings: &EditorSettings) {
        self.ensure_icons(ui.ctx());
        self.handle_shortcuts(ui);

        let available = ui.available_size() - egui::vec2(0.0, 6.0);
        ui.allocate_ui_with_layout(available, egui::Layout::top_down(egui::Align::Min), |ui| {
            let handle_width = 8.0;
            let min_sidebar = 180.0;
            let max_sidebar = (available.x - 180.0).max(min_sidebar);
            self.sidebar_width = self.sidebar_width.clamp(min_sidebar, max_sidebar);

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(self.sidebar_width, available.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.label(RichText::new("Folders").strong());
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .id_salt("folder_tree_scroll")
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                let root_name = self
                                    .project_root
                                    .file_name()
                                    .map(|name| name.to_string_lossy().to_string())
                                    .unwrap_or_else(|| self.project_root.display().to_string());
                                let root_selected = self.current_dir == self.project_root;
                                let icons = self.icons.as_ref().expect("icons must be initialized");
                                let root_is_empty = is_directory_empty(
                                    &self.project_root,
                                    settings.show_hidden_files,
                                );
                                let root_icon = if root_is_empty {
                                    icons.folder_empty.clone()
                                } else {
                                    icons.folder_open.clone()
                                };
                                let project_root = self.project_root.clone();
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Image::new(&root_icon)
                                            .fit_to_exact_size(egui::vec2(18.0, 18.0))
                                            .sense(egui::Sense::hover()),
                                    );
                                    if ui.selectable_label(root_selected, root_name).clicked() {
                                        self.open_dir(project_root.clone(), settings);
                                    }
                                });
                                ui.separator();
                                let root = self.project_root.clone();
                                let mut next_dir = None;
                                self.folder_tree_ui(ui, &root, 0, settings, &mut next_dir);
                                if let Some(dir) = next_dir {
                                    self.open_dir(dir, settings);
                                }
                            });

                        let remaining_height = ui.available_height().max(0.0);
                        if remaining_height > 0.0 {
                            let blank_response = ui.allocate_response(
                                egui::vec2(ui.available_width(), remaining_height),
                                egui::Sense::click(),
                            );
                            blank_response.context_menu(|ui| {
                                self.folder_area_context_menu(ui, settings);
                            });
                        }
                    },
                );

                let (handle_rect, handle_response) = ui.allocate_exact_size(
                    egui::vec2(handle_width, available.y),
                    egui::Sense::click_and_drag(),
                );
                if handle_response.hovered() || handle_response.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
                if handle_response.dragged() {
                    self.sidebar_width = (self.sidebar_width + handle_response.drag_delta().x)
                        .clamp(min_sidebar, max_sidebar);
                }
                paint_splitter(ui, handle_rect, &handle_response);

                let right_width = ui.available_width().max(120.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(right_width, available.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let asset_area_rect = ui.max_rect();
                        let asset_area_response = ui.interact(
                            asset_area_rect,
                            ui.id().with("assets_background_context"),
                            egui::Sense::click(),
                        );

                        ui.label(RichText::new("Assets").strong());
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .id_salt("content_grid_scroll")
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                self.content_grid_ui(ui, settings);
                            });

                        if asset_area_response.clicked() {
                            self.selected_path = None;
                        }
                        asset_area_response.context_menu(|ui| {
                            self.asset_area_context_menu(ui, self.current_dir.clone(), settings);
                        });
                    },
                );
            });
        });
        ui.add_space(4.0);

        if let Some(dir) = self.pending_open_dir.take() {
            self.open_dir(dir, settings);
        }

        if self.dragging_asset_path.is_some()
            && !ui.ctx().input(|input| input.pointer.primary_down())
        {
            self.dragging_asset_path = None;
        }
    }

    pub fn is_dragging_asset(&self) -> bool {
        self.dragging_asset_path.is_some()
    }

    /// Consume the currently dragged asset path. Returns `None` if nothing is being dragged.
    pub fn take_dragging_asset_path(&mut self) -> Option<PathBuf> {
        self.dragging_asset_path.take()
    }

    fn handle_shortcuts(&mut self, ui: &Ui) {
        let wants_rename = ui.ctx().input(|i| i.key_pressed(egui::Key::F2));
        if wants_rename && self.rename_state.is_none() {
            if let Some(path) = self.selected_path.clone() {
                self.start_rename(path);
            }
        }
    }

    fn content_grid_ui(&mut self, ui: &mut Ui, settings: &EditorSettings) {
        let cell_width = (settings.content_icon_size + 56.0).max(110.0);
        let cell_height = (settings.content_icon_size + 64.0).max(110.0);
        let columns = ((ui.available_width() / cell_width).floor() as usize).max(1);
        let entries_snapshot = self.entries.clone();

        for row in entries_snapshot.chunks(columns) {
            ui.horizontal(|ui| {
                for entry in row {
                    ui.push_id(&entry.relative_path, |ui| {
                        self.draw_content_entry(ui, entry, cell_width, cell_height, settings);
                    });
                }
            });
            ui.add_space(6.0);
        }
    }

    fn draw_content_entry(
        &mut self,
        ui: &mut Ui,
        entry: &ContentEntry,
        cell_width: f32,
        cell_height: f32,
        settings: &EditorSettings,
    ) {
        let selected = self.selected_path.as_ref() == Some(&entry.full_path);
        let is_cut = self
            .clipboard
            .as_ref()
            .map(|clipboard| clipboard.cut && clipboard.path == entry.full_path)
            .unwrap_or(false);
        let is_renaming = self
            .rename_state
            .as_ref()
            .map(|state| state.path == entry.full_path)
            .unwrap_or(false);

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(cell_width, cell_height),
            egui::Sense::click_and_drag(),
        );
        let visuals = ui.visuals();
        let fill = if selected {
            visuals.selection.bg_fill
        } else if response.hovered() {
            visuals.widgets.hovered.bg_fill
        } else {
            visuals.widgets.inactive.bg_fill
        };
        let fill = if is_cut {
            fill.gamma_multiply(0.45)
        } else {
            fill
        };
        ui.painter().rect_filled(rect, 8.0, fill);

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(egui::vec2(6.0, 6.0)))
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        let content_tint = if is_cut {
            Color32::from_white_alpha(110)
        } else {
            Color32::WHITE
        };

        self.draw_content_preview(&mut child, entry, settings, content_tint);
        child.add_space(8.0);

        if is_renaming {
            if let Some(rename_state) = self.rename_state.as_mut() {
                let text_edit = egui::TextEdit::singleline(&mut rename_state.buffer)
                    .desired_width(cell_width - 18.0);
                let edit_response = child.add(text_edit);
                if rename_state.request_focus {
                    edit_response.request_focus();
                    rename_state.request_focus = false;
                }

                let confirm = edit_response.lost_focus()
                    && child.ctx().input(|i| i.key_pressed(egui::Key::Enter));
                let cancel = child.ctx().input(|i| i.key_pressed(egui::Key::Escape));

                if confirm {
                    self.commit_rename(settings);
                } else if cancel {
                    self.rename_state = None;
                }
            }
        } else {
            child.add(
                egui::Label::new(
                    RichText::new(&entry.name)
                        .color(content_tint)
                        .text_style(egui::TextStyle::Small)
                        .strong(),
                )
                .wrap(),
            );
        }

        if response.clicked() {
            self.selected_path = Some(entry.full_path.clone());
        }
        if response.drag_started() || response.dragged() {
            self.selected_path = Some(entry.full_path.clone());
            self.dragging_asset_path = Some(entry.full_path.clone());
        }
        if response.double_clicked() {
            self.selected_path = Some(entry.full_path.clone());
            if entry.is_dir() {
                self.pending_open_dir = Some(entry.full_path.clone());
            } else {
                self.open_entry(entry, settings);
            }
        }
        if response.dragged() {
            response
                .clone()
                .on_hover_text(format!("Dragging {}", entry.name));
        }

        let entry_clone = entry.clone();
        response.context_menu(|ui| {
            self.selected_path = Some(entry_clone.full_path.clone());
            if !entry_clone.is_dir() && ui.button("Edit").clicked() {
                self.selected_path = Some(entry_clone.full_path.clone());
                self.edit_file(&entry_clone, settings);
                ui.close();
            }
            if entry_clone.is_dir() && ui.button("Open Folder").clicked() {
                self.selected_path = Some(entry_clone.full_path.clone());
                self.pending_open_dir = Some(entry_clone.full_path.clone());
                ui.close();
            }
            if ui.button("Copy").clicked() {
                self.copy_entry(entry_clone.full_path.clone(), false);
                ui.close();
            }
            if ui.button("Cut").clicked() {
                self.copy_entry(entry_clone.full_path.clone(), true);
                ui.close();
            }
            if ui
                .add_enabled(self.clipboard.is_some(), egui::Button::new("Paste"))
                .clicked()
            {
                self.paste_into(target_directory(&entry_clone.full_path), settings);
                ui.close();
            }
            ui.separator();
            self.live_rust_menu_ui(ui, &target_directory(&entry_clone.full_path), settings);
            if ui.button("Create Folder").clicked() {
                self.create_folder_in(&target_directory(&entry_clone.full_path), settings);
                ui.close();
            }
            if ui.button("Create WGSL Shader").clicked() {
                self.create_wgsl_shader(&target_directory(&entry_clone.full_path), settings);
                ui.close();
            }
            if ui.button("Create World").clicked() {
                self.create_world_in(&target_directory(&entry_clone.full_path), settings);
                ui.close();
            }
            if ui.button("Import").clicked() {
                self.import_assets_into(&target_directory(&entry_clone.full_path), settings);
                ui.close();
            }
            ui.separator();
            if ui.button("Rename").clicked() {
                self.selected_path = Some(entry_clone.full_path.clone());
                self.start_rename(entry_clone.full_path.clone());
                ui.close();
            }
            if ui.button("Delete").clicked() {
                self.delete_entry(entry_clone.full_path.clone(), settings);
                ui.close();
            }
        });
    }

    fn draw_content_preview(
        &mut self,
        ui: &mut Ui,
        entry: &ContentEntry,
        settings: &EditorSettings,
        tint: Color32,
    ) {
        let preview_size = Vec2::splat(settings.content_icon_size);
        if let Some(texture) = self.preview_texture(ui, entry) {
            let frame_rect = ui.allocate_exact_size(preview_size, egui::Sense::hover()).0;
            ui.painter().rect_filled(
                frame_rect,
                6.0,
                ui.visuals().extreme_bg_color.gamma_multiply(0.6),
            );
            let image_bounds = frame_rect.shrink(2.0);
            let source_size = texture.size_vec2();
            let fit_scale =
                (image_bounds.width() / source_size.x).min(image_bounds.height() / source_size.y);
            let fitted_size = source_size * fit_scale.max(0.0);
            let image_rect = egui::Rect::from_center_size(image_bounds.center(), fitted_size);
            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                tint,
            );
            return;
        }

        let icon = self.icon_for(entry, settings);
        ui.add(
            egui::Image::new(icon)
                .fit_to_exact_size(preview_size)
                .tint(tint)
                .sense(egui::Sense::hover()),
        );
    }

    fn preview_texture(&mut self, ui: &Ui, entry: &ContentEntry) -> Option<&TextureHandle> {
        if entry.kind != AssetKind::ImageFile {
            return None;
        }

        if !self.image_previews.contains_key(&entry.full_path) {
            let texture_name = format!(
                "content_browser_preview_{}",
                entry.relative_path.replace(['\\', '/', '.', ' '], "_")
            );
            if let Ok(texture) =
                load_texture_from_path(ui.ctx(), &texture_name, &entry.full_path, Some(256))
            {
                self.image_previews.insert(entry.full_path.clone(), texture);
            }
        }

        self.image_previews.get(&entry.full_path)
    }

    fn asset_context_menu(&mut self, ui: &mut Ui, target_dir: PathBuf, settings: &EditorSettings) {
        if ui.button("Copy").clicked() {
            if let Some(path) = self.selected_path.clone() {
                self.copy_entry(path, false);
            }
            ui.close();
        }
        if ui.button("Cut").clicked() {
            if let Some(path) = self.selected_path.clone() {
                self.copy_entry(path, true);
            }
            ui.close();
        }
        if ui
            .add_enabled(self.clipboard.is_some(), egui::Button::new("Paste"))
            .clicked()
        {
            self.paste_into(target_dir.clone(), settings);
            ui.close();
        }
        ui.separator();
        self.live_rust_menu_ui(ui, &target_dir, settings);
        if ui.button("Create Folder").clicked() {
            self.create_folder_in(&target_dir, settings);
            ui.close();
        }
        if ui.button("Create WGSL Shader").clicked() {
            self.create_wgsl_shader(&target_dir, settings);
            ui.close();
        }
        if ui.button("Create World").clicked() {
            self.create_world_in(&target_dir, settings);
            ui.close();
        }
        if ui.button("Import").clicked() {
            self.import_assets_into(&target_dir, settings);
            ui.close();
        }
    }

    fn asset_area_context_menu(
        &mut self,
        ui: &mut Ui,
        target_dir: PathBuf,
        settings: &EditorSettings,
    ) {
        if ui
            .add_enabled(self.clipboard.is_some(), egui::Button::new("Paste"))
            .clicked()
        {
            self.paste_into(target_dir.clone(), settings);
            ui.close();
        }
        ui.separator();
        self.live_rust_menu_ui(ui, &target_dir, settings);
        if ui.button("Create Folder").clicked() {
            self.create_folder_in(&target_dir, settings);
            ui.close();
        }
        if ui.button("Create WGSL Shader").clicked() {
            self.create_wgsl_shader(&target_dir, settings);
            ui.close();
        }
        if ui.button("Create World").clicked() {
            self.create_world_in(&target_dir, settings);
            ui.close();
        }
        if ui.button("Import").clicked() {
            self.import_assets_into(&target_dir, settings);
            ui.close();
        }
    }

    fn folder_area_context_menu(&mut self, ui: &mut Ui, settings: &EditorSettings) {
        if ui.button("Create Folder").clicked() {
            self.create_folder_in(&self.current_dir.clone(), settings);
            ui.close();
        }
        if ui.button("Create World").clicked() {
            self.create_world_in(&self.current_dir.clone(), settings);
            ui.close();
        }
        if ui
            .add_enabled(self.clipboard.is_some(), egui::Button::new("Paste"))
            .clicked()
        {
            self.paste_into(self.current_dir.clone(), settings);
            ui.close();
        }
        if self.current_dir != self.project_root && ui.button("Delete Folder").clicked() {
            self.delete_entry(self.current_dir.clone(), settings);
            ui.close();
        }
    }

    fn folder_entry_context_menu(
        &mut self,
        ui: &mut Ui,
        directory: PathBuf,
        settings: &EditorSettings,
    ) {
        if ui.button("Create Folder").clicked() {
            self.create_folder_in(&directory, settings);
            ui.close();
        }
        if ui.button("Create World").clicked() {
            self.create_world_in(&directory, settings);
            ui.close();
        }
        if ui
            .add_enabled(self.clipboard.is_some(), egui::Button::new("Paste"))
            .clicked()
        {
            self.paste_into(directory.clone(), settings);
            ui.close();
        }
        if directory != self.project_root && ui.button("Delete Folder").clicked() {
            self.delete_entry(directory, settings);
            ui.close();
        }
    }

    fn folder_tree_ui(
        &mut self,
        ui: &mut Ui,
        root: &Path,
        depth: usize,
        settings: &EditorSettings,
        next_dir: &mut Option<PathBuf>,
    ) {
        let directories = collect_subdirectories(root, settings.show_hidden_files);

        for directory in directories {
            let name = directory
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| directory.display().to_string());
            let selected = self.current_dir == directory;
            let expanded = self.expanded_dirs.contains(&directory);
            let has_children = has_visible_subdirectories(&directory, settings.show_hidden_files);
            let is_empty = is_directory_empty(&directory, settings.show_hidden_files);
            let icons = self.icons.as_ref().expect("icons must be initialized");
            let folder_icon = if expanded && has_children {
                icons.folder_open.clone()
            } else if is_empty {
                icons.folder_empty.clone()
            } else {
                icons.folder.clone()
            };

            let row = ui.horizontal(|ui| {
                ui.add_space((depth as f32) * 12.0);
                let folder_response = ui.add(
                    egui::Image::new(&folder_icon)
                        .fit_to_exact_size(egui::vec2(18.0, 18.0))
                        .sense(egui::Sense::click()),
                );
                if folder_response.clicked() {
                    if has_children {
                        if expanded {
                            self.expanded_dirs.remove(&directory);
                        } else {
                            self.expanded_dirs.insert(directory.clone());
                        }
                    } else {
                        self.ensure_directory_expanded(&directory);
                        *next_dir = Some(directory.clone());
                    }
                }

                let label = ui.selectable_label(selected, name);
                if label.clicked() {
                    self.ensure_directory_expanded(&directory);
                    *next_dir = Some(directory.clone());
                }
            });

            row.response.context_menu(|ui| {
                self.folder_entry_context_menu(ui, directory.clone(), settings);
            });

            if expanded {
                self.folder_tree_ui(ui, &directory, depth + 1, settings, next_dir);
            }
        }
    }

    fn start_rename(&mut self, path: PathBuf) {
        let buffer = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        self.rename_state = Some(RenameState {
            path,
            buffer,
            request_focus: true,
        });
    }

    fn commit_rename(&mut self, settings: &EditorSettings) {
        let Some(rename_state) = self.rename_state.take() else {
            return;
        };

        let trimmed = rename_state.buffer.trim();
        if trimmed.is_empty() {
            self.last_message = Some("Rename cancelled: empty name.".to_string());
            return;
        }

        let Some(parent) = rename_state.path.parent() else {
            self.last_message = Some("Rename failed: invalid parent path.".to_string());
            return;
        };

        let new_path = parent.join(trimmed);
        if new_path == rename_state.path {
            return;
        }

        match fs::rename(&rename_state.path, &new_path) {
            Ok(()) => {
                self.last_message = Some(format!("Renamed to {trimmed}"));
                self.selected_path = Some(new_path.clone());
                if self.current_dir == rename_state.path {
                    self.current_dir = new_path.clone();
                }
                self.refresh(settings);
            }
            Err(error) => {
                self.last_message = Some(format!("Rename failed: {error}"));
            }
        }
    }

    fn copy_entry(&mut self, path: PathBuf, cut: bool) {
        self.selected_path = Some(path.clone());
        self.clipboard = Some(ClipboardEntry { path, cut });
        self.last_message = Some(if cut {
            "Marked item for cut.".to_string()
        } else {
            "Copied item.".to_string()
        });
    }

    fn paste_into(&mut self, target: PathBuf, settings: &EditorSettings) {
        let Some(clipboard) = self.clipboard.clone() else {
            return;
        };

        let destination_dir = if target.is_dir() {
            target
        } else {
            target
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| self.current_dir.clone())
        };
        let Some(file_name) = clipboard.path.file_name() else {
            self.last_message = Some("Paste failed: invalid source path.".to_string());
            return;
        };
        let destination = destination_dir.join(file_name);

        if destination == clipboard.path {
            self.last_message =
                Some("Paste skipped: source and destination are the same.".to_string());
            return;
        }
        if destination.exists() {
            self.last_message = Some("Paste failed: destination already exists.".to_string());
            return;
        }

        let result = if clipboard.cut {
            fs::rename(&clipboard.path, &destination)
        } else if clipboard.path.is_dir() {
            copy_directory_recursive(&clipboard.path, &destination)
        } else {
            fs::copy(&clipboard.path, &destination).map(|_| ())
        };

        match result {
            Ok(()) => {
                if clipboard.cut {
                    self.clipboard = None;
                }
                self.selected_path = Some(destination);
                self.refresh(settings);
                self.last_message = Some("Paste completed.".to_string());
            }
            Err(error) => {
                self.last_message = Some(format!("Paste failed: {error}"));
            }
        }
    }

    fn delete_entry(&mut self, path: PathBuf, settings: &EditorSettings) {
        let result = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };

        match result {
            Ok(()) => {
                if self.selected_path.as_ref() == Some(&path) {
                    self.selected_path = None;
                }
                if self
                    .clipboard
                    .as_ref()
                    .map(|clipboard| clipboard.path == path)
                    .unwrap_or(false)
                {
                    self.clipboard = None;
                }
                self.expanded_dirs.remove(&path);
                self.refresh(settings);
                self.last_message = Some("Deleted item.".to_string());
            }
            Err(error) => {
                self.last_message = Some(format!("Delete failed: {error}"));
            }
        }
    }

    fn create_new_rust_file(
        &mut self,
        target_dir: &Path,
        kind: RustFileKind,
        settings: &EditorSettings,
    ) {
        let base_name = match kind {
            RustFileKind::Empty => "NewFile",
            RustFileKind::Script => "NewObject",
            RustFileKind::Archetype => "NewObjectDef",
        };
        let path = unique_file_path(target_dir, base_name, "rs");
        let content = match kind {
            RustFileKind::Empty => String::new(),
            RustFileKind::Script => object_script_template(
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("NewObject"),
            ),
            RustFileKind::Archetype => object_archetype_template(
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("NewObjectDef"),
            ),
        };

        match fs::write(&path, content) {
            Ok(()) => {
                self.refresh(settings);
                self.selected_path = Some(path.clone());
                self.last_message = Some(format!("Created {}.", path.display()));
            }
            Err(error) => {
                self.last_message = Some(format!("Create file failed: {error}"));
            }
        }
    }

    fn create_folder_in(&mut self, target_dir: &Path, settings: &EditorSettings) {
        let path = unique_directory_path(target_dir, "NewFolder");
        match fs::create_dir_all(&path) {
            Ok(()) => {
                self.ensure_directory_expanded(target_dir);
                self.refresh(settings);
                self.last_message = Some(format!("Created folder {}.", path.display()));
            }
            Err(error) => {
                self.last_message = Some(format!("Create folder failed: {error}"));
            }
        }
    }

    fn create_wgsl_shader(&mut self, target_dir: &Path, settings: &EditorSettings) {
        let path = unique_file_path(target_dir, "new_shader", "wgsl");
        match fs::write(&path, wgsl_shader_template()) {
            Ok(()) => {
                self.refresh(settings);
                self.selected_path = Some(path.clone());
                self.last_message = Some(format!("Created shader {}.", path.display()));
            }
            Err(error) => {
                self.last_message = Some(format!("Create shader failed: {error}"));
            }
        }
    }

    fn create_world_in(&mut self, target_dir: &Path, settings: &EditorSettings) {
        let path = unique_world_path(target_dir, "new_world");
        match runvy_project::save_world(&path, &runvy_project::create_empty_world().borrow()) {
            Ok(()) => {
                self.refresh(settings);
                self.selected_path = Some(path.clone());
                self.pending_open_world = Some(path.clone());
                self.last_message = Some(format!("Created world {}.", path.display()));
            }
            Err(error) => {
                self.last_message = Some(format!("Create world failed: {error}"));
            }
        }
    }

    fn import_assets_into(&mut self, target_dir: &Path, settings: &EditorSettings) {
        let Some(paths) = FileDialog::new()
            .set_directory(target_dir)
            .add_filter(
                "Supported assets",
                &[
                    "png", "jpg", "jpeg", "svg", "ogg", "wav", "ron", "rs", "wgsl",
                ],
            )
            .add_filter("Images", &["png", "jpg", "jpeg", "svg"])
            .add_filter("Audio", &["ogg", "wav"])
            .add_filter("Code", &["rs", "wgsl"])
            .add_filter("Worlds", &["ron"])
            .add_filter("3D Models", &["r3m", "glb"])
            .pick_files()
        else {
            return;
        };

        let mut imported = 0usize;
        for source in paths {
            let Some(name) = source.file_name() else {
                continue;
            };
            let destination = target_dir.join(name);
            if destination.exists() {
                continue;
            }
            if fs::copy(&source, &destination).is_ok() {
                imported += 1;
            }
        }

        self.refresh(settings);
        self.last_message = Some(format!("Imported {imported} asset(s)."));
    }

    fn open_entry(&mut self, entry: &ContentEntry, settings: &EditorSettings) {
        if entry.kind == AssetKind::WorldFile {
            self.pending_open_world = Some(entry.full_path.clone());
            return;
        }

        if entry.kind == AssetKind::ImageFile {
            self.open_with_system_default(&entry.full_path);
            return;
        }

        self.edit_file(entry, settings);
    }

    fn edit_file(&mut self, entry: &ContentEntry, settings: &EditorSettings) {
        if entry.is_dir() {
            return;
        }

        let executable = settings.external_editor_executable.trim();
        if executable.is_empty() {
            self.last_message = Some("External editor is not configured.".to_string());
            return;
        }

        let file = entry.full_path.to_string_lossy().to_string();
        let args: Vec<String> = settings
            .external_editor_args
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.replace("{file}", &file))
            .collect();

        match Command::new(executable).args(args).spawn() {
            Ok(_) => {
                self.last_message = Some(format!("Opened {} in external editor.", entry.name));
            }
            Err(error) => {
                self.last_message = Some(format!("Edit failed: {error}"));
            }
        }
    }

    fn open_with_system_default(&mut self, path: &Path) {
        let result = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "start", "", &path.to_string_lossy()])
                .spawn()
        } else if cfg!(target_os = "macos") {
            Command::new("open").arg(path).spawn()
        } else {
            Command::new("xdg-open").arg(path).spawn()
        };

        match result {
            Ok(_) => {
                self.last_message = Some(format!("Opened {}.", path.display()));
            }
            Err(error) => {
                self.last_message = Some(format!("Open failed: {error}"));
            }
        }
    }

    fn ensure_icons(&mut self, ctx: &egui::Context) {
        if self.icons.is_some() {
            return;
        }

        self.icons = Some(ContentBrowserIcons {
            folder: load_editor_icon(ctx, "content_browser_folder_icon", "folder"),
            folder_empty: load_editor_icon(
                ctx,
                "content_browser_folder_empty_icon",
                "folder-empty",
            ),
            folder_open: load_editor_icon(ctx, "content_browser_folder_open_icon", "folder-open"),
            file: load_editor_icon(ctx, "content_browser_file_icon", "file"),
            rust_file: load_editor_icon(ctx, "content_browser_rust_file_icon", "rust-file"),
            world_file: load_editor_icon(ctx, "content_browser_world_file_icon", "world"),
            image_file: load_editor_icon(ctx, "content_browser_image_file_icon", "image"),
            audio_file: load_editor_icon(ctx, "content_browser_audio_file_icon", "audio"),
            shader_file: load_editor_icon(ctx, "content_browser_shader_file_icon", "wgsl"),
            r3m_file: load_editor_icon(ctx, "content_browser_r3m_file_icon", "r3m"),
        });
    }

    fn icon_for(&self, entry: &ContentEntry, settings: &EditorSettings) -> &TextureHandle {
        let icons = self.icons.as_ref().expect("icons must be initialized");
        match entry.kind {
            AssetKind::Directory => {
                if is_directory_empty(&entry.full_path, settings.show_hidden_files) {
                    &icons.folder_empty
                } else {
                    &icons.folder
                }
            }
            AssetKind::RustFile => &icons.rust_file,
            AssetKind::WorldFile => &icons.world_file,
            AssetKind::ImageFile => &icons.image_file,
            AssetKind::AudioFile => &icons.audio_file,
            AssetKind::ShaderFile => &icons.shader_file,
            AssetKind::GenericFile => &icons.file,
            AssetKind::Runvy3DModelFile => &icons.r3m_file,
            AssetKind::PrefabFile => todo!(),
            AssetKind::UiFile => &icons.file,
        }
    }

    fn ensure_directory_expanded(&mut self, dir: &Path) {
        let mut current = Some(dir);
        while let Some(path) = current {
            if path.starts_with(&self.project_root) {
                self.expanded_dirs.insert(path.to_path_buf());
            }
            current = path.parent();
        }
    }

    fn live_rust_menu_ui(&mut self, ui: &mut Ui, target_dir: &Path, settings: &EditorSettings) {
        ui.menu_button("Live Rust", |ui| {
            if ui.button("New Empty Rust File").clicked() {
                self.create_new_rust_file(target_dir, RustFileKind::Empty, settings);
                ui.close();
            }
            if ui.button("New Rust Script").clicked() {
                self.create_new_rust_file(target_dir, RustFileKind::Script, settings);
                ui.close();
            }
            if ui.button("New Rust Object Definition").clicked() {
                self.create_new_rust_file(target_dir, RustFileKind::Archetype, settings);
                ui.close();
            }
        });
    }
}

#[derive(Clone, Copy)]
enum RustFileKind {
    Empty,
    Script,
    Archetype,
}

fn paint_splitter(ui: &Ui, rect: egui::Rect, response: &egui::Response) {
    let fill = if response.dragged() {
        ui.visuals().widgets.active.bg_fill.gamma_multiply(1.2)
    } else if response.hovered() {
        ui.visuals().widgets.hovered.bg_fill.gamma_multiply(1.35)
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    ui.painter()
        .rect_filled(rect, 3.0, fill.gamma_multiply(0.45));
    let line_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(4.0, rect.height()));
    ui.painter()
        .rect_filled(line_rect.shrink2(egui::vec2(0.0, 4.0)), 3.0, fill);
}

fn collect_directory_entries(
    project_root: &Path,
    current_dir: &Path,
    show_hidden_files: bool,
) -> Vec<ContentEntry> {
    let mut entries = Vec::new();
    let Ok(read_dir) = fs::read_dir(current_dir) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if is_ignored_path(&path, show_hidden_files) {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let relative_path = path
            .strip_prefix(project_root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());
        let kind = classify_asset_kind(&path);

        entries.push(ContentEntry {
            name,
            relative_path,
            full_path: path,
            kind,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    entries
}

fn collect_subdirectories(root: &Path, show_hidden_files: bool) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut directories: Vec<PathBuf> = read_dir
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| !is_ignored_path(path, show_hidden_files))
        .collect();
    directories.sort();
    directories
}

fn has_visible_subdirectories(path: &Path, show_hidden_files: bool) -> bool {
    let Ok(read_dir) = fs::read_dir(path) else {
        return false;
    };

    read_dir.flatten().any(|entry| {
        let path = entry.path();
        path.is_dir() && !is_ignored_path(&path, show_hidden_files)
    })
}

fn is_directory_empty(path: &Path, show_hidden_files: bool) -> bool {
    let Ok(read_dir) = fs::read_dir(path) else {
        return true;
    };

    !read_dir
        .flatten()
        .map(|entry| entry.path())
        .any(|path| !is_ignored_path(&path, show_hidden_files))
}

fn classify_asset_kind(path: &Path) -> AssetKind {
    if path.is_dir() {
        return AssetKind::Directory;
    }

    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some(_) if is_ui_file(path) => AssetKind::UiFile,
        Some(_) if is_world_file(path) => AssetKind::WorldFile,
        Some("rs") => AssetKind::RustFile,
        Some("png" | "jpg" | "jpeg" | "svg") => AssetKind::ImageFile,
        Some("ogg" | "wav" | "mp3") => AssetKind::AudioFile,
        Some("wgsl") => AssetKind::ShaderFile,
        _ => AssetKind::GenericFile,
    }
}

fn is_ui_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".ui.ron"))
        .unwrap_or(false)
}

fn is_world_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".world.ron"))
        .unwrap_or(false)
}

fn is_ignored_path(path: &Path, show_hidden_files: bool) -> bool {
    let file_name = path.file_name().and_then(|name| name.to_str());
    if matches!(file_name, Some(".git" | "target" | ".obsidian" | ".proj")) {
        return true;
    }
    if !show_hidden_files {
        if let Some(name) = file_name {
            return name.starts_with('.');
        }
    }
    false
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn unique_file_path(directory: &Path, base_name: &str, extension: &str) -> PathBuf {
    let mut index = 0usize;
    loop {
        let file_name = if index == 0 {
            format!("{base_name}.{extension}")
        } else {
            format!("{base_name}{index}.{extension}")
        };
        let path = directory.join(file_name);
        if !path.exists() {
            return path;
        }
        index += 1;
    }
}

fn unique_world_path(directory: &Path, base_name: &str) -> PathBuf {
    let mut index = 0usize;
    loop {
        let file_name = if index == 0 {
            format!("{base_name}.world.ron")
        } else {
            format!("{base_name}{index}.world.ron")
        };
        let path = directory.join(file_name);
        if !path.exists() {
            return path;
        }
        index += 1;
    }
}

fn unique_directory_path(directory: &Path, base_name: &str) -> PathBuf {
    let mut index = 0usize;
    loop {
        let file_name = if index == 0 {
            base_name.to_string()
        } else {
            format!("{base_name}{index}")
        };
        let path = directory.join(file_name);
        if !path.exists() {
            return path;
        }
        index += 1;
    }
}

fn target_directory(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

fn object_script_template(type_name: &str) -> String {
    format!(
        r#"use runvy_engine::{{
    runvy_core::{{
        components::Transform,
        ocs::{{Script, ScriptContext}},
        Vec3,
    }},
    RunvyScript,
}};

#[derive(RunvyScript)]
pub struct {type_name} {{
    #[serialize_field]
    speed: f32,
    #[serialize_field]
    direction: Vec3,
}}

impl {type_name} {{
    pub fn new() -> Self {{
        Self {{
            speed: 1.0,
            direction: Vec3::ZERO,
        }}
    }}
}}

impl Default for {type_name} {{
    fn default() -> Self {{
        Self::new()
    }}
}}

impl Script for {type_name} {{
    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {{
        let movement = self.direction.normalize_or_zero() * self.speed * dt;

        if let Some(transform) = ctx.get_component_mut::<Transform>() {{
            transform.position += movement;
        }}
    }}
}}
"#,
    )
}

fn object_archetype_template(type_name: &str) -> String {
    let type_name_snake = type_name
        .chars()
        .enumerate()
        .flat_map(|(index, ch)| {
            let mut chars = Vec::new();
            if ch.is_uppercase() && index > 0 {
                chars.push('_');
            }
            chars.extend(ch.to_lowercase());
            chars
        })
        .collect::<String>();

    format!(
        "use runvy_engine::{{\n    runvy_core::ocs::ObjectBuilder,\n    ObjectDef, RunvyObjectDef,\n}};\n\n#[derive(RunvyObjectDef)]\n#[runvy(name = \"{type_name_snake}\")]\npub struct {type_name};\n\nimpl ObjectDef for {type_name} {{\n    fn build(object: &mut ObjectBuilder) {{\n        object.name(\"{type_name}\");\n    }}\n}}\n",
        type_name_snake = type_name_snake,
    )
}

fn wgsl_shader_template() -> &'static str {
    "@group(0) @binding(0)\nvar<uniform> globals: mat4x4<f32>;\n\nstruct VertexInput {\n    @location(0) position: vec3<f32>,\n    @location(1) uv: vec2<f32>,\n};\n\nstruct VertexOutput {\n    @builtin(position) clip_position: vec4<f32>,\n    @location(0) uv: vec2<f32>,\n};\n\n@vertex\nfn vs_main(input: VertexInput) -> VertexOutput {\n    var out: VertexOutput;\n    out.clip_position = globals * vec4<f32>(input.position, 1.0);\n    out.uv = input.uv;\n    return out;\n}\n\n@fragment\nfn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {\n    return vec4<f32>(input.uv, 0.5, 1.0);\n}\n"
}

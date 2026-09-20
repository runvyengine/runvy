mod app_handler;
mod editor_commands;
mod helpers;
mod placeables;
mod project;
mod ui;
pub mod ui_editor;
mod viewport;
mod world_ops;

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use egui::{Align2, Layout, RichText};
use egui_wgpu::{Renderer as EguiRenderer, RendererOptions, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use rfd::FileDialog;
use runvy_core::components::{
    ActiveCamera, AudioListener, AudioSource, BackgroundMode, Camera, Collider2D,
    ComponentRuntimeKind, CursorInteractable, DirectionalLight, Mesh, MeshRenderer,
    ObjectDefinitionInstance, PhysicsCollision, SerializedTypeEntry, SerializedTypeKind,
    SerializedTypeStorage, SpriteRenderer, Tilemap, TilemapRenderer, Transform, UiRenderer,
};
use runvy_core::glam::{EulerRot, Mat4, Quat, Vec2, Vec3};
use runvy_core::ocs::{Object, ObjectId};
use runvy_core::registry::{RegisteredTypeKind, RuntimeRegistry};
use runvy_core::World;
use runvy_engine::Engine;
use runvy_project::{
    create_empty_project, create_empty_world, ensure_editor_bridge_files,
    ensure_release_windows_subsystem, load_project, load_world_with_runtime_registry, save_world,
    PlaceableObjectDescriptor, PlaceableObjectRecord, ProjectMetadataSnapshot,
    ProjectPaths, ProjectRegisteredTypeKind, ProjectRegisteredTypeRecord, WorldObjectAsset,
};

use runvy_render::{RenderTarget, Renderer};
use runvy_render_api::RenderQueue;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
#[cfg(target_os = "windows")]
use winit::platform::windows::{WindowAttributesExtWindows, WindowExtWindows};
use winit::window::{Window, WindowId};

use crate::content_browser::ContentBrowserState;
use crate::editor_camera::EditorCameraController;
use crate::editor_settings::EditorSettings;
use crate::inspector::{inspector_ui, TilePaintMode, TilePaintToolState};
use crate::style;

pub fn run(project_path: Option<PathBuf>) -> Result<(), winit::error::EventLoopError> {
    let event_loop = EventLoop::new()?;
    let mut app = EditorApp::new(project_path);
    event_loop.run_app(&mut app)
}

struct PanelState {
    hierarchy: bool,
    inspector: bool,
    bottom_bar: bool,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            hierarchy: true,
            inspector: true,
            bottom_bar: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BottomTab {
    ContentBrowser,
    Console,
}

struct ProjectDialogState {
    open: bool,
    name: String,
    location: String,
    error: Option<String>,
}

#[derive(Clone)]
struct ProjectSession {
    project: ProjectPaths,
    current_world_path: Option<PathBuf>,
}

struct ProjectLoadResult {
    project: ProjectPaths,
    metadata: ProjectMetadataSnapshot,
}

struct ProjectVersionPromptState {
    pending_result: ProjectLoadResult,
    project_root: PathBuf,
    project_name: String,
    project_version: String,
    editor_version: String,
}

struct ObjectClipboard {
    asset: WorldObjectAsset,
    cut_id: Option<ObjectId>,
}

struct PlaceObjectState {
    objects: Vec<PlaceableObjectDescriptor>,
    templates: HashMap<String, WorldObjectAsset>,
    registered_types: Vec<ProjectRegisteredTypeRecord>,
    source_stamp: Option<SystemTime>,
    pending_stamp: Option<SystemTime>,
    refresh_in_progress: bool,
    refresh_result: Option<Receiver<Result<ProjectMetadataSnapshot, String>>>,
}

impl Default for PlaceObjectState {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            templates: HashMap::new(),
            registered_types: Vec::new(),
            source_stamp: None,
            pending_stamp: None,
            refresh_in_progress: false,
            refresh_result: None,
        }
    }
}

#[derive(Clone, Copy)]
enum GizmoHandleKind {
    Translate,
    ScaleX,
    ScaleY,
    Rotate,
    PositionAxis(usize),
    RotationAxis(usize),
    ScaleAxis(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewportEditMode {
    Position,
    Rotation,
    Scale,
}

struct ViewportDragState {
    object_id: ObjectId,
    kind: GizmoHandleKind,
    offset: Vec2,
    start_position: Vec3,
    start_rotation_z: f32,
    start_pointer_angle: f32,
}

pub struct EditorApp<'window> {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer<'window>>,
    egui_state: Option<EguiWinitState>,
    egui_renderer: Option<EguiRenderer>,
    egui_ctx: egui::Context,
    runtime_engine: Engine,

    world: Rc<RefCell<World>>,
    scene_queue: RenderQueue,
    selection: Option<ObjectId>,
    selected_objects: HashSet<ObjectId>,
    content_browser: ContentBrowserState,
    panels: PanelState,
    settings: EditorSettings,
    editor_settings_open: bool,
    project_settings_open: bool,
    build_settings_open: bool,
    project_dialog: ProjectDialogState,
    project_session: Option<ProjectSession>,
    project_version_prompt: Option<ProjectVersionPromptState>,
    startup_project_path: Option<PathBuf>,
    runtime_process: Option<Child>,
    build_process: Option<Child>,
    project_load: Option<Receiver<Result<ProjectLoadResult, String>>>,
    place_object: PlaceObjectState,
    hierarchy_clipboard: Option<ObjectClipboard>,
    hierarchy_dragging_object: Option<ObjectId>,
    hierarchy_expanded: HashSet<ObjectId>,
    hierarchy_renaming: Option<ObjectId>,
    hierarchy_rename_buffer: String,
    pub console: runvy_core::Console,
    console_focus_requested: bool,
    output_tx: Sender<String>,
    output_rx: Receiver<String>,
    log_file: Option<std::fs::File>,

    editor_camera: EditorCameraController,
    viewport_target: Option<RenderTarget>,
    viewport_texture_id: Option<egui::TextureId>,
    pending_viewport_size: (u32, u32),
    viewport_hovered: bool,
    viewport_camera: Option<Camera>,
    viewport_edit_mode: ViewportEditMode,
    gizmo_enabled: bool,
    show_component_icons: bool,
    viewport_component_icon_size: f32,
    show_viewport_grid: bool,
    tile_paint: TilePaintToolState,
    snap_enabled: bool,
    snap_step: f32,
    gizmo_drag: Option<ViewportDragState>,
    modifiers: ModifiersState,
    bottom_tab: BottomTab,
    bottom_bar_height: f32,
    inspector_panel_width: f32,
    view_settings_open: bool,
    rendering_settings_open: bool,
    gizmo_settings_open: bool,

    pub ui_editor: ui_editor::UiEditorPanel,

    status_line: String,
    last_frame_time: Instant,

    // Global log (outside project scope)
    global_log_path: PathBuf,
    global_log_file: Option<std::fs::File>,
    // Project load error (persistent until dismissed)
    project_load_error: Option<String>,
}

impl<'window> EditorApp<'window> {
    fn world_object_ids(&self) -> Vec<ObjectId> {
        let mut ids = self.world.borrow().query::<Transform>();
        ids.sort_unstable();
        ids
    }

    fn first_object_id(&self) -> Option<ObjectId> {
        self.world.borrow().find_first_with::<Transform>()
    }

    fn select_object(&mut self, object_id: ObjectId, additive: bool) {
        if additive {
            if self.selected_objects.contains(&object_id) {
                self.selected_objects.remove(&object_id);
                if self.selection == Some(object_id) {
                    self.selection = self.selected_objects.iter().next().copied();
                }
                return;
            }
        } else {
            self.selected_objects.clear();
        }
        self.selected_objects.insert(object_id);
        self.selection = Some(object_id);
    }

    fn set_primary_selection(&mut self, object_id: Option<ObjectId>) {
        self.selected_objects.clear();
        if let Some(object_id) = object_id {
            self.selected_objects.insert(object_id);
        }
        self.selection = object_id;
    }

    fn clear_selection(&mut self) {
        self.selection = None;
        self.selected_objects.clear();
    }

    pub fn execute_console_input(&mut self) {
        let input = self.console.input_buffer.clone();
        if input.is_empty() {
            return;
        }
        self.console.add_message(format!("> {}", input));
        self.console.push_history(&input);

        let trimmed = input.trim();
        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("").to_lowercase();
        let args: Vec<&str> = parts.collect();

        // Handle "help" specially to include editor commands
        if cmd == "help" {
            if args.is_empty() {
                self.console.try_execute("help"); // show console commands
                self.console.add_message("");
                self.console.add_message("--- Editor Commands ---");
                for (name, desc) in editor_commands::descriptions() {
                    self.console.add_message(format!("  {:<12} {}", name, desc));
                }
            } else {
                let topic = args[0];
                // Check if it's an editor command
                let is_editor = editor_commands::descriptions().iter().any(|(n, _)| *n == topic);
                if is_editor {
                    self.console.add_message(format!("{} - Editor command", topic));
                    self.console.add_message("Usage: see editor documentation.");
                } else {
                    self.console.try_execute(trimmed);
                }
            }
        } else if !self.console.try_execute(trimmed) && !editor_commands::execute(self, &cmd, &args) {
            self.console.add_message(format!(
                "Unknown command: '{}'. Type 'help' for commands.",
                trimmed
            ));
        }

        self.console.input_buffer.clear();
        self.console_focus_requested = true;
    }

    fn new(project_path: Option<PathBuf>) -> Self {
        let startup_root = std::env::current_dir().unwrap_or_default();
        let (output_tx, output_rx) = mpsc::channel();
        let runtime_engine = Engine::new();
        let mut settings = EditorSettings::load();
        if settings.prune_missing_recent_projects() {
            let _ = settings.save();
        }
        let world_rc = helpers::create_preview_world();
        {
            let mut world_mut = world_rc.borrow_mut();
            world_mut.set_runtime_registry(Arc::new(runtime_engine.runtime_registry().clone()));
            world_mut.start(world_rc.clone());
        }
        let selection = world_rc.borrow().find_first_with::<Transform>();
        let selected_objects = selection.into_iter().collect();
        let project_dialog = ProjectDialogState {
            open: false,
            name: "MyGame".to_string(),
            location: dirs::document_dir()
                .unwrap_or_else(|| startup_root.clone())
                .to_string_lossy()
                .to_string(),
            error: None,
        };

        let global_log_path = dirs::home_dir()
            .unwrap_or_else(|| startup_root.clone())
            .join(".runvy_editor")
            .join("runvy-editor.log");
        let global_log_file = (|| -> Option<std::fs::File> {
            let parent = global_log_path.parent()?;
            std::fs::create_dir_all(parent).ok()?;
            std::fs::File::create(&global_log_path).ok()
        })();

        let mut console = runvy_core::Console::new();
        console.add_message("Editor started.");
        console.add_message("Type 'help' for available commands.");
        let editor_names: Vec<&str> = std::iter::once("editor")
            .chain(editor_commands::descriptions().iter().map(|(name, _)| *name))
            .collect();
        console.add_suggestion_names(&editor_names);

        Self {
            console,
            console_focus_requested: false,
            settings,
            editor_settings_open: false,
            project_settings_open: false,
            build_settings_open: false,
            project_dialog,
            project_session: None,
            project_version_prompt: None,
            startup_project_path: project_path,
            runtime_process: None,
            build_process: None,
            project_load: None,
            place_object: PlaceObjectState::default(),
            hierarchy_clipboard: None,
            hierarchy_dragging_object: None,
            hierarchy_expanded: HashSet::new(),
            hierarchy_renaming: None,
            hierarchy_rename_buffer: String::new(),
            window: None,
            renderer: None,
            egui_state: None,
            egui_renderer: None,
            egui_ctx: egui::Context::default(),
            runtime_engine,
            world: world_rc,
            scene_queue: RenderQueue::new(),
            selection,
            selected_objects,
            content_browser: ContentBrowserState::new(
                dirs::document_dir().unwrap_or_else(|| startup_root.clone()),
            ),
            panels: PanelState::default(),
            editor_camera: EditorCameraController::new(),
            viewport_target: None,
            viewport_texture_id: None,
            pending_viewport_size: style::panel_sizes::INITIAL_VIEWPORT,
            viewport_hovered: false,
            viewport_camera: None,
            viewport_edit_mode: ViewportEditMode::Position,
            gizmo_enabled: true,
            show_component_icons: true,
            viewport_component_icon_size: 18.0,
            show_viewport_grid: true,
            tile_paint: TilePaintToolState::default(),
            snap_enabled: false,
            snap_step: 1.0,
            gizmo_drag: None,
            modifiers: ModifiersState::default(),
            bottom_tab: BottomTab::ContentBrowser,
            bottom_bar_height: style::panel_sizes::BOTTOM_BAR_HEIGHT,
            inspector_panel_width: 320.0,
            view_settings_open: false,
            rendering_settings_open: false,
            gizmo_settings_open: false,
            status_line:
                "Viewport: wheel zoom, middle-drag pan, left click select, drag handles to edit, F frame selected."
                    .to_string(),
            last_frame_time: Instant::now(),
            output_tx,
            output_rx,
            log_file: None,
            ui_editor: ui_editor::UiEditorPanel::default(),
            global_log_path,
            global_log_file,
            project_load_error: None,
        }
    }
}

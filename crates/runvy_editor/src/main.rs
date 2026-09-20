use egui::ViewportId;
use std::sync::Arc;
use wgpu;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

struct EditorApp {
    window: Option<Arc<Window>>,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    // wgpu state
    render_state: Option<RenderState>,
    dock_state: DockState,
    quit_requested: bool,
}

struct RenderState {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    egui_renderer: egui_wgpu::Renderer,
}

impl EditorApp {
    fn new() -> Self {
        Self {
            window: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            render_state: None,
            dock_state: DockState::new(),
            quit_requested: false,
        }
    }

    fn init_gpu(&mut self, event_loop: &ActiveEventLoop) {
        let window = self
            .window
            .as_ref()
            .expect("window must exist before init_gpu")
            .clone();

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("request adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits:
                wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            experimental_features: Default::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .expect("request device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8UnormSrgb)
            .or_else(|| caps.formats.first().copied())
            .expect("surface format");

        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // egui renderer
        let egui_renderer =
            egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());

        // egui-winit state (converts winit events to egui)
        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            ViewportId::ROOT,
            event_loop,
            None,
            None,
            None,
        );

        self.egui_state = Some(egui_state);
        self.render_state = Some(RenderState {
            _instance: instance,
            surface,
            device,
            queue,
            surface_config,
            egui_renderer,
        });
    }

    fn render(&mut self) {
        let window = self.window.as_ref().expect("window").clone();
        let egui_state = self.egui_state.as_mut().expect("egui state");
        let render_state = self.render_state.as_mut().expect("render state");
        let RenderState {
            surface,
            device,
            queue,
            surface_config,
            egui_renderer,
            ..
        } = render_state;

        let raw_input = egui_state.take_egui_input(&window);
        let full_output = self.egui_ctx.run_ui(raw_input, |ui| {
            title_bar(ui, &mut self.quit_requested);
            egui::CentralPanel::default()
                .frame(egui::Frame::default())
                .show(ui, |ui| {
                    self.dock_state.ui(ui);
                });
            self.dock_state.enforce_single_pane_not_collapsed();
        });
        egui_state.handle_platform_output(&window, full_output.platform_output);

        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }
        for id in &full_output.textures_delta.free {
            egui_renderer.free_texture(id);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [surface_config.width, surface_config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        let surface_texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(device, surface_config);
                return;
            }
            _ => return,
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Editor Encoder"),
        });
        let extra_cmds = egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Editor UI Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.09,
                        g: 0.09,
                        b: 0.11,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        egui_renderer.render(
            &mut render_pass.forget_lifetime(),
            &clipped_primitives,
            &screen_descriptor,
        );

        let mut cmds = vec![encoder.finish()];
        cmds.extend(extra_cmds);
        queue.submit(cmds);
        let _ = device.poll(wgpu::PollType::Poll);
        surface_texture.present();
    }
}

impl ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = WindowAttributes::default()
                .with_title("Runvy Editor")
                .with_inner_size(PhysicalSize::new(1280, 800));
            let window = Arc::new(
                event_loop
                    .create_window(attrs)
                    .expect("create window failed"),
            );
            self.window = Some(window.clone());
            self.init_gpu(event_loop);
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::Resized(size) => {
                if let Some(rs) = &mut self.render_state {
                    rs.surface_config.width = size.width.max(1);
                    rs.surface_config.height = size.height.max(1);
                    rs.surface.configure(&rs.device, &rs.surface_config);
                }
            }
            _ => {
                if let Some(egui_state) = &mut self.egui_state {
                    let _ = egui_state.on_window_event(window, &event);
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.quit_requested {
            event_loop.exit();
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// --------------------------------------------------------------------------------
// Title bar
// --------------------------------------------------------------------------------

fn title_bar(ui: &mut egui::Ui, quit_requested: &mut bool) {
    egui::Panel::top("title_bar")
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(33, 33, 38))
                .inner_margin(egui::Margin::symmetric(6, 3)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        ui.close();
                    }
                    if ui.button("Open Project").clicked() {
                        ui.close();
                    }
                    if ui.button("Save Project").clicked() {
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        *quit_requested = true;
                        ui.close();
                    }
                });
                ui.menu_button("Project", |ui| {
                    if ui.button("Project Settings").clicked() {
                        ui.close();
                    }
                    if ui.button("Run").clicked() {
                        ui.close();
                    }
                    if ui.button("Build").clicked() {
                        ui.close();
                    }
                });
                ui.menu_button("Editor", |ui| {
                    if ui.button("Preferences").clicked() {
                        ui.close();
                    }
                    if ui.button("Keymaps").clicked() {
                        ui.close();
                    }
                    if ui.button("Reset Dock Layout").clicked() {
                        ui.close();
                    }
                });
            });
        });
}

// --------------------------------------------------------------------------------
// Dock panes
// --------------------------------------------------------------------------------

enum Pane {
    Viewport,
    Hierarchy,
    Inspector,
    Console,
}

struct DockState {
    state: egui_dock::DockState<Pane>,
}

impl DockState {
    fn new() -> Self {
        use egui_dock::{DockState, NodeIndex};

        let mut state = DockState::new(vec![Pane::Viewport]);
        // Left: hierarchy, Right column: inspector over console
        state
            .main_surface_mut()
            .split_left(NodeIndex::root(), 0.18, vec![Pane::Hierarchy]);
        // Right of the root (which is now the left subtree after the split).
        // The root node still holds the Viewport on the right side, so split it right:
        state.main_surface_mut().split_right(
            NodeIndex::root(),
            0.75,
            vec![Pane::Inspector, Pane::Console],
        );

        Self { state }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        egui_dock::DockArea::new(&mut self.state).show_inside(ui, &mut PaneViewer);
    }

    // A leaf node that contains only a single pane cannot be collapsed.
    fn enforce_single_pane_not_collapsed(&mut self) {
        for (_surface_index, surface) in self.state.iter_surfaces_mut_indexed() {
            let mut offending: Vec<egui_dock::NodeIndex> = Vec::new();
            for (node_index, node) in surface.iter_nodes_indexed() {
                if node.is_collapsed() && node.tabs_count() == 1 {
                    offending.push(node_index);
                }
            }

            for node_index in offending {
                surface[node_index].set_collapsed(false);

                let mut parent = node_index.parent();
                while let Some(parent_index) = parent {
                    parent = parent_index.parent();

                    let left_count = surface[parent_index.left()].collapsed_leaf_count();
                    let right_count = surface[parent_index.right()].collapsed_leaf_count();

                    let parent_node = &mut surface[parent_index];
                    parent_node.set_collapsed(false);
                    if parent_node.is_horizontal() {
                        parent_node.set_collapsed_leaf_count(left_count.max(right_count));
                    } else {
                        parent_node.set_collapsed_leaf_count(left_count + right_count);
                    }
                }
            }
        }
    }
}

struct PaneViewer;

impl egui_dock::TabViewer for PaneViewer {
    type Tab = Pane;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Pane::Viewport => "Viewport",
            Pane::Hierarchy => "Hierarchy",
            Pane::Inspector => "Inspector",
            Pane::Console => "Console",
        }
        .into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Pane::Viewport => {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::same(0),
                        egui::Color32::from_rgb(16, 16, 18),
                    );
                });
            }
            Pane::Hierarchy => {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for name in ["Main Camera", "Player", "Terrain", "Light"] {
                            let _ = ui.selectable_label(false, name);
                        }
                    });
            }
            Pane::Inspector => {
                ui.heading("Inspector");
                ui.separator();
                egui::Grid::new("inspector_grid")
                    .striped(true)
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Position");
                        ui.label("0.0, 0.0, 0.0");
                        ui.end_row();
                        ui.label("Scale");
                        ui.label("1.0, 1.0, 1.0");
                        ui.end_row();
                    });
            }
            Pane::Console => {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &["[info] Runvy Editor started", "[info] Dock layout ready"] {
                            ui.label(*line);
                        }
                    });
            }
        }
    }
}

fn main() -> Result<(), winit::error::EventLoopError> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = EditorApp::new();
    event_loop.run_app(&mut app)
}

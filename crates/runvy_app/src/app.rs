use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use runvy_core::components::EMPTY_TILE;
use runvy_core::components::{
    BackgroundMode, Camera, MeshRenderer, Sorting, SpriteRenderer, Tilemap, 
    Transform, UiRenderer, WorldAtmosphere,
};
use runvy_core::resources::input::{self, InputState};
use runvy_core::resources::Time;
use runvy_core::{glam, Console, Vec2};
use runvy_ecs::{R, W};
use runvy_render::Renderer;
use runvy_render_api::{InstanceData, Mesh3dParams, RenderQueue};

use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Fixed simulation timestep in seconds. Never changes.
const BASE_TIMESTEP: f32 = 1.0 / 60.0;

#[derive(Debug, Clone)]
pub struct RunvyWindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub show_fps_in_title: bool,
    pub window_icon: Option<String>,
    /// Where `RunvyApp` writes the auto-generated Luau type-definition module
    /// (`runvy.luau`) on startup so `luau-lsp` can type-check scripts. `Some(path)`
    /// enables generation (default `scripts/runvy.luau`, relative to the current
    /// working directory); `None` disables it.
    pub luau_types_path: Option<std::path::PathBuf>,
}

impl Default for RunvyWindowConfig {
    fn default() -> Self {
        Self {
            title: "Runvy Game".to_string(),
            width: 1280,
            height: 720,
            fullscreen: false,
            vsync: true,
            show_fps_in_title: false,
            window_icon: None,
            luau_types_path: Some(std::path::PathBuf::from("scripts/runvy.luau")),
        }
    }
}

pub struct App<'window> {
    pub window: Option<Arc<Window>>,
    pub renderer: Option<Renderer<'window>>,

    pub queue: RenderQueue,
    pub world: runvy_ecs::World,
    pub scheduler: runvy_ecs::Scheduler,

    // Timing
    pub last_time: Instant,
    pub accumulator: f32,
    pub frame_count: u32,
    pub current_fps: f32,
    pub last_fps_update: Instant,
    pub last_frame_time: f32,
    pub current_frame_time_ms: f32,
    pub current_render_time_ms: f32,
    pub current_update_time_ms: f32,
    pub interpolation_alpha: f32,

    pub config: RunvyWindowConfig,
    pub frame_start: Instant,
}

impl<'window> App<'window> {
    fn toggle_fullscreen(&mut self) {
        input::toggle_fullscreen();
        self.config.fullscreen = input::is_fullscreen().unwrap_or(false);
    }

    fn sync_camera(&mut self) {
        if let Some(renderer) = &self.renderer {
            let w = renderer.surface_config.width;
            let h = renderer.surface_config.height;
            for (_, cam) in self.world.query_mut::<runvy_ecs::W<Camera>>() {
                cam.resize(w, h);
            }
        }
    }

    fn render_ecs_sprites(&mut self, alpha: f32) {
        let Self {
            ref world,
            ref mut queue,
            ..
        } = self;
        let sort_orders: HashMap<u64, i32> = world
            .query::<R<Sorting>>()
            .map(|(e, s)| (e, s.order))
            .collect();
        for (entity, (transform, sprite)) in world.query::<(R<Transform>, R<SpriteRenderer>)>() {
            if let Some(tex) = sprite.texture() {
                let order = sort_orders.get(&entity).copied().unwrap_or(0);
                queue.draw_sprite(
                    tex.inner.clone(),
                    transform.interpolated_position(alpha),
                    transform.interpolated_rotation(alpha),
                    transform.scale,
                    sprite.color,
                    sprite.uv_rect,
                    order,
                    sprite.replace_color,
                    sprite.flip_x,
                    sprite.flip_y,
                );
            }
        }
    }

    fn render_ecs_tilemap(&mut self, camera: &Camera) {
        let visible = camera.ortho_visible_size();
        let half = visible * 0.5;
        let cam_pos = camera.position.truncate();
        let world_left = cam_pos.x - half.x;
        let world_right = cam_pos.x + half.x;
        let world_bottom = cam_pos.y - half.y;
        let world_top = cam_pos.y + half.y;

        let Self { ref mut queue, .. } = self;

        for (_entity, (transform, tm)) in
            self.world
                .query::<(R<Transform>, R<Tilemap>)>()
        {
            let Some(atlas) = tm.atlas.as_ref() else {
                continue;
            };
            let ts = tm.world_tile_size();
            if ts.x <= f32::EPSILON || ts.y <= f32::EPSILON {
                continue;
            }
            let base = transform.position.truncate();
            let scale = transform
                .scale
                .truncate()
                .abs()
                .max(Vec2::splat(f32::EPSILON));
            let world_ts = ts * scale;

            let local_left = (world_left - base.x) / scale.x;
            let local_right = (world_right - base.x) / scale.x;
            let local_bottom = (world_bottom - base.y) / scale.y;
            let local_top = (world_top - base.y) / scale.y;

            let min_cx = ((local_left / ts.x).floor() as i32)
                .max(tm.offset.x)
                .min(tm.offset.x + tm.width as i32 - 1);
            let max_cx = ((local_right / ts.x).floor() as i32)
                .max(tm.offset.x)
                .min(tm.offset.x + tm.width as i32 - 1);
            let min_cy = ((local_bottom / ts.y).floor() as i32)
                .max(tm.offset.y)
                .min(tm.offset.y + tm.height as i32 - 1);
            let max_cy = ((local_top / ts.y).floor() as i32)
                .max(tm.offset.y)
                .min(tm.offset.y + tm.height as i32 - 1);

            if min_cx > max_cx || min_cy > max_cy {
                continue;
            }

            let frame_count = atlas.frame_count();

            for layer in &tm.layers {
                if !layer.visible || layer.opacity <= 0.0 {
                    continue;
                }
                let alpha = layer.opacity.clamp(0.0, 1.0);
                let mut instances: Vec<InstanceData> = Vec::new();

                for cy in min_cy..=max_cy {
                    for cx in min_cx..=max_cx {
                        let array_x = (cx - tm.offset.x) as u32;
                        let array_y = (cy - tm.offset.y) as u32;
                        let id = layer.get(array_x, array_y);
                        if id == EMPTY_TILE {
                            continue;
                        }
                        let frame = tm.frame_for_id(id);
                        if frame >= frame_count {
                            continue;
                        }
                        let uv = atlas.uv_rect_for_frame(frame);
                        instances.push(InstanceData {
                            position: [
                                base.x + (cx as f32 + 0.5) * world_ts.x,
                                base.y + (cy as f32 + 0.5) * world_ts.y,
                                0.0,
                            ],
                            rotation: 0.0,
                            scale: [world_ts.x, world_ts.y, 1.0],
                            color: [1.0, 1.0, 1.0, alpha],
                            uv_offset: [uv.x, uv.y],
                            uv_size: [uv.width, uv.height],
                            flip: 0,
                        });
                    }
                }

                queue.draw_tiles_batch(atlas.texture.clone(), instances, layer.order);
            }
        }
    }

    fn render_ecs_ui(&mut self, camera: &Camera) {
        let viewport = glam::Vec2::new(
            camera.viewport_size.0.max(1) as f32,
            camera.viewport_size.1.max(1) as f32,
        );
        let camera_ref = Some(camera);

        let Self { ref world, .. } = self;

        let sort_orders: HashMap<u64, i32> = world
            .query::<R<Sorting>>()
            .map(|(e, s)| (e, s.order))
            .collect();

        let mut input = self.world.delete_resource::<InputState>();

        for (_, ui) in self.world.query_mut::<W<UiRenderer>>() {
            ui.layout(viewport, camera_ref);
            ui.process_interaction(camera_ref, &mut input);
        }

        self.world.add_resource(input);

        let mut ui_with_transform: Vec<u64> = Vec::new();
        for (entity, (ui, transform)) in self.world.query::<(R<UiRenderer>, R<Transform>)>() {
            let order = sort_orders.get(&entity).copied().unwrap_or(0);
            ui.build_render_commands(&mut self.queue, camera_ref, Some(transform), order);
            ui_with_transform.push(entity);
        }
        for (entity, ui) in self.world.query::<R<UiRenderer>>() {
            let order = sort_orders.get(&entity).copied().unwrap_or(0);
            if ui_with_transform.contains(&entity) {
                continue;
            }
            ui.build_render_commands(&mut self.queue, camera_ref, None, order);
        }
    }

    fn render_ecs_meshes(&mut self, alpha: f32) {
        let Self {
            ref world,
            ref mut queue,
            ..
        } = self;
        let sort_orders: HashMap<u64, i32> = world
            .query::<R<Sorting>>()
            .map(|(e, s)| (e, s.order))
            .collect();
        for (entity, (transform, renderer)) in world.query::<(R<Transform>, R<MeshRenderer>)>() {
            let Some(handle) = &renderer.mesh else {
                continue;
            };
            let mesh = &handle.inner;
            let model = glam::Mat4::from_scale_rotation_translation(
                transform.scale,
                transform.interpolated_rotation(alpha),
                transform.interpolated_position(alpha),
            );
            let mesh_id = mesh.vertices.as_ptr() as u64;
            let vtx: Vec<runvy_render_api::Vertex3D> = mesh
                .vertices
                .iter()
                .map(|v| runvy_render_api::Vertex3D {
                    position: v.position,
                    uv: v.uv,
                    normal: v.normal,
                    color: v.color,
                })
                .collect();
            let order = sort_orders.get(&entity).copied().unwrap_or(0);
            queue.draw_mesh_3d(Mesh3dParams {
                mesh_id,
                vertices: vtx,
                indices: mesh.indices.clone(),
                model_matrix: model,
                color: renderer.color,
                emission: [0.0; 3],
                use_vertex_color: true,
                order,
                depth: transform.position.z,
            });
        }
    }

    fn render(&mut self) {
        let render_start = Instant::now();

        let camera = self
            .world
            .query::<(R<Camera>, R<Transform>)>()
            .next()
            .map(|(_, (c, t))| {
                let mut resolved = *t;
                resolved.position = t.interpolated_position(self.interpolation_alpha);
                resolved.rotation = t.interpolated_rotation(self.interpolation_alpha);
                c.resolved_with_transform(Some(&resolved))
            })
            .or_else(|| self.world.query::<R<Camera>>().next().map(|(_, c)| *c))
            .unwrap_or_default();

        // Phase 1: populate queue from ECS (no renderer borrow)
        self.queue.clear();

        // Apply WorldAtmosphere if present (must be after clear which resets atmosphere)
        if let Some((_, atmosphere)) = self.world.query::<runvy_ecs::R<WorldAtmosphere>>().next() {
            use runvy_render_api::BackgroundModeData;
            let bg = match atmosphere.background {
                BackgroundMode::SolidColor { color } => BackgroundModeData::SolidColor {
                    color: color.to_vec3(),
                },
                BackgroundMode::VerticalGradient {
                    zenith_color,
                    horizon_color,
                    ground_color,
                    horizon_height,
                    smoothness,
                } => BackgroundModeData::VerticalGradient {
                    zenith_color: zenith_color.to_vec3(),
                    horizon_color: horizon_color.to_vec3(),
                    ground_color: ground_color.to_vec3(),
                    horizon_height,
                    smoothness,
                },
                BackgroundMode::Sky => BackgroundModeData::Sky,
            };
            self.queue.set_atmosphere(runvy_render_api::AtmosphereData {
                ambient_color: atmosphere.ambient_color.to_vec3(),
                ambient_intensity: atmosphere.ambient_intensity,
                background_intensity: atmosphere.background_intensity,
                background: bg,
            });
        }
        self.render_ecs_sprites(self.interpolation_alpha);
        self.render_ecs_meshes(self.interpolation_alpha);
        self.render_ecs_tilemap(&camera);
        self.render_ecs_ui(&camera);

        if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
            let time_scale = self.world.get_resource::<Time>().time_scale;
            let console = self.world.get_resource_mut::<Console>();

            console.current_fps = self.current_fps;
            console.current_frame_time_ms = self.current_frame_time_ms;
            console.current_render_time_ms = self.current_render_time_ms;
            console.current_update_time_ms = self.current_update_time_ms;
            console.draw_call_count = self.queue.commands.len();
            console.time_scale = time_scale;
            console.render(&mut self.queue, &camera);

            let camera_matrix = camera.matrix();
            let virtual_size = if matches!(
                camera.projection,
                runvy_core::components::ProjectionType::Perspective
            ) {
                glam::Vec2::new(
                    renderer.surface_config.width.max(1) as f32,
                    renderer.surface_config.height.max(1) as f32,
                )
            } else {
                camera.orthographic_size
            };

            renderer.draw(&self.queue, camera_matrix, virtual_size);

            self.current_render_time_ms = render_start.elapsed().as_secs_f32() * 1000.0;

            self.frame_count += 1;
            let now = Instant::now();
            if now.duration_since(self.last_fps_update).as_secs_f32() >= 1.0 {
                self.current_fps = self.frame_count as f32
                    / now.duration_since(self.last_fps_update).as_secs_f32();
                self.frame_count = 0;
                self.last_fps_update = now;
                self.config.title =
                    input::window_title().unwrap_or_else(|| self.config.title.clone());
                if self.config.show_fps_in_title {
                    window.set_title(&format!(
                        "{} - {:.1} FPS",
                        self.config.title, self.current_fps
                    ));
                } else {
                    window.set_title(&self.config.title);
                }
            }
        }
    }
}

impl<'window> ApplicationHandler for App<'window> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attr = Window::default_attributes()
                .with_title(self.config.title.to_string())
                .with_visible(false);
            let window = Arc::new(
                event_loop
                    .create_window(win_attr)
                    .expect("create window err."),
            );

            if let Some(icon_path) = &self.config.window_icon {
                match runvy_asset::load_window_icon(icon_path) {
                    Ok(icon) => {
                        window.set_window_icon(Some(icon));
                        println!("Window icon loaded: {}", icon_path);
                    }
                    Err(e) => {
                        eprintln!("Failed to load window icon '{}': {}", icon_path, e);
                    }
                }
            } else if let Ok(icon) = runvy_asset::load_window_icon(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/icon.png"
            )) {
                window.set_window_icon(Some(icon));
            }

            input::initialize_window_state(
                self.config.title.clone(),
                self.config.fullscreen,
                (self.config.width, self.config.height),
            );
            self.window = Some(window.clone());

            input::set_window_handle(&window);
            input::set_window_size(self.config.width, self.config.height);
            input::set_fullscreen(self.config.fullscreen);

            let renderer = Renderer::new(window.clone(), self.config.vsync);
            self.renderer = Some(renderer);
            self.sync_camera();
            window.request_redraw();
            window.set_visible(true);
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        self.frame_start = Instant::now();

        let frame_time = (self.frame_start - self.last_time).as_secs_f32().min(0.1);
        self.last_frame_time = frame_time;
        self.current_frame_time_ms = frame_time * 1000.0;
        self.last_time = self.frame_start;

        self.accumulator += frame_time;

        let update_start = Instant::now();

        while self.accumulator >= BASE_TIMESTEP {
            let mut input_state = self.world.delete_resource::<InputState>();
            {
                input_state.camera = self
                    .world
                    .query::<(runvy_ecs::R<Camera>, runvy_ecs::R<Transform>)>()
                    .next()
                    .map(|(_, (c, t))| c.resolved_with_transform(Some(t)))
                    .or_else(|| {
                        self.world
                            .query::<runvy_ecs::R<Camera>>()
                            .next()
                            .map(|(_, c)| *c)
                    });
            }

            self.world.add_resource(input_state);

            for (_, transform) in self.world.query_mut::<runvy_ecs::W<Transform>>() {
                transform.prepare_for_update();
            }

            {
                let time = self.world.get_resource_mut::<Time>();
                time.tick += 1;
                time.unscaled_delta = BASE_TIMESTEP;
                time.delta = BASE_TIMESTEP * time.time_scale;
                time.unscaled_elapsed += time.unscaled_delta;
                time.elapsed += time.delta;
            }

            self.scheduler
                .run_stage(runvy_ecs::Stage::Update, &mut self.world);

            self.world.get_resource_mut::<InputState>().update_frame();

            self.accumulator -= BASE_TIMESTEP;
        }

        self.interpolation_alpha = self.accumulator / BASE_TIMESTEP;

        self.current_update_time_ms = update_start.elapsed().as_secs_f32() * 1000.0;

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                let had_window = self.window.is_some();
                if let Some(wgpu_ctx) = self.renderer.as_mut() {
                    wgpu_ctx.resize((new_size.width, new_size.height));
                    for (_, cam) in self.world.query_mut::<runvy_ecs::W<Camera>>() {
                        cam.resize(new_size.width, new_size.height);
                    }
                    self.config.width = new_size.width;
                    self.config.height = new_size.height;
                    input::initialize_window_state(
                        input::window_title().unwrap_or_else(|| self.config.title.clone()),
                        input::is_fullscreen().unwrap_or(self.config.fullscreen),
                        (new_size.width, new_size.height),
                    );
                }
                if had_window {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();

                let fps_max = self.world.get_resource_mut::<Console>().fps_max;

                if fps_max.is_finite() && fps_max > 0.0 {
                    let min_frame_time = Duration::from_secs_f32(1.0 / fps_max);
                    let elapsed = self.frame_start.elapsed();

                    if elapsed < min_frame_time {
                        let remaining = min_frame_time - elapsed;
                        if remaining > Duration::from_millis(1) {
                            std::thread::sleep(remaining - Duration::from_millis(1));
                        }
                        while self.frame_start.elapsed() < min_frame_time {}
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::F11),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                self.toggle_fullscreen();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let mut console = self.world.delete_resource::<Console>();
                console.handle_keyboard(&event, event.state, &mut self.world);
                let visible = console.is_visible();
                self.world.add_resource(console);

                if !visible {
                    if let PhysicalKey::Code(key_code) = event.physical_key {
                        let input_state = self.world.get_resource_mut::<InputState>();
                        if event.state == ElementState::Pressed {
                            input_state.keys_pressed.insert(key_code);
                            input_state.keys_just_pressed.insert(key_code);
                        } else {
                            input_state.keys_pressed.remove(&key_code);
                            input_state.keys_just_pressed.remove(&key_code);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let input_state = self.world.get_resource_mut::<InputState>();
                input_state.mouse_position = (position.x as f32, position.y as f32);
            }

            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                let input_state = self.world.get_resource_mut::<InputState>();
                input_state.mouse_wheel_delta = y;
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let input_state = self.world.get_resource_mut::<InputState>();
                if state == ElementState::Pressed {
                    input_state.mouse_buttons_pressed.insert(button);
                    input_state.mouse_buttons_just_pressed.insert(button);
                } else {
                    input_state.mouse_buttons_pressed.remove(&button);
                    input_state.mouse_buttons_just_released.insert(button);
                }
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            let input_state = self.world.get_resource_mut::<InputState>();
            input_state.mouse_delta.0 += delta.0 as f32;
            input_state.mouse_delta.1 += delta.1 as f32;
        }
    }
}

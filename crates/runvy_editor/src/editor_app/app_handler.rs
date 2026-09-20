use super::*;

impl<'window> ApplicationHandler for EditorApp<'window> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        event_loop.set_control_flow(ControlFlow::Poll);

        let window_icon =
            runvy_asset::load_window_icon_from_bytes(include_bytes!("../../assets/icon.png")).ok();
        #[cfg(target_os = "windows")]
        let taskbar_icon =
            runvy_asset::load_window_icon_from_bytes(include_bytes!("../../assets/big_icon.png"))
                .ok();

        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut window_attributes = Window::default_attributes()
            .with_title(&self.window_title())
            .with_visible(false)
            .with_inner_size(egui_winit::winit::dpi::LogicalSize::new(1600.0, 960.0))
            .with_min_inner_size(egui_winit::winit::dpi::LogicalSize::new(1200.0, 720.0));
        #[cfg(target_os = "windows")]
        {
            window_attributes = window_attributes.with_taskbar_icon(taskbar_icon.clone());
        }
        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create editor window"),
        );
        if let Some(icon) = window_icon {
            window.set_window_icon(Some(icon));
        }
        #[cfg(target_os = "windows")]
        if let Some(icon) = taskbar_icon {
            window.set_taskbar_icon(Some(icon));
        }

        let renderer = Renderer::new(window.clone(), true);
        let egui_renderer = EguiRenderer::new(
            renderer.device(),
            renderer.surface_format(),
            RendererOptions::default(),
        );
        let egui_state = EguiWinitState::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            Some(renderer.device().limits().max_texture_dimension_2d as usize),
        );

        self.window = Some(window.clone());
        self.renderer = Some(renderer);
        self.egui_renderer = Some(egui_renderer);
        self.egui_state = Some(egui_state);
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(data) = std::fs::read(r"C:\Windows\Fonts\arialbd.ttf") {
            fonts.font_data.insert(
                "arial_bold".to_owned(),
                egui::FontData::from_owned(data).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Name("Arial Bold".into()))
                .or_default()
                .insert(0, "arial_bold".to_owned());
        }
        self.egui_ctx.set_fonts(fonts);
        style::apply_editor_style(&self.egui_ctx);
        self.egui_ctx.set_zoom_factor(self.settings.ui_scale);
        self.content_browser.refresh(&self.settings);
        self.ensure_viewport_target();
        window.set_visible(true);

        if let Some(path) = self.startup_project_path.clone() {
            self.begin_load_project(path);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.clone() else {
            return;
        };

        let camera_captured = self.editor_camera.handle_window_event(&window, &event);
        if let Some(egui_state) = self.egui_state.as_mut() {
            let response = egui_state.on_window_event(&window, &event);
            if response.repaint || camera_captured {
                window.request_redraw();
            }
        }

        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed()
                    && !event.repeat
                    && !self.editor_camera.is_look_active()
                    && self.modifiers.control_key()
                    && matches!(event.physical_key, PhysicalKey::Code(KeyCode::Space))
                {
                    self.panels.bottom_bar = !self.panels.bottom_bar;
                    window.request_redraw();
                }
                if event.state.is_pressed()
                    && !event.repeat
                    && !self.editor_camera.is_look_active()
                    && self.modifiers.control_key()
                    && matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyS))
                {
                    self.save_current_world();
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
                // Backtick toggles console tab
                if event.state.is_pressed()
                    && !event.repeat
                    && !self.editor_camera.is_look_active()
                    && !self.modifiers.control_key()
                    && !self.modifiers.alt_key()
                    && matches!(event.physical_key, PhysicalKey::Code(KeyCode::Backquote))
                {
                    self.panels.bottom_bar = true;
                    self.bottom_tab = BottomTab::Console;
                    window.request_redraw();
                }
            }
            WindowEvent::CloseRequested => {
                let shutdown_window = window.clone();
                self.stop_project();
                if let Some(mut child) = self.build_process.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                self.editor_camera.shutdown(&shutdown_window);
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize((size.width, size.height));
                }
            }
            WindowEvent::RedrawRequested => self.render_frame(),
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        self.editor_camera.handle_device_event(&event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.poll_output();
        self.poll_project_load();
        self.update_runtime_process_state();
        self.update_build_process_state();
        self.refresh_project_metadata(false);
        self.poll_place_object_refresh();
        if let Some(window) = self.window.as_ref() {
            window.set_title(&self.window_title());
            window.request_redraw();
        }
    }
}

use super::*;

impl<'window> EditorApp<'window> {
    pub(super) fn ensure_viewport_target(&mut self) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let Some(egui_renderer) = self.egui_renderer.as_mut() else {
            return;
        };

        let needs_recreate = self
            .viewport_target
            .as_ref()
            .map(|target| target.size() != self.pending_viewport_size)
            .unwrap_or(true);

        if !needs_recreate {
            return;
        }

        let target = renderer.create_render_target(self.pending_viewport_size);
        if let Some(texture_id) = self.viewport_texture_id {
            egui_renderer.update_egui_texture_from_wgpu_texture(
                renderer.device(),
                target.sample_view(),
                wgpu::FilterMode::Linear,
                texture_id,
            );
        } else {
            let texture_id = egui_renderer.register_native_texture(
                renderer.device(),
                target.sample_view(),
                wgpu::FilterMode::Linear,
            );
            self.viewport_texture_id = Some(texture_id);
        }
        self.viewport_target = Some(target);
    }

    pub(super) fn update_scene_preview(&mut self) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let Some(target) = self.viewport_target.as_ref() else {
            return;
        };

        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;

        self.editor_camera
            .set_viewport_hovered(self.viewport_hovered);
        self.editor_camera.update(dt);

        let camera = self.editor_camera.camera(target.size());
        self.viewport_camera = Some(camera);
        let virtual_size = Vec2::new(target.size().0 as f32, target.size().1 as f32);

        self.world.borrow_mut().repair_hierarchy();
        self.scene_queue.clear();
        self.world.borrow_mut().render(&mut self.scene_queue, 1.0);
        renderer.draw_to_target(target, &self.scene_queue, camera.matrix(), virtual_size);
    }

    pub(super) fn handle_viewport_interaction(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        response: &egui::Response,
    ) {
        if self.viewport_hovered && ctx.input(|input| input.key_pressed(egui::Key::F)) {
            self.frame_selected_object();
        }

        if !self.editor_camera.is_orthographic() {
            let camera = self.editor_camera.camera(self.pending_viewport_size);
            self.viewport_camera = Some(camera);
            if !ctx.input(|input| input.pointer.primary_down()) {
                self.gizmo_drag = None;
            }
            if self.gizmo_enabled && response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    self.try_begin_3d_gizmo_drag(response.rect, camera, pointer_pos);
                }
            }
            if self.gizmo_drag.is_some() && ctx.input(|input| input.pointer.primary_down()) {
                let delta = ctx.input(|input| input.pointer.delta());
                self.drag_selected_object_3d(delta);
            }
            if response.clicked_by(egui::PointerButton::Primary) && self.gizmo_drag.is_none() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let additive = ctx.input(|input| input.modifiers.shift);
                    let picked = self.pick_object_at_screen(response.rect, camera, pointer_pos);
                    if let Some(object_id) = picked {
                        self.select_object(object_id, additive);
                    } else if !additive {
                        self.clear_selection();
                    }
                }
            }
            self.draw_viewport_overlay(ui, response.rect, camera);
            return;
        }

        if response.hovered() {
            let scroll_delta = ctx.input(|input| input.smooth_scroll_delta.y);
            if scroll_delta.abs() > f32::EPSILON {
                let zoom_factor = if scroll_delta > 0.0 { 0.99 } else { 1.01 };
                if let Some(pointer_pos) = response.hover_pos() {
                    self.zoom_viewport_at(response.rect, pointer_pos, zoom_factor);
                }
            }
        }

        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = ctx.input(|input| input.pointer.delta()) * ctx.pixels_per_point();
            self.pan_viewport_by_pixels(Vec2::new(delta.x, delta.y));
        }

        if !ctx.input(|input| input.pointer.primary_down()) {
            self.gizmo_drag = None;
        }

        if response.hovered()
            && ctx.input(|input| input.pointer.primary_down())
            && self.tile_paint.mode != TilePaintMode::None
        {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let world_pos = self.viewport_world_pos(response.rect, pointer_pos);
                if self.paint_tile_under_cursor(world_pos) {
                    let camera = self.editor_camera.camera(self.pending_viewport_size);
                    self.viewport_camera = Some(camera);
                    self.draw_viewport_overlay(ui, response.rect, camera);
                    return;
                }
            }
        }

        if self.gizmo_enabled && response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let world_pos = self.viewport_world_pos(response.rect, pointer_pos);
                self.try_begin_gizmo_drag(world_pos);
            }
        }

        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let world_pos = self.viewport_world_pos(response.rect, pointer_pos);
                if self.gizmo_drag.is_none() {
                    let additive = ctx.input(|input| input.modifiers.shift);
                    if let Some(object_id) = self.pick_object_at_world(world_pos) {
                        self.select_object(object_id, additive);
                    } else if !additive {
                        self.clear_selection();
                    }
                }
            }
        }

        if self.gizmo_drag.is_some() && ctx.input(|input| input.pointer.primary_down()) {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let world_pos = self.viewport_world_pos(response.rect, pointer_pos);
                self.drag_selected_object(world_pos);
            }
        }

        // Content browser drop handling
        if response.hovered()
            && !ctx.input(|input| input.pointer.primary_down())
            && self.content_browser.is_dragging_asset()
        {
            if let Some(path) = self.content_browser.take_dragging_asset_path() {
                self.spawn_from_asset_path(&path, None);
                self.status_line = format!("Dropped asset: {}", path.display());
            }
        }

        let camera = self.editor_camera.camera(self.pending_viewport_size);
        self.viewport_camera = Some(camera);
        self.draw_viewport_overlay(ui, response.rect, camera);
    }

    fn pan_viewport_by_pixels(&mut self, pixel_delta: Vec2) {
        let camera = self.editor_camera.camera(self.pending_viewport_size);
        let origin = camera.screen_to_world((0.0, 0.0));
        let shifted = camera.screen_to_world((pixel_delta.x, pixel_delta.y));
        self.editor_camera.pan(origin - shifted);
    }

    fn zoom_viewport_at(&mut self, rect: egui::Rect, pointer_pos: egui::Pos2, factor: f32) {
        let local = pointer_pos - rect.min;
        let before = self
            .editor_camera
            .camera(self.pending_viewport_size)
            .screen_to_world((local.x, local.y));
        self.editor_camera.zoom_by_factor(factor);
        let after = self
            .editor_camera
            .camera(self.pending_viewport_size)
            .screen_to_world((local.x, local.y));
        self.editor_camera.pan(before - after);
    }

    fn viewport_world_pos(&mut self, rect: egui::Rect, pointer_pos: egui::Pos2) -> Vec2 {
        let local = pointer_pos - rect.min;
        self.editor_camera
            .camera(self.pending_viewport_size)
            .screen_to_world((local.x, local.y))
    }

    fn paint_tile_under_cursor(&mut self, world_pos: Vec2) -> bool {
        let Some(object_id) = self.selection else {
            return false;
        };
        let mut world_rc = self.world.borrow_mut();
        let Some(object) = world_rc.object_mut(object_id) else {
            return false;
        };
        let Some(transform) = object.get_component::<Transform>().cloned() else {
            return false;
        };
        let Some(tilemap) = object.get_component_mut::<Tilemap>() else {
            return false;
        };
        if tilemap.layers.is_empty() {
            self.status_line = "Tilemap has no layers to paint.".to_string();
            return true;
        }

        let tile_size = tilemap.world_tile_size();
        if tile_size.x <= f32::EPSILON || tile_size.y <= f32::EPSILON {
            return true;
        }

        let scale = transform
            .scale
            .truncate()
            .abs()
            .max(Vec2::splat(f32::EPSILON));
        let local = (world_pos - transform.position.truncate()) / scale;
        let tile_x = (local.x / tile_size.x).floor() as i32;
        let tile_y = (local.y / tile_size.y).floor() as i32;
        let layer = self
            .tile_paint
            .layer
            .min(tilemap.layers.len().saturating_sub(1) as u32) as usize;

        match self.tile_paint.mode {
            TilePaintMode::None => {}
            TilePaintMode::Paint => {
                if tilemap.atlas.is_none() {
                    self.status_line = "Assign a Tilemap atlas before painting.".to_string();
                    return true;
                }
                tilemap.paint_tile(layer, tile_x, tile_y, tilemap.selected_tile);
            }
            TilePaintMode::Erase => {
                tilemap.erase_tile(layer, tile_x, tile_y);
            }
        }
        true
    }

    fn draw_viewport_overlay(&self, ui: &mut egui::Ui, rect: egui::Rect, camera: Camera) {
        let painter = ui.painter_at(rect);

        if self.show_viewport_grid {
            if self.editor_camera.is_orthographic() {
                self.draw_viewport_grid(&painter, rect, camera);
            } else {
                self.draw_viewport_floor_grid(&painter, rect, camera);
            }
        }

        if self.show_component_icons {
            self.draw_component_icons(&painter, rect, camera);
        }
        self.draw_directional_light_arrows(&painter, rect, camera);

        for object_id in self.selected_objects.iter().copied().collect::<Vec<_>>() {
            self.draw_selected_camera_overlay(&painter, rect, camera, object_id);
            if self.editor_camera.is_orthographic() {
                if let Some(screen_rect) = self.object_screen_rect(rect, camera, object_id) {
                    helpers::draw_rect_outline(
                        &painter,
                        screen_rect,
                        egui::Color32::from_rgb(96, 180, 255),
                        2.0,
                    );
                }
            } else {
                self.draw_perspective_outline(&painter, rect, camera, object_id);
            }
        }

        if let Some(object_id) = self.selection {
            if self.gizmo_enabled && self.editor_camera.is_orthographic() {
                self.draw_transform_gizmo(&painter, rect, camera, object_id);
            } else if self.gizmo_enabled {
                self.draw_transform_gizmo_3d(&painter, rect, camera, object_id);
            }
        }
    }

    fn draw_selected_camera_overlay(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        editor_camera: Camera,
        object_id: ObjectId,
    ) {
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return;
        };
        let Some(camera) = object.get_component::<Camera>() else {
            return;
        };
        let resolved = camera.resolved_with_transform(object.get_component::<Transform>());

        if resolved.projection == runvy_core::components::ProjectionType::Orthographic {
            if matches!(
                editor_camera.projection,
                runvy_core::components::ProjectionType::Perspective
            ) {
                self.draw_orthographic_camera_volume(painter, rect, editor_camera, resolved);
            } else {
                let half = resolved.ortho_visible_size() * 0.5;
                let min = resolved.position.truncate() - half;
                let max = resolved.position.truncate() + half;
                let top_left =
                    helpers::world_to_screen(rect, editor_camera, Vec2::new(min.x, max.y));
                let bottom_right =
                    helpers::world_to_screen(rect, editor_camera, Vec2::new(max.x, min.y));
                helpers::draw_rect_outline(
                    painter,
                    egui::Rect::from_two_pos(top_left, bottom_right),
                    egui::Color32::from_rgb(255, 216, 96),
                    1.5,
                );
                let center =
                    helpers::world_to_screen(rect, editor_camera, resolved.position.truncate());
                let target =
                    helpers::world_to_screen(rect, editor_camera, resolved.target.truncate());
                painter.line_segment(
                    [center, target],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 216, 96)),
                );
            }
            return;
        }

        let aspect = resolved.aspect();
        let forward = resolved.forward();
        if forward.length_squared() <= f32::EPSILON {
            return;
        }
        let right = forward.cross(resolved.up).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        if right.length_squared() <= f32::EPSILON || up.length_squared() <= f32::EPSILON {
            return;
        }

        let near_center = resolved.position + forward * resolved.near.max(0.01);
        let far_center = resolved.position + forward * (resolved.near.max(0.01) * 3.0);
        let near_half_h = (resolved.fov * 0.5).tan() * resolved.near.max(0.01);
        let near_half_w = near_half_h * aspect;
        let far_half_h = (resolved.fov * 0.5).tan() * resolved.near.max(0.01) * 3.0;
        let far_half_w = far_half_h * aspect;

        let near = [
            near_center + up * near_half_h - right * near_half_w,
            near_center + up * near_half_h + right * near_half_w,
            near_center - up * near_half_h + right * near_half_w,
            near_center - up * near_half_h - right * near_half_w,
        ];
        let far = [
            far_center + up * far_half_h - right * far_half_w,
            far_center + up * far_half_h + right * far_half_w,
            far_center - up * far_half_h + right * far_half_w,
            far_center - up * far_half_h - right * far_half_w,
        ];
        let color = egui::Color32::from_rgb(255, 216, 96);
        for face in [&near[..], &far[..]] {
            for edge in 0..4 {
                let segment = [face[edge], face[(edge + 1) % 4]];
                self.draw_projected_polyline(painter, rect, editor_camera, &segment, color);
            }
        }
        for edge in 0..4 {
            let segment = [near[edge], far[edge]];
            self.draw_projected_polyline(painter, rect, editor_camera, &segment, color);
        }
        self.draw_projected_polyline(
            painter,
            rect,
            editor_camera,
            &[resolved.position, resolved.position + forward * 2.0],
            color,
        );
    }

    fn draw_directional_light_arrows(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        camera: Camera,
    ) {
        for object_id in self.world_object_ids() {
            let world = self.world.borrow();
            let Some(object) = world.object(object_id) else {
                continue;
            };
            let Some(light) = object.get_component::<DirectionalLight>() else {
                continue;
            };
            let Some(matrix) = self.world.borrow().world_transform_matrix(object_id, 1.0) else {
                continue;
            };
            let origin = matrix.transform_point3(Vec3::ZERO);
            let direction = light.direction.normalize_or_zero();
            if direction.length_squared() <= f32::EPSILON {
                continue;
            }

            let color = egui::Color32::from_rgb(255, 225, 120);
            if self.editor_camera.is_orthographic() {
                let start = helpers::world_to_screen(rect, camera, origin.truncate());
                let end =
                    helpers::world_to_screen(rect, camera, (origin + direction * 1.75).truncate());
                draw_screen_arrow(painter, start, end, color);
            } else {
                let Some(start) = helpers::world3_to_screen(rect, camera, origin) else {
                    continue;
                };
                let Some(end) = helpers::world3_to_screen(rect, camera, origin + direction * 2.0)
                else {
                    continue;
                };
                draw_screen_arrow(painter, start, end, color);
            }
        }
    }

    fn draw_orthographic_camera_volume(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        editor_camera: Camera,
        resolved: Camera,
    ) {
        let forward = resolved.forward();
        if forward.length_squared() <= f32::EPSILON {
            return;
        }
        let right = forward.cross(resolved.up).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        if right.length_squared() <= f32::EPSILON || up.length_squared() <= f32::EPSILON {
            return;
        }

        let visible_size = resolved.ortho_visible_size();
        let half_w = visible_size.x * 0.5;
        let half_h = visible_size.y * 0.5;
        let max_depth = visible_size.x.max(visible_size.y).max(32.0);
        let near = resolved.near.clamp(-max_depth, max_depth);
        let far = resolved.far.clamp(-max_depth, max_depth);

        let near_center = resolved.position + forward * near;
        let far_center = resolved.position + forward * far;

        let near_face = [
            near_center + up * half_h - right * half_w,
            near_center + up * half_h + right * half_w,
            near_center - up * half_h + right * half_w,
            near_center - up * half_h - right * half_w,
        ];
        let far_face = [
            far_center + up * half_h - right * half_w,
            far_center + up * half_h + right * half_w,
            far_center - up * half_h + right * half_w,
            far_center - up * half_h - right * half_w,
        ];

        let color = egui::Color32::from_rgb(255, 216, 96);
        for face in [&near_face[..], &far_face[..]] {
            for edge in 0..4 {
                let segment = [face[edge], face[(edge + 1) % 4]];
                self.draw_projected_polyline(painter, rect, editor_camera, &segment, color);
            }
        }
        for edge in 0..4 {
            let segment = [near_face[edge], far_face[edge]];
            self.draw_projected_polyline(painter, rect, editor_camera, &segment, color);
        }
        self.draw_projected_polyline(
            painter,
            rect,
            editor_camera,
            &[
                resolved.position,
                resolved.position + forward * (max_depth * 0.5),
            ],
            color,
        );
    }

    fn draw_viewport_floor_grid(&self, painter: &egui::Painter, rect: egui::Rect, camera: Camera) {
        let center_x = camera.position.x.round() as i32;
        let center_z = camera.position.z.round() as i32;
        let radius = 24;

        for x in (center_x - radius)..=(center_x + radius) {
            let color = if x == 0 {
                egui::Color32::from_rgb(100, 120, 150)
            } else if x % 5 == 0 {
                egui::Color32::from_gray(72)
            } else {
                egui::Color32::from_gray(55)
            };
            let points = ((center_z - radius)..=(center_z + radius))
                .map(|z| Vec3::new(x as f32, 0.0, z as f32))
                .collect::<Vec<_>>();
            self.draw_projected_polyline(painter, rect, camera, &points, color);
        }

        for z in (center_z - radius)..=(center_z + radius) {
            let color = if z == 0 {
                egui::Color32::from_rgb(150, 100, 100)
            } else if z % 5 == 0 {
                egui::Color32::from_gray(72)
            } else {
                egui::Color32::from_gray(55)
            };
            let points = ((center_x - radius)..=(center_x + radius))
                .map(|x| Vec3::new(x as f32, 0.0, z as f32))
                .collect::<Vec<_>>();
            self.draw_projected_polyline(painter, rect, camera, &points, color);
        }
    }

    fn draw_projected_polyline(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        camera: Camera,
        points: &[Vec3],
        color: egui::Color32,
    ) {
        let mut previous = None;
        for &point in points {
            let current = helpers::world3_to_screen(rect, camera, point);
            if let (Some(a), Some(b)) = (previous, current) {
                painter.line_segment([a, b], egui::Stroke::new(1.0, color));
            }
            previous = current;
        }
    }

    fn draw_viewport_grid(&self, painter: &egui::Painter, rect: egui::Rect, camera: Camera) {
        let visible_size = camera.ortho_visible_size();
        let half_width = visible_size.x * 0.5;
        let half_height = visible_size.y * 0.5;
        let min_x = (camera.position.x - half_width).floor() as i32 - 1;
        let max_x = (camera.position.x + half_width).ceil() as i32 + 1;
        let min_y = (camera.position.y - half_height).floor() as i32 - 1;
        let max_y = (camera.position.y + half_height).ceil() as i32 + 1;

        for x in min_x..=max_x {
            let start = helpers::world_to_screen(rect, camera, Vec2::new(x as f32, min_y as f32));
            let end = helpers::world_to_screen(rect, camera, Vec2::new(x as f32, max_y as f32));
            let color = if x == 0 {
                egui::Color32::from_gray(90)
            } else {
                egui::Color32::from_gray(45)
            };
            painter.line_segment([start, end], egui::Stroke::new(1.0, color));
        }

        for y in min_y..=max_y {
            let start = helpers::world_to_screen(rect, camera, Vec2::new(min_x as f32, y as f32));
            let end = helpers::world_to_screen(rect, camera, Vec2::new(max_x as f32, y as f32));
            let color = if y == 0 {
                egui::Color32::from_gray(90)
            } else {
                egui::Color32::from_gray(45)
            };
            painter.line_segment([start, end], egui::Stroke::new(1.0, color));
        }
    }

    fn try_begin_gizmo_drag(&mut self, world_pos: Vec2) -> bool {
        let Some(object_id) = self.selection else {
            return false;
        };
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return false;
        };
        let Some(transform) = object.get_component::<Transform>() else {
            return false;
        };

        let center = transform.position.truncate();
        let handles = helpers::gizmo_handles(center, transform.scale);
        let pick_radius = self.editor_camera.get_zoom() * 0.04;

        for (kind, handle_position) in handles {
            if world_pos.distance(handle_position) <= pick_radius {
                let (_, _, start_rotation_z) = transform.rotation.to_euler(EulerRot::XYZ);
                self.gizmo_drag = Some(ViewportDragState {
                    object_id,
                    kind,
                    offset: center - world_pos,
                    start_position: transform.position,
                    start_rotation_z,
                    start_pointer_angle: helpers::vec2_angle(world_pos - center),
                });
                return true;
            }
        }

        false
    }

    fn drag_selected_object(&mut self, world_pos: Vec2) {
        let Some(drag) = self.gizmo_drag.as_ref() else {
            return;
        };
        let mut world = self.world.borrow_mut();
        let Some(object) = world.object_mut(drag.object_id) else {
            return;
        };
        let Some(transform) = object.get_component_mut::<Transform>() else {
            return;
        };

        match drag.kind {
            GizmoHandleKind::Translate => {
                let mut target = world_pos + drag.offset;
                if self.snap_enabled {
                    let step = self.snap_step.max(0.1);
                    target.x = (target.x / step).round() * step;
                    target.y = (target.y / step).round() * step;
                }

                transform.position.x = target.x;
                transform.position.y = target.y;
                transform.previous_position = transform.position;
            }
            GizmoHandleKind::ScaleX => {
                let center = drag.start_position.truncate();
                let mut width = ((world_pos.x - center.x).abs() * 2.0).max(0.1);
                if self.snap_enabled {
                    let step = self.snap_step.max(0.1);
                    width = (width / step).round() * step;
                }
                transform.scale.x = width;
            }
            GizmoHandleKind::ScaleY => {
                let center = drag.start_position.truncate();
                let mut height = ((world_pos.y - center.y).abs() * 2.0).max(0.1);
                if self.snap_enabled {
                    let step = self.snap_step.max(0.1);
                    height = (height / step).round() * step;
                }
                transform.scale.y = height;
            }
            GizmoHandleKind::Rotate => {
                let center = drag.start_position.truncate();
                let current_angle = helpers::vec2_angle(world_pos - center);
                let new_rotation_z =
                    drag.start_rotation_z + (current_angle - drag.start_pointer_angle);
                let (x, y, _) = transform.rotation.to_euler(EulerRot::XYZ);
                transform.rotation = Quat::from_euler(EulerRot::XYZ, x, y, new_rotation_z);
                transform.previous_rotation = transform.rotation;
            }
            GizmoHandleKind::PositionAxis(_)
            | GizmoHandleKind::RotationAxis(_)
            | GizmoHandleKind::ScaleAxis(_) => {}
        }
    }

    fn try_begin_3d_gizmo_drag(
        &mut self,
        rect: egui::Rect,
        camera: Camera,
        pointer_pos: egui::Pos2,
    ) -> bool {
        let Some(object_id) = self.selection else {
            return false;
        };
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return false;
        };
        let Some(transform) = object.get_component::<Transform>() else {
            return false;
        };
        for (axis, _, handle_pos, _) in self.gizmo_3d_handles(rect, camera, transform.position) {
            if pointer_pos.distance(handle_pos) <= 12.0 {
                let (_, _, start_rotation_z) = transform.rotation.to_euler(EulerRot::XYZ);
                self.gizmo_drag = Some(ViewportDragState {
                    object_id,
                    kind: match self.viewport_edit_mode {
                        ViewportEditMode::Position => GizmoHandleKind::PositionAxis(axis),
                        ViewportEditMode::Rotation => GizmoHandleKind::RotationAxis(axis),
                        ViewportEditMode::Scale => GizmoHandleKind::ScaleAxis(axis),
                    },
                    offset: Vec2::ZERO,
                    start_position: transform.position,
                    start_rotation_z,
                    start_pointer_angle: 0.0,
                });
                return true;
            }
        }

        false
    }

    fn drag_selected_object_3d(&mut self, pixel_delta: egui::Vec2) {
        let Some(drag) = self.gizmo_drag.as_ref() else {
            return;
        };
        let mut world = self.world.borrow_mut();
        let Some(object) = world.object_mut(drag.object_id) else {
            return;
        };
        let Some(transform) = object.get_component_mut::<Transform>() else {
            return;
        };

        let delta = (pixel_delta.x - pixel_delta.y) * 0.02;
        match drag.kind {
            GizmoHandleKind::PositionAxis(axis) => {
                let mut position = transform.position;
                position[axis] += delta;
                if self.snap_enabled {
                    let step = self.snap_step.max(0.1);
                    position[axis] = (position[axis] / step).round() * step;
                }
                transform.position = position;
                transform.previous_position = transform.position;
            }
            GizmoHandleKind::RotationAxis(axis) => {
                let (mut x, mut y, mut z) = transform.rotation.to_euler(EulerRot::XYZ);
                let rotation_delta = delta * 0.75;
                match axis {
                    0 => x += rotation_delta,
                    1 => y += rotation_delta,
                    _ => z += rotation_delta,
                }
                transform.rotation = Quat::from_euler(EulerRot::XYZ, x, y, z);
                transform.previous_rotation = transform.rotation;
            }
            GizmoHandleKind::ScaleAxis(axis) => {
                let mut scale = transform.scale;
                scale[axis] = (scale[axis] + delta).max(0.05);
                if self.snap_enabled {
                    let step = self.snap_step.max(0.1);
                    scale[axis] = (scale[axis] / step).round() * step;
                }
                transform.scale = scale;
            }
            _ => {}
        }
    }

    fn pick_object_at_world(&self, world_pos: Vec2) -> Option<ObjectId> {
        let mut best: Option<(ObjectId, f32, f32)> = None;

        for object_id in self.world_object_ids() {
            let world = self.world.borrow();
            let Some(object) = world.object(object_id) else {
                continue;
            };
            let Some((min, max)) = self.object_bounds_2d(object_id, object) else {
                continue;
            };

            let contains = world_pos.x >= min.x
                && world_pos.x <= max.x
                && world_pos.y >= min.y
                && world_pos.y <= max.y;
            if !contains {
                continue;
            }

            let center = (min + max) * 0.5;
            let area = (max.x - min.x).abs() * (max.y - min.y).abs();
            let distance = center.distance_squared(world_pos);

            match best {
                Some((_, best_area, _)) if area > best_area => continue,
                Some((_, best_area, best_distance))
                    if (area - best_area).abs() <= f32::EPSILON && distance >= best_distance =>
                {
                    continue;
                }
                _ => best = Some((object_id, area, distance)),
            }
        }

        best.map(|entry| entry.0)
    }

    fn pick_object_at_screen(
        &self,
        rect: egui::Rect,
        camera: Camera,
        pointer_pos: egui::Pos2,
    ) -> Option<ObjectId> {
        let mut best: Option<(ObjectId, f32)> = None;
        for object_id in self.world_object_ids() {
            let Some(screen_rect) = self.object_screen_rect_any(rect, camera, object_id) else {
                continue;
            };
            if !screen_rect.contains(pointer_pos) {
                continue;
            }
            let area = screen_rect.width().abs() * screen_rect.height().abs();
            match best {
                Some((_, best_area)) if area >= best_area => {}
                _ => best = Some((object_id, area)),
            }
        }
        best.map(|entry| entry.0)
    }

    fn object_screen_rect_any(
        &self,
        rect: egui::Rect,
        camera: Camera,
        object_id: ObjectId,
    ) -> Option<egui::Rect> {
        if self.editor_camera.is_orthographic() {
            return self.object_screen_rect(rect, camera, object_id);
        }

        let world = self.world.borrow();
        let object = world.object(object_id)?;
        let (min, max) = helpers::object_world_bounds_3d(object)?;
        let matrix = world.world_transform_matrix(object_id, 1.0)?;
        let local_position = object.get_component::<Transform>()?.position;
        let local_matrix = Mat4::from_translation(-local_position);
        let to_world = matrix * local_matrix;

        let mut min_screen = egui::pos2(f32::INFINITY, f32::INFINITY);
        let mut max_screen = egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);
        let mut projected_any = false;
        for corner in helpers::aabb_corners(min, max) {
            let world = to_world.transform_point3(corner);
            let Some(screen) = helpers::world3_to_screen(rect, camera, world) else {
                continue;
            };
            min_screen.x = min_screen.x.min(screen.x);
            min_screen.y = min_screen.y.min(screen.y);
            max_screen.x = max_screen.x.max(screen.x);
            max_screen.y = max_screen.y.max(screen.y);
            projected_any = true;
        }
        projected_any.then(|| egui::Rect::from_min_max(min_screen, max_screen).expand(4.0))
    }

    fn object_screen_rect(
        &self,
        rect: egui::Rect,
        camera: Camera,
        object_id: ObjectId,
    ) -> Option<egui::Rect> {
        let world = self.world.borrow();
        let object = world.object(object_id)?;
        let (min, max) = self.object_bounds_2d(object_id, object)?;
        let top_left = helpers::world_to_screen(rect, camera, Vec2::new(min.x, max.y));
        let bottom_right = helpers::world_to_screen(rect, camera, Vec2::new(max.x, min.y));
        Some(egui::Rect::from_two_pos(top_left, bottom_right))
    }

    fn object_bounds_2d(&self, object_id: ObjectId, object: &Object) -> Option<(Vec2, Vec2)> {
        let (min, max) = helpers::object_world_bounds_2d(object)?;
        let matrix = self.world.borrow().world_transform_matrix(object_id, 1.0)?;
        let local_position = object.get_component::<Transform>()?.position;
        let local_matrix = Mat4::from_translation(-local_position);
        let to_world = matrix * local_matrix;
        let corners = [
            Vec3::new(min.x, min.y, local_position.z),
            Vec3::new(min.x, max.y, local_position.z),
            Vec3::new(max.x, min.y, local_position.z),
            Vec3::new(max.x, max.y, local_position.z),
        ];
        let mut world_min = Vec2::splat(f32::INFINITY);
        let mut world_max = Vec2::splat(f32::NEG_INFINITY);
        for corner in corners {
            let transformed = to_world.transform_point3(corner).truncate();
            world_min = world_min.min(transformed);
            world_max = world_max.max(transformed);
        }
        Some((world_min, world_max))
    }

    fn object_world_position(&self, object_id: ObjectId) -> Option<Vec3> {
        self.world
            .borrow()
            .world_transform_matrix(object_id, 1.0)
            .map(|matrix| matrix.transform_point3(Vec3::ZERO))
    }

    fn draw_transform_gizmo(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        camera: Camera,
        object_id: ObjectId,
    ) {
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return;
        };
        let Some(transform) = object.get_component::<Transform>() else {
            return;
        };

        let matrix = world
            .world_transform_matrix(object_id, 1.0)
            .unwrap_or_else(|| {
                Mat4::from_scale_rotation_translation(
                    transform.scale,
                    transform.rotation,
                    transform.position,
                )
            });
        let (world_scale, _, world_position) = matrix.to_scale_rotation_translation();
        let center = world_position.truncate();
        let handles = helpers::gizmo_handles(center, world_scale);
        let center_screen = helpers::world_to_screen(rect, camera, center);
        let x_screen = helpers::world_to_screen(rect, camera, handles[1].1);
        let y_screen = helpers::world_to_screen(rect, camera, handles[2].1);
        let r_screen = helpers::world_to_screen(rect, camera, handles[3].1);

        painter.line_segment(
            [center_screen, x_screen],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 99, 99)),
        );
        painter.line_segment(
            [center_screen, y_screen],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(104, 196, 125)),
        );
        painter.line_segment(
            [center_screen, r_screen],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 210, 64)),
        );

        helpers::draw_gizmo_handle(
            painter,
            center_screen,
            egui::Color32::from_rgb(96, 180, 255),
            "P",
        );
        helpers::draw_gizmo_handle(
            painter,
            x_screen,
            egui::Color32::from_rgb(255, 99, 99),
            "SX",
        );
        helpers::draw_gizmo_handle(
            painter,
            y_screen,
            egui::Color32::from_rgb(104, 196, 125),
            "SY",
        );
        helpers::draw_gizmo_handle(
            painter,
            r_screen,
            egui::Color32::from_rgb(255, 210, 64),
            "R",
        );
    }

    fn draw_transform_gizmo_3d(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        camera: Camera,
        object_id: ObjectId,
    ) {
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return;
        };
        let Some(transform) = object.get_component::<Transform>() else {
            return;
        };
        let origin = world
            .world_transform_matrix(object_id, 1.0)
            .map(|matrix| matrix.transform_point3(Vec3::ZERO))
            .unwrap_or(transform.position);
        let Some(center_screen) = helpers::world3_to_screen(rect, camera, origin) else {
            return;
        };

        for (_, _, handle_pos, color) in self.gizmo_3d_handles(rect, camera, origin) {
            painter.line_segment([center_screen, handle_pos], egui::Stroke::new(2.0, color));
        }

        for (axis, _, handle_pos, color) in self.gizmo_3d_handles(rect, camera, origin) {
            let label = match (self.viewport_edit_mode, axis) {
                (ViewportEditMode::Position, 0) => "X",
                (ViewportEditMode::Position, 1) => "Y",
                (ViewportEditMode::Position, _) => "Z",
                (ViewportEditMode::Rotation, 0) => "RX",
                (ViewportEditMode::Rotation, 1) => "RY",
                (ViewportEditMode::Rotation, _) => "RZ",
                (ViewportEditMode::Scale, 0) => "SX",
                (ViewportEditMode::Scale, 1) => "SY",
                (ViewportEditMode::Scale, _) => "SZ",
            };
            helpers::draw_gizmo_handle(painter, handle_pos, color, label);
        }
    }

    fn gizmo_3d_handles(
        &self,
        rect: egui::Rect,
        camera: Camera,
        origin: Vec3,
    ) -> Vec<(usize, Vec3, egui::Pos2, egui::Color32)> {
        let distance = (camera.position - origin).length().max(1.0);
        let length = (distance * 0.18).clamp(0.75, 4.0);
        let axes = [
            (0, Vec3::X, egui::Color32::from_rgb(255, 99, 99)),
            (1, Vec3::Y, egui::Color32::from_rgb(104, 196, 125)),
            (2, Vec3::Z, egui::Color32::from_rgb(96, 180, 255)),
        ];

        axes.into_iter()
            .filter_map(|(axis, direction, color)| {
                let world_end = origin + direction * length;
                let screen_end = helpers::world3_to_screen(rect, camera, world_end)?;
                Some((axis, world_end, screen_end, color))
            })
            .collect()
    }

    fn draw_perspective_outline(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        camera: Camera,
        object_id: ObjectId,
    ) {
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return;
        };
        let Some((min, max)) = helpers::object_world_bounds_3d(object) else {
            return;
        };
        let corners = helpers::aabb_corners(min, max);
        let edges = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];

        for (a, b) in edges {
            let segment = [corners[a], corners[b]];
            self.draw_projected_polyline(
                painter,
                rect,
                camera,
                &segment,
                egui::Color32::from_rgb(96, 180, 255),
            );
        }
    }

    fn draw_component_icons(&self, painter: &egui::Painter, rect: egui::Rect, camera: Camera) {
        for object_id in self.world_object_ids() {
            let world = self.world.borrow();
            let Some(object) = world.object(object_id) else {
                continue;
            };

            let mut visible_count = 0usize;
            for info in object.component_infos() {
                if info.type_id() == TypeId::of::<Transform>() {
                    continue;
                }

                let world_anchor = helpers::component_anchor_world(object, info.type_id())
                    .or_else(|| self.object_world_position(object_id));
                let Some(world_anchor) = world_anchor else {
                    continue;
                };

                let screen_anchor = if self.editor_camera.is_orthographic() {
                    Some(helpers::world_to_screen(
                        rect,
                        camera,
                        world_anchor.truncate(),
                    ))
                } else {
                    helpers::world3_to_screen(rect, camera, world_anchor)
                };
                let Some(screen_anchor) = screen_anchor else {
                    continue;
                };

                let icon_name = helpers::component_icon_name(info.type_id(), info.kind());
                let texture = crate::editor_textures::load_component_icon(
                    &self.egui_ctx,
                    &format!("viewport_component_icon_{icon_name}"),
                    icon_name,
                );
                let offset = helpers::icon_offset(visible_count);
                let position = screen_anchor + offset;
                helpers::draw_component_icon(
                    painter,
                    texture.id(),
                    position,
                    self.viewport_component_icon_size,
                );
                visible_count += 1;
            }
        }
    }

    pub(super) fn frame_selected_object(&mut self) {
        let Some(object_id) = self.selection else {
            return;
        };
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return;
        };
        let Some(transform) = object.get_component::<Transform>() else {
            return;
        };

        if self.editor_camera.is_orthographic() {
            let Some((min, max)) = helpers::object_world_bounds_2d(object) else {
                return;
            };

            let center = (min + max) * 0.5;
            let size = max - min;
            let view_height = size.y.max(size.x * 0.75).max(4.0) * 1.5;
            self.editor_camera.frame_2d(center, view_height);
        } else {
            let distance = transform.scale.length().max(1.0) * 3.0;
            self.editor_camera.frame_3d(transform.position, distance);
        }
    }
}

fn draw_screen_arrow(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    color: egui::Color32,
) {
    let direction = end - start;
    if direction.length_sq() <= 1.0 {
        return;
    }

    painter.line_segment([start, end], egui::Stroke::new(2.0, color));

    let dir = direction.normalized();
    let normal = egui::vec2(-dir.y, dir.x);
    let head_len = 9.0;
    let head_width = 5.0;
    let left = end - dir * head_len + normal * head_width;
    let right = end - dir * head_len - normal * head_width;
    painter.line_segment([end, left], egui::Stroke::new(2.0, color));
    painter.line_segment([end, right], egui::Stroke::new(2.0, color));
}

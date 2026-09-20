use std::collections::HashSet;

use runvy_core::components::{Camera, ProjectionType};
use runvy_core::glam::{Vec2, Vec3};
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

pub struct EditorCameraController {
    projection: ProjectionType,
    ortho_center: Vec2,
    ortho_view_height: f32,
    perspective_position: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    fov_degrees: f32,
    sensitivity: f32,
    near: f32,
    far: f32,
    look_active: bool,
    viewport_hovered: bool,
    pressed_keys: HashSet<KeyCode>,
}

impl EditorCameraController {
    pub fn new() -> Self {
        Self {
            projection: ProjectionType::Orthographic,
            ortho_center: Vec2::ZERO,
            ortho_view_height: 18.0,
            perspective_position: Vec3::new(0.0, 1.4, 6.0),
            yaw: 0.0,
            pitch: -0.24,
            speed: 4.0,
            fov_degrees: 75.0,
            sensitivity: 1.0,
            near: 0.1,
            far: 1000.0,
            look_active: false,
            viewport_hovered: false,
            pressed_keys: HashSet::new(),
        }
    }

    pub fn projection(&self) -> ProjectionType {
        self.projection
    }

    pub fn set_projection(&mut self, projection: ProjectionType) {
        self.projection = projection;
        if projection == ProjectionType::Orthographic {
            self.look_active = false;
            self.pressed_keys.clear();
        }
    }

    pub fn is_orthographic(&self) -> bool {
        self.projection == ProjectionType::Orthographic
    }

    pub fn is_perspective(&self) -> bool {
        self.projection == ProjectionType::Perspective
    }

    pub fn set_viewport_hovered(&mut self, hovered: bool) {
        self.viewport_hovered = hovered;
    }

    pub fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        if !self.is_perspective() {
            return false;
        }

        match event {
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard_input(event),
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => {
                let should_activate = *state == ElementState::Pressed && self.viewport_hovered;
                self.set_look_active(window, should_activate);
                true
            }
            _ => false,
        }
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if !self.is_perspective() || !self.look_active {
            return;
        }

        if let DeviceEvent::MouseMotion { delta } = event {
            self.yaw -= delta.0 as f32 * self.sensitivity / 100.0;
            self.pitch -= delta.1 as f32 * self.sensitivity / 100.0;
            self.pitch = self.pitch.clamp(-1.5, 1.5);
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_perspective() {
            return;
        }

        let mut movement = Vec3::ZERO;
        let forward = self.forward();
        let right = self.right();

        if self.pressed_keys.contains(&KeyCode::KeyW) {
            movement += forward;
        }
        if self.pressed_keys.contains(&KeyCode::KeyS) {
            movement -= forward;
        }
        if self.pressed_keys.contains(&KeyCode::KeyD) {
            movement += right;
        }
        if self.pressed_keys.contains(&KeyCode::KeyA) {
            movement -= right;
        }
        if self.pressed_keys.contains(&KeyCode::Space) {
            movement += Vec3::Y;
        }
        if self.pressed_keys.contains(&KeyCode::ControlLeft) {
            movement -= Vec3::Y;
        }

        if movement.length_squared() > 0.0 {
            let speed = if self.pressed_keys.contains(&KeyCode::ShiftLeft)
                || self.pressed_keys.contains(&KeyCode::ShiftRight)
            {
                self.speed * 2.0
            } else {
                self.speed
            };
            self.perspective_position += movement.normalize() * speed * dt;
        }
    }

    pub fn camera(&self, viewport_size: (u32, u32)) -> Camera {
        match self.projection {
            ProjectionType::Orthographic => {
                let aspect = viewport_size.0.max(1) as f32 / viewport_size.1.max(1) as f32;
                let view_height = self.ortho_view_height.max(0.5);
                let view_width = view_height * aspect;

                Camera {
                    position: Vec3::new(self.ortho_center.x, self.ortho_center.y, 0.0),
                    target: Vec3::new(self.ortho_center.x, self.ortho_center.y, -1.0),
                    up: Vec3::Y,
                    projection: ProjectionType::Orthographic,
                    orthographic_size: Vec2::new(view_width, view_height),
                    near: -1000.0,
                    far: 1000.0,
                    fov: 0.0,
                    viewport_size,
                }
            }
            ProjectionType::Perspective => {
                let mut camera = Camera::new_perspective(
                    self.perspective_position,
                    self.perspective_position + self.forward(),
                    Vec3::Y,
                    self.fov_degrees.to_radians(),
                    self.near,
                    self.far,
                );
                camera.resize(viewport_size.0, viewport_size.1);
                camera
            }
        }
    }

    pub fn pan(&mut self, delta: Vec2) {
        if self.is_orthographic() {
            self.ortho_center += delta;
        }
    }

    pub fn set_center(&mut self, center: Vec2) {
        if self.is_orthographic() {
            self.ortho_center = center;
        }
    }

    pub fn get_zoom(&self) -> f32 {
        self.ortho_view_height
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.ortho_view_height = zoom.clamp(1.0, 500.0);
    }

    pub fn zoom_by_factor(&mut self, factor: f32) {
        self.set_zoom(self.ortho_view_height * factor);
    }

    pub fn get_fov(&self) -> f32 {
        self.fov_degrees
    }

    pub fn set_fov(&mut self, degrees: f32) {
        self.fov_degrees = degrees.clamp(20.0, 130.0);
    }

    pub fn get_speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.01);
    }

    pub fn get_sensitivity(&self) -> f32 {
        self.sensitivity
    }

    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity.max(0.01);
    }

    pub fn near(&self) -> f32 {
        self.near
    }

    pub fn set_near(&mut self, near: f32) {
        self.near = near.clamp(0.001, self.far - 0.001);
    }

    pub fn far(&self) -> f32 {
        self.far
    }

    pub fn set_far(&mut self, far: f32) {
        self.far = far.max(self.near + 0.001);
    }

    pub fn frame_2d(&mut self, center: Vec2, view_height: f32) {
        self.ortho_center = center;
        self.ortho_view_height = view_height.clamp(1.0, 500.0);
    }

    pub fn frame_3d(&mut self, target: Vec3, distance: f32) {
        let distance = distance.max(1.0);
        self.perspective_position = target - self.forward() * distance;
    }

    pub fn shutdown(&mut self, window: &Window) {
        self.set_look_active(window, false);
    }

    pub fn is_look_active(&self) -> bool {
        self.look_active
    }

    fn handle_keyboard_input(&mut self, event: &KeyEvent) -> bool {
        let PhysicalKey::Code(code) = event.physical_key else {
            return false;
        };

        if !self.look_active {
            return false;
        }

        match event.state {
            ElementState::Pressed => {
                self.pressed_keys.insert(code);
            }
            ElementState::Released => {
                self.pressed_keys.remove(&code);
            }
        }
        self.look_active
    }

    fn set_look_active(&mut self, window: &Window, active: bool) {
        if !self.is_perspective() {
            self.look_active = false;
            self.pressed_keys.clear();
            return;
        }

        self.look_active = active;
        if !active {
            self.pressed_keys.clear();
        }
        window.set_cursor_visible(!active);
        let _ = window.set_cursor_grab(if active {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        });
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin()).normalize()
    }
}

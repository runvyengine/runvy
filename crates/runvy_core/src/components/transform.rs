use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Debug, Copy, runvy_macros::Scriptable)]
#[script(crate = "::runvy_script_api", builtin)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Default::default(),
            rotation: Default::default(),
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn rotate_x(&mut self, angle: f32) {
        self.rotation *= Quat::from_rotation_x(angle.to_radians());
    }

    pub fn rotate_y(&mut self, angle: f32) {
        self.rotation *= Quat::from_rotation_y(angle.to_radians());
    }

    pub fn rotate_z(&mut self, angle: f32) {
        self.rotation *= Quat::from_rotation_z(angle.to_radians());
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GlobalTransform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,

    pub previous_position: Vec3,
    pub previous_rotation: Quat,
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            previous_position: Vec3::ZERO,
            previous_rotation: Quat::IDENTITY,
        }
    }
}

impl GlobalTransform {
    pub fn from_local(local: &Transform) -> Self {
        Self {
            position: local.position,
            rotation: local.rotation,
            scale: local.scale,
            previous_position: local.position,
            previous_rotation: local.rotation,
        }
    }

    pub fn mul_transform(&self, local: &Transform) -> Self {
        let position = self.position + self.rotation * (self.scale * local.position);

        let rotation = self.rotation * local.rotation;
        let scale = self.scale * local.scale;

        Self {
            position,
            rotation,
            scale,
            previous_position: position,
            previous_rotation: rotation,
        }
    }

    pub fn prepare_for_update(&mut self) {
        self.previous_position = self.position;
        self.previous_rotation = self.rotation;
    }

    pub fn interpolated_position(&self, interpolation_factor: f32) -> Vec3 {
        Vec3::lerp(
            self.previous_position,
            self.position,
            interpolation_factor.clamp(0.0, 1.0),
        )
    }

    pub fn interpolated_rotation(&self, interpolation_factor: f32) -> Quat {
        self.previous_rotation
            .slerp(self.rotation, interpolation_factor.clamp(0.0, 1.0))
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

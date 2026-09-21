use runvy_engine::{
    core::{
        components::{Mesh, MeshRenderer, Transform},
        resources::Scene,
        Quat, Vec3,
    },
    ecs::World,
};

use crate::camera_ctrl::spawn_camera;

pub struct FirstScene {}

impl Scene for FirstScene {
    fn name(&self) -> &str {
        "first_scene"
    }

    fn build(&self, world: &mut World) {
        world.spawn((
            Transform {
                position: Vec3::new(-1.5, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                ..Transform::default()
            },
            MeshRenderer::new(Mesh::cube(1.0)),
        ));

        world.spawn((
            Transform {
                position: Vec3::new(1.5, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(-2.0, 2.0, 2.0),
                ..Transform::default()
            },
            MeshRenderer::new(Mesh::cube(1.0)),
        ));

        let _ = spawn_camera(world);
    }
}

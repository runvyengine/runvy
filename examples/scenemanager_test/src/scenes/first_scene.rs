use runvy_engine::{
    core::{
        components::{Mesh, MeshRenderer, Timer, Transform},
        resources::Scene,
        Quat, Vec3,
    },
    ecs::World,
};

use crate::{
    camera_ctrl::{restore_camera, save_camera, spawn_camera},
    scenes::SCENE_SWITCH_INTERVAL,
};

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
                scale: Vec3::new(2.0, 2.0, 2.0),
                ..Transform::default()
            },
            MeshRenderer::new(Mesh::cube(1.0)),
        ));

        let _ = spawn_camera(world);

        world.spawn((Timer::new(SCENE_SWITCH_INTERVAL, true, false, None),));
    }

    fn on_enter(&self, world: &mut World) {
        restore_camera(world);
    }

    fn on_exit(&self, world: &mut World) {
        save_camera(world);
    }
}

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

pub struct SecondScene {}

impl Scene for SecondScene {
    fn name(&self) -> &str {
        "second_scene"
    }

    fn build(&self, world: &mut World) {
        let count = 8;
        let radius = 2.5;

        for i in 0..count {
            let angle = i as f32 / count as f32 * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;

            world.spawn((
                Transform {
                    position: Vec3::new(x, 0.0, z),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(0.7),
                    ..Transform::default()
                },
                MeshRenderer::new(Mesh::cube(1.0)),
            ));
        }

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

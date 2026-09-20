use runvy_engine::app::{RunvyApp, RunvyWindowConfig};
use runvy_engine::core::components::{MeshRenderer, Transform};
use runvy_engine::core::glam::Quat;
use runvy_engine::core::resources::{Scene, SceneManager, Time};
use runvy_engine::ecs::{World, R, W};
use runvy_engine::system;

use crate::scenes::FirstScene;

mod camera_ctrl;
mod scenes;

#[system(Update)]
fn rotate_cubes(world: &mut World) {
    let dt = world.get_resource::<Time>().delta;
    for (_, (transform, _mesh)) in world.query_mut::<(W<Transform>, R<MeshRenderer>)>() {
        transform.rotation *= Quat::from_rotation_y(0.5 * dt);
    }
}

fn main() {
    let mut world = World::new();

    let first_s = FirstScene {};
    first_s.build(&mut world);

    let mut sm = SceneManager::default();

    sm.switch_to(first_s.name(), &mut world);

    world.add_resource(sm);

    let config = RunvyWindowConfig {
        title: "Runvy 3D Sandbox - rotating cubes".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
        luau_types_path: None,
    };

    let _ = RunvyApp::run_with_config(world, config);
}

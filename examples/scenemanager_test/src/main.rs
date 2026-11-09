use runvy_engine::app::{RunvyApp, RunvyWindowConfig};
use runvy_engine::core::components::{MeshRenderer, Timer, Transform};
use runvy_engine::core::glam::Quat;
use runvy_engine::core::resources::{SceneManager, Time};
use runvy_engine::ecs::{QueryMut, Res, World, R, W};
use runvy_engine::system;

use crate::camera_ctrl::SavedCamera;
use crate::scenes::{FirstScene, SecondScene};

mod camera_ctrl;
mod scenes;

#[system(Update)]
fn rotate_cubes(time: Res<Time>, q: QueryMut<(W<Transform>, R<MeshRenderer>)>) {
    let dt = time.delta;
    for (_, (transform, _mesh)) in q {
        transform.rotation *= Quat::from_rotation_y(0.5 * dt);
    }
}

#[system(Update)]
fn auto_switch_scene(world: &mut World) {
    let fired = world
        .query::<R<Timer>>()
        .any(|(_, timer)| timer.times_fired > 0);

    if !fired {
        return;
    }

    let next = match world.get_resource::<SceneManager>().active() {
        Some("first_scene") => "second_scene",
        _ => "first_scene",
    };

    let mut scenes = world.delete_resource::<SceneManager>();
    scenes.switch_to(next, world);
    world.add_resource(scenes);
}

fn main() {
    let mut world = World::new();

    world.init_resource::<SavedCamera>();

    let mut sm = SceneManager::default();
    sm.register(FirstScene {});
    sm.register(SecondScene {});

    sm.switch_to("first_scene", &mut world);

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

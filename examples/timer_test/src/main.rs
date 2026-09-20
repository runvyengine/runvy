use std::sync::Mutex;

use runvy_engine::{
    app::RunvyApp,
    core::components::{Timer, Transform},
    ecs::World,
};

fn main() {
    let mut world = World::new();

    world.spawn((
        Transform::default(),
        Timer::new(
            1.0,
            true,
            true,
            Some(Mutex::new(Box::new(|| println!("Timeouted")))),
        ),
    ));

    let _ = RunvyApp::run_default(world);
}

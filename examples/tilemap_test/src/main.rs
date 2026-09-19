use runa_engine::{
    app::RunaApp,
    core::components::{Camera, Transform},
    ecs::World,
};

mod tilemap;

fn main() {
    let mut world = World::new();

    world.spawn((Transform::default(),));

    tilemap::spawn_tilemap(&mut world);

    world.spawn((Transform::default(), Camera::new_orthographic(32.0, 18.0)));

    let _ = RunaApp::run_default(world);
}

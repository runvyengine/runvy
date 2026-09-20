use runvy_engine::{
    app::{RunvyApp, RunvyWindowConfig}, core::components::{Camera, Transform}, ecs::World,
};

mod tilemap;

fn main() {
    let mut world = World::new();

    world.spawn((Transform::default(),));

    tilemap::spawn_tilemap(&mut world);

    let config = RunvyWindowConfig {
        title: "Runvy Tilemap Test".into(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
        luau_types_path: Some("scripts".into()),
    };
    
    world.spawn((Transform::default(), Camera::new_orthographic(32.0, 18.0)));

    let _ = RunvyApp::run_with_config(world, config);
}

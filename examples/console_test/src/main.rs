use std::sync::Once;

use runvy_engine::{
    app::{RunvyApp, RunvyWindowConfig},
    core::{components::Camera, Console, MessageLevel},
    ecs::{self, ResMut},
    system,
};

fn main() {
    let mut world = ecs::World::new();

    world.spawn((Camera::new_orthographic(32.0, 18.0),));

    let config = RunvyWindowConfig {
        title: "Runvy Sandbox".to_string(),
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

static INIT: Once = Once::new();

#[system]
fn test(mut console: ResMut<Console>) {
    INIT.call_once(|| {
        console.add_message_with_level("Default message test", MessageLevel::Info);
        console.add_message_with_level("Info message test", MessageLevel::Info);
        console.add_message_with_level("Warning message test", MessageLevel::Warning);
        console.add_message_with_level("Error message test", MessageLevel::Error);
    });
}

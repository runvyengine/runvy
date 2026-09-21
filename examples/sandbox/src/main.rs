use runvy_engine::app::{RunvyApp, RunvyWindowConfig};
use runvy_engine::core::components::{Camera, SpriteRenderer, Transform};
use runvy_engine::core::glam::Vec3;
use runvy_engine::core::resources::{input::InputState, Time};
use runvy_engine::core::KeyCode;
use runvy_engine::ecs::{QueryMut, Res, ResMut, W};
use runvy_engine::prelude::{Console, MessageLevel};
use runvy_engine::system;
use runvy_engine::{asset, ecs};

#[system]
fn player_movement(
    time: Res<Time>,
    mut input: ResMut<InputState>,
    mut console: ResMut<Console>,
    q: QueryMut<W<Transform>>,
) {
    let speed = 8.0;
    let dt = time.delta;
    let mut first_pos = None;

    for (_, transform) in q {
        if first_pos.is_none() {
            first_pos = Some(transform.position);
        }
        let mut dir = Vec3::ZERO;
        if input.is_key_pressed(KeyCode::KeyW) {
            dir.y += 1.0;
        }
        if input.is_key_pressed(KeyCode::KeyS) {
            dir.y -= 1.0;
        }
        if input.is_key_pressed(KeyCode::KeyD) {
            dir.x += 1.0;
        }
        if input.is_key_pressed(KeyCode::KeyA) {
            dir.x -= 1.0;
        }
        transform.position += dir.normalize_or_zero() * speed * dt;
    }

    if input.is_key_just_pressed(KeyCode::KeyF) {
        console.add_message_with_level(
            format!("Player at {:?}", first_pos),
            MessageLevel::Info,
        );
    }
}

fn main() {
    let mut world = ecs::World::new();

    let texture = asset::load_image!("assets/art/Charactert.png");
    world.spawn((
        Transform {
            position: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(1., 1., 16.),
            ..Transform::default()
        },
        SpriteRenderer::new(Some(texture)),
    ));

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

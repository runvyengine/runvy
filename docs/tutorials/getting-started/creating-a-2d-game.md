# Creating a 2D Game

This guide shows the current recommended Runvy pattern for a small 2D game.
Applies to version 0.7.6 and later.

## 1. Create the application

Let's build a 2D scene with a character sprite and a simple WASD controller.

Start with an empty application in `main.rs`:

```rust
// main.rs

use runvy_core::runvy_ecs::World;
use runvy_engine::runvy_app::{RunvyApp, RunvyWindowConfig};

fn main() {
    // Create the world
    let mut world = World::new();

    // Configure the window
    let config = RunvyWindowConfig {
        title: "Small 2D scene in Runvy".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: false,
        window_icon: None,
    };

    let _ = RunvyApp::run_with_config(world, config);
}
```

`run_with_config` takes the world first and the config second. For quick
tests you can also use `RunvyApp::run_default(world)` — it creates a default
window and config for you.

## 2. Character movement controller

Now let's create the system that moves the character:

```rust
// main.rs

// Handles WASD movement for an entity with `Transform`.
// W means "write" — we want to change the component data.
// R means "read" — we only read the component data.
#[system]
fn player_movement(world: &mut World) {
    let speed = 8.0;
    let dt = 1.0 / 60.0;

    for (_, transform) in world.query_mut::<W<Transform>>() {
        let mut dir = Vec3::ZERO;
        if InputState::is_key_pressed(KeyCode::KeyW) {
            dir.y += 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyS) {
            dir.y -= 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyD) {
            dir.x += 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyA) {
            dir.x -= 1.0;
        }
        transform.position += dir.normalize_or_zero() * speed * dt;
    }
}
```

Note: `player_movement` runs for **every** entity that has a `Transform`. In
this scene only the character has one, so it's safe. Once you add more moving
objects, give them a small marker component (like `CameraController` in the 3D
guide) and query for it — then the system only affects the entities you choose.

## 3. The finished file

```rust
// main.rs

use runvy_core::components::{Camera, SpriteRenderer, Transform};
use runvy_core::glam::Vec3;
use runvy_core::input::InputState;
use runvy_core::runvy_ecs::{World, W};
use runvy_core::KeyCode;
use runvy_engine::runvy_app::{RunvyApp, RunvyWindowConfig};
use runvy_engine::system;

#[system]
fn player_movement(world: &mut World) {
    let speed = 8.0;
    let dt = 1.0 / 60.0;

    for (_, transform) in world.query_mut::<W<Transform>>() {
        let mut dir = Vec3::ZERO;
        if InputState::is_key_pressed(KeyCode::KeyW) {
            dir.y += 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyS) {
            dir.y -= 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyD) {
            dir.x += 1.0;
        }
        if InputState::is_key_pressed(KeyCode::KeyA) {
            dir.x -= 1.0;
        }
        transform.position += dir.normalize_or_zero() * speed * dt;
    }
}

fn main() {
    let mut world = World::new();

    // Load the character sprite
    let texture = runvy_asset::load_image!("assets/art/Charactert.png");

    // Spawn the character
    world.spawn((
        Transform {
            position: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(1.0, 1.0, 16.0), // Z in scale is pixels-per-unit for `SpriteRenderer`
            ..Transform::default()
        },
        SpriteRenderer::new(Some(texture)),
    ));

    // Spawn the camera
    world.spawn((Camera::new_orthographic(32.0, 18.0),));

    let config = RunvyWindowConfig {
        title: "Runvy Sandbox".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
    };

    let _ = RunvyApp::run_with_config(world, config);
}
```

## Result

You should now see the character in the center of the screen. It moves with
**WASD** (W/S — up/down, A/D — left/right) inside the 32x18 world-unit view
of the orthographic camera.

For a complete version of this scene, see `examples/sandbox`.

## Next Steps

- [Collider2D](../components/physics-collision.md)
- [Tilemap](../tilemap/tilemap.md)

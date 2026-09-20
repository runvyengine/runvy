# Creating a 3D Game

This guide shows the current recommended Runvy pattern for a small 3D scene.
Applies to version 0.7.6 and later.

## 1. Create the application

Let's build a small 3D scene with a first-person flying camera and rotating cubes.

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
        title: "Small 3D scene in Runvy".to_string(),
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

## 2. Camera controller

Now let's create the component, the system that moves the camera, and a
constructor that spawns the camera entity:

```rust
// camera_controller.rs
use runvy_core::{
    components::{Camera, Transform},
    glam::{Quat, Vec3},
    input::{lock_cursor, show_cursor, InputState},
    runvy_ecs::{World, W},
    KeyCode,
};
use runvy_engine::system;

// Our own component. The system below works with it.
struct CameraController {
    yaw: f32,
    pitch: f32,
    sensitivity: f32,
    speed: f32,
}

impl CameraController {
    fn new(sensitivity: f32, speed: f32) -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            sensitivity,
            speed,
        }
    }
}

// Handles mouse look and WASD movement for the camera entity.
// W means "write" — we want to change the component data.
// R means "read" — we only read the component data.
#[system]
fn camera_controller_system(world: &mut World) {
    // Hide and lock the cursor (FPS-style).
    show_cursor(false);
    lock_cursor(true);

    let dt = 1.0 / 60.0;
    let (dx, dy) = InputState::mouse_delta();

    for (_, (transform, ctrl)) in world.query_mut::<(W<Transform>, W<CameraController>)>() {
        ctrl.yaw -= dx * ctrl.sensitivity;
        ctrl.pitch -= dy * ctrl.sensitivity;
        ctrl.pitch = ctrl.pitch.clamp(-89.0, 89.0);
        transform.rotation = Quat::from_rotation_y(ctrl.yaw.to_radians())
            * Quat::from_rotation_x(ctrl.pitch.to_radians());

        let forward = transform.rotation * -Vec3::Z;
        let right = transform.rotation * Vec3::X;
        let mut move_dir = Vec3::ZERO;

        if InputState::is_key_pressed(KeyCode::KeyW) {
            move_dir += forward;
        }
        if InputState::is_key_pressed(KeyCode::KeyS) {
            move_dir -= forward;
        }
        if InputState::is_key_pressed(KeyCode::KeyD) {
            move_dir += right;
        }
        if InputState::is_key_pressed(KeyCode::KeyA) {
            move_dir -= right;
        }
        if InputState::is_key_pressed(KeyCode::Space) {
            move_dir += Vec3::Y;
        }
        if InputState::is_key_pressed(KeyCode::ShiftLeft) {
            move_dir -= Vec3::Y;
        }

        transform.position += move_dir.normalize_or_zero() * ctrl.speed * dt;
    }
}

// Spawns the camera entity and returns its id.
pub fn spawn_camera(world: &mut World) -> u64 {
    world.spawn((
        Transform {
            position: Vec3::new(0.0, 0.0, 10.0),
            ..Transform::default()
        },
        CameraController::new(0.1, 5.0),
        Camera::new_perspective(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y, 90.0, 0.1, 1000.0),
    ))
}
```

Note that `Camera` keeps its local position and look target (`ZERO` and
`-Z`), while the entity `Transform` holds the world position and orientation.
The renderer combines both every frame.

## 3. Spawn cubes and run

The final step: spawn a few cubes so you can see the camera move, and start
the app with our world:

```rust
// main.rs
use runvy_core::components::{Mesh, MeshRenderer, Transform};
use runvy_core::glam::{Quat, Vec3};
use runvy_core::runvy_ecs::{World, R, W};
use runvy_engine::runvy_app::{RunvyApp, RunvyWindowConfig};
use runvy_engine::system;

use crate::camera_controller::spawn_camera;

mod camera_controller;

// Rotates every object that has both Transform and MeshRenderer.
#[system]
fn rotate_cubes(world: &mut World) {
    let dt = 1.0 / 60.0;
    for (_, (transform, _mesh)) in world.query_mut::<(W<Transform>, R<MeshRenderer>)>() {
        transform.rotation *= Quat::from_rotation_y(0.5 * dt);
    }
}

fn main() {
    let mut world = World::new();

    // First cube
    world.spawn((
        Transform {
            position: Vec3::new(-1.5, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            ..Transform::default()
        },
        MeshRenderer::new(Mesh::cube(1.0)),
    ));

    // Second cube
    world.spawn((
        Transform {
            position: Vec3::new(1.5, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(-2.0, 2.0, 2.0),
            ..Transform::default()
        },
        MeshRenderer::new(Mesh::cube(1.0)),
    ));

    // Our entity with camera and controller
    let _ = spawn_camera(&mut world);

    let config = RunvyWindowConfig {
        title: "Runvy 3D Sandbox - rotating cubes".to_string(),
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

You should now see two rotating cubes in the center of the screen. The camera
flies with **WASD** and **Space/Shift** (up/down) and looks around with the
mouse; the cursor is hidden and locked.

For a complete version of this scene, see `examples/sandbox_3d`.

## Next Steps

- [Input](../systems/input.md)
- [Renderer Notes](../../architecture/renderer.md)

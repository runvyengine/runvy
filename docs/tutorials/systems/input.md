<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Input System

The input system handles keyboard and mouse input. In the current runtime it also exposes control over the main game window.

## Keyboard Input

```rust
use runvy_engine::runvy_core::input_system::*;

if Input::is_key_pressed(KeyCode::KeyW) {
    // continuous action
}

if Input::is_key_just_pressed(KeyCode::Space) {
    // one-shot action
}
```

## Mouse Input

```rust
use runvy_engine::runvy_core::input_system::*;
use winit::event::MouseButton;

if Input::is_mouse_button_just_pressed(MouseButton::Left) {
    println!("Clicked");
}

if let Some(mouse_pos) = Input::get_mouse_world_position() {
    println!("{mouse_pos:?}");
}
```

## Window Control

The current runtime is single-window:

```rust
use runvy_engine::runvy_core::input_system::*;

set_window_title("Debug View");
set_fullscreen(true);
toggle_fullscreen();
set_window_size(1600, 900);
set_window_position(120, 80);
move_window_by(16, 0);
center_window();
```

## Movement Example

```rust
use runvy_engine::runvy_core::{
    components::Transform,
    glam::Vec3,
    input_system::*,
    ocs::{Object, Script, ScriptContext},
};

pub struct PlayerController {
    speed: f32,
}

impl Script for PlayerController {
    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        let mut direction = Vec3::ZERO;

        if Input::is_key_pressed(KeyCode::KeyW) { direction.y += 1.0; }
        if Input::is_key_pressed(KeyCode::KeyS) { direction.y -= 1.0; }
        if Input::is_key_pressed(KeyCode::KeyA) { direction.x -= 1.0; }
        if Input::is_key_pressed(KeyCode::KeyD) { direction.x += 1.0; }

        if let Some(transform) = ctx.get_component_mut::<Transform>() {
            transform.position += direction.normalize_or_zero() * self.speed * dt;
        }
    }
}

fn create_player() -> Object {
    Object::new("Player")
        .with(Transform::default())
        .with(PlayerController { speed: 5.0 })
}
```

## Notes

- use `is_key_just_pressed` for one-shot actions
- use `is_key_pressed` for continuous actions
- multiply movement by `dt`
- mouse world position requires a valid camera
- window-control functions affect the main runtime window only
- multi-window runtime support is not implemented


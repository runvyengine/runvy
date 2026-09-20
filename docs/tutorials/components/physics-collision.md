<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Collision Components

Runvy currently has simple collision detection, not a full physics engine.

Relevant components:

- `Collider2D` for simple AABB overlap checks
- `PhysicsCollision` for older runtime-facing size data

If you want script-facing 2D overlap checks, use `Collider2D`.

## Composition Example

```rust
use runvy_engine::runvy_core::{components::{Collider2D, Transform}, ocs::Object};

let player = Object::new("Player")
    .with(Transform::default())
    .with(Collider2D::new(32.0, 32.0));
```

## Movement Example

```rust
use runvy_engine::runvy_core::{
    components::{Collider2D, Transform},
    glam::Vec3,
    input_system::*,
    ocs::{Object, Script, ScriptContext},
};

pub struct PlayerController {
    speed: f32,
}

impl PlayerController {
    pub fn new() -> Self {
        Self { speed: 5.0 }
    }
}

impl Script for PlayerController {
    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        let Some(current_position) = ctx
            .get_component::<Transform>()
            .map(|transform| transform.position)
        else {
            return;
        };

        let mut direction = Vec3::ZERO;
        if Input::is_key_pressed(KeyCode::KeyW) { direction.y += 1.0; }
        if Input::is_key_pressed(KeyCode::KeyS) { direction.y -= 1.0; }
        if Input::is_key_pressed(KeyCode::KeyA) { direction.x -= 1.0; }
        if Input::is_key_pressed(KeyCode::KeyD) { direction.x += 1.0; }

        let next_position = current_position + direction.normalize_or_zero() * self.speed * dt;

        if !ctx.would_collide_2d_at(next_position.truncate()) {
            if let Some(transform) = ctx.get_component_mut::<Transform>() {
                transform.position = next_position;
            }
        }
    }
}

fn create_player() -> Object {
    Object::new("Player")
        .with(Transform::default())
        .with(Collider2D::new(32.0, 32.0))
        .with(PlayerController::new())
}
```

## Notes

- `Collider2D` is detection-only
- it does not push objects apart or solve penetration
- the current helper methods are intended for simple gameplay movement checks
- `size` is stored internally as half extents


<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Transform Component

`Transform` stores an object's position, rotation, and scale.

In the current runtime model:

- `Transform` is a normal component
- `Object::new(...)` auto-adds it
- every object is guaranteed to have one

That means the common case is:

```rust
let object = Object::new("Mover");
```

You only add `Transform` explicitly when you want non-default initial values:

```rust
use runvy_engine::runvy_core::{Quat, Vec3};
use runvy_engine::runvy_core::components::Transform;

let object = Object::new("Mover").with(Transform {
    position: Vec3::new(4.0, 2.0, 0.0),
    rotation: Quat::IDENTITY,
    scale: Vec3::splat(2.0),
    previous_position: Vec3::new(4.0, 2.0, 0.0),
    previous_rotation: Quat::IDENTITY,
});
```

## Common Usage

```rust
use runvy_engine::runvy_core::glam::Vec3;

if let Some(transform) = ctx.get_component_mut::<Transform>() {
    transform.position = Vec3::new(1.0, 2.0, 0.0);
    transform.scale = Vec3::new(2.0, 2.0, 1.0);
    transform.rotate_z(45.0);
}
```

## Behavior Example

```rust
use runvy_engine::runvy_core::{
    components::Transform,
    ocs::{Object, Script, ScriptContext},
};

pub struct Mover {
    speed: f32,
}

impl Mover {
    pub fn new() -> Self {
        Self { speed: 2.0 }
    }
}

impl Script for Mover {
    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        if let Some(transform) = ctx.get_component_mut::<Transform>() {
            transform.position.x += self.speed * dt;
        }
    }
}

fn create_mover() -> Object {
    Object::new("Mover").with(Mover::new())
}
```

## Notes

- `Transform` cannot be removed from an object
- use `dt` for frame-rate independent movement
- for 2D games, keep Z near zero and rotate around Z
- `Transform` is data; movement logic belongs in scripts or systems


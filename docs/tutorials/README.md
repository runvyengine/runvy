<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->

# Runvy Engine Tutorials

These guides assume the current runtime model:

- `Object` is a component container with identity
- `Transform` exists by default
- scripts are behavior attachments
- world mutations from scripts use deferred commands
- code-first bootstrap stays first-class
- registration is no longer needed — use `#[derive(Component)]` directly

## Getting Started

1. [Creating Your First App](getting-started/creating-your-first-app.md)
2. [Creating a 2D Game](getting-started/creating-a-2d-game.md)
3. [Creating a 3D Game](getting-started/creating-a-3d-game.md)
4. [Creating Scripts](scripts/creating-scripts.md)
5. [Registration And Archetypes (archived)](advanced/registration-and-archetypes.md)

## Core Concepts

- [Scripts](scripts/creating-scripts.md)
- [Transform](components/transform.md)
- [Input](systems/input.md)
- [Object Model Notes](../architecture/object-model.md)
- [Registration And Archetypes (archived)](advanced/registration-and-archetypes.md)

## Components and Systems

- [SpriteRenderer](components/sprite-renderer.md)
- [SpriteAnimator](components/sprite-animator.md)
- [Sorting](components/sorting.md)
- [CursorInteractable](components/cursor-interactable.md)
- [Collider2D / PhysicsCollision](components/physics-collision.md)
- [Audio](systems/audio.md)
- [Tilemap](tilemap/tilemap.md)

## Getting Started Example

Needs to be rewritten to match the current data.

```rust
use runvy_engine::prelude::*;

#[derive(Component)]
struct Health {
    current: i32,
    max: i32,
}

struct MyBehavior;

impl Script for MyBehavior {
    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        let _ = (ctx, dt);
    }
}

fn main() {
    let engine = Engine::new();
    let world_rc = engine.create_world();

    world_rc.borrow_mut().spawn_bundle((
        Transform::default(),
        SpriteRenderer::default(),
        Health { current: 100, max: 100 },
        MyBehavior,
    ));

    let _ = RunvyApp::run_with_config(
        world_rc,
        RunvyWindowConfig::default(),
    );
}
```

## Why The Tutorials Use This Style

This style keeps:

- composition explicit
- behavior local to scripts
- editor dependency out of runtime code

It also gives Runvy a future path for editor tools without moving the source of truth away from the runtime object model.

<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# SpriteAnimator Component

`SpriteAnimator` plays grid-based sprite sheet animations by updating the `SpriteRenderer` on the same object.

This keeps responsibilities separate:

- `SpriteRenderer` owns texture rendering, `pixels_per_unit`, and current `uv_rect`
- `SpriteAnimator` owns sprite sheet layout, clips, playback state, current frame, and FPS

## Basic Usage

```rust
use runvy_engine::runvy_asset::load_image;
use runvy_engine::runvy_core::{
    components::{SpriteAnimationClip, SpriteAnimator, SpriteRenderer, SpriteSheet},
    ocs::Object,
};

let object = Object::new("Player")
    .with(SpriteRenderer::new(Some(load_image!("assets/player_sheet.png"))))
    .with(
        SpriteAnimator::new(SpriteSheet::new(4, 2))
            .with_clip(SpriteAnimationClip::new("Idle", 0, 3, 8.0))
            .with_clip(SpriteAnimationClip::new("Run", 4, 7, 12.0)),
    );
```

`SpriteAnimator` requires `SpriteRenderer` on the same object. If the renderer is missing, the animator does nothing.

## Switching Clips

Switch clips from gameplay code:

```rust
if let Some(animator) = ctx.get_component_mut::<SpriteAnimator>() {
    animator.play_clip("Run");
}
```

The current implementation intentionally does not include a state machine.
Use explicit clip switching first; a future state machine can be layered on top of clips without changing the render path.

## Editor (Frozen)

> The editor integration is currently frozen. This section is preserved for reference.

The inspector exposes:

- sheet columns and rows
- current clip and current frame
- playback state
- clip name, start frame, end frame, FPS, and loop flag

When an object has both `SpriteRenderer` and `SpriteAnimator`, the editor previews the current sprite sheet frame instead of the full sheet.


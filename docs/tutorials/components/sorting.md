<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Sorting Component

`Sorting` controls 2D render order independently from spawn order.

```rust
use runvy_engine::runvy_core::{
    components::{Sorting, SpriteRenderer},
    ocs::Object,
};

let object = Object::new("Foreground Sprite")
    .with(SpriteRenderer::default())
    .with(Sorting::new(10));
```

Lower `order` values render earlier. Higher `order` values render later and appear on top when sprites overlap.

## When To Use It

- Use `Transform.position.z` for spatial depth when that is enough.
- Use `Sorting.order` for explicit 2D layering such as background, props, characters, and UI-like world sprites.
- Tilemaps also respect the same 2D ordering path, so sprites no longer depend on object spawn order.

## Editor

The inspector exposes `order` directly. The component picker and component card use the `c-Sorting` icon.


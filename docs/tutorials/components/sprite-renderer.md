<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# SpriteRenderer Component

`SpriteRenderer` displays a 2D texture.

Attach it during object composition:

```rust
use runvy_engine::runvy_asset::load_image;
use runvy_engine::runvy_core::{components::SpriteRenderer, ocs::Object};

let object = Object::new("Sprite")
    .with(SpriteRenderer::new(Some(load_image!("assets/player.png"))));
```

By default, `SpriteRenderer::new(...)` uses `16` pixels per world unit.
You can override that through `pixels_per_unit` when you need a different sprite-to-world scale.

## Player Example

```rust
use runvy_engine::runvy_asset::load_image;
use runvy_engine::runvy_core::{
    components::{SpriteRenderer, Transform},
    ocs::{Object, Script, ScriptContext},
};

pub struct PlayerController;

impl Script for PlayerController {
    fn update(&mut self, ctx: &mut ScriptContext, dt: f32) {
        if let Some(transform) = ctx.get_component_mut::<Transform>() {
            transform.position.x += dt;
        }
    }
}

fn create_player() -> Object {
    Object::new("Player")
        .with(Transform::default())
        .with(SpriteRenderer::new(Some(load_image!("assets/player.png"))))
        .with(PlayerController)
}
```

## Pixels Per Unit

`pixels_per_unit` controls how texture pixel size turns into world size.

- `16 px` at `16 PPU` becomes `1.0` world unit
- `32 px` at `16 PPU` becomes `2.0` world units
- `16 px` at `32 PPU` becomes `0.5` world units

You can set it explicitly:

```rust
let object = Object::new("Sprite")
    .with(sprite!("assets/player.png"))
    .with(Transform::default());
```

The editor inspector (frozen) exposed the same property through `SpriteRendererAsset`.

## Sprite Sheets

`SpriteRenderer` can render a sub-region of the texture through `uv_rect`.
For animation, prefer adding `SpriteAnimator` to the same object instead of changing `uv_rect` manually.
`SpriteAnimator` owns frame timing and updates `SpriteRenderer` before rendering.

## Notes

- PNG is the most practical default format
- attach `Transform` alongside `SpriteRenderer` for placement
- keep rendering data in components and behavior in scripts
- use `sprite!("path")` or `SpriteRenderer::from_path("path")` for convenience


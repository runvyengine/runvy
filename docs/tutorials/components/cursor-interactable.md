<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# CursorInteractable Component

`CursorInteractable` makes an object respond to hover and click state.

## Composition Example

```rust
use runvy_engine::runvy_core::{components::CursorInteractable, ocs::Object};

let mut interactable = CursorInteractable::new(100.0, 50.0);
interactable.set_on_hover_enter(|| println!("Hover enter"));
interactable.set_on_hover_exit(|| println!("Hover exit"));

let object = Object::new("Button").with(interactable);
```

## Behavior Example

```rust
use runvy_engine::runvy_core::{
    components::{CursorInteractable, SpriteRenderer, Transform},
    input_system::*,
    ocs::{Object, Script, ScriptContext},
};

pub struct ButtonBehavior;

impl Script for ButtonBehavior {
    fn update(&mut self, ctx: &mut ScriptContext, _dt: f32) {
        if let Some(interactable) = ctx.get_component::<CursorInteractable>() {
            if interactable.is_hovered && Input::is_mouse_button_just_pressed(MouseButton::Left) {
                println!("Button clicked");
            }
        }
    }
}

fn create_button() -> Object {
    let mut interactable = CursorInteractable::new(200.0, 60.0);
    interactable.set_on_hover_enter(|| println!("Button hover"));
    interactable.set_on_hover_exit(|| println!("Button unhover"));

    Object::new("Button")
        .with(Transform::default())
        .with(SpriteRenderer {
            texture: Some(runvy_engine::runvy_asset::load_image!("assets/button.png")),
            texture_path: Some("assets/button.png".to_string()),
        })
        .with(interactable)
        .with(ButtonBehavior)
}
```

## Notes

- `CursorInteractable` defines bounds and hover state
- click handling usually lives in a script
- callbacks are best used for lightweight state transitions or debug feedback


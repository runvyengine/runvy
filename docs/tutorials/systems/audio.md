# Audio System

Runvy supports simple 2D and spatial audio through `AudioSource` and `AudioListener`.

## Composition

Needs to be rewritten to match the current data.

Attach audio components when building the object:

```rust
use runvy_engine::runvy_core::{components::AudioSource, ocs::Object};

let object = Object::new("Sound Source").with(
    AudioSource::with_asset(runvy_engine::runvy_asset::load_audio!("assets/sound.ogg"))
);
```

For spatial audio:

```rust
let emitter = Object::new("Emitter")
    .with(Transform::default())
    .with(AudioSource::with_asset_3d(
        runvy_engine::runvy_asset::load_audio!("assets/ambient.ogg")
    ));
```

## Listener

```rust
let listener = Object::new("Listener")
    .with(Transform::default())
    .with(AudioListener::new());
```

## Triggering Audio from a Script

```rust
use runvy_engine::runvy_core::{
    components::{AudioSource, Transform},
    input_system::*,
    ocs::{Object, Script, ScriptContext},
};

pub struct PlayerAudio;

impl Script for PlayerAudio {
    fn update(&mut self, ctx: &mut ScriptContext, _dt: f32) {
        if Input::is_key_just_pressed(KeyCode::Space) {
            if let Some(audio) = ctx.get_component_mut::<AudioSource>() {
                audio.play();
            }
        }
    }
}

fn create_player() -> Object {
    Object::new("Player")
        .with(Transform::default())
        .with(AudioSource::with_asset(
            runvy_engine::runvy_asset::load_audio!("assets/jump.ogg")
        ))
        .with(PlayerAudio)
}
```

## Spatial Emitter Example

```rust
fn create_emitter() -> Object {
    let mut transform = Transform::default();
    transform.position.x = 5.0;

    let mut audio = AudioSource::with_asset_3d(
        runvy_engine::runvy_asset::load_audio!("assets/ambient.ogg")
    );
    audio.looped = true;
    audio.play_on_awake = true;

    Object::new("Emitter")
        .with(transform)
        .with(audio)
}
```

## Notes

- `AudioSource` stores playback state and sound data
- `AudioListener` represents the active listening point
- one active listener is used at a time
- attach audio data during composition, trigger playback from behavior

use runvy_engine::app::{RunvyApp, RunvyWindowConfig};
use runvy_engine::core::components::{AudioListener, AudioSource, Camera, Transform};
use runvy_engine::core::resources::input::InputState;
use runvy_engine::core::KeyCode;
use runvy_engine::ecs::{QueryMut, ResMut, W};
use runvy_engine::system;
use runvy_engine::{asset, ecs};

#[system]
fn toggle_sound(mut input: ResMut<InputState>, q: QueryMut<W<AudioSource>>) {
    if input.is_key_just_pressed(KeyCode::Space) {
        for (_, source) in q {
            if source.playing {
                source.stop();
            } else {
                source.play();
            }
        }
    }
}

fn main() {
    let mut world = ecs::World::new();

    world.spawn((Camera::new_orthographic(320.0, 180.0),));
    world.spawn((AudioListener::new(), Transform::default()));

    let audio_asset = asset::load_audio!("assets/audio/test.ogg");
    let mut source = AudioSource::with_asset(audio_asset);
    source.looped = true;
    source.play();

    world.spawn((source,));

    let config = RunvyWindowConfig {
        title: "Runvy Sound Test — Space to toggle".to_string(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: false,
        show_fps_in_title: true,
        window_icon: None,
        luau_types_path: None,
    };

    let _ = RunvyApp::run_with_config(world, config);
}

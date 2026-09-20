use crate::scene::SceneManager;
use runvy_ecs::World;

/// Central engine handle.
///
/// Owns resources that must survive scene switches, such as the
/// [`SceneManager`].  It lives *outside* the ECS `World` so that
/// `World::clear()` during `switch_scene` never destroys it.
pub struct Engine {
    pub scene_manager: SceneManager,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            scene_manager: SceneManager::new(),
        }
    }

    /// Switch to the named scene:
    ///
    /// 1. Calls `on_exit` on the current scene.
    /// 2. Clears the entire `World`.
    /// 3. Calls `build` + `on_enter` on the target scene.
    ///
    /// The `Engine` itself (and therefore the `SceneManager`) is **not**
    /// stored in the world, so it survives the clear.
    pub fn switch_scene(&mut self, name: &str, world: &mut World) {
        self.scene_manager.switch_to(name, world);
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

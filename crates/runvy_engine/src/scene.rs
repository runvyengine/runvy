use runvy_ecs::World;
use serde::{Deserialize, Serialize};
use std::path::Path;

inventory::collect!(SceneDescriptor);

pub struct SceneDescriptor {
    pub name: &'static str,
    pub factory: fn() -> Box<dyn Scene>,
}

pub trait Scene: Send + 'static {
    fn name(&self) -> &str;
    fn build(&self, world: &mut World);
    fn on_enter(&self, _world: &mut World) {}
    fn on_exit(&self, _world: &mut World) {}
}

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub current_scene: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            current_scene: String::new(),
            metadata: serde_json::Value::Object(Default::default()),
        }
    }
}

pub struct SceneManager {
    scenes: Vec<Box<dyn Scene>>,
    active: Option<String>,
}

impl SceneManager {
    pub fn new() -> Self {
        let mut sm = Self {
            scenes: Vec::new(),
            active: None,
        };
        sm.collect_registered();
        sm
    }

    fn collect_registered(&mut self) {
        for desc in inventory::iter::<SceneDescriptor> {
            let scene = (desc.factory)();
            self.scenes.push(scene);
        }
    }

    pub fn register(&mut self, scene: impl Scene) {
        self.scenes.push(Box::new(scene));
    }

    pub fn switch_to(&mut self, name: &str, world: &mut World) {
        let idx = match self.scenes.iter().position(|s| s.name() == name) {
            Some(i) => i,
            None => {
                eprintln!("SceneManager: scene '{}' not found", name);
                return;
            }
        };

        if let Some(ref current) = self.active {
            if current == name {
                return;
            }
            if let Some(old_idx) = self.scenes.iter().position(|s| s.name() == current) {
                self.scenes[old_idx].on_exit(world);
            }
        }

        world.clear();
        self.scenes[idx].build(world);
        self.scenes[idx].on_enter(world);
        self.active = Some(name.to_string());
    }

    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn save(&self) -> SaveData {
        SaveData {
            current_scene: self.active.clone().unwrap_or_default(),
            metadata: serde_json::Value::Object(Default::default()),
        }
    }

    pub fn load(&mut self, data: &SaveData, world: &mut World) {
        if !data.current_scene.is_empty() {
            self.switch_to(&data.current_scene, world);
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let data = self.save();
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        world: &mut World,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let data: SaveData = serde_json::from_str(&json)?;
        self.load(&data, world);
        Ok(())
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

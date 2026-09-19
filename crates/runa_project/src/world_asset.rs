use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use runa_asset::{AudioAsset, Handle, TextureAsset};
use runa_core::components::ui::{CanvasSpace, UiAssetFile, UiRenderer};
use runa_core::components::{
    ActiveCamera, AudioSource, BackgroundMode, Camera, Collider2D, Material, Mesh, MeshRenderer,
    ObjectDefinitionInstance, PhysicsCollision, ProjectionType, SerializedField,
    SerializedTypeEntry, SerializedTypeKind, SerializedTypeStorage, Sorting, SpriteAnimationClip,
    SpriteAnimator, SpriteRenderer, SpriteSheet, Tilemap, TilemapLayer, TilemapRenderer, Transform,
    WorldAtmosphere, EMPTY_TILE, DEFAULT_SPRITE_PIXELS_PER_UNIT,
};
use runa_core::glam::{IVec2, Quat, USizeVec2, Vec2, Vec3};
use runa_core::ocs::{Object, ObjectComponentInfo};
use runa_core::World;
use serde::{Deserialize, Serialize};

use crate::project::{find_project_manifest, load_project, ProjectError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldAsset {
    #[serde(default = "default_world_asset_version")]
    pub version: u32,
    #[serde(default)]
    pub atmosphere: WorldAtmosphereAsset,
    pub objects: Vec<WorldObjectAsset>,
}

fn default_world_asset_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldAtmosphereAsset {
    #[serde(default = "default_ambient_color")]
    pub ambient_color: [f32; 3],
    #[serde(default = "default_ambient_intensity")]
    pub ambient_intensity: f32,
    #[serde(default = "default_background_intensity")]
    pub background_intensity: f32,
    #[serde(default)]
    pub background: BackgroundModeAsset,
}

impl Default for WorldAtmosphereAsset {
    fn default() -> Self {
        Self::from_atmosphere(&WorldAtmosphere::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackgroundModeAsset {
    SolidColor {
        color: [f32; 3],
    },
    VerticalGradient {
        zenith_color: [f32; 3],
        horizon_color: [f32; 3],
        ground_color: [f32; 3],
        horizon_height: f32,
        smoothness: f32,
    },
    Sky,
}

impl Default for BackgroundModeAsset {
    fn default() -> Self {
        let BackgroundMode::VerticalGradient {
            zenith_color,
            horizon_color,
            ground_color,
            horizon_height,
            smoothness,
        } = BackgroundMode::default()
        else {
            unreachable!("default atmosphere background must be a vertical gradient")
        };

        Self::VerticalGradient {
            zenith_color: zenith_color.to_array(),
            horizon_color: horizon_color.to_array(),
            ground_color: ground_color.to_array(),
            horizon_height,
            smoothness,
        }
    }
}

fn default_ambient_color() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

fn default_ambient_intensity() -> f32 {
    0.15
}

fn default_background_intensity() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldObjectAsset {
    pub name: String,
    pub object_id: Option<String>,
    #[serde(default)]
    pub parent: Option<usize>,
    pub transform: TransformAsset,
    pub mesh_renderer: Option<MeshRendererAsset>,
    pub sprite_renderer: Option<SpriteRendererAsset>,
    #[serde(default)]
    pub sprite_animator: Option<SpriteAnimatorAsset>,
    #[serde(default)]
    pub sorting: Option<SortingAsset>,
    pub tilemap: Option<TilemapAsset>,
    pub camera: Option<CameraAsset>,
    pub active_camera: bool,
    pub audio_source: Option<AudioSourceAsset>,
    #[serde(default)]
    pub collider2d: Option<Collider2DAsset>,
    pub physics_collision: Option<PhysicsCollisionAsset>,
    #[serde(default)]
    pub ui_renderer: Option<UiRendererAsset>,
    pub serialized_components: Vec<SerializedObjectTypeAsset>,
    pub serialized_scripts: Vec<SerializedObjectTypeAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceableObjectDescriptor {
    pub id: String,
    pub name: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceableObjectRecord {
    pub descriptor: PlaceableObjectDescriptor,
    pub object: WorldObjectAsset,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectRegisteredTypeKind {
    Component,
    Script,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectRegistrationSource {
    BuiltIn,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRegisteredTypeRecord {
    pub type_name: String,
    pub kind: ProjectRegisteredTypeKind,
    pub source: ProjectRegistrationSource,
    pub editor_addable: bool,
    pub default_fields: Vec<SerializedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadataSnapshot {
    pub object_records: Vec<PlaceableObjectRecord>,
    pub registered_types: Vec<ProjectRegisteredTypeRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedObjectTypeAsset {
    pub type_name: String,
    pub fields: Vec<SerializedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformAsset {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Default for TransformAsset {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshRendererAsset {
    pub primitive: MeshPrimitiveAsset,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpriteRendererAsset {
    pub sprite: Option<String>,
    #[serde(default = "default_sprite_pixels_per_unit")]
    pub pixels_per_unit: f32,
    #[serde(default = "default_sprite_uv_rect")]
    pub uv_rect: [f32; 4],
}

fn default_sprite_pixels_per_unit() -> f32 {
    DEFAULT_SPRITE_PIXELS_PER_UNIT
}

fn default_sprite_uv_rect() -> [f32; 4] {
    SpriteRenderer::FULL_UV_RECT
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteAnimatorAsset {
    #[serde(default = "default_sprite_sheet_columns")]
    pub columns: u32,
    #[serde(default = "default_sprite_sheet_rows")]
    pub rows: u32,
    #[serde(default)]
    pub clips: Vec<SpriteAnimationClipAsset>,
    #[serde(default)]
    pub current_clip: Option<String>,
    #[serde(default)]
    pub current_frame: u32,
    #[serde(default = "default_sprite_animator_playing")]
    pub playing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteAnimationClipAsset {
    pub name: String,
    pub start_frame: u32,
    pub end_frame: u32,
    pub fps: f32,
    pub looping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortingAsset {
    pub order: i32,
    pub y_sort: bool,
    pub y_offset: f32,
}

fn default_sprite_sheet_columns() -> u32 {
    1
}

fn default_sprite_sheet_rows() -> u32 {
    1
}

fn default_sprite_animator_playing() -> bool {
    true
}

fn default_tilemap_pixels_per_unit() -> f32 {
    16.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshPrimitiveAsset {
    Cube { size: f32 },
    Quad { width: f32, height: f32 },
    Plane { width: f32, depth: f32 },
    Pyramid { width: f32, height: f32, depth: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraAsset {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub perspective: bool,
    pub ortho_size: [f32; 2],
    pub near: f32,
    pub far: f32,
    pub fov_radians: f32,
    pub viewport_size: [u32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSourceAsset {
    pub source: Option<String>,
    pub volume: f32,
    pub looped: bool,
    pub play_on_awake: bool,
    pub spatial: bool,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Default for CameraAsset {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            perspective: true,
            ortho_size: [320.0, 180.0],
            near: 0.1,
            far: 1000.0,
            fov_radians: 75.0_f32.to_radians(),
            viewport_size: [1280, 720],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsCollisionAsset {
    pub size: [f32; 2],
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collider2DAsset {
    pub half_size: [f32; 2],
    pub enabled: bool,
    #[serde(default)]
    pub is_trigger: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiRendererAsset {
    #[serde(default)]
    pub ui_asset_path: Option<String>,
    #[serde(default)]
    pub space: String,
    #[serde(default)]
    pub inline_ui: Option<UiAssetFile>,
}

impl Default for UiRendererAsset {
    fn default() -> Self {
        Self {
            ui_asset_path: None,
            space: "Screen".to_string(),
            inline_ui: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilemapAsset {
    pub width: u32,
    pub height: u32,
    pub tile_size: [u32; 2],
    pub offset: [i32; 2],
    #[serde(default = "default_tilemap_pixels_per_unit")]
    pub pixels_per_unit: f32,
    #[serde(default)]
    pub atlas: Option<TilemapAtlasAsset>,
    #[serde(default)]
    pub selected_tile: u32,
    pub layers: Vec<TilemapLayerAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilemapAtlasAsset {
    pub texture: Option<String>,
    #[serde(default = "default_sprite_sheet_columns")]
    pub columns: u32,
    #[serde(default = "default_sprite_sheet_rows")]
    pub rows: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilemapLayerAsset {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    #[serde(default)]
    pub tiles: Vec<Option<u32>>,
    #[serde(default, rename = "self_order")]
    pub order: i32,
}

pub fn create_empty_world() -> Rc<RefCell<World>> {
    let mut world = World::default();

    let mut camera = Object::new("Main Camera");
    camera.add_component(Camera::default());
    camera.add_component(ActiveCamera);
    world.spawn(camera);

    Rc::new(RefCell::new(world))
}

pub fn save_world(path: impl AsRef<Path>, world: &World) -> Result<(), ProjectError> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    let asset = WorldAsset::from_world(world);
    let content = ron::ser::to_string_pretty(&asset, ron::ser::PrettyConfig::default())?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_world(path: impl AsRef<Path>) -> Result<World, ProjectError> {
    let content = fs::read_to_string(path.as_ref())?;
    let asset: WorldAsset = ron::from_str(&content)?;
    let project_root = project_root_for_world_path(path.as_ref());
    Ok(asset.into_world_with_project_root(project_root.as_deref()))
}

pub fn load_world_with_runtime_registry(
    path: impl AsRef<Path>,
    runtime_registry: &runa_core::registry::RuntimeRegistry,
) -> Result<Rc<RefCell<World>>, ProjectError> {
    let content = fs::read_to_string(path.as_ref())?;
    let asset: WorldAsset = ron::from_str(&content)?;
    let project_root = project_root_for_world_path(path.as_ref());
    Ok(asset.into_world_with_runtime_registry(project_root.as_deref(), runtime_registry))
}

pub fn load_world_with_object_loader<F>(
    path: impl AsRef<Path>,
    object_loader: F,
) -> Result<Rc<RefCell<World>>, ProjectError>
where
    F: Fn(&str) -> Option<WorldObjectAsset>,
{
    let content = fs::read_to_string(path.as_ref())?;
    let asset: WorldAsset = ron::from_str(&content)?;
    let project_root = project_root_for_world_path(path.as_ref());
    Ok(asset.into_world_with_object_loader(project_root.as_deref(), object_loader))
}

impl WorldAsset {
    pub fn from_world(world: &World) -> Self {
        let object_ids = world.query::<Transform>();
        let object_index: HashMap<_, _> = object_ids
            .iter()
            .enumerate()
            .map(|(index, object_id)| (*object_id, index))
            .collect();

        Self {
            version: default_world_asset_version(),
            atmosphere: WorldAtmosphereAsset::from_atmosphere(world.atmosphere()),
            objects: object_ids
                .into_iter()
                .filter_map(|object_id| world.object(object_id))
                .map(|object| WorldObjectAsset::from_object_with_parent_map(object, &object_index))
                .collect(),
        }
    }

    pub fn into_world(self) -> World {
        self.into_world_with_project_root(None)
    }

    pub fn into_world_with_project_root(self, project_root: Option<&Path>) -> World {
        let mut world = World::default();
        world.set_atmosphere(self.atmosphere.into_atmosphere());
        let mut parent_links = Vec::new();
        let mut spawned_ids = Vec::new();
        for object in self.objects.into_iter() {
            parent_links.push(object.parent);
            spawned_ids.push(world.spawn(object.into_object(project_root)));
        }
        apply_parent_links(&mut world, &spawned_ids, &parent_links);
        world
    }

    pub fn into_world_with_runtime_registry(
        self,
        project_root: Option<&Path>,
        runtime_registry: &runa_core::registry::RuntimeRegistry,
    ) -> Rc<RefCell<World>> {
        let mut world = World::default();
        world.set_atmosphere(self.atmosphere.into_atmosphere());
        let mut parent_links = Vec::new();
        let mut spawned_ids = Vec::new();
        for object in self.objects.into_iter() {
            parent_links.push(object.parent);
            spawned_ids.push(world.spawn(
                object.into_object_with_runtime_registry(project_root, Some(runtime_registry)),
            ));
        }
        apply_parent_links(&mut world, &spawned_ids, &parent_links);
        Rc::new(RefCell::new(world))
    }

    pub fn into_world_with_object_loader<F>(
        self,
        project_root: Option<&Path>,
        object_loader: F,
    ) -> Rc<RefCell<World>>
    where
        F: Fn(&str) -> Option<WorldObjectAsset>,
    {
        let mut world = World::default();
        world.set_atmosphere(self.atmosphere.into_atmosphere());
        let mut parent_links = Vec::new();
        let mut spawned_ids = Vec::new();
        for object in self.objects.into_iter() {
            parent_links.push(object.parent);
            spawned_ids.push(
                world.spawn(object.into_object_with_object_loader(project_root, &object_loader)),
            );
        }
        apply_parent_links(&mut world, &spawned_ids, &parent_links);
        Rc::new(RefCell::new(world))
    }
}

fn apply_parent_links(
    world: &mut World,
    spawned_ids: &[runa_core::ocs::ObjectId],
    parent_links: &[Option<usize>],
) {
    for (index, parent_index) in parent_links.iter().enumerate() {
        let Some(parent_index) = parent_index else {
            continue;
        };
        let (Some(child_id), Some(parent_id)) = (
            spawned_ids.get(index).copied(),
            spawned_ids.get(*parent_index).copied(),
        ) else {
            continue;
        };
        world.set_parent(child_id, Some(parent_id));
    }
}

impl WorldAtmosphereAsset {
    fn from_atmosphere(atmosphere: &WorldAtmosphere) -> Self {
        Self {
            ambient_color: atmosphere.ambient_color.to_array(),
            ambient_intensity: atmosphere.ambient_intensity,
            background_intensity: atmosphere.background_intensity,
            background: BackgroundModeAsset::from_background(atmosphere.background),
        }
    }

    fn into_atmosphere(self) -> WorldAtmosphere {
        WorldAtmosphere {
            ambient_color: Vec3::from_array(self.ambient_color),
            ambient_intensity: self.ambient_intensity.max(0.0),
            background_intensity: self.background_intensity.max(0.0),
            background: self.background.into_background(),
        }
    }
}

impl BackgroundModeAsset {
    fn from_background(background: BackgroundMode) -> Self {
        match background {
            BackgroundMode::SolidColor { color } => Self::SolidColor {
                color: color.to_array(),
            },
            BackgroundMode::VerticalGradient {
                zenith_color,
                horizon_color,
                ground_color,
                horizon_height,
                smoothness,
            } => Self::VerticalGradient {
                zenith_color: zenith_color.to_array(),
                horizon_color: horizon_color.to_array(),
                ground_color: ground_color.to_array(),
                horizon_height,
                smoothness,
            },
            BackgroundMode::Sky => Self::Sky,
        }
    }

    fn into_background(self) -> BackgroundMode {
        match self {
            Self::SolidColor { color } => BackgroundMode::SolidColor {
                color: Vec3::from_array(color),
            },
            Self::VerticalGradient {
                zenith_color,
                horizon_color,
                ground_color,
                horizon_height,
                smoothness,
            } => BackgroundMode::VerticalGradient {
                zenith_color: Vec3::from_array(zenith_color),
                horizon_color: Vec3::from_array(horizon_color),
                ground_color: Vec3::from_array(ground_color),
                horizon_height: horizon_height.clamp(0.0, 1.0),
                smoothness: smoothness.max(0.001),
            },
            Self::Sky => BackgroundMode::Sky,
        }
    }
}

impl WorldObjectAsset {
    pub fn from_object(object: &Object) -> Self {
        Self::from_object_with_parent_map(object, &HashMap::new())
    }

    fn from_object_with_parent_map(
        object: &Object,
        object_index: &HashMap<runa_core::ocs::ObjectId, usize>,
    ) -> Self {
        let transform = object
            .get_component::<Transform>()
            .cloned()
            .unwrap_or_else(Transform::default);

        Self {
            name: object.name.clone(),
            object_id: object
                .get_component::<ObjectDefinitionInstance>()
                .map(|instance| instance.object_id.clone()),
            parent: object
                .parent()
                .and_then(|parent| object_index.get(&parent).copied()),
            transform: TransformAsset::from_transform(&transform),
            mesh_renderer: object
                .get_component::<MeshRenderer>()
                .and_then(MeshRendererAsset::from_component),
            sprite_renderer: object
                .get_component::<SpriteRenderer>()
                .map(SpriteRendererAsset::from_component),
            sprite_animator: object
                .get_component::<SpriteAnimator>()
                .map(SpriteAnimatorAsset::from_component),
            sorting: object
                .get_component::<Sorting>()
                .map(SortingAsset::from_component),
            tilemap: object
                .get_component::<Tilemap>()
                .map(TilemapAsset::from_component),
            camera: object
                .get_component::<Camera>()
                .map(CameraAsset::from_component),
            active_camera: object.get_component::<ActiveCamera>().is_some(),
            audio_source: object
                .get_component::<AudioSource>()
                .map(AudioSourceAsset::from_component),
            collider2d: object
                .get_component::<Collider2D>()
                .map(Collider2DAsset::from_component),
            physics_collision: object
                .get_component::<PhysicsCollision>()
                .map(PhysicsCollisionAsset::from_component),
            ui_renderer: object
                .get_component::<UiRenderer>()
                .map(UiRendererAsset::from_component),
            serialized_components: collect_serialized_type_assets(
                object,
                SerializedTypeKind::Component,
            ),
            serialized_scripts: collect_serialized_type_assets(object, SerializedTypeKind::Script),
        }
    }

    pub fn into_object(self, project_root: Option<&Path>) -> Object {
        self.into_object_with_runtime_registry(project_root, None)
    }

    pub fn into_object_with_runtime_registry(
        self,
        project_root: Option<&Path>,
        runtime_registry: Option<&runa_core::registry::RuntimeRegistry>,
    ) -> Object {
        let WorldObjectAsset {
            name,
            object_id,
            transform,
            mesh_renderer,
            sprite_renderer,
            sprite_animator,
            sorting,
            tilemap,
            camera,
            active_camera,
            audio_source,
            collider2d,
            physics_collision,
            ui_renderer,
            serialized_components,
            serialized_scripts,
            parent: _,
        } = self;

        if let (Some(object_def_id), Some(registry)) = (object_id.as_ref(), runtime_registry) {
            let key = runa_core::registry::ObjectDefKey::from(object_def_id.clone());
            if registry.archetypes().contains_key(&key) {
                let mut temp_world = World::default();
                temp_world.set_runtime_registry(Arc::new(registry.clone()));
                if let Some(spawned_id) = temp_world.spawn_def_by_key(&key) {
                    if let Some(object) = temp_world.take_object(spawned_id) {
                        return apply_asset_overrides_to_object(
                            object,
                            name,
                            object_id,
                            transform,
                            mesh_renderer,
                            sprite_renderer,
                            sprite_animator,
                            sorting,
                            tilemap,
                            camera,
                            active_camera,
                            audio_source,
                            collider2d,
                            physics_collision,
                            ui_renderer,
                            serialized_components,
                            serialized_scripts,
                            project_root,
                            runtime_registry,
                        );
                    }
                }
            }
        }

        apply_asset_overrides_to_object(
            Object::new(name.clone()),
            name,
            object_id,
            transform,
            mesh_renderer,
            sprite_renderer,
            sprite_animator,
            sorting,
            tilemap,
            camera,
            active_camera,
            audio_source,
            collider2d,
            physics_collision,
            ui_renderer,
            serialized_components,
            serialized_scripts,
            project_root,
            runtime_registry,
        )
    }

    pub fn into_object_with_object_loader<F>(
        self,
        project_root: Option<&Path>,
        object_loader: F,
    ) -> Object
    where
        F: Fn(&str) -> Option<WorldObjectAsset>,
    {
        if let Some(object_id) = &self.object_id {
            if let Some(mut spawned) = object_loader(object_id) {
                spawned.object_id = Some(object_id.clone());
                spawned.name = if self.name.is_empty() {
                    spawned.name
                } else {
                    self.name
                };
                spawned.transform = self.transform;
                if self.mesh_renderer.is_some() {
                    spawned.mesh_renderer = self.mesh_renderer;
                }
                if self.sprite_renderer.is_some() {
                    spawned.sprite_renderer = self.sprite_renderer;
                }
                if self.sprite_animator.is_some() {
                    spawned.sprite_animator = self.sprite_animator;
                }
                if self.sorting.is_some() {
                    spawned.sorting = self.sorting;
                }
                if self.tilemap.is_some() {
                    spawned.tilemap = self.tilemap;
                }
                if self.camera.is_some() {
                    spawned.camera = self.camera;
                }
                spawned.active_camera = self.active_camera;
                if self.audio_source.is_some() {
                    spawned.audio_source = self.audio_source;
                }
                if self.collider2d.is_some() {
                    spawned.collider2d = self.collider2d;
                }
                if self.physics_collision.is_some() {
                    spawned.physics_collision = self.physics_collision;
                }
                if self.ui_renderer.is_some() {
                    spawned.ui_renderer = self.ui_renderer;
                }
                if !self.serialized_components.is_empty() {
                    spawned.serialized_components = self.serialized_components;
                }
                if !self.serialized_scripts.is_empty() {
                    spawned.serialized_scripts = self.serialized_scripts;
                }
                return spawned.into_object_with_runtime_registry(project_root, None);
            }
        }

        self.into_object_with_runtime_registry(project_root, None)
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_asset_overrides_to_object(
    mut object: Object,
    name: String,
    object_id: Option<String>,
    transform: TransformAsset,
    mesh_renderer: Option<MeshRendererAsset>,
    sprite_renderer: Option<SpriteRendererAsset>,
    sprite_animator: Option<SpriteAnimatorAsset>,
    sorting: Option<SortingAsset>,
    tilemap: Option<TilemapAsset>,
    camera: Option<CameraAsset>,
    active_camera: bool,
    audio_source: Option<AudioSourceAsset>,
    collider2d: Option<Collider2DAsset>,
    physics_collision: Option<PhysicsCollisionAsset>,
    ui_renderer: Option<UiRendererAsset>,
    serialized_components: Vec<SerializedObjectTypeAsset>,
    serialized_scripts: Vec<SerializedObjectTypeAsset>,
    project_root: Option<&Path>,
    runtime_registry: Option<&runa_core::registry::RuntimeRegistry>,
) -> Object {
    object.name = name;
    object.add_component(transform.into_component());

    if let Some(mesh_renderer) = mesh_renderer {
        let _ = object.remove_component_type_id(TypeId::of::<MeshRenderer>());
        object.add_component(mesh_renderer.into_component());
    }
    if let Some(sprite_renderer) = sprite_renderer {
        let _ = object.remove_component_type_id(TypeId::of::<SpriteRenderer>());
        object.add_component(sprite_renderer.into_component(project_root));
    }
    if let Some(sprite_animator) = sprite_animator {
        let _ = object.remove_component_type_id(TypeId::of::<SpriteAnimator>());
        object.add_component(sprite_animator.into_component());
    }
    if let Some(sorting) = sorting {
        let _ = object.remove_component_type_id(TypeId::of::<Sorting>());
        object.add_component(sorting.into_component());
    }
    apply_sprite_animator_frame_to_renderer(&mut object);
    if let Some(tilemap) = tilemap {
        let _ = object.remove_component_type_id(TypeId::of::<Tilemap>());
        let _ = object.remove_component_type_id(TypeId::of::<TilemapRenderer>());
        object.add_component(tilemap.into_component(project_root));
        object.add_component(TilemapRenderer::new());
    }
    if let Some(camera) = camera {
        let _ = object.remove_component_type_id(TypeId::of::<Camera>());
        let transform = object
            .get_component::<Transform>()
            .cloned()
            .unwrap_or_else(Transform::default);
        object.add_component(camera.into_component_with_transform(&transform));
    }
    if active_camera {
        if object.get_component::<ActiveCamera>().is_none() {
            object.add_component(ActiveCamera);
        }
    } else {
        let _ = object.remove_component_type_id(TypeId::of::<ActiveCamera>());
    }
    if let Some(audio_source) = audio_source {
        let _ = object.remove_component_type_id(TypeId::of::<AudioSource>());
        object.add_component(audio_source.into_component(project_root));
    }
    if let Some(collider2d) = collider2d {
        let _ = object.remove_component_type_id(TypeId::of::<Collider2D>());
        object.add_component(collider2d.into_component());
    }
    if let Some(physics_collision) = physics_collision {
        let _ = object.remove_component_type_id(TypeId::of::<PhysicsCollision>());
        object.add_component(physics_collision.into_component());
    }
    if let Some(ui_renderer) = ui_renderer {
        let _ = object.remove_component_type_id(TypeId::of::<UiRenderer>());
        object.add_component(ui_renderer.into_component(project_root));
    }
    if let Some(object_id) = object_id {
        let _ = object.remove_component_type_id(TypeId::of::<ObjectDefinitionInstance>());
        object.add_component(ObjectDefinitionInstance::new(object_id));
    }

    apply_serialized_type_assets(
        &mut object,
        runtime_registry,
        SerializedTypeKind::Component,
        serialized_components,
    );
    apply_serialized_type_assets(
        &mut object,
        runtime_registry,
        SerializedTypeKind::Script,
        serialized_scripts,
    );

    object
}

fn apply_sprite_animator_frame_to_renderer(object: &mut Object) {
    let Some(uv_rect) = object
        .get_component::<SpriteAnimator>()
        .map(|animator| animator.sheet.uv_rect_for_frame(animator.current_frame))
    else {
        return;
    };

    if let Some(sprite) = object.get_component_mut::<SpriteRenderer>() {
        sprite.set_uv_rect(uv_rect);
    }
}

fn collect_serialized_type_assets(
    object: &Object,
    kind: SerializedTypeKind,
) -> Vec<SerializedObjectTypeAsset> {
    let mut assets = Vec::new();
    for info in object.component_infos() {
        let matches_kind = get_matches_type(kind, info);
        if !matches_kind || is_builtin_serialized_type(info.type_id()) {
            continue;
        }

        if let Some(fields) = object
            .with_component_by_type_id(info.type_id(), |component| component.serialized_fields())
        {
            let type_name = object
                .runtime_registry()
                .and_then(|registry| {
                    registry
                        .types()
                        .get_by_id(info.type_id())
                        .map(|metadata| metadata.type_name().to_string())
                })
                .unwrap_or_else(|| info.type_name().to_string());
            assets.push(SerializedObjectTypeAsset { type_name, fields });
        }
    }

    if let Some(storage) = object.get_component::<SerializedTypeStorage>() {
        for entry in storage.entries_of_kind(kind) {
            assets.push(SerializedObjectTypeAsset {
                type_name: entry.type_name.clone(),
                fields: entry.fields.clone(),
            });
        }
    }

    dedup_serialized_assets(assets)
}

fn get_matches_type(kind: SerializedTypeKind, info: ObjectComponentInfo) -> bool {
    match kind {
        SerializedTypeKind::Component => {
            info.kind() == runa_core::components::ComponentRuntimeKind::Component
        }
        SerializedTypeKind::Script => {
            info.kind() == runa_core::components::ComponentRuntimeKind::Script
        }
    }
}

fn dedup_serialized_assets(
    assets: Vec<SerializedObjectTypeAsset>,
) -> Vec<SerializedObjectTypeAsset> {
    let mut deduped: Vec<SerializedObjectTypeAsset> = Vec::new();
    for asset in assets {
        if let Some(existing) = deduped
            .iter_mut()
            .find(|existing| existing.type_name == asset.type_name)
        {
            *existing = asset;
        } else {
            deduped.push(asset);
        }
    }
    deduped
}

fn apply_serialized_type_assets(
    object: &mut Object,
    runtime_registry: Option<&runa_core::registry::RuntimeRegistry>,
    kind: SerializedTypeKind,
    assets: Vec<SerializedObjectTypeAsset>,
) {
    for asset in assets {
        if let Some(type_id) = find_existing_object_type_id(object, kind, &asset.type_name) {
            for_field(object, &asset, type_id);
            continue;
        }

        if let Some(registry) = runtime_registry {
            if let Some(metadata) = find_registered_type_metadata(registry, &asset.type_name) {
                let type_id = metadata.type_id();
                let has_runtime_instance =
                    object.with_component_by_type_id(type_id, |_| ()).is_some();
                if has_runtime_instance || registry.add_type_to_object(object, type_id) {
                    for_field(object, &asset, type_id);
                    continue;
                }
            }
        }

        let mut storage = object
            .get_component::<SerializedTypeStorage>()
            .cloned()
            .unwrap_or_default();
        storage.upsert(SerializedTypeEntry {
            type_name: asset.type_name,
            kind,
            fields: asset.fields,
        });
        if object.get_component::<SerializedTypeStorage>().is_some() {
            object.remove_component_type_id(TypeId::of::<SerializedTypeStorage>());
        }
        object.add_component(storage);
    }
}

fn for_field(object: &mut Object, asset: &SerializedObjectTypeAsset, type_id: TypeId) {
    for field in &asset.fields {
        let _ = object.with_component_mut_by_type_id(type_id, |component| {
            component.set_serialized_field(&field.name, field.value.clone())
        });
    }
}

fn find_existing_object_type_id(
    object: &Object,
    kind: SerializedTypeKind,
    type_name: &str,
) -> Option<TypeId> {
    let matches_kind = |info: &ObjectComponentInfo| match kind {
        SerializedTypeKind::Component => {
            info.kind() == runa_core::components::ComponentRuntimeKind::Component
        }
        SerializedTypeKind::Script => {
            info.kind() == runa_core::components::ComponentRuntimeKind::Script
        }
    };

    if let Some(info) = object
        .component_infos()
        .into_iter()
        .filter(matches_kind)
        .find(|info| info.type_name() == type_name)
    {
        return Some(info.type_id());
    }

    let short_name = short_type_name(type_name);
    let mut matches = object
        .component_infos()
        .into_iter()
        .filter(matches_kind)
        .filter(|info| short_type_name(info.type_name()) == short_name);

    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }

    Some(first.type_id())
}

fn find_registered_type_metadata(
    registry: &runa_core::registry::RuntimeRegistry,
    type_name: &str,
) -> Option<runa_core::registry::TypeMetadata> {
    if let Some(metadata) = registry.types().get_by_name(type_name) {
        return Some(metadata.clone());
    }

    let short_name = short_type_name(type_name);
    let mut matches = registry
        .types()
        .registered_types()
        .into_iter()
        .filter(|metadata| short_type_name(metadata.type_name()) == short_name);

    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }

    Some(first)
}

fn short_type_name(type_name: &str) -> &str {
    type_name.rsplit("::").next().unwrap_or(type_name)
}

fn is_builtin_serialized_type(type_id: TypeId) -> bool {
    use std::any::TypeId;

    [
        TypeId::of::<Transform>(),
        TypeId::of::<MeshRenderer>(),
        TypeId::of::<SpriteRenderer>(),
        TypeId::of::<SpriteAnimator>(),
        TypeId::of::<Sorting>(),
        TypeId::of::<Tilemap>(),
        TypeId::of::<TilemapRenderer>(),
        TypeId::of::<Camera>(),
        TypeId::of::<ActiveCamera>(),
        TypeId::of::<AudioSource>(),
        TypeId::of::<Collider2D>(),
        TypeId::of::<PhysicsCollision>(),
        TypeId::of::<ObjectDefinitionInstance>(),
        TypeId::of::<SerializedTypeStorage>(),
        TypeId::of::<UiRenderer>(),
    ]
    .contains(&type_id)
}

impl TransformAsset {
    fn from_transform(transform: &Transform) -> Self {
        Self {
            position: transform.position.to_array(),
            rotation: transform.rotation.to_array(),
            scale: transform.scale.to_array(),
        }
    }

    fn into_component(self) -> Transform {
        Transform {
            position: Vec3::from_array(self.position),
            rotation: Quat::from_array(self.rotation),
            scale: Vec3::from_array(self.scale),
            previous_position: Vec3::from_array(self.position),
            previous_rotation: Quat::from_array(self.rotation),
        }
    }
}

impl MeshRendererAsset {
    fn from_component(component: &MeshRenderer) -> Option<Self> {
        let primitive = infer_mesh_primitive(component.get_mesh_handle())?;
        Some(Self {
            primitive,
            color: component.color,
        })
    }

    fn into_component(self) -> MeshRenderer {
        let mesh = match self.primitive {
            MeshPrimitiveAsset::Cube { size } => Mesh::cube(size),
            MeshPrimitiveAsset::Quad { width, height } => Mesh::quad(width, height),
            MeshPrimitiveAsset::Plane { width, depth } => Mesh::plane(width, depth),
            MeshPrimitiveAsset::Pyramid {
                width,
                height,
                depth,
            } => Mesh::pyramid(width, height, depth),
        };

        let mut renderer = MeshRenderer::new(mesh);
        renderer.set_material(0, Material::default());
        renderer.color = self.color;
        renderer
    }
}

impl SpriteRendererAsset {
    fn from_component(component: &SpriteRenderer) -> Self {
        Self {
            sprite: component.texture_path.clone(),
            pixels_per_unit: component.pixels_per_unit,
            uv_rect: component.uv_rect,
        }
    }

    fn into_component(self, project_root: Option<&Path>) -> SpriteRenderer {
        let mut sprite = SpriteRenderer::default();
        sprite.pixels_per_unit = self.pixels_per_unit.max(f32::EPSILON);
        sprite.set_uv_rect(self.uv_rect);
        if let Some(path) = self.sprite {
            if let Some(project_root) = project_root {
                if let Some(handle) = load_texture_handle(project_root, &path) {
                    sprite.set_texture(Some(handle), Some(path));
                } else {
                    sprite.texture_path = Some(path);
                }
            } else {
                sprite.texture_path = Some(path);
            }
        }
        sprite
    }
}

impl SpriteAnimatorAsset {
    fn from_component(component: &SpriteAnimator) -> Self {
        Self {
            columns: component.sheet.columns,
            rows: component.sheet.rows,
            clips: component
                .clips
                .iter()
                .map(SpriteAnimationClipAsset::from_component)
                .collect(),
            current_clip: component.current_clip.clone(),
            current_frame: component.current_frame,
            playing: component.playing,
        }
    }

    fn into_component(self) -> SpriteAnimator {
        let clips = if self.clips.is_empty() {
            vec![SpriteAnimationClip::new("Default", 0, 0, 12.0)]
        } else {
            self.clips
                .into_iter()
                .map(SpriteAnimationClipAsset::into_component)
                .collect()
        };

        SpriteAnimator::from_clips(
            SpriteSheet::new(self.columns, self.rows),
            clips,
            self.current_clip,
            self.current_frame,
            self.playing,
        )
    }
}

impl SpriteAnimationClipAsset {
    fn from_component(component: &SpriteAnimationClip) -> Self {
        Self {
            name: component.name.clone(),
            start_frame: component.start_frame,
            end_frame: component.end_frame,
            fps: component.fps,
            looping: component.looping,
        }
    }

    fn into_component(self) -> SpriteAnimationClip {
        SpriteAnimationClip {
            name: self.name,
            start_frame: self.start_frame,
            end_frame: self.end_frame,
            fps: self.fps,
            looping: self.looping,
        }
    }
}

impl UiRendererAsset {
    fn from_component(component: &UiRenderer) -> Self {
        Self {
            ui_asset_path: component.ui_asset_path.clone(),
            space: "Screen".to_string(),
            inline_ui: None,
        }
    }

    fn into_component(self, project_root: Option<&Path>) -> UiRenderer {
        if let Some(inline) = self.inline_ui {
            inline.into_ui_renderer(project_root)
        } else if let Some(asset_path) = &self.ui_asset_path {
            let full_path = project_root.map(|r| r.join(asset_path));
            if let Some(path) = full_path {
                if let Ok(asset) = crate::ui_asset::load_ui_asset(&path) {
                    let mut renderer = asset.into_ui_renderer(project_root);
                    renderer.ui_asset_path = self.ui_asset_path;
                    renderer
                } else {
                    UiRenderer::new(CanvasSpace::Screen)
                }
            } else {
                UiRenderer::new(CanvasSpace::Screen)
            }
        } else {
            UiRenderer::new(CanvasSpace::Screen)
        }
    }
}

impl SortingAsset {
    fn from_component(component: &Sorting) -> Self {
        Self {
            order: component.order,
            y_sort: component.y_sort,
            y_offset: component.y_offset,
        }
    }

    fn into_component(self) -> Sorting {
        Sorting {
            order: self.order,
            y_sort: false,
            y_offset: 0.0,
        }
    }
}

impl CameraAsset {
    fn from_component(camera: &Camera) -> Self {
        Self {
            position: camera.position.to_array(),
            target: camera.target.to_array(),
            up: camera.up.to_array(),
            perspective: matches!(camera.projection, ProjectionType::Perspective),
            ortho_size: camera.orthographic_size.to_array(),
            near: camera.near,
            far: camera.far,
            fov_radians: camera.fov,
            viewport_size: [camera.viewport_size.0, camera.viewport_size.1],
        }
    }

    fn into_component_with_transform(self, transform: &Transform) -> Camera {
        let inverse_rotation = transform.rotation.inverse();
        let local_position =
            inverse_rotation * (Vec3::from_array(self.position) - transform.position);
        let local_target = inverse_rotation * (Vec3::from_array(self.target) - transform.position);
        let local_up = inverse_rotation * Vec3::from_array(self.up);

        if self.perspective {
            let mut camera = Camera::new_perspective(
                local_position,
                local_target,
                local_up,
                self.fov_radians,
                self.near,
                self.far,
            );
            camera.resize(self.viewport_size[0], self.viewport_size[1]);
            camera
        } else {
            let mut camera = Camera::new_orthographic(self.ortho_size[0], self.ortho_size[1]);
            camera.position = local_position;
            camera.target = local_target;
            camera.up = local_up;
            camera.near = self.near;
            camera.far = self.far;
            camera.resize(self.viewport_size[0], self.viewport_size[1]);
            camera
        }
    }
}

impl PhysicsCollisionAsset {
    fn from_component(collision: &PhysicsCollision) -> Self {
        Self {
            size: collision.size.to_array(),
            enabled: collision.enabled,
        }
    }

    fn into_component(self) -> PhysicsCollision {
        PhysicsCollision {
            size: Vec2::from_array(self.size),
            enabled: self.enabled,
        }
    }
}

impl Collider2DAsset {
    fn from_component(collider: &Collider2D) -> Self {
        Self {
            half_size: collider.half_size.to_array(),
            enabled: collider.enabled,
            is_trigger: collider.is_trigger,
        }
    }

    fn into_component(self) -> Collider2D {
        Collider2D {
            half_size: Vec2::from_array(self.half_size),
            enabled: self.enabled,
            is_trigger: self.is_trigger,
        }
    }
}

impl AudioSourceAsset {
    fn from_component(audio: &AudioSource) -> Self {
        Self {
            source: audio.source_path.clone(),
            volume: audio.volume,
            looped: audio.looped,
            play_on_awake: audio.play_on_awake,
            spatial: audio.spatial,
            min_distance: audio.min_distance,
            max_distance: audio.max_distance,
        }
    }

    fn into_component(self, project_root: Option<&Path>) -> AudioSource {
        let mut audio = if self.spatial {
            AudioSource::new3d()
        } else {
            AudioSource::new2d()
        };
        if let Some(path) = self.source {
            if let Some(project_root) = project_root {
                if let Ok(asset) =
                    AudioAsset::from_file(project_root.to_string_lossy().as_ref(), &path)
                {
                    audio.set_asset_with_path(Some(Arc::new(asset)), Some(path));
                } else {
                    audio.source_path = Some(path);
                }
            } else {
                audio.source_path = Some(path);
            }
        }
        audio.volume = self.volume;
        audio.looped = self.looped;
        audio.play_on_awake = self.play_on_awake;
        audio.min_distance = self.min_distance;
        audio.max_distance = self.max_distance;
        audio
    }
}

impl TilemapAsset {
    fn from_component(tilemap: &Tilemap) -> Self {
        Self {
            width: tilemap.width,
            height: tilemap.height,
            tile_size: [tilemap.tile_size.x as u32, tilemap.tile_size.y as u32],
            offset: [tilemap.offset.x, tilemap.offset.y],
            pixels_per_unit: tilemap.pixels_per_unit,
            atlas: tilemap.atlas.as_ref().map(|atlas| TilemapAtlasAsset {
                texture: atlas.texture_path.clone(),
                columns: atlas.columns,
                rows: atlas.rows,
            }),
            selected_tile: tilemap.selected_tile,
            layers: tilemap
                .layers
                .iter()
                .map(|layer| TilemapLayerAsset::from_component(layer, tilemap))
                .collect(),
        }
    }

    fn into_component(self, project_root: Option<&Path>) -> Tilemap {
        let mut tilemap = Tilemap {
            width: self.width,
            height: self.height,
            tile_size: USizeVec2::new(self.tile_size[0] as usize, self.tile_size[1] as usize),
            offset: IVec2::new(self.offset[0], self.offset[1]),
            layers: Vec::new(),
            atlas: None,
            selected_tile: self.selected_tile,
            pixels_per_unit: self.pixels_per_unit.max(f32::EPSILON),
            generation: 0,
        };
        if let Some(atlas) = self.atlas {
            if let (Some(project_root), Some(path)) = (project_root, atlas.texture.clone()) {
                if let Some(texture) = load_texture_handle(project_root, &path) {
                    tilemap.set_atlas(Some(texture), Some(path), atlas.columns, atlas.rows);
                }
            }
        }
        for layer in self.layers {
            tilemap
                .layers
                .push(layer.into_component(self.width, self.height, &tilemap));
        }
        tilemap
    }
}

impl TilemapLayerAsset {
    fn from_component(layer: &TilemapLayer, tilemap: &Tilemap) -> Self {
        Self {
            name: layer.name.clone(),
            visible: layer.visible,
            opacity: layer.opacity,
            tiles: layer
                .tiles
                .iter()
                .map(|&id| {
                    if id == EMPTY_TILE {
                        return None;
                    }
                    let frame = tilemap.frame_for_id(id);
                    (frame < tilemap.atlas_frame_count()).then_some(frame)
                })
                .collect(),
            order: layer.order,
        }
    }

    fn into_component(self, width: u32, height: u32, tilemap: &Tilemap) -> TilemapLayer {
        let mut layer = TilemapLayer::new(self.name, width, height);
        layer.visible = self.visible;
        layer.opacity = self.opacity;
        layer.order = self.order;
        for (index, frame) in self.tiles.into_iter().enumerate() {
            let Some(frame) = frame else {
                continue;
            };
            if let Some(target) = layer.tiles.get_mut(index) {
                *target = tilemap.id_for_frame(frame);
            }
        }
        layer
    }
}

fn infer_mesh_primitive(mesh: Handle<Mesh>) -> Option<MeshPrimitiveAsset> {
    let hint = mesh.inner.primitive_hint?;
    let (min, max) = mesh_bounds(mesh.inner.as_ref())?;
    let size = max - min;
    match hint {
        runa_core::components::BuiltinMeshPrimitive::Cube => Some(MeshPrimitiveAsset::Cube {
            size: size.x.abs().max(size.y.abs()).max(size.z.abs()),
        }),
        runa_core::components::BuiltinMeshPrimitive::Quad => Some(MeshPrimitiveAsset::Quad {
            width: size.x.abs(),
            height: size.y.abs(),
        }),
        runa_core::components::BuiltinMeshPrimitive::Plane => Some(MeshPrimitiveAsset::Plane {
            width: size.x.abs(),
            depth: size.z.abs(),
        }),
        runa_core::components::BuiltinMeshPrimitive::Pyramid => Some(MeshPrimitiveAsset::Pyramid {
            width: size.x.abs(),
            height: size.y.abs(),
            depth: size.z.abs(),
        }),
    }
}

fn mesh_bounds(mesh: &Mesh) -> Option<(Vec3, Vec3)> {
    let first = mesh.vertices.first()?;
    let mut min = Vec3::from_array(first.position);
    let mut max = Vec3::from_array(first.position);
    for vertex in &mesh.vertices {
        let p = Vec3::from_array(vertex.position);
        min = min.min(p);
        max = max.max(p);
    }
    Some((min, max))
}

fn project_root_for_world_path(path: &Path) -> Option<PathBuf> {
    let manifest_path = find_project_manifest(path)?;
    load_project(manifest_path)
        .ok()
        .map(|project| project.root_dir)
}

fn load_texture_handle(project_root: &Path, relative_path: &str) -> Option<Handle<TextureAsset>> {
    let texture = TextureAsset::load(&project_root.join(relative_path)).ok()?;
    Some(Handle {
        inner: Arc::new(texture),
    })
}

#[cfg(test)]
mod tests {
    use super::{apply_serialized_type_assets, SerializedObjectTypeAsset, WorldObjectAsset};
    use crate::load_world_with_runtime_registry;
    use runa_core::components::{
        SerializedField, SerializedFieldAccess, SerializedFieldValue, SerializedTypeKind,
        SerializedTypeStorage,
    };
    use runa_core::ocs::{Object, Script, ScriptContext};
    use runa_core::registry::RuntimeRegistry;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_world_with_runtime_registry() {
        let temp_dir = TempDir::new().unwrap();
        let world_path = temp_dir.path().join("world.ron");
        let world_content = r#"
(
    version: 1,
    objects: [
        (
            name: "TestObject",
            transform: (
                position: (0.0, 0.0, 0.0),
                rotation: (0.0, 0.0, 0.0, 1.0),
                scale: (1.0, 1.0, 1.0),
            ),
            active_camera: false,
            serialized_components: [],
            serialized_scripts: [],
        ),
    ],
)
"#;
        fs::write(&world_path, world_content).unwrap();

        let registry = RuntimeRegistry::new();
        let world = load_world_with_runtime_registry(&world_path, &registry).unwrap();
        assert_eq!(
            world
                .borrow()
                .query::<runa_core::components::Transform>()
                .len(),
            1
        );
    }

    #[test]
    fn test_load_world_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let world_path = temp_dir.path().join("invalid.ron");
        fs::write(&world_path, "invalid content").unwrap();

        let registry = RuntimeRegistry::new();
        let result = load_world_with_runtime_registry(&world_path, &registry);
        assert!(result.is_err());
    }

    #[derive(Clone)]
    struct TestScript {
        speed: f32,
    }

    impl SerializedFieldAccess for TestScript {
        fn serialized_fields(&self) -> Vec<SerializedField> {
            vec![SerializedField {
                name: "speed".to_string(),
                value: SerializedFieldValue::F32(self.speed),
            }]
        }

        fn set_serialized_field(&mut self, field_name: &str, value: SerializedFieldValue) -> bool {
            match (field_name, value) {
                ("speed", SerializedFieldValue::F32(speed)) => {
                    self.speed = speed;
                    true
                }
                _ => false,
            }
        }
    }

    impl Script for TestScript {
        fn update(&mut self, _ctx: &mut ScriptContext, _dt: f32) {}
    }

    #[test]
    fn serialized_script_fields_apply_to_existing_runtime_instance() {
        let mut registry = RuntimeRegistry::new();
        let metadata = registry.register_script_named_factory::<TestScript, _>(
            std::any::type_name::<TestScript>(),
            || TestScript { speed: 1.0 },
        );

        let mut object = Object::new("Runtime Script");
        assert!(registry.add_type_to_object(&mut object, metadata.type_id()));

        apply_serialized_type_assets(
            &mut object,
            Some(&registry),
            SerializedTypeKind::Script,
            vec![SerializedObjectTypeAsset {
                type_name: std::any::type_name::<TestScript>().to_string(),
                fields: vec![SerializedField {
                    name: "speed".to_string(),
                    value: SerializedFieldValue::F32(9.5),
                }],
            }],
        );

        let applied_speed = object
            .get_component::<TestScript>()
            .map(|script| script.speed)
            .unwrap_or_default();
        assert!((applied_speed - 9.5).abs() < f32::EPSILON);
    }

    #[test]
    fn archetype_override_preserves_serialized_script_asset_data() {
        let mut base_object = Object::new("Base");
        base_object.add_component(TestScript { speed: 1.0 });
        let base_asset = WorldObjectAsset::from_object(&base_object);

        let mut override_asset = WorldObjectAsset::from_object(&Object::new("Override"));
        override_asset.object_id = Some("test_script_archetype".to_string());
        override_asset.serialized_scripts = vec![SerializedObjectTypeAsset {
            type_name: std::any::type_name::<TestScript>().to_string(),
            fields: vec![SerializedField {
                name: "speed".to_string(),
                value: SerializedFieldValue::F32(4.0),
            }],
        }];

        let object = override_asset
            .into_object_with_object_loader(None, |_object_id| Some(base_asset.clone()));

        let serialized_speed = object
            .get_component::<SerializedTypeStorage>()
            .and_then(|storage| {
                storage
                    .entries_of_kind(SerializedTypeKind::Script)
                    .find(|entry| entry.type_name == std::any::type_name::<TestScript>())
            })
            .and_then(|entry| entry.fields.first())
            .and_then(|field| match &field.value {
                SerializedFieldValue::F32(value) => Some(*value),
                _ => None,
            })
            .unwrap_or_default();

        assert!((serialized_speed - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn runtime_registry_loading_prefers_archetype_state_for_nonserialized_scripts() {
        let mut registry = RuntimeRegistry::new();
        registry.register_archetype_named("test_rotator", || {
            let mut object = Object::new("Rotator");
            object.add_component(TestScript { speed: 7.0 });
            object
        });

        let asset = WorldObjectAsset {
            name: "Rotator Instance".to_string(),
            object_id: Some("test_rotator".to_string()),
            transform: super::TransformAsset::default(),
            mesh_renderer: None,
            sprite_renderer: None,
            sprite_animator: None,
            sorting: None,
            tilemap: None,
            camera: None,
            active_camera: false,
            audio_source: None,
            collider2d: None,
            physics_collision: None,
            ui_renderer: None,
            serialized_components: Vec::new(),
            serialized_scripts: Vec::new(),
            parent: None,
        };

        let object = asset.into_object_with_runtime_registry(None, Some(&registry));
        let loaded_speed = object
            .get_component::<TestScript>()
            .map(|script| script.speed)
            .unwrap_or_default();

        assert!((loaded_speed - 7.0).abs() < f32::EPSILON);
    }

    #[test]
    fn archetype_loading_applies_serialized_fields_to_existing_unregistered_script_instance() {
        let mut registry = RuntimeRegistry::new();
        registry.register_archetype_named("test_rotator", || {
            let mut object = Object::new("Rotator");
            object.add_component(TestScript { speed: 7.0 });
            object
        });

        let asset = WorldObjectAsset {
            name: "Rotator Instance".to_string(),
            object_id: Some("test_rotator".to_string()),
            transform: super::TransformAsset::default(),
            mesh_renderer: None,
            sprite_renderer: None,
            sprite_animator: None,
            sorting: None,
            tilemap: None,
            camera: None,
            active_camera: false,
            audio_source: None,
            collider2d: None,
            physics_collision: None,
            ui_renderer: None,
            serialized_components: Vec::new(),
            serialized_scripts: vec![SerializedObjectTypeAsset {
                type_name: "TestScript".to_string(),
                fields: vec![SerializedField {
                    name: "speed".to_string(),
                    value: SerializedFieldValue::F32(0.0),
                }],
            }],
            parent: None,
        };

        let object = asset.into_object_with_runtime_registry(None, Some(&registry));
        let loaded_speed = object
            .get_component::<TestScript>()
            .map(|script| script.speed)
            .unwrap_or_default();

        assert!(loaded_speed.abs() < f32::EPSILON);
    }

    #[test]
    fn runtime_registry_loading_creates_plain_script_from_serialized_entry() {
        let mut registry = RuntimeRegistry::new();
        registry.register_script_named_factory::<TestScript, _>("TestScript", || TestScript {
            speed: 1.0,
        });

        let asset = WorldObjectAsset {
            name: "Plain Script Object".to_string(),
            object_id: None,
            transform: super::TransformAsset::default(),
            mesh_renderer: None,
            sprite_renderer: None,
            sprite_animator: None,
            sorting: None,
            tilemap: None,
            camera: None,
            active_camera: false,
            audio_source: None,
            collider2d: None,
            physics_collision: None,
            ui_renderer: None,
            serialized_components: Vec::new(),
            serialized_scripts: vec![SerializedObjectTypeAsset {
                type_name: "TestScript".to_string(),
                fields: vec![SerializedField {
                    name: "speed".to_string(),
                    value: SerializedFieldValue::F32(3.25),
                }],
            }],
            parent: None,
        };

        let object = asset.into_object_with_runtime_registry(None, Some(&registry));
        let loaded_speed = object
            .get_component::<TestScript>()
            .map(|script| script.speed)
            .unwrap_or_default();

        assert!((loaded_speed - 3.25).abs() < f32::EPSILON);
        assert!(object.get_component::<SerializedTypeStorage>().is_none());
    }
}

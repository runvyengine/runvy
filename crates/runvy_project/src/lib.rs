pub mod placeable_objects;
mod project;
mod scaffold;
pub mod ui_asset;
mod world_asset;

pub use project::{
    find_project_manifest, load_project, ProjectAppConfig, ProjectBuildConfig, ProjectError,
    ProjectManifest, ProjectPaths,
};
pub use scaffold::{
    cached_bridge_path, create_empty_project, ensure_bridge_binary, ensure_editor_bridge_files,
    ensure_release_windows_subsystem,
};
pub use world_asset::{
    create_empty_world, load_world, load_world_with_object_loader,
    load_world_with_runtime_registry, save_world, AudioSourceAsset, CameraAsset,
    MeshPrimitiveAsset, MeshRendererAsset, PhysicsCollisionAsset, PlaceableObjectDescriptor,
    PlaceableObjectRecord, ProjectMetadataSnapshot, ProjectRegisteredTypeKind,
    ProjectRegisteredTypeRecord, ProjectRegistrationSource, SerializedObjectTypeAsset,
    SpriteRendererAsset, TilemapAsset, TilemapLayerAsset, TransformAsset, UiRendererAsset,
    WorldAsset, WorldObjectAsset,
};
pub mod runvy3d;

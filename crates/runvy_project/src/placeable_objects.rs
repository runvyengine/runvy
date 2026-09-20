use crate::world_asset::{
    AudioSourceAsset, CameraAsset, MeshPrimitiveAsset, MeshRendererAsset, PhysicsCollisionAsset,
    PlaceableObjectDescriptor, PlaceableObjectRecord, SpriteRendererAsset, TilemapAsset,
    TilemapLayerAsset, TransformAsset, WorldObjectAsset,
};

pub fn default_records() -> Vec<PlaceableObjectRecord> {
    vec![
        record("empty", "Empty", "Basic", empty()),
        record("camera", "Camera", "Basic", camera()),
        record("cube", "Cube", "Basic", cube()),
        record("floor", "Floor", "Basic", floor()),
        record("sprite", "Sprite", "2D", sprite()),
        record("tilemap", "Tilemap", "2D", tilemap()),
        record("audio-source", "Audio Source", "Audio", audio_source()),
    ]
}

fn record(
    id: &str,
    name: &str,
    category: &str,
    object: WorldObjectAsset,
) -> PlaceableObjectRecord {
    PlaceableObjectRecord {
        descriptor: PlaceableObjectDescriptor {
            id: id.to_string(),
            name: name.to_string(),
            category: category.to_string(),
        },
        object,
    }
}

fn base_object(name: &str) -> WorldObjectAsset {
    WorldObjectAsset {
        name: name.to_string(),
        object_id: None,
        parent: None,
        transform: TransformAsset::default(),
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
        serialized_components: Vec::new(),
        ui_renderer: None,
        serialized_scripts: Vec::new(),
    }
}

pub fn empty() -> WorldObjectAsset {
    base_object("Empty")
}

pub fn camera() -> WorldObjectAsset {
    WorldObjectAsset {
        name: "Camera".to_string(),
        object_id: None,
        parent: None,
        transform: TransformAsset {
            position: [0.0, 2.0, 6.0],
            ..TransformAsset::default()
        },
        mesh_renderer: None,
        sprite_renderer: None,
        sprite_animator: None,
        sorting: None,
        tilemap: None,
        camera: Some(CameraAsset::default()),
        active_camera: true,
        audio_source: None,
        collider2d: None,
        physics_collision: None,
        serialized_components: Vec::new(),
        ui_renderer: None,
        serialized_scripts: Vec::new(),
    }
}

pub fn cube() -> WorldObjectAsset {
    WorldObjectAsset {
        name: "Cube".to_string(),
        object_id: None,
        parent: None,
        transform: TransformAsset {
            position: [0.0, 0.75, 0.0],
            scale: [1.0, 1.0, 1.0],
            ..TransformAsset::default()
        },
        mesh_renderer: Some(MeshRendererAsset {
            primitive: MeshPrimitiveAsset::Cube { size: 1.5 },
            color: [0.95, 0.55, 0.22, 1.0],
        }),
        sprite_renderer: None,
        sprite_animator: None,
        sorting: None,
        tilemap: None,
        camera: None,
        active_camera: false,
        audio_source: None,
        collider2d: None,
        physics_collision: Some(PhysicsCollisionAsset {
            size: [0.75, 0.75],
            enabled: true,
        }),
        serialized_components: Vec::new(),
        ui_renderer: None,
        serialized_scripts: Vec::new(),
    }
}

pub fn floor() -> WorldObjectAsset {
    WorldObjectAsset {
        name: "Floor".to_string(),
        object_id: None,
        parent: None,
        transform: TransformAsset {
            position: [0.0, -1.5, 0.0],
            scale: [8.0, 0.2, 8.0],
            ..TransformAsset::default()
        },
        mesh_renderer: Some(MeshRendererAsset {
            primitive: MeshPrimitiveAsset::Cube { size: 1.0 },
            color: [0.24, 0.27, 0.32, 1.0],
        }),
        sprite_renderer: None,
        sprite_animator: None,
        sorting: None,
        tilemap: None,
        camera: None,
        active_camera: false,
        audio_source: None,
        collider2d: None,
        physics_collision: Some(PhysicsCollisionAsset {
            size: [4.0, 4.0],
            enabled: true,
        }),
        serialized_components: Vec::new(),
        ui_renderer: None,
        serialized_scripts: Vec::new(),
    }
}

pub fn sprite() -> WorldObjectAsset {
    WorldObjectAsset {
        name: "Sprite".to_string(),
        object_id: None,
        parent: None,
        transform: TransformAsset::default(),
        mesh_renderer: None,
        sprite_renderer: Some(SpriteRendererAsset {
            sprite: None,
            pixels_per_unit: 16.0,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        }),
        sprite_animator: None,
        sorting: None,
        tilemap: None,
        camera: None,
        active_camera: false,
        audio_source: None,
        collider2d: None,
        physics_collision: None,
        serialized_components: Vec::new(),
        ui_renderer: None,
        serialized_scripts: Vec::new(),
    }
}

pub fn tilemap() -> WorldObjectAsset {
    WorldObjectAsset {
        name: "Tilemap".to_string(),
        object_id: None,
        parent: None,
        transform: TransformAsset::default(),
        mesh_renderer: None,
        sprite_renderer: None,
        sprite_animator: None,
        sorting: None,
        tilemap: Some(TilemapAsset {
            width: 16,
            height: 16,
            tile_size: [32, 32],
            offset: [-8, -8],
            pixels_per_unit: 16.0,
            atlas: None,
            selected_tile: 0,
            layers: vec![TilemapLayerAsset {
                name: "Base".to_string(),
                visible: true,
                opacity: 1.0,
                tiles: Vec::new(),
                order: 0,
            }],
        }),
        camera: None,
        active_camera: false,
        audio_source: None,
        collider2d: None,
        physics_collision: None,
        serialized_components: Vec::new(),
        ui_renderer: None,
        serialized_scripts: Vec::new(),
    }
}

pub fn audio_source() -> WorldObjectAsset {
    WorldObjectAsset {
        name: "Audio Source".to_string(),
        object_id: None,
        parent: None,
        transform: TransformAsset::default(),
        mesh_renderer: None,
        sprite_renderer: None,
        sprite_animator: None,
        sorting: None,
        tilemap: None,
        camera: None,
        active_camera: false,
        audio_source: Some(AudioSourceAsset {
            source: None,
            volume: 1.0,
            looped: false,
            play_on_awake: false,
            spatial: false,
            min_distance: 1.0,
            max_distance: 100.0,
        }),
        collider2d: None,
        physics_collision: None,
        serialized_components: Vec::new(),
        ui_renderer: None,
        serialized_scripts: Vec::new(),
    }
}

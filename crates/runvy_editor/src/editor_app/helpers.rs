use runvy_core::components::DirectionalLight;
use runvy_core::components::PointLight;
use runvy_core::components::Sorting;
use runvy_core::components::SpriteAnimator;

use super::*;

pub(super) fn create_preview_world() -> Rc<RefCell<World>> {
    let mut world = World::default();

    let mut cube = Object::new("Preview Cube");
    let mut cube_transform = Transform::default();
    cube_transform.position = Vec3::new(0.0, 0.6, 0.0);
    cube_transform.scale = Vec3::splat(1.2);
    cube.add_component(cube_transform);
    let mut cube_mesh = MeshRenderer::new(Mesh::cube(1.5));
    cube_mesh.color = [1.0, 0.55, 0.2, 1.0];
    cube.add_component(cube_mesh);
    world.spawn(cube);

    let mut floor = Object::new("Floor");
    let mut floor_transform = Transform::default();
    floor_transform.position = Vec3::new(0.0, -1.5, 0.0);
    floor_transform.scale = Vec3::new(8.0, 0.2, 8.0);
    floor.add_component(floor_transform);
    let mut floor_mesh = MeshRenderer::new(Mesh::cube(1.0));
    floor_mesh.color = [0.24, 0.27, 0.32, 1.0];
    floor.add_component(floor_mesh);
    floor.add_component(PhysicsCollision::new(8.0, 8.0));
    world.spawn(floor);

    Rc::new(RefCell::new(world))
}

pub(super) fn object_title(object: &Object) -> String {
    if object.name.trim().is_empty() {
        "Object".to_string()
    } else {
        object.name.clone()
    }
}

pub(super) fn short_type_name(type_name: &str) -> &str {
    type_name.rsplit("::").next().unwrap_or(type_name)
}

pub(super) fn vec2_angle(value: Vec2) -> f32 {
    value.y.atan2(value.x)
}

pub(super) fn object_world_bounds_2d(object: &Object) -> Option<(Vec2, Vec2)> {
    let transform = object.get_component::<Transform>()?;
    let center = transform.position.truncate();
    let mut min = center - Vec2::splat(0.25);
    let mut max = center + Vec2::splat(0.25);

    if let Some(collider) = object.get_component::<Collider2D>() {
        expand_bounds(
            &mut min,
            &mut max,
            center - collider.half_size,
            center + collider.half_size,
        );
    }

    if object.get_component::<SpriteRenderer>().is_some() {
        let half =
            Vec2::new(transform.scale.x.abs(), transform.scale.y.abs()).max(Vec2::splat(0.5)) * 0.5;
        expand_bounds(&mut min, &mut max, center - half, center + half);
    }

    if let Some(tilemap) = object.get_component::<Tilemap>() {
        let tile_size = Vec2::new(tilemap.tile_size.x as f32, tilemap.tile_size.y as f32);
        let tile_min = center
            + Vec2::new(
                tilemap.offset.x as f32 * tile_size.x,
                tilemap.offset.y as f32 * tile_size.y,
            );
        let tile_max = center
            + Vec2::new(
                (tilemap.offset.x + tilemap.width as i32) as f32 * tile_size.x,
                (tilemap.offset.y + tilemap.height as i32) as f32 * tile_size.y,
            );
        expand_bounds(&mut min, &mut max, tile_min, tile_max);
    }

    if object.get_component::<MeshRenderer>().is_some() {
        let half =
            Vec2::new(transform.scale.x.abs(), transform.scale.y.abs()).max(Vec2::splat(1.0)) * 0.5;
        expand_bounds(&mut min, &mut max, center - half, center + half);
    }

    if let Some(collision) = object.get_component::<PhysicsCollision>() {
        let half = collision.size * 0.5;
        expand_bounds(&mut min, &mut max, center - half, center + half);
    }

    Some((min, max))
}

fn expand_bounds(min: &mut Vec2, max: &mut Vec2, other_min: Vec2, other_max: Vec2) {
    min.x = min.x.min(other_min.x);
    min.y = min.y.min(other_min.y);
    max.x = max.x.max(other_max.x);
    max.y = max.y.max(other_max.y);
}

pub(super) fn object_world_bounds_3d(object: &Object) -> Option<(Vec3, Vec3)> {
    let transform = object.get_component::<Transform>()?;
    let center = transform.position;
    let mut min = center - Vec3::splat(0.1);
    let mut max = center + Vec3::splat(0.1);

    if let Some(collider) = object.get_component::<Collider2D>() {
        expand_bounds_3d(
            &mut min,
            &mut max,
            center - Vec3::new(collider.half_size.x, collider.half_size.y, 0.05),
            center + Vec3::new(collider.half_size.x, collider.half_size.y, 0.05),
        );
    }

    if object.get_component::<SpriteRenderer>().is_some() {
        let half = Vec3::new(
            transform.scale.x.abs().max(0.5) * 0.5,
            transform.scale.y.abs().max(0.5) * 0.5,
            0.05,
        );
        expand_bounds_3d(&mut min, &mut max, center - half, center + half);
    }

    if let Some(tilemap) = object.get_component::<Tilemap>() {
        let tile_min = center
            + Vec3::new(
                tilemap.offset.x as f32 * tilemap.tile_size.x as f32,
                tilemap.offset.y as f32 * tilemap.tile_size.y as f32,
                -0.05,
            );
        let tile_max = center
            + Vec3::new(
                (tilemap.offset.x + tilemap.width as i32) as f32 * tilemap.tile_size.x as f32,
                (tilemap.offset.y + tilemap.height as i32) as f32 * tilemap.tile_size.y as f32,
                0.05,
            );
        expand_bounds_3d(&mut min, &mut max, tile_min, tile_max);
    }

    if object.get_component::<MeshRenderer>().is_some() {
        let half = Vec3::new(
            transform.scale.x.abs().max(1.0) * 0.5,
            transform.scale.y.abs().max(1.0) * 0.5,
            transform.scale.z.abs().max(1.0) * 0.5,
        );
        expand_bounds_3d(&mut min, &mut max, center - half, center + half);
    }

    if let Some(collision) = object.get_component::<PhysicsCollision>() {
        let half = Vec3::new(collision.size.x, collision.size.y, 0.05);
        expand_bounds_3d(&mut min, &mut max, center - half, center + half);
    }

    if let Some(interactable) = object.get_component::<CursorInteractable>() {
        expand_bounds_3d(
            &mut min,
            &mut max,
            center - interactable.bounds_size,
            center + interactable.bounds_size,
        );
    }

    Some((min, max))
}

fn expand_bounds_3d(min: &mut Vec3, max: &mut Vec3, other_min: Vec3, other_max: Vec3) {
    min.x = min.x.min(other_min.x);
    min.y = min.y.min(other_min.y);
    min.z = min.z.min(other_min.z);
    max.x = max.x.max(other_max.x);
    max.y = max.y.max(other_max.y);
    max.z = max.z.max(other_max.z);
}

pub(super) fn gizmo_handles(center: Vec2, scale: Vec3) -> [(GizmoHandleKind, Vec2); 4] {
    let half = Vec2::new(scale.x.abs().max(0.5), scale.y.abs().max(0.5)) * 0.5;
    let offset = 0.45;
    [
        (GizmoHandleKind::Translate, center),
        (
            GizmoHandleKind::ScaleX,
            center + Vec2::new(half.x + offset, 0.0),
        ),
        (
            GizmoHandleKind::ScaleY,
            center + Vec2::new(0.0, half.y + offset),
        ),
        (
            GizmoHandleKind::Rotate,
            center + Vec2::new(half.x + offset, half.y + offset),
        ),
    ]
}

pub(super) fn component_icon_name(type_id: TypeId, kind: ComponentRuntimeKind) -> &'static str {
    if type_id == TypeId::of::<Camera>() {
        "c-Camera"
    } else if type_id == TypeId::of::<AudioSource>() {
        "c-AudioSource"
    } else if type_id == TypeId::of::<AudioListener>() {
        "c-AudioListener"
    } else if type_id == TypeId::of::<Collider2D>() {
        "c-Collider2D"
    } else if type_id == TypeId::of::<PhysicsCollision>() {
        "c-PhysicsCollision"
    } else if type_id == TypeId::of::<SpriteRenderer>() {
        "c-SpriteRenderer"
    } else if type_id == TypeId::of::<SpriteAnimator>() {
        "c-SpriteAnimator"
    } else if type_id == TypeId::of::<Sorting>() {
        "c-Sorting"
    } else if type_id == TypeId::of::<MeshRenderer>() {
        "c-MeshRenderer"
    } else if type_id == TypeId::of::<DirectionalLight>() {
        "c-DirectionalLight"
    } else if type_id == TypeId::of::<PointLight>() {
        "c-PointLight"
    } else if type_id == TypeId::of::<Tilemap>() || type_id == TypeId::of::<TilemapRenderer>() {
        "c-TilemapRenderer"
    } else if type_id == TypeId::of::<UiRenderer>() {
        "c-Canvas"
    } else if type_id == TypeId::of::<CursorInteractable>() {
        "c-CursorInteractable"
    } else if type_id == TypeId::of::<ActiveCamera>() {
        "c-ActiveCamera"
    } else if type_id == TypeId::of::<Transform>() {
        "c-Transform"
    } else if kind == ComponentRuntimeKind::Script {
        "c-Script"
    } else {
        "c-Object"
    }
}

pub(super) fn component_anchor_world(object: &Object, type_id: TypeId) -> Option<Vec3> {
    if type_id == TypeId::of::<Camera>() {
        return object
            .get_component::<Camera>()
            .map(|camera| camera.resolved_with_transform(object.get_component::<Transform>()))
            .map(|camera| camera.position);
    }

    object
        .get_component::<Transform>()
        .map(|transform| transform.position)
}

pub(super) fn icon_offset(index: usize) -> egui::Vec2 {
    let offsets = [
        egui::vec2(0.0, -18.0),
        egui::vec2(18.0, -8.0),
        egui::vec2(-18.0, -8.0),
        egui::vec2(0.0, 10.0),
        egui::vec2(18.0, 10.0),
        egui::vec2(-18.0, 10.0),
    ];
    offsets[index % offsets.len()]
}

pub(super) fn draw_component_icon(
    painter: &egui::Painter,
    texture_id: egui::TextureId,
    position: egui::Pos2,
    size: f32,
) {
    let rect = egui::Rect::from_center_size(position, egui::vec2(size, size));
    painter.image(
        texture_id,
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

pub(super) fn draw_gizmo_handle(
    painter: &egui::Painter,
    position: egui::Pos2,
    color: egui::Color32,
    label: &str,
) {
    painter.circle_filled(position, 7.0, color);
    painter.circle_stroke(position, 7.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
    painter.text(
        position,
        Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(8.0),
        egui::Color32::BLACK,
    );
}

pub(super) fn aabb_corners(min: Vec3, max: Vec3) -> [Vec3; 8] {
    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]
}

pub(super) fn world_to_screen(rect: egui::Rect, camera: Camera, world: Vec2) -> egui::Pos2 {
    let visible_size = camera.ortho_visible_size();
    let half_width = visible_size.x * 0.5;
    let half_height = visible_size.y * 0.5;

    let ndc_x = (world.x - camera.position.x) / half_width;
    let ndc_y = (world.y - camera.position.y) / half_height;

    let local_x = (ndc_x + 1.0) * 0.5 * rect.width();
    let local_y = (1.0 - ndc_y) * 0.5 * rect.height();
    egui::pos2(rect.left() + local_x, rect.top() + local_y)
}

pub(super) fn world3_to_screen(
    rect: egui::Rect,
    camera: Camera,
    world: Vec3,
) -> Option<egui::Pos2> {
    let clip = camera.matrix() * world.extend(1.0);
    if clip.w.abs() <= f32::EPSILON {
        return None;
    }

    let ndc = clip.truncate() / clip.w;
    if ndc.z < -1.0 || ndc.z > 1.0 {
        return None;
    }

    let screen_x = rect.left() + (ndc.x + 1.0) * 0.5 * rect.width();
    let screen_y = rect.top() + (1.0 - (ndc.y + 1.0) * 0.5) * rect.height();
    Some(egui::pos2(screen_x, screen_y))
}

pub(super) fn draw_rect_outline(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    thickness: f32,
) {
    let stroke = egui::Stroke::new(thickness, color);
    painter.line_segment([rect.left_top(), rect.right_top()], stroke);
    painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
    painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
}

pub(super) fn ensure_world_extension(path: PathBuf) -> PathBuf {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".world.ron"))
        .unwrap_or(false)
    {
        path
    } else {
        PathBuf::from(format!("{}.world.ron", path.display()))
    }
}

pub(super) fn default_browse_root() -> PathBuf {
    std::env::current_dir().unwrap_or_default()
}

use runvy_ecs::{Children, Entity, World, R};

use crate::components::{GlobalTransform, Transform};

/// Recomputes every [`GlobalTransform`] from the local [`Transform`] hierarchy.
///
/// Call this after the `Start`/`Update` stages and before rendering.
pub fn propagate_transforms(world: &mut World) {
    let roots: Vec<Entity> = world
        .query::<R<Transform>>()
        .filter(|(entity, _)| match world.parent(*entity) {
            None => true,
            Some(parent) => !world.contains(parent) || world.get::<Transform>(parent).is_none(),
        })
        .map(|(entity, _)| entity)
        .collect();

    let mut stack = roots;

    while let Some(entity) = stack.pop() {
        // Compute the world transform if this entity has a local Transform.
        if let Some(local) = world.get::<Transform>(entity).copied() {
            let next = match world.parent(entity) {
                Some(parent) => match world.get::<GlobalTransform>(parent).copied() {
                    Some(parent_global) => parent_global.mul_transform(&local),
                    None => GlobalTransform::from_local(&local),
                },
                None => GlobalTransform::from_local(&local),
            };

            if let Some(global) = world.get_mut::<GlobalTransform>(entity) {
                // Keep the previous values so render interpolation still works.
                global.position = next.position;
                global.rotation = next.rotation;
                global.scale = next.scale;
            } else {
                // Automatic insertion: users only spawn Transform.
                world.add_component(entity, next);
            }
        }

        // Traverse children even if the intermediate entity has no Transform.
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glam::{Quat, Vec3};

    #[test]
    fn propagates_parent_child_and_grandchild() {
        let mut world = World::new();
        let parent = world.spawn((Transform {
            position: Vec3::new(10.0, 0.0, 0.0),
            ..Transform::default()
        },));
        let child = world.spawn((Transform {
            position: Vec3::new(0.0, 5.0, 0.0),
            ..Transform::default()
        },));
        let grandchild = world.spawn((Transform {
            position: Vec3::new(0.0, 0.0, 2.0),
            ..Transform::default()
        },));

        world.add_child(parent, child).unwrap();
        world.add_child(child, grandchild).unwrap();
        propagate_transforms(&mut world);

        assert_eq!(
            world.get::<GlobalTransform>(parent).unwrap().position,
            Vec3::new(10.0, 0.0, 0.0)
        );
        assert_eq!(
            world.get::<GlobalTransform>(child).unwrap().position,
            Vec3::new(10.0, 5.0, 0.0)
        );
        assert_eq!(
            world.get::<GlobalTransform>(grandchild).unwrap().position,
            Vec3::new(10.0, 5.0, 2.0)
        );
    }

    #[test]
    fn propagates_through_parent_without_transform() {
        #[derive(Clone, Copy)]
        struct Anchor;

        let mut world = World::new();
        let anchor = world.spawn((Anchor,));
        let child = world.spawn((Transform {
            position: Vec3::new(3.0, 4.0, 5.0),
            ..Transform::default()
        },));

        world.add_child(anchor, child).unwrap();
        propagate_transforms(&mut world);

        assert_eq!(
            world.get::<GlobalTransform>(child).unwrap().position,
            Vec3::new(3.0, 4.0, 5.0)
        );
        assert!(world.get::<GlobalTransform>(anchor).is_none());
    }

    #[test]
    fn applies_parent_rotation_and_scale() {
        let mut world = World::new();
        let parent = world.spawn((Transform {
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            scale: Vec3::splat(2.0),
            ..Transform::default()
        },));
        let child = world.spawn((Transform {
            position: Vec3::new(1.0, 0.0, 0.0),
            ..Transform::default()
        },));

        world.add_child(parent, child).unwrap();
        propagate_transforms(&mut world);

        let child_global = world.get::<GlobalTransform>(child).unwrap();
        assert!(child_global
            .position
            .abs_diff_eq(Vec3::new(0.0, 2.0, 0.0), 1e-5));
        assert!(child_global.scale.abs_diff_eq(Vec3::splat(2.0), 1e-5));
    }

    #[test]
    fn preserves_previous_values_for_interpolation() {
        let mut world = World::new();
        let entity = world.spawn((Transform::default(),));

        propagate_transforms(&mut world);
        world
            .get_mut::<GlobalTransform>(entity)
            .unwrap()
            .prepare_for_update();

        world.get_mut::<Transform>(entity).unwrap().position = Vec3::new(10.0, 0.0, 0.0);
        propagate_transforms(&mut world);

        let global = world.get::<GlobalTransform>(entity).unwrap();
        assert_eq!(global.interpolated_position(0.5), Vec3::new(5.0, 0.0, 0.0));
    }
}

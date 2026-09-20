<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Object Model

This note describes the current Runvy runtime architecture.

## Runtime Contract

Runvy treats the runtime object graph as the primary model:

- `World`
- `Object`
- components
- behavior scripts

The editor is expected to adapt to this model later. The runtime does not depend on editor-only construction rules.

## Object

`Object` is the entity/container unit in the world.

It contains:

- `ObjectId`
- a name
- a component container

`Transform` is a normal component, but it is auto-added in `Object::new(...)` and guaranteed to exist. There is no embedded-transform dual model anymore.

## Components

Components are the unit of runtime state and attachment.

Examples:

- `Transform`
- `SpriteRenderer`
- `MeshRenderer`
- `Camera`
- `AudioSource`
- `Collider2D`
- marker components
- script components

Runvy intentionally avoids string-based tag systems as the primary gameplay linking mechanism. Prefer typed marker/data components instead.

Define components with `#[derive(Component)]`:

```rust
#[derive(Component)]
struct Health {
    current: i32,
}

#[derive(Component)]
struct PlayerController {
    speed: f32,
}
```

No registration is needed — deriving `Component` is sufficient for the runtime to manage the type.

## Scripts

Scripts are attachable behavior components.

Lifecycle today:

1. `start()`
2. `update()`

Access happens through `ScriptContext`:

- own object id / handle
- own components
- read-only world queries
- deferred commands

Scripts do not construct objects anymore. Composition happens in factory code; scripts only describe runtime behavior.

## World Access

`World` no longer exposes its internal object storage directly.

The public surface is:

- `spawn_bundle(...)` — spawn an object with a tuple of components
- `spawn_object(object)` — spawn a pre-built `Object`
- `despawn(id)`
- `object(id)` / `object_mut(id)` — get the whole Object
- `get::<T>(id)` / `get_mut::<T>(id)` — get a specific component
- `find_first_with::<T>()` — find the first entity with component `T`
- `find_all_with::<T>()` — find all entities with component `T`

`find_all_with::<T>()` replaces the old `query::<T>()` and returns `ObjectId`s. Order is intentionally not part of the contract.

## Deferred Commands

World mutations during script lifecycle are deferred.

Current command path:

```rust
if let Some(id) = ctx.id() {
    ctx.commands().despawn(id);
}
```

Commands are applied after lifecycle/update phases. This avoids invalidating iteration while the world is running behavior.

## How Objects Are Created

### Bundle spawn (recommended for simple cases)

```rust
world.spawn_bundle((
    Transform::default(),
    SpriteRenderer::new(None),
    PlayerController { speed: 0.25 },
));
```

### Named object spawn

```rust
world.spawn_object(
    Object::new("Player")
        .with(Camera::new_orthographic(320.0, 180.0))
        .with(ActiveCamera)
        .with(SpriteRenderer::new(Some(texture)))
        .with(PlayerController::new()),
);
```

## Why Registration Was Removed

Older Runvy versions required explicit registration of components, scripts, and archetypes through `Engine::register()`, `RunvyComponent`, `RunvyScript`, and `RunvyArchetype` derives. This was used for:

- editor/tooling metadata
- serialization bootstrap hooks
- archetype factories

In practice, the registration step added boilerplate without delivering enough value at the current stage of the engine. The code-first approach lets users define components with a single derive and use them immediately.

Metadata for future editor/tooling integration can be re-added later through a separate, opt-in system without burdening every user with bootstrap code.

## Why `construct()` Was Removed

Older Runvy scripts used `construct()` to add components.

That made scripts:

- behavior
- object factories
- tooling bridge points

all at once.

That coupling became a bottleneck.

Now:

- composition happens in object factories / bootstrap code
- scripts only describe runtime behavior
- serialization (frozen — will be re-added later for editor) can target object/component state
- editor compatibility can be built on the same runtime model

## Current Limits

The new core model is much cleaner, but not everything is finished:

- generic component serialization is not registry-driven yet
- one concrete component type per object is still the storage rule
- `Object` still internally uses a raw world pointer
- fixed update / events are not implemented yet

So the architectural direction is in place, but some lower-level runtime cleanup is still ahead.


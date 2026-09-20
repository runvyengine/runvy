<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Runtime And Editor Update Notes (Archived)

> **This document is archived.** The editor, registry, and serialization system
> described below are **frozen** and will be refactored in a later release.
> The current code-first API does not require any of these features.
>
> See [Object Model Notes](object-model.md) and
> [Registration And Archetypes](../tutorials/advanced/registration-and-archetypes.md)
> for the current approach.

This document summarizes the larger runtime/editor changes that landed in `0.6.0-alpha.1`.

## Runtime

- Added `late_update()` to the script lifecycle
- `World::update()` now performs:
  1. `update()` for all objects
  2. `late_update()` for all objects
- `Transform` interpolation is used consistently by the render path for 2D and 3D objects
- Camera constructors no longer expose `viewport_size`
- `viewport_size` is runtime-owned and updated from the active render target/window size

## Cameras

- Orthographic cameras now use a consistent right-handed view/projection path
- Orthographic visible size is aspect-aware through `Camera::ortho_visible_size()`
- `screen_to_world()` uses the same visible-size model
- Camera handling was aligned more closely to reduce fullscreen/wide-screen drift

## Registry And Serialization (Frozen)

- Runtime registry metadata was used for editor/tooling
- Derived components and scripts could expose serialized fields through `#[serialize_field]`
- Editor-visible defaults came from `Default`
- Editor/project flows could store serialized script/component state

## Editor (Frozen)

- Editor was split into smaller modules
- Inspector presented `Object` / `Transform` / `Components` / `Scripts`
- Component and script creation consulted the runtime type registry
- SVG icon loading was added for editor-only UI/icon assets
- `Content Browser -> Live Rust` separated script-file and archetype-file creation

## Serialized Runtime Data (Frozen)

- Project/world loading reapplied serialized fields onto runtime script/component instances
- Archetype-backed object overrides kept serialized script/component entries

## Current Known Follow-Ups

- There is still no full pixel-perfect 2D pipeline
- Runtime pixel snapping is not implemented yet
- Some warnings remain in `runvy_render` and `runvy_editor`
- Prefab/template unification is still incomplete


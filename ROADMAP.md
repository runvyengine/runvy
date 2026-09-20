<!--
⚠️ ROADMAP IS BEING REWRITTEN — ECS MIGRATION IN PROGRESS

The old OCS (Object/Component/System) has been removed.
The engine now uses `runvy_ecs` exclusively.
-->

# Runvy Engine Roadmap — ECS Migration Track

> **Active migration (v0.6+):** `runvy_core::ocs` → `runvy_ecs` crate.
> Old OCS documentation is outdated and has been removed.

## Immediate (current session)

- [x] `runvy_ecs` crate: BlobVec, Archetype, World, Query (Fetch GAT), Bundle macro
- [x] `#[system]` proc macro with inventory-based auto-registration
- [x] Scheduler integrated into `App` — auto-runs in fixed-timestep loop
- [x] `runvy_engine` re-exports `runvy_ecs`; `runvy_app` hosts ECS World + Scheduler
- [x] Port Script-based logic to `#[system]` functions
- [x] Remove `runvy_core::ocs` and `runvy_core::codefirst`

## Next

- [ ] Move component defs from `runvy_core::components` to plain structs
- [ ] Add command queue (deferred spawn/despawn) to `runvy_ecs::World`

## Performance invariants (new ECS)

These must never regress:

- Unused features must have zero runtime cost (Cargo features + `#[cfg]`)
- No per-frame GPU buffer allocations — use pools or persistent buffers
- Archetype queries must not allocate per call
- Archetype column access is O(1) (index by ComponentInfo index)
- `Entity` lookup is O(1) via location map

## How to contribute

See [`CONTRIBUTING.md`](CONTRIBUTING.md) (docs being rewritten for ECS).

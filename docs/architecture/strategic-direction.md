<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->

# Strategic Direction: Code-First Engine

| Status   | Last updated |
| -------- | ------------ |
| Accepted | 2026-06-23   |

## Decision

**Freeze `runvy_editor` feature development. Focus all effort on the code-first
Rust API until `runvy_core` reaches API stability (target: v0.10).**

The editor remains in the workspace as a prototype — it compiles, it launches,
but it receives no new features. It will be rewritten from scratch once the
core API is stable and the architectural lessons from the prototype are
codified.

## Context

The project started with a dual vision: "code-first runtime + convenient
editor." In practice this created tension:

| Area             | Code-first                        | Editor                                  |
| ---------------- | --------------------------------- | --------------------------------------- |
| **Size**         | ~11k lines (core + engine layers) | ~11k lines                              |
| **Coupling**     | Self-contained                    | Tight to egui + wgpu + winit specifics  |
| **API pressure** | Designed for script ergonomics    | Designed for reflection + serialization |
| **Maintenance**  | Low                               | High (egui API churn, GPU state, DPI)   |
| **User base**    | Primary (Rust gamedevs)           | Secondary (non-Rust users)              |

Maintaining both at once meant neither was excellent. The editor consumed 42%
of the codebase but only served a prototype workflow. The core had no
dedicated DX budget — it was optimized for "good enough for the editor."

### What the prototype taught us

The editor prototype was invaluable. It revealed:

1. **Reflection needs** — `#[serialize_field]`, `TypeRegistry`, name-based
   type lookup were all driven by editor requirements. These are now
   archived; future editor serialization will use a different approach.
2. **Rc<RefCell<World>> is painful** — shared mutability through the whole
   stack creates borrow-checking headaches. The next editor should work with
   a cleaner separation (e.g., commands → world snapshot).
3. **egui is great for tools but not for game UIs** — the canvas-space UI
   module in core was an attempt to decouple game UI from editor UI, but it
   duplicated effort.
4. **Object + HashMap<TypeId, Box<dyn Component>> is simple but slow** —
   the trait-object ECS works for prototypes but needs optimisation for
   production use.

These lessons are now captured and will inform the editor rewrite when the
time comes.

## What "Code-First" Means for Runvy

Code-first means **Rust code is the primary way to build a game**. The engine
exposes its power through typed, composable, documented APIs — not through
drag-and-drop or visual scripting.

A great code-first engine should feel like:

```rust
// 🎯 TARGET API (not yet implemented — aspirational)
use runvy::prelude::*;

#[derive(Component)]
struct Health(f32);

fn spawn_player(world: &mut World) {
    world.spawn((
        Player,
        Health(100.0),
        SpriteRenderer::from("hero.png"),
    ));
}

fn heal_system(mut q: Query<&mut Health, With<Player>>) {
    for mut health in q.iter_mut() {
        health.0 = (health.0 + 10.0).min(100.0);
    }
}
```

**Current API** (works today):

```rust
fn spawn_player(world: &mut World) -> ObjectId {
    world.spawn(
        Object::new("Player")
            .with(Player)
            .with(Health(100.0))
            .with(SpriteRenderer::from("hero.png")),
    )
}
```

Not like:

```rust
// What it shouldn't feel like (manual Vec<Box<dyn Component>>)
fn spawn_player(world: &mut World) -> u64 {
    let mut obj = Object::new("Player");
    obj.add_component(Box::new(Player));
    obj.add_component(Box::new(Health(100.0)));
    let id = obj.id();
    world.add_object(obj);
    id
}
```

## Code-First Design Principles

### 1. Type-driven, not string-driven

- Use Rust types for everything: components, queries, events, resources.
- No string tags, no runtime type names in hot paths.
- Derive macros generate registration boilerplate automatically.

### 2. Zero boilerplate for common cases

- `#[derive(Component)]` should be enough — auto-register, auto-generate
  serialization, auto-create default.
- `world.spawn((A, B, C))` — tuples of components, no manual builder.
- Extending the engine should feel like extending Rust, not configuring
  a framework.

### 3. Productive defaults, explicit overrides

- Components implement `Default` — `SpriteRenderer::default()` gives
  a red placeholder.
- `Camera` defaults to orthographic 16:9.
- `world.spawn(Player)` works with no other required components.
- Every default can be overridden in one chained call.

### 4. Discoverable through the type system

- `q.iter()` vs `q.iter_mut()` — borrow checking guides correct usage.
- Wrong queries fail at compile time, not runtime.
- `cargo doc` produces one reference: every public type with examples.

### 5. No hidden state, no global singletons

- Resources are explicit parameters, not `unsafe static mut`.
- `World` is passed around, not cloned or global.
- Systems are functions with clear inputs and outputs.

### 6. Performance is a feature

- Archetype-based storage (group components by their type set).
- Chunk iteration, cache-friendly layout.
- Zero-cost abstraction — unused features compile to nothing (`#[cfg]`).
- Runtime feature flags: `features = ["physics"]` adds Rapier, `features = []` adds zero bytes.

### 7. Great error messages

- `#[derive(Component)]` on an enum? → `error: Component must be a struct`
- `world.spawn(())` → `warning: spawned object has no components`
- Missing required component in query → compile error, not panic

## Solo Development Workflow

### Task tracking

**Use GitHub Issues, even as a solo developer.** Here's why:

| Tool            | Solo                       | With team            |
| --------------- | -------------------------- | -------------------- |
| GitHub Issues   | Milestones, labels, search | Same, plus assignees |
| Local TODO.md   | Only you can see it        | Doesn't scale        |
| ROADMAP.md      | High-level only            | High-level only      |
| GitHub Projects | Kanban board               | Shared board         |

**Recommended labels:**

```
area:core      area:render    area:editor
area:docs      area:build     area:examples
perf           bug            enhancement
good-first-issue  help-wanted
```

### Workflow

```
1.  ROADMAP.md  →  high-level milestones (what are we building next?)
       ↓
2.  GitHub Issues →  actionable tasks from roadmap items
       ↓
3.  GitHub Project →  track progress (TODO / In Progress / Done)
       ↓
4.  Commits →  "feat(core): add Query::iter_mut" (conventional commits)
       ↓
5.  CHANGELOG.md  →  summarise for users
```

### Solo tips

- **Don't over-organise.** No sprints, no story points, no standups. Just
  labels + milestones.
- **One milestone at a time.** Set the current release (e.g., "v0.7") and
  move everything else to "Backlog."
- **Close issues when done.** If it's trivial, commit directly to `main`
  with `fix: ...` and close. If it's complex, use a feature branch.
- **Use the ROADMAP as your north star.** Update it every time you finish
  a milestone. It's the first thing contributors read.
- **Keep a scratch file** (`tasks.md` in project root) for raw TODO notes
  during a coding session. Don't pollute GitHub Issues with "fix typo in
  line 42."
- **Conventional commits are worth it even alone.** `git log --oneline`
  becomes a readable diary, and `git-cliff` can auto-generate changelogs.

### Alternative (no GitHub)

If you prefer local-only:

```
docs/
  roadmap.md     — high-level plan
  milestones/    — one file per milestone with checklists
tasks.md         — today's TODO (ephemeral, update daily)
CHANGELOG.md     — record of what shipped
```

This works perfectly well for solo. GitHub Issues add discoverability for
potential contributors, which matters if you want the project to grow.

## Consequences

### What changes

- **Editor crate**: no new features. Bug fixes only if they block
  core development.
- **Core crate**: dedicated DX budget. Each release must answer "is the
  public API noticeably better than last time?"
- **Examples**: treated as integration tests. Each example exercises
  a different dimension of the API.
- **CI**: `cargo check --workspace --exclude runvy_editor` as the fast path.
  Editor builds remain optional (`--features editor`).

### What stays the same

- Editor code is not deleted. It lives in `main` history and the crate
  remains compilable.
- The prototype's lessons are documented in this ADR and referenced in
  future editor design.
- Dual licensing (MIT / Apache 2.0) remains.

### What we gain

- Clear priority for contributors and users.
- Faster iteration on the core API.
- Less maintenance burden from UI framework churn.
- Better code-first DX before asking users to learn the engine.

### What we lose

- Visual feedback loop for non-Rust users.
- Dogfooding the rendering API through editor UI.
- Near-term ability to attract non-programmer gamedevs.

## When to re-evaluate

The editor freeze is not permanent. Re-evaluate when:

- `runvy_core` v0.10 is released on crates.io
- The public API has been stable for 2+ minor releases
- There is community demand (or contributors) for an editor
- The renderer supports enough features to make an editor useful
  (multi-viewport, PBR preview, asset thumbnails)

At that point, start a clean `runvy_editor2` crate with the architecture
lessons from the prototype.

## Related documents

- [ROADMAP.md](../ROADMAP.md) — timeline and checklist view
- [Runtime And Editor Update Notes](runtime-and-editor-update.md) —
  retrospective on the editor prototype
- [Object Model Notes](object-model.md) — how World/ECS works today

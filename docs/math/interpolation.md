<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->
# Interpolation & Smoothing

Runvy provides a set of interpolation, easing, and smoothing utilities modelled after Unity's `Mathf`.
All functions live in `runvy_core::math` and are re-exported through `runvy_engine::prelude::*`.

---

## Basic Interpolation

### `lerp(a, b, t)`
Clamped linear interpolation — `t` is clamped to `[0, 1]`.

```rust
let value = lerp(0.0, 10.0, 0.5);  // 5.0
```

### `lerp_unclamped(a, b, t)`
Linear interpolation without clamping `t`.

### `inverse_lerp(a, b, value)`
Returns `t` such that `lerp(a, b, t) == value`.

```rust
let t = inverse_lerp(0.0, 10.0, 5.0);  // 0.5
```

### `remap(value, from_a, from_b, to_a, to_b)`
Remaps `value` from one range to another.

```rust
let v = remap(50.0, 0.0, 100.0, 0.0, 1.0);  // 0.5
```

---

## Angle Interpolation

### `lerp_angle(a, b, t)`
Lerp between two angles in degrees, taking the shortest path.

```rust
let a = lerp_angle(350.0, 10.0, 0.5);  // 0.0 (wraps around)
```

### `move_towards_angle(current, target, max_delta)`
Moves an angle toward `target` by at most `max_delta` degrees.

---

## Smooth Step

### `smooth_step(a, b, t)`
Hermite interpolation — smooth start and end, third order.

```rust
let v = smooth_step(0.0, 1.0, 0.5);  // 0.5
```

### `smoother_step(a, b, t)`
Fifth-order version with even smoother acceleration/deceleration.

---

## Easing Curves

All easing functions take a normalised time `t` in `[0, 1]`:

| Function | Behaviour |
|----------|-----------|
| `ease_in_quad(t)` | Quadratic ease-in |
| `ease_out_quad(t)` | Quadratic ease-out |
| `ease_in_out_quad(t)` | Quadratic ease-in-out |
| `ease_in_cubic(t)` | Cubic ease-in |
| `ease_out_cubic(t)` | Cubic ease-out |
| `ease_in_out_cubic(t)` | Cubic ease-in-out |
| `ease_in_expo(t)` | Exponential ease-in |
| `ease_out_expo(t)` | Exponential ease-out |
| `ease_in_out_expo(t)` | Exponential ease-in-out |
| `ease_in_elastic(t)` | Elastic ease-in (overshoot + bounce) |
| `ease_out_elastic(t)` | Elastic ease-out |
| `ease_in_out_elastic(t)` | Elastic ease-in-out |
| `ease_in_bounce(t)` | Bouncing ease-in |
| `ease_out_bounce(t)` | Bouncing ease-out |

Example — animate from `0` to `10` over `2` seconds with cubic ease-out:

```rust
let elapsed = 1.0;   // half way
let t = elapsed / 2.0;
let value = lerp(0.0, 10.0, ease_out_cubic(t));
```

---

## Smooth Damp

Unity-style spring-physics smoothing. Handles variable frame rates and produces natural-looking motion.

### `smooth_damp(current, target, current_velocity, smooth_time, max_speed, delta_time)`

```rust
let mut velocity = 0.0;
let mut value = 0.0;

// In your update loop:
value = smooth_damp(value, 10.0, &mut velocity, 0.3, f32::INFINITY, dt);
```

- `smooth_time` — approximate time (seconds) to reach the target.
- `max_speed` — clamps the maximum speed (use `f32::INFINITY` for no limit).
- `current_velocity` — the current velocity (modified each frame, persist between frames).

### `smooth_damp_unlimited(current, target, current_velocity, smooth_time, delta_time)`

Same as `smooth_damp` with `max_speed = f32::INFINITY`.

### `smooth_damp_vec3(current, target, current_velocity, smooth_time, max_speed, delta_time)`

Per-component `smooth_damp` for `Vec3`. Useful for camera follow:

```rust
let mut vel = Vec3::ZERO;
camera_pos = smooth_damp_vec3(camera_pos, target_pos, &mut vel, 0.3, f32::INFINITY, dt);
```

---

## Move Towards

### `move_towards(current, target, max_delta)`
Moves `current` toward `target` by at most `max_delta` (clamped step).

```rust
let v = move_towards(0.0, 10.0, 3.0);  // 3.0
```

---

## Extension Trait: `LerpExt`

Available on `f32`, `Vec2`, `Vec3`, `Vec4` via `use runvy_engine::prelude::*`:

```rust
let v = 0.0_f32.lerp(10.0, 0.5);      // 5.0
let v = Vec3::ZERO.lerp(Vec3::ONE, 0.5); // (0.5, 0.5, 0.5)
let v = 5.0_f32.move_towards(10.0, 2.0); // 7.0
```

| Method | Description |
|--------|-------------|
| `a.lerp(b, t)` | Clamped linear interpolation |
| `a.lerp_unclamped(b, t)` | Unclamped linear interpolation |
| `a.smooth_step(b, t)` | Hermite interpolation |
| `a.move_towards(b, max_delta)` | Step towards by max_delta |


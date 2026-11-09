use crate::audio::AudioEngine;
use crate::collision2d::intersects_world;
use crate::collision3d::intersects_world as intersects_world_3d;
use crate::components::{
    AudioListener, AudioSource, Collider2D, Collider3D, CursorInteractable, OnTriggerEnter2D,
    OnTriggerEnter3D, OnTriggerExit2D, OnTriggerExit3D, OnTriggerStay2D, OnTriggerStay3D,
    SpriteAnimator, SpriteRenderer, Timer, Transform, WorldCollider2D, WorldCollider3D,
};
use crate::resources::event::EventBus;
use crate::resources::input::InputState;
use crate::resources::{CollisionTracker2D, CollisionTracker3D, Time};
use runvy_ecs::{Entity, Query, QueryMut, Res, ResMut, R, W};
use runvy_macros::system;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use winit::event::MouseButton;

fn audio_engine() -> &'static Mutex<Option<AudioEngine>> {
    static ENGINE: OnceLock<Mutex<Option<AudioEngine>>> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut e = AudioEngine::new();
        match e.initialize() {
            Ok(()) => {
                e.set_master_volume(0.5);
                Mutex::new(Some(e))
            }
            Err(err) => {
                eprintln!("audio_system: failed to initialize: {}", err);
                Mutex::new(None)
            }
        }
    })
}

#[system(Update, "crate")]
pub fn cursor_interaction(
    mut input: ResMut<InputState>,
    q: QueryMut<(W<CursorInteractable>, R<Transform>)>,
) {
    let world_pos = match input.get_mouse_world_position() {
        Some(p) => p,
        None => return,
    };
    let mouse_down = input.is_mouse_button_just_pressed(MouseButton::Left);

    for (_, (interactable, transform)) in q {
        interactable.is_hovered = interactable.contains_point(world_pos, transform.position);
        if mouse_down && interactable.is_hovered {
            if let Some(cb) = interactable.on_click_mut() {
                if let Ok(f) = cb.get_mut() {
                    f();
                }
            }
        }
        interactable.update_callbacks();
    }
}

#[system(Update, "crate")]
pub fn audio_system(
    sources: QueryMut<W<AudioSource>>,
    listeners: Query<(R<AudioListener>, R<Transform>)>,
) {
    let mut guard = audio_engine().lock().unwrap();
    let Some(engine) = guard.as_mut() else {
        return;
    };

    for (_, source) in sources {
        if source.play_requested {
            source.sound_id = engine.play(source);
            source.play_requested = false;
            source.playing = source.sound_id.is_some();
        }
        if source.stop_requested {
            if let Some(id) = source.sound_id {
                engine.stop(id);
            }
            source.sound_id = None;
            source.stop_requested = false;
            source.playing = false;
        }
    }

    for (_, (listener, transform)) in listeners {
        if listener.active {
            engine.set_listener(transform.position, transform.rotation, listener.volume);
        }
    }

    engine.update_spatial_volumes();
    engine.cleanup();
}

#[system(Update, "crate")]
pub fn eventbus_system(mut bus: ResMut<EventBus>) {
    bus.process();
}

#[system(Update, "crate")]
pub fn sprite_animator_system(
    time: Res<Time>,
    q: QueryMut<(W<SpriteAnimator>, W<SpriteRenderer>)>,
) {
    let dt = time.delta;
    for (_, (animator, sprite)) in q {
        let uv = animator.tick(dt);
        sprite.uv_rect = uv;
    }
}

struct Collider2DSnapshot {
    entity: Entity,
    collider: Collider2D,
    world: WorldCollider2D,
}

#[system(Update, "crate")]
pub fn collision_2d_system(
    q: Query<(R<Transform>, R<Collider2D>)>,
    mut tracker: ResMut<CollisionTracker2D>,
    mut bus: ResMut<EventBus>,
) {
    // ── Pass 1: read. Copy each enabled collider into a Vec, resolved to
    //    world space exactly once. The query borrow ends here. ──────────────
    let mut colliders: Vec<Collider2DSnapshot> = Vec::new();

    for (entity, (t, c)) in q {
        let collider = *c;
        if !collider.enabled {
            continue;
        }
        colliders.push(Collider2DSnapshot {
            entity,
            collider,
            world: collider.to_world(t),
        });
    }

    // ── Pass 2: narrow. Pure math on the Vec, no world access. ─────────────
    let n = colliders.len();
    let mut current_contacts: HashMap<Entity, HashSet<Entity>> = HashMap::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if intersects_world(&colliders[i].world, &colliders[j].world) {
                let a = colliders[i].entity;
                let b = colliders[j].entity;
                current_contacts.entry(a).or_default().insert(b);
                current_contacts.entry(b).or_default().insert(a);
            }
        }
    }

    // ── Pass 3: write. Diff against last frame, emit trigger events. ────────
    let is_trigger: HashMap<Entity, bool> = colliders
        .iter()
        .map(|c| (c.entity, c.collider.is_trigger))
        .collect();

    let events: Vec<(Entity, Entity, u8)> = {
        let prev = &tracker.contacts;
        let mut out = Vec::new();
        for (entity, others) in &current_contacts {
            let prev_others = prev.get(entity);
            for other in others {
                let was = prev_others.is_some_and(|s| s.contains(other));
                out.push((*entity, *other, if was { 2 } else { 0 }));
            }
            if let Some(prev_others) = prev_others {
                for other in prev_others {
                    if !others.contains(other) {
                        out.push((*entity, *other, 1));
                    }
                }
            }
        }
        tracker.contacts = current_contacts;
        out
    };

    for (this, other, kind) in events {
        let trigger = is_trigger.get(&this).copied().unwrap_or(false)
            || is_trigger.get(&other).copied().unwrap_or(false);
        if !trigger {
            continue;
        }
        match kind {
            0 => bus.emit(OnTriggerEnter2D { this, other }),
            1 => bus.emit(OnTriggerExit2D { this, other }),
            _ => bus.emit(OnTriggerStay2D { this, other }),
        }
    }
}

struct Collider3DSnapshot {
    entity: Entity,
    collider: Collider3D,
    world: WorldCollider3D,
}

#[system(Update, "crate")]
pub fn collision_3d_system(
    q: Query<(R<Transform>, R<Collider3D>)>,
    mut tracker: ResMut<CollisionTracker3D>,
    mut bus: ResMut<EventBus>,
) {
    let mut colliders: Vec<Collider3DSnapshot> = Vec::new();

    for (entity, (t, c)) in q {
        let collider = *c;
        if !collider.enabled {
            continue;
        }
        colliders.push(Collider3DSnapshot {
            entity,
            collider,
            world: collider.to_world(t),
        });
    }

    let n = colliders.len();
    let mut current_contacts: HashMap<Entity, HashSet<Entity>> = HashMap::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if intersects_world_3d(&colliders[i].world, &colliders[j].world) {
                let a = colliders[i].entity;
                let b = colliders[j].entity;
                current_contacts.entry(a).or_default().insert(b);
                current_contacts.entry(b).or_default().insert(a);
            }
        }
    }

    let is_trigger: HashMap<Entity, bool> = colliders
        .iter()
        .map(|c| (c.entity, c.collider.is_trigger))
        .collect();

    let events: Vec<(Entity, Entity, u8)> = {
        let prev = &tracker.contacts;
        let mut out = Vec::new();
        for (entity, others) in &current_contacts {
            let prev_others = prev.get(entity);
            for other in others {
                let was = prev_others.is_some_and(|s| s.contains(other));
                out.push((*entity, *other, if was { 2 } else { 0 }));
            }
            if let Some(prev_others) = prev_others {
                for other in prev_others {
                    if !others.contains(other) {
                        out.push((*entity, *other, 1));
                    }
                }
            }
        }
        tracker.contacts = current_contacts;
        out
    };

    for (this, other, kind) in events {
        let trigger = is_trigger.get(&this).copied().unwrap_or(false)
            || is_trigger.get(&other).copied().unwrap_or(false);
        if !trigger {
            continue;
        }
        match kind {
            0 => bus.emit(OnTriggerEnter3D { this, other }),
            1 => bus.emit(OnTriggerExit3D { this, other }),
            _ => bus.emit(OnTriggerStay3D { this, other }),
        }
    }
}

#[system(Start, "crate")]
pub fn timer_start_system(q: QueryMut<W<Timer>>) {
    for (_, timer) in q {
        if timer.autostart {
            timer.autostart = false;
            timer.restart();
        }
    }
}

#[system(Update, "crate")]
pub fn timer_system(time: Res<Time>, q: QueryMut<W<Timer>>) {
    let dt = time.delta;

    for (_, timer) in q {
        if !timer.running {
            continue;
        }

        timer.remaining -= dt.max(0.0);

        if timer.remaining <= 0.0 {
            timer.running = false;
            timer.times_fired += 1;

            if let Some(cb) = timer.on_timeout_mut() {
                if let Ok(f) = cb.get_mut() {
                    f();
                }
            }

            if timer.cyclical {
                timer.restart();
            }
        }
    }
}

use std::any::{self, Any, TypeId};
use std::collections::HashMap;

use crate::archetype::{Archetype, ArchetypeId, BlobColumn, Bundle};
use crate::blob_vec::ComponentInfo;
use crate::Entity;

#[derive(Clone, Copy)]
pub(crate) struct Location {
    pub archetype_id: ArchetypeId,
    pub row: u32,
}

/// All entity data: the archetypes, the type-key → archetype index and the
/// per-entity location map.
///
/// This is structurally disjoint from [`ResourceStore`], which is what allows
/// [`World::split`] to lend out both halves at the same time — resources and
/// components never borrow-conflict.
pub struct EntityStore {
    pub archetypes: Vec<Archetype>,
    archetype_by_key: HashMap<Vec<TypeId>, ArchetypeId>,
    next_archetype_id: u32,
    entity_location: HashMap<Entity, Location>,
    next_entity: u64,
}

impl EntityStore {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            archetype_by_key: HashMap::new(),
            next_archetype_id: 0,
            entity_location: HashMap::new(),
            next_entity: 1,
        }
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        let type_ids = B::type_ids();
        let infos = B::component_infos();
        let key = type_ids.clone();

        let arch_id = self.find_or_create_archetype(&key, &infos);
        let arch = &mut self.archetypes[arch_id.0 as usize];

        let entity = self.next_entity;
        self.next_entity += 1;
        let row = arch.entity_count();

        arch.entities.push(entity);
        bundle.put(&mut arch.columns);
        self.entity_location.insert(
            entity,
            Location {
                archetype_id: arch_id,
                row: row as u32,
            },
        );

        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        let Some(loc) = self.entity_location.remove(&entity) else {
            return false;
        };
        let arch = &mut self.archetypes[loc.archetype_id.0 as usize];
        let row = loc.row as usize;

        for col in &mut arch.columns {
            unsafe { col.blob.swap_remove(row) }
        }
        let last = arch.entities.swap_remove(row);

        if row < arch.entities.len() {
            if let Some(last_loc) = self.entity_location.get_mut(&last) {
                last_loc.row = row as u32;
            }
        }

        true
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let loc = self.entity_location.get(&entity)?;
        let arch = self.archetypes.get(loc.archetype_id.0 as usize)?;
        let col = arch.column(TypeId::of::<T>())?;
        let ptr = col.blob.get(loc.row as usize)? as *const T;
        unsafe { Some(&*ptr) }
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let loc = self.entity_location.get(&entity)?;
        let arch = self.archetypes.get_mut(loc.archetype_id.0 as usize)?;
        let col = arch.column_mut(TypeId::of::<T>())?;
        let ptr = col.blob.get(loc.row as usize)? as *mut T;
        unsafe { Some(&mut *ptr) }
    }

    /// Adds (or, if already present, replaces) a component on an existing entity.
    ///
    /// This moves the entity into a new archetype (`old types + T`). Returns `false`
    /// if the entity does not exist.
    pub fn add_component<T: 'static>(&mut self, entity: Entity, value: T) -> bool {
        // Upsert: replace in place when the component is already attached.
        if let Some(slot) = self.get_mut::<T>(entity) {
            *slot = value;
            return true;
        }

        let loc = match self.entity_location.get(&entity) {
            Some(l) => *l,
            None => return false,
        };
        let old_arch_id = loc.archetype_id;
        let old_row = loc.row as usize;

        let old_infos: Vec<ComponentInfo> = self.archetypes[old_arch_id.0 as usize]
            .columns
            .iter()
            .map(|c| c.info.clone())
            .collect();
        let mut new_infos = old_infos.clone();
        new_infos.push(ComponentInfo::of::<T>());
        let new_key: Vec<TypeId> = new_infos.iter().map(|i| i.type_id).collect();
        let new_arch_id = self.find_or_create_archetype(&new_key, &new_infos);

        let new_row;
        {
            let (old_arch, new_arch) =
                Self::split_two(&mut self.archetypes, old_arch_id.0, new_arch_id.0);
            new_row = new_arch.entities.len();
            for (i, col) in old_arch.columns.iter().enumerate() {
                if let Some(src) = col.blob.get(old_row) {
                    // Copy the component's bytes into the new column; the old column
                    // is later removed with `swap_remove_no_drop`, so ownership is
                    // transferred exactly once (no double-free).
                    unsafe { new_arch.columns[i].blob.push(src) };
                }
            }
            let val_ptr = &value as *const T as *mut u8;
            unsafe { new_arch.columns[old_infos.len()].blob.push(val_ptr) };
            std::mem::forget(value);
            new_arch.entities.push(entity);
        }
        self.entity_location.insert(
            entity,
            Location {
                archetype_id: new_arch_id,
                row: new_row as u32,
            },
        );

        unsafe { self.remove_from_archetype_no_drop(old_arch_id, old_row) };
        true
    }

    /// Removes component `T` from an entity, returning it. Returns `None` if the
    /// entity or the component is absent. Moves the entity into a new archetype
    /// (`old types - T`).
    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let loc = self.entity_location.get(&entity)?;
        let old_arch_id = loc.archetype_id;
        let old_row = loc.row as usize;
        let old_arch = &self.archetypes[old_arch_id.0 as usize];
        if !old_arch.has_type(TypeId::of::<T>()) {
            return None;
        }

        // Move the `T` value out (it is returned to the caller; not dropped here).
        let t_col = old_arch.column(TypeId::of::<T>()).unwrap();
        let t_src = t_col.blob.get(old_row).unwrap() as *const T;
        let value = unsafe { std::ptr::read(t_src) };

        let old_infos: Vec<ComponentInfo> =
            old_arch.columns.iter().map(|c| c.info.clone()).collect();
        let new_infos: Vec<ComponentInfo> = old_infos
            .iter()
            .filter(|i| i.type_id != TypeId::of::<T>())
            .cloned()
            .collect();
        let new_key: Vec<TypeId> = new_infos.iter().map(|i| i.type_id).collect();
        let new_arch_id = self.find_or_create_archetype(&new_key, &new_infos);

        let new_row;
        {
            let (old_arch, new_arch) =
                Self::split_two(&mut self.archetypes, old_arch_id.0, new_arch_id.0);
            new_row = new_arch.entities.len();
            for col in &old_arch.columns {
                if col.type_id() == TypeId::of::<T>() {
                    continue;
                }
                let idx = new_arch
                    .columns
                    .iter()
                    .position(|c| c.type_id() == col.type_id())
                    .unwrap();
                if let Some(src) = col.blob.get(old_row) {
                    unsafe { new_arch.columns[idx].blob.push(src) };
                }
            }
            new_arch.entities.push(entity);
        }
        self.entity_location.insert(
            entity,
            Location {
                archetype_id: new_arch_id,
                row: new_row as u32,
            },
        );

        // Remove the entity from the old archetype. The `T` column is already moved
        // out (no drop); the rest were copied, so also use `swap_remove_no_drop`.
        let arch = &mut self.archetypes[old_arch_id.0 as usize];
        let last = arch.entities.len() - 1;
        let swapped = if old_row != last {
            Some(arch.entities[last])
        } else {
            None
        };
        for col in &mut arch.columns {
            unsafe { col.blob.swap_remove_no_drop(old_row) };
        }
        arch.entities.swap_remove(old_row);
        if let Some(e) = swapped {
            if let Some(l) = self.entity_location.get_mut(&e) {
                l.row = old_row as u32;
            }
        }

        Some(value)
    }

    /// Removes the entity's row from `arch_id` without dropping any component (the
    /// components were copied into a new archetype). Fixes the location of the
    /// entity that gets swapped into the freed slot.
    unsafe fn remove_from_archetype_no_drop(&mut self, arch_id: ArchetypeId, row: usize) {
        let arch = &mut self.archetypes[arch_id.0 as usize];
        let last = arch.entities.len() - 1;
        let swapped = if row != last {
            Some(arch.entities[last])
        } else {
            None
        };
        for col in &mut arch.columns {
            col.blob.swap_remove_no_drop(row);
        }
        arch.entities.swap_remove(row);
        if let Some(e) = swapped {
            if let Some(l) = self.entity_location.get_mut(&e) {
                l.row = row as u32;
            }
        }
    }

    /// Returns mutable borrows of two distinct archetypes so their columns can be
    /// copied between without the borrow checker complaining about a single
    /// `&mut self.archetypes` borrow.
    fn split_two(archetypes: &mut [Archetype], a: u32, b: u32) -> (&mut Archetype, &mut Archetype) {
        if a < b {
            let (front, back) = archetypes.split_at_mut(b as usize);
            (&mut front[a as usize], &mut back[0])
        } else {
            let (front, back) = archetypes.split_at_mut(a as usize);
            (&mut back[0], &mut front[b as usize])
        }
    }

    /// Clears every entity and archetype, resetting the id counter. Resources
    /// are intentionally left untouched (see [`crate::world::ResourceStore`]).
    pub fn clear(&mut self) {
        self.archetypes.clear();
        self.archetype_by_key.clear();
        self.next_archetype_id = 0;
        self.entity_location.clear();
        self.next_entity = 1;
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entity_location.contains_key(&entity)
    }

    pub fn entity_count(&self) -> usize {
        self.entity_location.len()
    }

    fn find_or_create_archetype(&mut self, key: &[TypeId], infos: &[ComponentInfo]) -> ArchetypeId {
        if let Some(&id) = self.archetype_by_key.get(key) {
            return id;
        }
        let id = ArchetypeId(self.next_archetype_id);
        self.next_archetype_id += 1;

        let columns: Vec<BlobColumn> = infos
            .iter()
            .map(|info| BlobColumn::new(info.clone()))
            .collect();
        let arch = Archetype::new(id, columns);
        self.archetypes.push(arch);
        self.archetype_by_key.insert(key.to_vec(), id);
        id
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Type-erased store of singleton resources.
///
/// Kept structurally separate from the entity data inside [`World`] so that the
/// two halves can be borrowed independently via [`World::split`].
pub struct ResourceStore {
    map: HashMap<TypeId, Box<dyn Any>>,
}

impl ResourceStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Adds a resource of type `T` to the store.
    ///
    /// # Panics
    /// Panics if a resource of type `T` is already present.
    pub fn add<T: 'static>(&mut self, resource: T) {
        let key = TypeId::of::<T>();

        if self.map.contains_key(&key) {
            panic!("Resource {} already added.", any::type_name::<T>());
        }

        self.map.insert(key, Box::new(resource));
    }

    /// Adds a resource of type `T`, returning `false` (and keeping the value)
    /// if a resource of type `T` is already present. Never panics.
    pub fn try_add<T: 'static>(&mut self, resource: T) -> bool {
        let key = TypeId::of::<T>();
        if self.map.contains_key(&key) {
            return false;
        }
        self.map.insert(key, Box::new(resource));
        true
    }

    /// Removes and returns the resource of type `T`.
    ///
    /// # Panics
    /// Panics if the resource of type `T` is not present.
    pub fn delete<T: 'static>(&mut self) -> T {
        self.try_delete::<T>()
            .unwrap_or_else(|| panic!("Resource {} not found.", any::type_name::<T>()))
    }

    /// Removes and returns the resource of type `T`, or `None` if absent.
    pub fn try_delete<T: 'static>(&mut self) -> Option<T> {
        let key = TypeId::of::<T>();
        self.map
            .remove(&key)
            .and_then(|b| b.downcast::<T>().ok())
            .map(|b| *b)
    }

    /// Returns an immutable borrow of the resource of type `T`.
    ///
    /// # Panics
    /// Panics if the resource of type `T` is not present.
    pub fn get<T: 'static>(&self) -> &T {
        self.try_get::<T>()
            .unwrap_or_else(|| panic!("Resource {} not found.", any::type_name::<T>()))
    }

    /// Returns an immutable borrow of the resource of type `T`, or `None` if absent.
    pub fn try_get<T: 'static>(&self) -> Option<&T> {
        let key = TypeId::of::<T>();
        self.map.get(&key).and_then(|b| b.downcast_ref::<T>())
    }

    /// Returns a mutable borrow of the resource of type `T`.
    ///
    /// # Panics
    /// Panics if the resource of type `T` is not present.
    pub fn get_mut<T: 'static>(&mut self) -> &mut T {
        self.try_get_mut::<T>()
            .unwrap_or_else(|| panic!("Resource {} not found.", any::type_name::<T>()))
    }

    /// Returns a mutable borrow of the resource of type `T`, or `None` if absent.
    pub fn try_get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let key = TypeId::of::<T>();
        self.map.get_mut(&key).and_then(|b| b.downcast_mut::<T>())
    }

    /// Inserts the default value of `T` into the resource store
    /// if a resource of type `T` is not already present.
    pub fn init<T: Default + 'static>(&mut self) {
        self.map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(T::default()));
    }

    /// Returns a raw, const pointer to the resource of type `T`, or `None` if absent.
    ///
    /// Useful for reaching resources through split borrows (see
    /// [`World::split`]). The caller is responsible for keeping the borrow
    /// within the borrow of `self`.
    pub fn get_raw<T: 'static>(&self) -> Option<*const T> {
        self.try_get::<T>().map(|v| v as *const T)
    }

    /// Returns a raw, mutable pointer to the resource of type `T`, or `None` if absent.
    ///
    /// The caller is responsible for keeping the borrow within the borrow of
    /// `self` and for not aliasing other references to the same resource.
    pub fn get_raw_mut<T: 'static>(&mut self) -> Option<*mut T> {
        self.try_get_mut::<T>().map(|v| v as *mut T)
    }
}

impl Default for ResourceStore {
    fn default() -> Self {
        Self::new()
    }
}

pub struct World {
    pub resources: ResourceStore,
    pub entities: EntityStore,
    start_pending: bool,
}

impl World {
    pub fn new() -> Self {
        Self {
            resources: ResourceStore::new(),
            entities: EntityStore::new(),
            start_pending: false,
        }
    }

    /// Splits the world into its two structurally disjoint halves: resources and
    /// entity storage. Because the halves never overlap, borrows taken from each
    /// half can coexist — so you can read a resource while mutably querying
    /// components, without the "double borrow" dance:
    ///
    /// ```ignore
    /// let (resources, entities) = world.split();
    /// let dt = resources.get::<Time>().delta;
    /// for (_, (t,)) in entities.query_mut::<(W<Transform>,)>() {
    ///     t.position += Vec3::X * dt;
    /// }
    /// ```
    pub fn split(&mut self) -> (&mut ResourceStore, &mut EntityStore) {
        (&mut self.resources, &mut self.entities)
    }

    // ─── Entity API ──────────────────────────────────────────

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        self.entities.spawn(bundle)
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.entities.despawn(entity)
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        self.entities.get(entity)
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.entities.get_mut(entity)
    }

    pub fn add_component<T: 'static>(&mut self, entity: Entity, value: T) -> bool {
        self.entities.add_component(entity, value)
    }

    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        self.entities.remove_component(entity)
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.start_pending = true;
    }

    /// Requests another run of the `Start` stage, e.g. after resetting the
    /// world for a new scene.
    pub fn request_start(&mut self) {
        self.start_pending = true;
    }

    /// Clears and returns the pending `Start` request.
    pub fn take_start_request(&mut self) -> bool {
        std::mem::take(&mut self.start_pending)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(entity)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.entity_count()
    }

    // ─── Resource API ────────────────────────────────────────

    pub fn add_resource<T: 'static>(&mut self, resource: T) {
        self.resources.add(resource)
    }

    pub fn try_add_resource<T: 'static>(&mut self, resource: T) -> bool {
        self.resources.try_add(resource)
    }

    pub fn delete_resource<T: 'static>(&mut self) -> T {
        self.resources.delete()
    }

    pub fn try_delete_resource<T: 'static>(&mut self) -> Option<T> {
        self.resources.try_delete()
    }

    pub fn get_resource<T: 'static>(&self) -> &T {
        self.resources.get()
    }

    pub fn try_get_resource<T: 'static>(&self) -> Option<&T> {
        self.resources.try_get()
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> &mut T {
        self.resources.get_mut()
    }

    pub fn try_get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources.try_get_mut()
    }

    pub fn init_resource<T: Default + 'static>(&mut self) {
        self.resources.init::<T>()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::W;

    #[derive(Debug, PartialEq, Default)]
    struct A {
        v: i32,
    }
    #[derive(Debug, PartialEq)]
    struct B {
        s: String,
    }

    #[test]
    fn add_and_remove_component() {
        let mut w = World::new();
        let e = w.spawn((A { v: 1 },));
        assert!(w.get::<A>(e).is_some());
        assert!(w.get::<B>(e).is_none());

        // Add a new component.
        assert!(w.add_component(e, B { s: "hi".into() }));
        assert_eq!(w.get::<B>(e).unwrap().s, "hi");
        // Existing component is untouched.
        assert_eq!(w.get::<A>(e).unwrap().v, 1);

        // Upsert replaces in place.
        assert!(w.add_component(e, B { s: "bye".into() }));
        assert_eq!(w.get::<B>(e).unwrap().s, "bye");

        // Remove returns the value and detaches it.
        let removed = w.remove_component::<B>(e).unwrap();
        assert_eq!(removed.s, "bye");
        assert!(w.get::<B>(e).is_none());
        assert!(w.get::<A>(e).is_some());

        // Removing an absent component yields None.
        assert!(w.remove_component::<B>(e).is_none());
        // Removing from a nonexistent entity yields None.
        assert!(w.remove_component::<A>(999).is_none());
    }

    #[test]
    fn add_component_nonexistent_entity() {
        let mut w = World::new();
        assert!(!w.add_component(123, A { v: 1 }));
    }

    #[test]
    fn split_lends_resources_and_entities_independently() {
        #[derive(Default)]
        struct Time {
            delta: f32,
        }

        let mut w = World::new();
        w.spawn((A { v: 1 },));
        w.add_resource(Time { delta: 0.5 });

        {
            let (resources, entities) = w.split();
            let dt = resources.get::<Time>().delta;
            for (_, a) in entities.query_mut::<W<A>>() {
                a.v += dt as i32;
            }
            assert!(resources.try_get::<Time>().is_some());
            assert!(!resources.try_add(Time { delta: 1.0 }));
        }

        assert_eq!(w.get::<A>(1).unwrap().v, 1);
    }
}
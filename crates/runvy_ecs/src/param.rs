use std::any::{self, TypeId};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use crate::query::{Fetch, Query, QueryMut};
use crate::world::{EntityStore, ResourceStore, World};
use crate::CommandQueue;

/// A copyable, lifetime-carrying view of the whole world.
///
/// It hands out `&'w` / `&'w mut` borrows of resources and entity storage.
/// Soundness is NOT checked by the borrow checker here — the caller is
/// responsible for ensuring the accesses stay disjoint (in practice this is
/// done by [`SystemParam`] access validation before fetching).
#[derive(Clone, Copy)]
pub struct UnsafeWorldCell<'w> {
    ptr: NonNull<World>,
    marker: PhantomData<&'w mut World>,
}

impl<'w> UnsafeWorldCell<'w> {
    /// # Safety
    ///
    /// The returned borrows must never alias: the caller has to prove access
    /// disjointness (e.g. through [`SystemParam`] validation) before creating
    /// more than one mutable borrow at a time.
    pub unsafe fn new(world: &'w mut World) -> Self {
        Self {
            ptr: NonNull::from(world),
            marker: PhantomData,
        }
    }

    pub fn world(&self) -> &'w World {
        unsafe { self.ptr.as_ref() }
    }

    pub fn resources(&self) -> &'w ResourceStore {
        unsafe { &self.ptr.as_ref().resources }
    }

    pub fn resources_mut(&self) -> &'w mut ResourceStore {
        unsafe { &mut (*self.ptr.as_ptr()).resources }
    }

    pub fn entities(&self) -> &'w EntityStore {
        unsafe { &self.ptr.as_ref().entities }
    }

    pub fn entities_mut(&self) -> &'w mut EntityStore {
        unsafe { &mut (*self.ptr.as_ptr()).entities }
    }
}

/// Immutable access to a singleton resource, borrowed from the world for the
/// lifetime of a system run.
pub struct Res<'w, T>(&'w T);

impl<'w, T> Deref for Res<'w, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

/// Mutable access to a singleton resource, borrowed from the world for the
/// lifetime of a system run.
pub struct ResMut<'w, T>(&'w mut T);

impl<'w, T> Deref for ResMut<'w, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

impl<'w, T> DerefMut for ResMut<'w, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0
    }
}

/// What a single [`SystemParam`] reads or writes: resource type IDs and
/// component type IDs. Used to reject systems whose params would alias.
#[derive(Default, Clone)]
pub struct ParamAccess {
    pub(super) reads_res: Vec<TypeId>,
    pub(super) writes_res: Vec<TypeId>,
    pub(super) reads_comp: Vec<TypeId>,
    pub(super) writes_comp: Vec<TypeId>,
}

/// # Safety
///
/// Implementations must uphold the accesses reported by `access_impls`: the
/// borrows returned by `get` may only touch those resources/components and only
/// with the reported mutability.
pub unsafe trait SystemParam {
    /// The concrete borrow-carrying type handed to the system function.
    type Item<'w>;

    /// The set of accesses this param takes on the world.
    fn access_impls() -> Vec<ParamAccess>;

    /// Gets the param value out of a raw world view.
    ///
    /// # Safety
    ///
    /// The caller must ensure the accesses of this param do not alias any
    /// other live borrow taken from the same cell, and that the cell outlives
    /// the returned item.
    unsafe fn get<'w>(cell: UnsafeWorldCell<'w>) -> Self::Item<'w>;
}

fn query_access<M: Fetch>() -> ParamAccess {
    let mut acc = ParamAccess::default();
    for (tid, write) in M::access() {
        if write {
            acc.writes_comp.push(tid);
        } else {
            acc.reads_comp.push(tid);
        }
    }
    acc
}

fn conflicts(a: &ParamAccess, b: &ParamAccess) -> bool {
    let mut c = false;
    c |= a
        .writes_res
        .iter()
        .any(|t| b.reads_res.contains(t) || b.writes_res.contains(t));
    c |= b
        .writes_res
        .iter()
        .any(|t| a.reads_res.contains(t) || a.writes_res.contains(t));
    c |= a
        .writes_comp
        .iter()
        .any(|t| b.reads_comp.contains(t) || b.writes_comp.contains(t));
    c |= b
        .writes_comp
        .iter()
        .any(|t| a.reads_comp.contains(t) || a.writes_comp.contains(t));
    c
}

fn validate_params(accesses: &[ParamAccess]) {
    for (i, a) in accesses.iter().enumerate() {
        for (j, b) in accesses.iter().enumerate().skip(i + 1) {
            if conflicts(a, b) {
                panic!(
                    "System params {i} and {j} conflict: one mutably accesses the \
                     same resource or the same component that the other reads or \
                     writes. Split them into separate systems."
                );
            }
        }
    }
}

unsafe impl<'w, T: 'static> SystemParam for Res<'w, T> {
    type Item<'new> = Res<'new, T>;

    fn access_impls() -> Vec<ParamAccess> {
        vec![ParamAccess {
            reads_res: vec![TypeId::of::<T>()],
            ..Default::default()
        }]
    }

    unsafe fn get<'new>(cell: UnsafeWorldCell<'new>) -> Self::Item<'new> {
        let ptr = cell.resources().get_raw::<T>().unwrap_or_else(|| {
            panic!(
                "system param `Res<{}>`: resource was not added to the world",
                any::type_name::<T>()
            )
        });
        Res(&*ptr)
    }
}

unsafe impl<'w, T: 'static> SystemParam for ResMut<'w, T> {
    type Item<'new> = ResMut<'new, T>;

    fn access_impls() -> Vec<ParamAccess> {
        vec![ParamAccess {
            writes_res: vec![TypeId::of::<T>()],
            ..Default::default()
        }]
    }

    unsafe fn get<'new>(cell: UnsafeWorldCell<'new>) -> Self::Item<'new> {
        let ptr = cell.resources_mut().get_raw_mut::<T>().unwrap_or_else(|| {
            panic!(
                "system param `ResMut<{}>`: resource was not added to the world",
                any::type_name::<T>()
            )
        });
        ResMut(&mut *ptr)
    }
}

unsafe impl<'w, M: Fetch> SystemParam for Query<'w, M> {
    type Item<'new> = Query<'new, M>;

    fn access_impls() -> Vec<ParamAccess> {
        vec![query_access::<M>()]
    }

    unsafe fn get<'new>(cell: UnsafeWorldCell<'new>) -> Self::Item<'new> {
        Query::new(cell.entities())
    }
}

unsafe impl<'w, M: Fetch> SystemParam for QueryMut<'w, M> {
    type Item<'new> = QueryMut<'new, M>;

    fn access_impls() -> Vec<ParamAccess> {
        vec![query_access::<M>()]
    }

    unsafe fn get<'new>(cell: UnsafeWorldCell<'new>) -> Self::Item<'new> {
        QueryMut::new(cell.entities_mut())
    }
}

unsafe impl SystemParam for CommandQueue {
    type Item<'w> = CommandQueue;

    fn access_impls() -> Vec<ParamAccess> {
        vec![ParamAccess::default()]
    }

    unsafe fn get<'w>(_cell: UnsafeWorldCell<'w>) -> Self::Item<'w> {
        CommandQueue
    }
}

macro_rules! impl_system_param_tuple {
    ($($T:ident),+) => {
        #[allow(unused_parens)]
        unsafe impl<$($T: SystemParam),+> SystemParam for ($($T,)+) {
            type Item<'w> = ($($T::Item<'w>),+ ,);

            fn access_impls() -> Vec<ParamAccess> {
                let mut all = Vec::new();
                $(all.extend($T::access_impls());)+
                all
            }

            unsafe fn get<'w>(cell: UnsafeWorldCell<'w>) -> Self::Item<'w> {
                validate_params(&Self::access_impls());
                let out = (
                    $(unsafe { $T::get(cell) }),+ ,
                );
                out
            }
        }
    };
}

impl_system_param_tuple!(A);
impl_system_param_tuple!(A, B);
impl_system_param_tuple!(A, B, C);
impl_system_param_tuple!(A, B, C, D);
impl_system_param_tuple!(A, B, C, D, E);
impl_system_param_tuple!(A, B, C, D, E, F);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::W;

    #[derive(Debug, PartialEq, Default)]
    struct Time {
        delta: i32,
    }
    #[derive(Debug, PartialEq, Default)]
    struct Score {
        points: i32,
    }
    #[derive(Debug, PartialEq, Default)]
    struct Health {
        hp: i32,
    }

    #[test]
    fn res_and_query_borrow_independently() {
        let mut world = World::new();
        world.add_resource(Time { delta: 3 });
        let e = world.spawn((Health { hp: 10 },));

        {
            let cell = unsafe { UnsafeWorldCell::new(&mut world) };
            let (time, q) = unsafe { <(Res<Time>, QueryMut<W<Health>>) as SystemParam>::get(cell) };
            assert_eq!(time.delta, 3);
            for (_, health) in q {
                health.hp += time.delta;
            }
        }

        assert_eq!(world.get::<Health>(e).unwrap().hp, 13);
    }

    #[test]
    fn two_disjoint_resources_ok() {
        let mut world = World::new();
        world.add_resource(Time { delta: 1 });
        world.add_resource(Score { points: 5 });

        {
            let cell = unsafe { UnsafeWorldCell::new(&mut world) };
            let (time, mut score) =
                unsafe { <(Res<Time>, ResMut<Score>) as SystemParam>::get(cell) };
            score.points += time.delta;
        }

        assert_eq!(world.get_resource::<Score>().points, 6);
    }

    #[test]
    #[should_panic(expected = "conflict")]
    fn two_resmuts_same_resource_panics() {
        let mut world = World::new();
        world.add_resource(Score { points: 0 });

        let cell = unsafe { UnsafeWorldCell::new(&mut world) };
        let _ = unsafe { <(ResMut<Score>, ResMut<Score>) as SystemParam>::get(cell) };
    }

    #[test]
    #[should_panic(expected = "conflict")]
    fn read_write_same_resource_panics() {
        let mut world = World::new();
        world.add_resource(Score { points: 0 });

        let cell = unsafe { UnsafeWorldCell::new(&mut world) };
        let _ = unsafe { <(Res<Score>, ResMut<Score>) as SystemParam>::get(cell) };
    }

    #[test]
    fn commands_param_is_free() {
        let mut world = World::new();
        world.add_resource(Score { points: 0 });
        {
            let cell = unsafe { UnsafeWorldCell::new(&mut world) };
            let (queue, _) = unsafe { <(CommandQueue, Res<Score>) as SystemParam>::get(cell) };
            queue.spawn((Health { hp: 1 },));
        }
        crate::commands::apply_commands(&mut world);
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn missing_resource_panics_with_type_name() {
        let mut world = World::new();
        let cell = unsafe { UnsafeWorldCell::new(&mut world) };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = unsafe { <(Res<Score>,) as SystemParam>::get(cell) };
        }));
        let err = result.unwrap_err();
        let msg = err
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| err.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(msg.contains("Score"), "message was: {msg}");
    }
}
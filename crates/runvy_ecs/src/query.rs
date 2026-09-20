use std::any::TypeId;
use std::marker::PhantomData;

use crate::archetype::Archetype;
use crate::world::World;
use crate::Entity;

pub struct R<T>(PhantomData<T>);
pub struct W<T>(PhantomData<T>);

/// # Safety
///
/// Implementations must return correct type IDs from `type_ids()` and valid
/// references from `extract_const` / `extract_mut`. Invalid pointers or wrong
/// type IDs lead to undefined behavior.
pub unsafe trait Fetch: 'static {
    type Item<'w>;
    fn type_ids() -> Vec<TypeId>;
    unsafe fn extract_const<'w>(ptrs: &[*const u8], row: usize) -> Self::Item<'w>;
    unsafe fn extract_mut<'w>(ptrs: &[*mut u8], row: usize) -> Self::Item<'w>;
}

unsafe impl<T: 'static> Fetch for R<T> {
    type Item<'w> = &'w T;
    fn type_ids() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }
    unsafe fn extract_const<'w>(ptrs: &[*const u8], row: usize) -> &'w T {
        &*((ptrs[0] as *const T).add(row))
    }
    unsafe fn extract_mut<'w>(ptrs: &[*mut u8], row: usize) -> &'w T {
        &*((ptrs[0] as *const T).add(row))
    }
}

unsafe impl<T: 'static> Fetch for W<T> {
    type Item<'w> = &'w mut T;
    fn type_ids() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }
    unsafe fn extract_const<'w>(_ptrs: &[*const u8], _row: usize) -> &'w mut T {
        panic!("W<T> used in immutable query; use query_mut")
    }
    unsafe fn extract_mut<'w>(ptrs: &[*mut u8], row: usize) -> &'w mut T {
        &mut *((ptrs[0] as *mut T).add(row))
    }
}

macro_rules! impl_fetch_tuple {
    ($($T:ident),+) => {
        #[allow(unused_assignments)]
        unsafe impl<$($T: Fetch),+> Fetch for ($($T,)+) {
            type Item<'w> = ($($T::Item<'w>),+);
            fn type_ids() -> Vec<TypeId> {
                let mut ids = Vec::new();
                $(ids.extend($T::type_ids());)+
                ids
            }
            unsafe fn extract_const<'w>(ptrs: &[*const u8], row: usize) -> Self::Item<'w> {
                let mut offset = 0;
                let out = (
                    $({
                        let n = $T::type_ids().len();
                        let item = $T::extract_const(&ptrs[offset..offset + n], row);
                        offset += n;
                        item
                    }),+
                );
                let _ = offset;
                out
            }
            unsafe fn extract_mut<'w>(ptrs: &[*mut u8], row: usize) -> Self::Item<'w> {
                let mut offset = 0;
                let out = (
                    $({
                        let n = $T::type_ids().len();
                        let item = $T::extract_mut(&ptrs[offset..offset + n], row);
                        offset += n;
                        item
                    }),+
                );
                let _ = offset;
                out
            }
        }
    };
}

impl_fetch_tuple!(A, B);
impl_fetch_tuple!(A, B, C);
impl_fetch_tuple!(A, B, C, D);
impl_fetch_tuple!(A, B, C, D, E);
impl_fetch_tuple!(A, B, C, D, E, F);

fn collect_ptrs(arch: &Archetype, type_ids: &[TypeId]) -> Option<Vec<*const u8>> {
    type_ids
        .iter()
        .map(|tid| Some(arch.column(*tid)?.blob.as_ptr()))
        .collect()
}

fn collect_ptrs_mut(arch: &mut Archetype, type_ids: &[TypeId]) -> Option<Vec<*mut u8>> {
    type_ids
        .iter()
        .map(|tid| Some(arch.column_mut(*tid)?.blob.as_mut_ptr()))
        .collect()
}

struct Table<'w> {
    entities: &'w [Entity],
    ptrs: Vec<*const u8>,
    len: usize,
}

struct TableMut<'w> {
    entities: &'w [Entity],
    ptrs: Vec<*mut u8>,
    len: usize,
}

pub struct Query<'w, M> {
    tables: Vec<Table<'w>>,
    table_idx: usize,
    row: usize,
    _marker: PhantomData<&'w M>,
}

impl<'w, M: Fetch> Query<'w, M> {
    pub fn new(world: &'w World) -> Self {
        let type_ids = M::type_ids();
        let mut tables = Vec::new();
        for arch in &world.archetypes {
            if let Some(ptrs) = collect_ptrs(arch, &type_ids) {
                tables.push(Table {
                    entities: &arch.entities,
                    ptrs,
                    len: arch.entity_count(),
                });
            }
        }
        Self {
            tables,
            table_idx: 0,
            row: 0,
            _marker: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }
}

impl<'w, M: Fetch> Iterator for Query<'w, M> {
    type Item = (Entity, M::Item<'w>);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let t = self.tables.get(self.table_idx)?;
            if self.row < t.len {
                let entity = t.entities[self.row];
                let item = unsafe { M::extract_const(&t.ptrs, self.row) };
                self.row += 1;
                return Some((entity, item));
            }
            self.table_idx += 1;
            self.row = 0;
        }
    }
}

pub struct QueryMut<'w, M> {
    tables: Vec<TableMut<'w>>,
    table_idx: usize,
    row: usize,
    _marker: PhantomData<&'w mut M>,
}

impl<'w, M: Fetch> QueryMut<'w, M> {
    pub fn new(world: &'w mut World) -> Self {
        let type_ids = M::type_ids();
        let mut tables = Vec::new();
        for arch in &mut world.archetypes {
            if !arch.has_type(type_ids[0]) {
                continue;
            }
            let Some(ptrs) = collect_ptrs_mut(arch, &type_ids) else {
                continue;
            };
            let entities = &arch.entities as *const Vec<Entity>;
            tables.push(TableMut {
                entities: unsafe { &*entities },
                ptrs,
                len: arch.entity_count(),
            });
        }
        Self {
            tables,
            table_idx: 0,
            row: 0,
            _marker: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }
}

impl<'w, M: Fetch> Iterator for QueryMut<'w, M> {
    type Item = (Entity, M::Item<'w>);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let t = self.tables.get(self.table_idx)?;
            if self.row < t.len {
                let entity = t.entities[self.row];
                let item = unsafe { M::extract_mut(&t.ptrs, self.row) };
                self.row += 1;
                return Some((entity, item));
            }
            self.table_idx += 1;
            self.row = 0;
        }
    }
}

impl World {
    pub fn query<M: Fetch>(&self) -> Query<'_, M> {
        Query::new(self)
    }
    pub fn query_mut<M: Fetch>(&mut self) -> QueryMut<'_, M> {
        QueryMut::new(self)
    }
    pub fn entities_with<T: 'static>(&self) -> Vec<Entity> {
        let tid = TypeId::of::<T>();
        self.archetypes
            .iter()
            .filter(|a| a.has_type(tid))
            .flat_map(|a| a.entities.iter().copied())
            .collect()
    }
}

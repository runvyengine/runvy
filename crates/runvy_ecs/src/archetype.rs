use std::any::TypeId;

use crate::blob_vec::{BlobVec, ComponentInfo};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ArchetypeId(pub u32);

pub struct BlobColumn {
    pub info: ComponentInfo,
    pub blob: BlobVec,
}

impl BlobColumn {
    pub fn new(info: ComponentInfo) -> Self {
        Self {
            blob: BlobVec::new_with_info(info.clone()),
            info,
        }
    }

    pub fn type_id(&self) -> TypeId {
        self.info.type_id
    }
}

pub struct Archetype {
    pub id: ArchetypeId,
    pub entities: Vec<u64>,
    pub columns: Vec<BlobColumn>,
}

impl Archetype {
    pub fn new(id: ArchetypeId, mut columns: Vec<BlobColumn>) -> Self {
        columns.sort_by_key(|c| c.type_id());
        Self {
            id,
            entities: Vec::new(),
            columns,
        }
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn has_type(&self, type_id: TypeId) -> bool {
        self.columns
            .binary_search_by_key(&type_id, |c| c.type_id())
            .is_ok()
    }

    pub fn column(&self, type_id: TypeId) -> Option<&BlobColumn> {
        let idx = self
            .columns
            .binary_search_by_key(&type_id, |c| c.type_id())
            .ok()?;
        Some(&self.columns[idx])
    }

    pub fn column_mut(&mut self, type_id: TypeId) -> Option<&mut BlobColumn> {
        let idx = self
            .columns
            .binary_search_by_key(&type_id, |c| c.type_id())
            .ok()?;
        Some(&mut self.columns[idx])
    }
}

pub trait Bundle: Sized {
    fn type_ids() -> Vec<TypeId>;
    fn component_infos() -> Vec<ComponentInfo>;
    fn put(self, columns: &mut [BlobColumn]);
}

macro_rules! impl_bundle {
    ($($T:ident),+) => {
        #[allow(non_snake_case, unused_assignments)]
        impl<$($T: 'static),+> Bundle for ($($T,)+) {
            fn type_ids() -> Vec<TypeId> {
                vec![$(TypeId::of::<$T>()),+]
            }

            fn component_infos() -> Vec<ComponentInfo> {
                vec![$(ComponentInfo::of::<$T>()),+]
            }

            fn put(self, columns: &mut [BlobColumn]) {
                let ($($T,)+) = self;
                $({
                    let tid = TypeId::of::<$T>();
                    let col = columns
                        .iter_mut()
                        .find(|c| c.type_id() == tid)
                        .expect("Missing column in archetype for bundle component");
                    let ptr = &$T as *const $T as *mut u8;
                    unsafe { col.blob.push(ptr) };
                    std::mem::forget($T);
                })+
            }
        }
    };
}

impl_bundle!(A);
impl_bundle!(A, B);
impl_bundle!(A, B, C);
impl_bundle!(A, B, C, D);
impl_bundle!(A, B, C, D, E);
impl_bundle!(A, B, C, D, E, F);
impl_bundle!(A, B, C, D, E, F, G);
impl_bundle!(A, B, C, D, E, F, G, H);

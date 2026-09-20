use std::any::TypeId;

use crate::blob_vec::{BlobVec, ComponentInfo};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
    pub fn new(id: ArchetypeId, columns: Vec<BlobColumn>) -> Self {
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
        self.columns.iter().any(|c| c.type_id() == type_id)
    }

    pub fn column(&self, type_id: TypeId) -> Option<&BlobColumn> {
        self.columns.iter().find(|c| c.type_id() == type_id)
    }

    pub fn column_mut(&mut self, type_id: TypeId) -> Option<&mut BlobColumn> {
        self.columns.iter_mut().find(|c| c.type_id() == type_id)
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
                let mut idx = 0;
                let ($($T,)+) = self;
                $({
                    let ptr = &$T as *const $T as *mut u8;
                    unsafe { columns[idx].blob.push(ptr) };
                    std::mem::forget($T);
                    idx += 1;
                })+
                let _ = idx;
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

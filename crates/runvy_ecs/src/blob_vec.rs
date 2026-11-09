use std::alloc::{alloc, dealloc, realloc, Layout};
use std::any::TypeId;
use std::mem::needs_drop;
use std::ptr::NonNull;

// ─── ComponentInfo ────────────────────────────────────────────

pub type DropFn = unsafe fn(*mut u8);

#[derive(Clone, Debug)]
pub struct ComponentInfo {
    pub type_id: TypeId,
    pub layout: Layout,
    pub drop_fn: Option<DropFn>,
}

impl ComponentInfo {
    pub fn of<T: 'static>() -> Self {
        let drop_fn = if needs_drop::<T>() {
            Some(drop_as::<T> as DropFn)
        } else {
            None
        };
        Self {
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop_fn,
        }
    }
}

unsafe fn drop_as<T>(ptr: *mut u8) {
    std::ptr::drop_in_place(ptr as *mut T)
}

// ─── BlobVec ──────────────────────────────────────────────────

/// Type-erased column storage that preserves memory layout and alignment.
///
/// Unlike a naive `Vec<u8>` which only guarantees 1-byte alignment, `BlobVec`
/// allocates raw blocks explicitly respecting `ComponentInfo::layout.align()`.
/// This prevents undefined behavior (unaligned pointer dereference) when reading
/// components of types with alignment > 1 (e.g. `f32`, `f64`, `Vec3`, `Mat4`).
pub struct BlobVec {
    info: ComponentInfo,
    len: usize,
    capacity: usize,
    data: NonNull<u8>,
}

unsafe impl Send for BlobVec {}
unsafe impl Sync for BlobVec {}

impl BlobVec {
    /// # Safety
    ///
    /// `T` must match the component type that was originally stored in this column.
    /// Misusing the type parameter leads to undefined behavior.
    pub unsafe fn new<T: 'static>() -> Self {
        Self::new_with_info(ComponentInfo::of::<T>())
    }

    pub fn new_with_info(info: ComponentInfo) -> Self {
        let align = info.layout.align();
        // For zero-sized types or zero-capacity state, use a non-null properly aligned dangling pointer.
        let data = NonNull::new(align as *mut u8).unwrap_or(NonNull::dangling());
        Self {
            capacity: if info.layout.size() == 0 {
                usize::MAX
            } else {
                0
            },
            len: 0,
            info,
            data,
        }
    }

    pub fn info(&self) -> &ComponentInfo {
        &self.info
    }

    pub fn get(&self, index: usize) -> Option<*mut u8> {
        if index >= self.len {
            return None;
        }
        let size = self.info.layout.size();
        unsafe { Some(self.data.as_ptr().add(index * size)) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_ptr()
    }

    /// # Safety
    ///
    /// `value` must point to a valid initialised value of the component type
    /// stored in this column. The value is copied into storage.
    pub unsafe fn push(&mut self, value: *mut u8) {
        let size = self.info.layout.size();
        if size == 0 {
            self.len += 1;
            return;
        }

        if self.len == self.capacity {
            self.grow();
        }

        let dst = self.data.as_ptr().add(self.len * size);
        std::ptr::copy_nonoverlapping(value, dst, size);
        self.len += 1;
    }

    fn grow(&mut self) {
        let size = self.info.layout.size();
        let align = self.info.layout.align();
        let new_capacity = if self.capacity == 0 {
            4
        } else {
            self.capacity * 2
        };

        let new_layout = Layout::from_size_align(new_capacity * size, align)
            .expect("Failed to create layout for BlobVec capacity");

        let new_ptr = if self.capacity == 0 {
            unsafe { alloc(new_layout) }
        } else {
            let old_layout = Layout::from_size_align(self.capacity * size, align)
                .expect("Failed to create old layout for BlobVec");
            unsafe { realloc(self.data.as_ptr(), old_layout, new_capacity * size) }
        };

        let non_null = match NonNull::new(new_ptr) {
            Some(p) => p,
            None => std::alloc::handle_alloc_error(new_layout),
        };

        self.data = non_null;
        self.capacity = new_capacity;
    }

    /// # Safety
    ///
    /// `index` must be in-bounds. After removal the last element is moved into
    /// `index`'s slot, so any existing reference into slot `index` is invalidated.
    pub unsafe fn swap_remove(&mut self, index: usize) {
        assert!(
            index < self.len,
            "Index out of bounds in BlobVec::swap_remove"
        );
        let size = self.info.layout.size();

        if let Some(drop) = self.info.drop_fn {
            let target = self.data.as_ptr().add(index * size);
            drop(target);
        }

        if size > 0 && index != self.len - 1 {
            let last = self.data.as_ptr().add((self.len - 1) * size);
            let dst = self.data.as_ptr().add(index * size);
            std::ptr::copy_nonoverlapping(last, dst, size);
        }

        self.len -= 1;
    }

    /// Like [`swap_remove`](Self::swap_remove) but does **not** call the component's
    /// `Drop`. Used when the element at `index` was already moved out (e.g. returned
    /// to the caller) or copied into another column, so dropping it here would be a
    /// double-free.
    ///
    /// # Safety
    ///
    /// `index` must be in-bounds and the value at `index` must no longer be owned by
    /// this column (moved out or duplicated elsewhere).
    pub unsafe fn swap_remove_no_drop(&mut self, index: usize) {
        assert!(
            index < self.len,
            "Index out of bounds in BlobVec::swap_remove_no_drop"
        );
        let size = self.info.layout.size();

        if size > 0 && index != self.len - 1 {
            let last = self.data.as_ptr().add((self.len - 1) * size);
            let dst = self.data.as_ptr().add(index * size);
            std::ptr::copy_nonoverlapping(last, dst, size);
        }

        self.len -= 1;
    }

    pub fn clear(&mut self) {
        if let Some(drop) = self.info.drop_fn {
            let size = self.info.layout.size();
            for i in 0..self.len {
                unsafe {
                    drop(self.data.as_ptr().add(i * size));
                }
            }
        }
        self.len = 0;
    }
}

impl Drop for BlobVec {
    fn drop(&mut self) {
        self.clear();
        let size = self.info.layout.size();
        if size > 0 && self.capacity > 0 {
            let layout = Layout::from_size_align(self.capacity * size, self.info.layout.align())
                .expect("Failed to create layout for BlobVec deallocation");
            unsafe {
                dealloc(self.data.as_ptr(), layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[repr(align(16))]
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Aligned16 {
        data: [u8; 16],
    }

    #[test]
    fn test_alignment_guarantee() {
        let mut bv = unsafe { BlobVec::new::<Aligned16>() };
        assert_eq!(bv.info().layout.align(), 16);

        for i in 0..20 {
            let mut val = Aligned16 {
                data: [i as u8; 16],
            };
            unsafe { bv.push(&mut val as *mut Aligned16 as *mut u8) };
        }

        assert_eq!(bv.len(), 20);
        for i in 0..20 {
            let ptr = bv.get(i).unwrap();
            // Address MUST be aligned to 16 bytes!
            assert_eq!(
                ptr as usize % 16,
                0,
                "Pointer {ptr:?} is not 16-byte aligned"
            );
            let read = unsafe { *(ptr as *const Aligned16) };
            assert_eq!(read.data[0], i as u8);
        }
    }

    #[derive(Debug)]
    struct DropTracker(Arc<AtomicUsize>);
    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_drop_on_swap_remove_and_clear() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut bv = unsafe { BlobVec::new::<DropTracker>() };

        for _ in 0..5 {
            let mut item = DropTracker(counter.clone());
            unsafe { bv.push(&mut item as *mut DropTracker as *mut u8) };
            std::mem::forget(item);
        }

        assert_eq!(counter.load(Ordering::SeqCst), 0);

        // Remove element 1
        unsafe { bv.swap_remove(1) };
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(bv.len(), 4);

        // Clear remaining 4 elements
        bv.clear();
        assert_eq!(counter.load(Ordering::SeqCst), 5);
        assert_eq!(bv.len(), 0);
    }

    struct ZstMarker;

    #[test]
    fn test_zero_sized_types() {
        let mut bv = unsafe { BlobVec::new::<ZstMarker>() };
        assert_eq!(bv.info().layout.size(), 0);

        for _ in 0..100 {
            let mut item = ZstMarker;
            unsafe { bv.push(&mut item as *mut ZstMarker as *mut u8) };
        }

        assert_eq!(bv.len(), 100);
        unsafe { bv.swap_remove(50) };
        assert_eq!(bv.len(), 99);
        bv.clear();
        assert_eq!(bv.len(), 0);
    }
}

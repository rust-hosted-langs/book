/// A memory arena implemented as an ever growing pool of blocks.
/// Currently implemented on top of stickyimmix without any gc which includes unnecessary
/// overhead.
use std::ptr::NonNull;

use stickyimmix::{
    AllocError, AllocHeader, AllocObject, AllocRaw, ArraySize, Mark, RawPtr, SizeClass,
    StickyImmixHeap,
};

use crate::headers::TypeList;

/// Allocation header for an Arena-allocated value
pub struct ArenaHeader {}

/// Since we're not using this functionality in an Arena, the impl is just
/// a set of no-ops.
impl AllocHeader for ArenaHeader {
    type TypeId = TypeList;

    fn new<O: AllocObject<Self::TypeId>>(
        _size: u32,
        _size_class: SizeClass,
        _mark: Mark,
    ) -> ArenaHeader {
        ArenaHeader {}
    }

    fn new_array(_size: ArraySize, _size_class: SizeClass, _mark: Mark) -> ArenaHeader {
        ArenaHeader {}
    }

    fn mark(&mut self) {}

    fn is_marked(&self) -> bool {
        true
    }

    fn size_class(&self) -> SizeClass {
        SizeClass::Small
    }

    fn size(&self) -> u32 {
        1
    }

    fn type_id(&self) -> TypeList {
        TypeList::Symbol
    }
}

/// A non-garbage-collected pool of memory blocks for interned values.
/// These values are not dropped on Arena deallocation.
/// Values must be "atomic", that is, not composed of other object
/// pointers that need to be traced.
// ANCHOR: DefArena
pub struct Arena {
    heap: StickyImmixHeap<ArenaHeader>,
}
// ANCHOR_END: DefArena

impl Arena {
    pub fn new() -> Arena {
        Arena {
            heap: StickyImmixHeap::new(),
        }
    }
}

impl AllocRaw for Arena {
    type Header = ArenaHeader;

    // ANCHOR: DefArenaAlloc
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>
    where
        T: AllocObject<TypeList>,
    {
        self.heap.alloc(object)
    }
    // ANCHOR_END: DefArenaAlloc

    fn alloc_array(&self, _size_bytes: ArraySize) -> Result<RawPtr<u8>, AllocError> {
        unimplemented!()
    }

    fn get_header(_object: NonNull<()>) -> NonNull<Self::Header> {
        unimplemented!()
    }

    fn get_object(_header: NonNull<Self::Header>) -> NonNull<()> {
        unimplemented!()
    }
}

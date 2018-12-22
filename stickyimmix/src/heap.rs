use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::{replace, size_of};
use std::ptr::{write, NonNull};

use crate::allocator::{
    alloc_size_of, AllocError, AllocHeader, AllocObject, AllocRaw, AllocTypeId, Mark, SizeClass,
};
use crate::bumpblock::BumpBlock;
use crate::constants;
use crate::rawptr::RawPtr;

/// A list of blocks as the current block being allocated into and a list
/// of full blocks
struct BlockList {
    head: Option<BumpBlock>,
    overflow: Option<BumpBlock>,
    rest: Vec<BumpBlock>,
    // free: Vec<BumpBlock>,
    // recycle: Vec<BumpBlock>
    // large: Vec<Thing>
}

impl BlockList {
    fn new() -> BlockList {
        BlockList {
            head: None,
            overflow: None,
            rest: Vec::new(),
        }
    }

    /// Allocate a space for a medium object into an overflow block
    fn overflow_alloc(&mut self, alloc_size: usize) -> Result<*const u8, AllocError> {
        assert!(alloc_size <= constants::BLOCK_CAPACITY);

        let space = match self.overflow {
            // We already have an overflow block to try to use...
            Some(ref mut overflow) => {
                // This is a medium object that might fit in the current block...
                match overflow.inner_alloc(alloc_size) {
                    // the block has a suitable hole
                    Some(space) => space,

                    // the block does not have a suitable hole
                    None => {
                        // TODO this just allocates a new block, but should look at
                        // the free block list first
                        let previous = replace(overflow, BumpBlock::new()?);

                        self.rest.push(previous);

                        overflow.inner_alloc(alloc_size).expect("Unexpected error!")
                    }
                }
            }

            // We have no blocks to work with yet so make one
            None => {
                let mut overflow = BumpBlock::new()?;

                // earlier check for object size < block size should
                // mean we dont fail this expectation
                let space = overflow
                    .inner_alloc(alloc_size)
                    .expect("We expected this object to fit!");

                self.overflow = Some(overflow);

                space
            }
        } as *const u8;

        Ok(space)
    }
}

/// A type that implements `AllocRaw` to provide a low-level heap interface.
/// Does not allocate internally on initialization.
pub struct StickyImmixHeap<H> {
    blocks: UnsafeCell<BlockList>,

    _header_type: PhantomData<*const H>,
}

impl<H> StickyImmixHeap<H> {
    pub fn new() -> StickyImmixHeap<H> {
        StickyImmixHeap {
            blocks: UnsafeCell::new(BlockList::new()),
            _header_type: PhantomData,
        }
    }

    // Allocate a space for a small, medium or large object
    fn inner_alloc(
        &self,
        alloc_size: usize,
        size_class: SizeClass,
    ) -> Result<*const u8, AllocError> {
        let blocks = unsafe { &mut *self.blocks.get() };

        // TODO handle large objects
        if size_class == SizeClass::Large {
            // simply fail for objects larger than the block size
            return Err(AllocError::BadRequest);
        }

        let space = match blocks.head {
            // We already have a block to try to use...
            Some(ref mut head) => {
                // If this is a medium object that doesn't fit in the hole, use overflow
                if size_class == SizeClass::Medium && alloc_size > head.current_hole_size() {
                    return blocks.overflow_alloc(alloc_size);
                }

                // This is a small object that might fit in the current block...
                match head.inner_alloc(alloc_size) {
                    // the block has a suitable hole
                    Some(space) => space,

                    // the block does not have a suitable hole
                    None => {
                        // TODO this just allocates a new block, but should look at
                        // recycled blocks first
                        let previous = replace(head, BumpBlock::new()?);

                        blocks.rest.push(previous);

                        head.inner_alloc(alloc_size).expect("Unexpected error!")
                    }
                }
            }

            // We have no blocks to work with yet so make one
            None => {
                let mut head = BumpBlock::new()?;

                // earlier check for object size < block size should
                // mean we dont fail this expectation
                let space = head
                    .inner_alloc(alloc_size)
                    .expect("We expected this object to fit!");

                blocks.head = Some(head);

                space
            }
        } as *const u8;

        Ok(space)
    }
}

impl<H: AllocHeader> AllocRaw for StickyImmixHeap<H> {
    type Header = H;

    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>
    where
        T: AllocObject<<Self::Header as AllocHeader>::TypeId>,
    {
        let header_size = size_of::<Self::Header>();
        let object_size = size_of::<T>();
        let total_size = header_size + object_size;

        // allocated size - round to next word boundary
        let alloc_size = alloc_size_of(total_size);
        let size_class = SizeClass::get_for_size(alloc_size)?;

        // attempt to allocate enough space for the header and the object
        let space = self.inner_alloc(alloc_size, size_class)?;

        // write the header into the allocated space
        let header = Self::Header::new::<T>(object_size as u32, size_class, Mark::Allocated);
        unsafe {
            write(space as *mut Self::Header, header);
        }

        // write the object into the allocated space
        let object_offset = header_size as isize;
        let object_space = unsafe { space.offset(object_offset) };
        unsafe {
            write(object_space as *mut T, object);
        }

        Ok(RawPtr::new(object_space as *const T))
    }

    /// Return the object header for a given object pointer
    fn get_header(object: NonNull<()>) -> NonNull<Self::Header> {
        unsafe {
            NonNull::new_unchecked(
                object
                    .cast::<Self::Header>()
                    .as_ptr()
                    .offset(0 - size_of::<Self::Header>() as isize),
            )
        }
    }

    /// Return the object from it's header address
    fn get_object(header: NonNull<Self::Header>) -> NonNull<()> {
        unsafe {
            NonNull::new_unchecked(
                header
                    .cast::<()>()
                    .as_ptr()
                    .offset(size_of::<Self::Header>() as isize),
            )
        }
    }
}

impl<H> Default for StickyImmixHeap<H> {
    fn default() -> StickyImmixHeap<H> {
        StickyImmixHeap::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::allocator::{AllocObject, Mark, SizeClass};

    struct TestHeader {
        size_class: SizeClass,
        mark: Mark,
        type_id: TestTypeId,
        size_bytes: u32,
    }

    enum TestTypeId {
        Biggish,
        Stringish,
        Usizeish,
    }

    impl AllocTypeId for TestTypeId {}

    impl AllocHeader for TestHeader {
        type TypeId = TestTypeId;

        fn new<O: AllocObject<Self::TypeId>>(size: u32, size_class: SizeClass, mark: Mark) -> Self {
            TestHeader {
                size_class: size_class,
                mark: mark,
                type_id: O::TYPE_ID,
                size_bytes: size,
            }
        }

        fn mark(&mut self) {}

        fn is_marked(&self) -> bool {
            true
        }

        fn size_class(&self) -> SizeClass {
            SizeClass::Small
        }

        fn size(&self) -> u32 {
            8
        }
    }

    struct Big {
        _huge: [u8; constants::BLOCK_SIZE + 1],
    }

    impl Big {
        fn make() -> Big {
            Big {
                _huge: [0u8; constants::BLOCK_SIZE + 1],
            }
        }
    }

    impl AllocObject<TestTypeId> for Big {
        const TYPE_ID: TestTypeId = TestTypeId::Biggish;
    }

    impl AllocObject<TestTypeId> for String {
        const TYPE_ID: TestTypeId = TestTypeId::Stringish;
    }

    impl AllocObject<TestTypeId> for usize {
        const TYPE_ID: TestTypeId = TestTypeId::Usizeish;
    }

    #[test]
    fn test_memory() {
        let mem = StickyImmixHeap::<TestHeader>::new();

        match mem.alloc(String::from("foo")) {
            Ok(s) => {
                let orig = unsafe { s.as_ref() };
                assert!(*orig == String::from("foo"));
            }

            Err(_) => assert!(false, "Allocation failed"),
        }
    }

    #[test]
    fn test_too_big() {
        let mem = StickyImmixHeap::<TestHeader>::new();
        assert!(mem.alloc(Big::make()) == Err(AllocError::BadRequest));
    }

    #[test]
    fn test_many_obs() {
        let mem = StickyImmixHeap::<TestHeader>::new();

        let mut obs = Vec::new();

        // allocate a sequence of numbers
        for i in 0..(constants::BLOCK_SIZE * 3) {
            match mem.alloc(i as usize) {
                Err(_) => assert!(false, "Allocation failed unexpectedly"),
                Ok(ptr) => obs.push(ptr),
            }
        }

        // check that all values of allocated words match the original
        // numbers written, that no heap corruption occurred
        for (i, ob) in obs.iter().enumerate() {
            println!("{} {}", i, unsafe { ob.as_ref() });
            assert!(i == unsafe { *ob.as_ref() })
        }
    }
}

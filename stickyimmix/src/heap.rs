use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::{replace, size_of};
use std::ptr::{write, NonNull};
use std::slice::from_raw_parts_mut;

use crate::allocator::{
    alloc_size_of, AllocError, AllocHeader, AllocObject, AllocRaw, ArraySize, Mark, SizeClass,
};
use crate::bumpblock::BumpBlock;
use crate::constants;
use crate::rawptr::RawPtr;

/// A list of blocks as the current block being allocated into and a list
/// of full blocks
// TODO:
// free: Vec<BumpBlock>,
// recycle: Vec<BumpBlock>
// large: Vec<Thing>
// ANCHOR: DefBlockList
struct BlockList {
    head: Option<BumpBlock>,
    overflow: Option<BumpBlock>,
    rest: Vec<BumpBlock>,
}
// ANCHOR_END: DefBlockList

impl BlockList {
    fn new() -> BlockList {
        BlockList {
            head: None,
            overflow: None,
            rest: Vec::new(),
        }
    }

    /// Allocate a space for a medium object into an overflow block
    // TODO this just allocates a new block on demand, but should look at the free block list first
    // ANCHOR: DefOverflowAlloc
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
    // ANCHOR_END: DefOverflowAlloc
}

/// A type that implements `AllocRaw` to provide a low-level heap interface.
/// Does not allocate internally on initialization.
// ANCHOR: DefStickyImmixHeap
pub struct StickyImmixHeap<H> {
    blocks: UnsafeCell<BlockList>,

    _header_type: PhantomData<*const H>,
}
// ANCHOR_END: DefStickyImmixHeap

impl<H> StickyImmixHeap<H> {
    pub fn new() -> StickyImmixHeap<H> {
        StickyImmixHeap {
            blocks: UnsafeCell::new(BlockList::new()),
            _header_type: PhantomData,
        }
    }

    /// Find a space for a small, medium or large object
    // TODO this just allocates a new block, but should look at
    // recycled blocks first
    fn find_space(
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

    /// Allocate space for object `T`, creating an header for it and writing the object
    /// and the header into the space
    // ANCHOR: DefAlloc
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>
    where
        T: AllocObject<<Self::Header as AllocHeader>::TypeId>,
    {
        // calculate the total size of the object and it's header
        let header_size = size_of::<Self::Header>();
        let object_size = size_of::<T>();
        let total_size = header_size + object_size;

        // round the size to the next word boundary to keep objects aligned and get the size class
        let alloc_size = alloc_size_of(total_size);
        let size_class = SizeClass::get_for_size(alloc_size)?;

        // attempt to allocate enough space for the header and the object
        let space = self.find_space(alloc_size, size_class)?;

        // instantiate an object header for type T, setting the mark bit to "allocated"
        let header = Self::Header::new::<T>(object_size as ArraySize, size_class, Mark::Allocated);

        // write the header into the front of the allocated space
        unsafe {
            write(space as *mut Self::Header, header);
        }

        // write the object into the allocated space after the header
        let object_space = unsafe { space.offset(header_size as isize) };
        unsafe {
            write(object_space as *mut T, object);
        }

        // return a pointer to the object in the allocated space
        Ok(RawPtr::new(object_space as *const T))
    }
    // ANCHOR_END: DefAlloc

    /// Allocate space for an array, creating an header for it, writing the header into the space
    /// and returning a pointer to the array space
    // ANCHOR: DefAllocArray
    fn alloc_array(&self, size_bytes: ArraySize) -> Result<RawPtr<u8>, AllocError> {
        // calculate the total size of the array and it's header
        let header_size = size_of::<Self::Header>();
        let total_size = header_size + size_bytes as usize;

        // round the size to the next word boundary to keep objects aligned and get the size class
        let alloc_size = alloc_size_of(total_size);
        let size_class = SizeClass::get_for_size(alloc_size)?;

        // attempt to allocate enough space for the header and the array
        let space = self.find_space(alloc_size, size_class)?;

        // instantiate an object header for an array, setting the mark bit to "allocated"
        let header = Self::Header::new_array(size_bytes, size_class, Mark::Allocated);

        // write the header into the front of the allocated space
        unsafe {
            write(space as *mut Self::Header, header);
        }

        // calculate where the array will begin after the header
        let array_space = unsafe { space.offset(header_size as isize) };

        // Initialize object_space to zero here.
        // If using the system allocator for any objects (SizeClass::Large, for example),
        // the memory may already be zeroed.
        let array = unsafe { from_raw_parts_mut(array_space as *mut u8, size_bytes as usize) };
        // The compiler should recognize this as optimizable
        for byte in array {
            *byte = 0;
        }

        // return a pointer to the array in the allocated space
        Ok(RawPtr::new(array_space as *const u8))
    }
    // ANCHOR_END: DefAllocArray

    /// Return the object header for a given object pointer
    // ANCHOR: DefGetHeader
    fn get_header(object: NonNull<()>) -> NonNull<Self::Header> {
        unsafe { NonNull::new_unchecked(object.cast::<Self::Header>().as_ptr().offset(-1)) }
    }
    // ANCHOR_END: DefGetHeader

    /// Return the object from it's header address
    // ANCHOR: DefGetObject
    fn get_object(header: NonNull<Self::Header>) -> NonNull<()> {
        unsafe { NonNull::new_unchecked(header.as_ptr().offset(1).cast::<()>()) }
    }
    // ANCHOR_END: DefGetObject
}

impl<H> Default for StickyImmixHeap<H> {
    fn default() -> StickyImmixHeap<H> {
        StickyImmixHeap::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::allocator::{AllocObject, AllocTypeId, Mark, SizeClass};
    use std::slice::from_raw_parts;

    struct TestHeader {
        size_class: SizeClass,
        mark: Mark,
        type_id: TestTypeId,
        size_bytes: u32,
    }

    #[derive(PartialEq, Copy, Clone)]
    enum TestTypeId {
        Biggish,
        Stringish,
        Usizeish,
        Array,
    }

    impl AllocTypeId for TestTypeId {}

    impl AllocHeader for TestHeader {
        type TypeId = TestTypeId;

        fn new<O: AllocObject<Self::TypeId>>(size: u32, size_class: SizeClass, mark: Mark) -> Self {
            TestHeader {
                size_class,
                mark,
                type_id: O::TYPE_ID,
                size_bytes: size,
            }
        }

        fn new_array(size: u32, size_class: SizeClass, mark: Mark) -> Self {
            TestHeader {
                size_class,
                mark,
                type_id: TestTypeId::Array,
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

        fn type_id(&self) -> TestTypeId {
            self.type_id
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

            Err(_) => panic!("Allocation failed"),
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

    #[test]
    fn test_array() {
        let mem = StickyImmixHeap::<TestHeader>::new();

        let size = 2048;

        match mem.alloc_array(size) {
            Err(_) => assert!(false, "Array allocation failed unexpectedly"),

            Ok(ptr) => {
                // Validate that array is zero initialized all the way through
                let ptr = ptr.as_ptr();

                let array = unsafe { from_raw_parts(ptr, size as usize) };

                for byte in array {
                    assert!(*byte == 0);
                }
            }
        }
    }

    #[test]
    fn test_header() {
        let mem = StickyImmixHeap::<TestHeader>::new();

        match mem.alloc(String::from("foo")) {
            Ok(s) => {
                let untyped_ptr = s.as_untyped();
                let header_ptr = StickyImmixHeap::<TestHeader>::get_header(untyped_ptr);
                dbg!(header_ptr);
                let header = unsafe { &*header_ptr.as_ptr() as &TestHeader };

                assert!(header.type_id() == TestTypeId::Stringish);
            }

            Err(_) => panic!("Allocation failed"),
        }
    }
}

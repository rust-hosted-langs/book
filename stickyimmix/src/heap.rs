
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::{replace, size_of};
use std::ptr::write;

use allocator::{AllocError, AllocTypeId, AllocRaw, AllocHeader, alloc_size_of};
use bumpblock::BumpBlock;
use constants;
use rawptr::RawPtr;


/// A list of blocks as the current block being allocated into and a list
/// of full blocks
struct BlockList {
    head: Option<BumpBlock>,
    rest: Vec<BumpBlock>,

    // overflow: Vec<BumpBlock>.
    // free: Vec<BumpBlock>,
    // recycle: Vec<BumpBlock>
    // large: Vec<Thing>
}


impl BlockList {
    fn new() -> BlockList {
        BlockList {
            head: None,
            rest: Vec::new(),
        }
    }
}


/// A type that implements `AllocRaw` to provide a low-level heap interface.
/// Does not allocate internally on initialization.
pub struct StickyImmixHeap<H> {
    blocks: UnsafeCell<BlockList>,

    _header_type: PhantomData<*const H>
}


impl<H> StickyImmixHeap<H> {
    pub fn new() -> StickyImmixHeap<H> {
        StickyImmixHeap {
            blocks: UnsafeCell::new(BlockList::new()),
            _header_type: PhantomData
        }
    }
}


impl<H: AllocHeader> AllocRaw for StickyImmixHeap<H> {
    type Header = H;

    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError> {
        let blocks = unsafe { &mut *self.blocks.get() };

        let header_size = size_of::<Self::Header>();
        let object_size = size_of::<T>();
        let alloc_size = alloc_size_of(header_size + object_size);

        // TODO handle large objects
        if alloc_size > constants::BLOCK_SIZE {
            // simply fail for objects larger than the block size
            return Err(AllocError::BadRequest)
        }

        let space = match blocks.head {
            Some(ref mut head) => {

                if alloc_size > constants::LINE_SIZE && alloc_size > head.current_hole_size() {
                    // TODO use overflow
                }

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
            },

            // Newly created heap, no blocks allocated yet
            None => {
                let mut head = BumpBlock::new()?;

                // earlier check for object size < block size should
                // mean we dont fail this expectation
                let space = head.inner_alloc(alloc_size).expect("Unexpected error!");

                blocks.head = Some(head);

                space
            },
        } as *mut T;

        unsafe { write(space, object); }

        Ok(RawPtr::new(space))
    }

    /// Return the object header for a given object pointer
    fn get_header(_object: *const ()) -> *const Self::Header {
        unimplemented!() // TODO
    }

    /// Return the object from it's header address
    fn get_object(_header: *const Self::Header) -> *const () {
        unimplemented!() // TODO
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
    use allocator::{AllocObject, Mark, SizeClass};

    struct TestHeader;

    struct TestTypeId;
    impl AllocTypeId for TestTypeId {}

    impl AllocHeader for TestHeader {
        type TypeId = TestTypeId;

        fn new<O: AllocObject<Self::TypeId>>(_size: u32, _size_class: SizeClass, _mark: Mark) -> Self {
            TestHeader {}
        }

        fn mark(&mut self) {}

        fn is_marked(&self) -> bool { true }

        fn size_class(&self) -> SizeClass { SizeClass::Small }

        fn size(&self) -> u32 { 8 }
    }


    struct Big {
        _huge: [u8; constants::BLOCK_SIZE + 1]
    }

    impl Big {
        fn make() -> Big {
            Big {
                _huge: [0u8; constants::BLOCK_SIZE + 1]
            }
        }
    }


    #[test]
    fn test_memory() {
        let mem = StickyImmixHeap::<TestHeader>::new();

        match mem.alloc(String::from("foo")) {
            Ok(s) => {
                let orig = unsafe { &*s.get() };
                assert!(*orig == String::from("foo"));
            },

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
            println!("{} {}", i, unsafe { *ob.get() });
            assert!(i == unsafe { *ob.get() })
        }
    }
}


use std::cell::UnsafeCell;
use std::mem::replace;

use allocator::{AllocError, AllocRaw, alloc_size_of};
use block::Block;
use constants;
use rawptr::RawPtr;


/// A list of blocks as the current block being allocated into and a list
/// of full blocks
struct BlockList {
    head: Option<Block>,
    rest: Vec<Block>,

    // overflow: Vec<Block>.
    // free: Vec<Block>,
    // recycle: Vec<Block>
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
struct Heap {
    blocks: UnsafeCell<BlockList>,
}


impl Heap {
    pub fn new() -> Heap {
        Heap {
            blocks: UnsafeCell::new(BlockList::new()),
        }
    }
}


impl AllocRaw for Heap {
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError> {
        let blocks = unsafe { &mut *self.blocks.get() };

        // simply fail for objects larger than the block size
        let alloc_size = alloc_size_of::<T>();

        if alloc_size > constants::BLOCK_SIZE {
            return Err(AllocError::BadRequest)
        }

        match blocks.head {
            Some(ref mut head) => {

                match head.inner_alloc(object, alloc_size) {
                    Ok(ptr) => return Ok(RawPtr::new(ptr)),

                    Err(object) => {
                        let previous = replace(head, Block::new()?);

                        blocks.rest.push(previous);

                        if let Ok(ptr) = head.inner_alloc(object, alloc_size) {
                            return Ok(RawPtr::new(ptr));
                        }
                    }
                }
            },

            None => {
                let mut head = Block::new()?;

                if let Ok(ptr) = head.inner_alloc(object, alloc_size) {
                    blocks.head = Some(head);
                    return Ok(RawPtr::new(ptr))
                }
                // earlier check for object size < block size should
                // mean we dont fall through to here
            },
        }

        Err(AllocError::OOM)
    }
}


impl Default for Heap {
    fn default() -> Heap {
        Heap::new()
    }
}


#[cfg(test)]
mod tests {

    use super::*;

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
        let mem = Heap::new();

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
        let mem = Heap::new();
        assert!(mem.alloc(Big::make()) == Err(AllocError::BadRequest));
    }

    #[test]
    fn test_many_obs() {
        let mem = Heap::new();

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
            assert!(i == unsafe { *ob.get() })
        }
    }
}

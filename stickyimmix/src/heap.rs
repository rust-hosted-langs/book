
use std::cell::UnsafeCell;
use std::mem::replace;
use std::ptr::write;

use blockalloc::{Block as RawBlock,
                 BlockError as RawBlockError};

use allocator::{AllocError, AllocRaw, alloc_size_of};
use constants;
use blockmeta::BlockMeta;
use rawptr::RawPtr;


impl From<RawBlockError> for AllocError {
    fn from(error: RawBlockError) -> AllocError {
        match error {
            RawBlockError::BadRequest => AllocError::BadRequest,
            RawBlockError::OOM => AllocError::OOM,
        }
    }
}


/// A block of heap. This maintains the bump start and limit per block
/// and the mark flags in a separate `meta` struct.  A pointer to the
/// `meta`` struct is placed in the very first word of the block memory
/// to provide fast access when in the object marking phase.
/// Thus allocation in the first line of the block doesn't begin at
/// offset 0 but after this `meta` pointer.
struct Block {
    bump_start: usize,
    bump_limit: usize,
    block: RawBlock,
    meta: Box<BlockMeta>,
}


impl Block {
    /// Create a new block of heap space and it's metadata, placing a
    /// pointer to the metadata in the first word of the block.
    fn new() -> Result<Block, AllocError> {
        let mut block = Block {
            bump_start: alloc_size_of::<usize>(),
            bump_limit: constants::BLOCK_SIZE,
            block: RawBlock::new(constants::BLOCK_SIZE)?,
            meta: BlockMeta::new_boxed(),
        };

        let meta_ptr: *const BlockMeta = &*block.meta;
        unsafe { block.write(meta_ptr, 0) };

        Ok(block)
    }

    /// Write an object into the block at the given offset. The offset is not
    /// checked for overflow, hence this function is unsafe.
    unsafe fn write<T>(&mut self, object: T, offset: usize) -> *mut T {
        let p = self.block.as_ptr().offset(offset as isize) as *mut T;
        write(p, object);
        p
    }

    /// Write an object into the block at the internal bump-allocation offset,
    /// returning the object without allocating it if the result would
    /// overflow the block.
    fn inner_alloc<T>(&mut self, object: T) -> Result<*mut T, T> {
        let size = alloc_size_of::<T>();

        let next_bump = self.bump_start + size;

        if next_bump > self.bump_limit {
            // TODO find an available hole?
            Err(object)
        } else {
            let offset = self.bump_start;
            self.bump_start = next_bump;
            Ok(unsafe { self.write(object, offset) })
        }
    }
}


/// A list of blocks as the current block being allocated into and a list
/// of full blocks
struct BlockList {
    head: Option<Block>,
    rest: Vec<Block>,
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
        let object_size = alloc_size_of::<T>();
        if object_size > constants::BLOCK_SIZE {
            return Err(AllocError::BadRequest)
        }

        match blocks.head {
            Some(ref mut head) => {

                match head.inner_alloc(object) {
                    Ok(ptr) => return Ok(RawPtr::new(ptr)),

                    Err(object) => {
                        let previous = replace(head, Block::new()?);

                        blocks.rest.push(previous);

                        if let Ok(ptr) = head.inner_alloc(object) {
                            return Ok(RawPtr::new(ptr));
                        }
                    }
                }
            },

            None => {
                let mut head = Block::new()?;

                if let Ok(ptr) = head.inner_alloc(object) {
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

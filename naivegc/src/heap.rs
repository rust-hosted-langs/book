
use std::cell::UnsafeCell;
use std::mem::{replace, size_of};
use std::ptr::write;

use blockalloc::{Block, BlockError, BlockSize};


/// An allocation error type
#[derive(Debug, PartialEq)]
enum AllocError {
    /// Some attribute of the allocation, most likely the size requested,
    /// could not be fulfilled
    BadRequest,
    /// Out of memory - allocating the space failed
    OOM,
}


impl From<BlockError> for AllocError {
    fn from(error: BlockError) -> AllocError {
        match error {
            BlockError::BadRequest => AllocError::BadRequest,
            BlockError::OOM => AllocError::OOM,
        }
    }
}


/// A type that describes allocation of an object into a heap space, returning
/// a bare pointer type on success
trait AllocBare {
    fn alloc<T>(&self, object: T) -> Result<*mut T, AllocError>;
}


/// Return the allocated size of an object as it's size_of::<T>() value rounded
/// up to a double-word boundary
fn alloc_size_of<T>() -> usize {
    let align = size_of::<usize>() * 2;
    (size_of::<T>() & !(align - 1)) + align
}



/// A block with it's bump-allocation offset
struct BumpBlock {
    block: Block,
    bump: usize,
}


impl BumpBlock {
    fn new(size: usize) -> Result<BumpBlock, AllocError> {
        Ok(BumpBlock {
            block: Block::new(size)?,
            bump: 0
        })
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

        let next_bump = self.bump + size;

        if next_bump > self.block.size() {
            Err(object)
        } else {
            let offset = self.bump;
            self.bump = next_bump;
            Ok(unsafe { self.write(object, offset) })
        }
    }
}


/// A list of blocks as the current block being allocated into and a list
/// of full blocks
struct BlockList {
    head: Option<BumpBlock>,
    rest: Vec<BumpBlock>,
}


impl BlockList {
    fn new() -> BlockList {
        BlockList {
            head: None,
            rest: Vec::new(),
        }
    }
}


/// A type that implements `AllocBare` to provide a low-level heap interface.
/// Does not allocate internally on initialization.
struct Heap {
    blocks: UnsafeCell<BlockList>,
    block_size: BlockSize,
}


impl Heap {
    pub fn new(block_size: BlockSize) -> Heap {
        Heap {
            blocks: UnsafeCell::new(BlockList::new()),
            block_size: block_size
        }
    }
}


impl AllocBare for Heap {
    fn alloc<T>(&self, object: T) -> Result<*mut T, AllocError> {
        let blocks = unsafe { &mut *self.blocks.get() };

        // simply fail for objects larger than the block size
        let object_size = alloc_size_of::<T>();
        if object_size > self.block_size {
            return Err(AllocError::BadRequest)
        }

        match blocks.head {
            Some(ref mut head) => {

                match head.inner_alloc(object) {
                    Ok(ptr) => return Ok(ptr),

                    Err(object) => {
                        let previous = replace(head, BumpBlock::new(self.block_size)?);

                        blocks.rest.push(previous);

                        if let Ok(ptr) = head.inner_alloc(object) {
                            return Ok(ptr);
                        }
                    }
                }
            },

            None => {
                let mut head = BumpBlock::new(self.block_size)?;

                if let Ok(ptr) = head.inner_alloc(object) {
                    blocks.head = Some(head);
                    return Ok(ptr)
                }
                // earlier check for object size < block size should
                // mean we dont fall through to here
            },
        }

        Err(AllocError::OOM)
    }
}


const DEFAULT_BLOCK_SIZE: usize = 4096 * 8;


impl Default for Heap {
    fn default() -> Heap {
        Heap::new(DEFAULT_BLOCK_SIZE)
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    struct Big {
        _huge: [u8; DEFAULT_BLOCK_SIZE + 1]
    }

    impl Big {
        fn make() -> Big {
            Big {
                _huge: [0u8; DEFAULT_BLOCK_SIZE + 1]
            }
        }
    }


    #[test]
    fn test_memory() {
        let mem = Heap::new(DEFAULT_BLOCK_SIZE);

        match mem.alloc(String::from("foo")) {
            Ok(s) => {
                let orig = unsafe { &*s };
                assert!(*orig == String::from("foo"));
            },

            Err(_) => assert!(false, "Allocation failed"),
        }
    }

    #[test]
    fn test_too_big() {
        let mem = Heap::new(DEFAULT_BLOCK_SIZE);
        assert!(mem.alloc(Big::make()) == Err(AllocError::BadRequest));
    }

    #[test]
    fn test_many_obs() {
        let mem = Heap::new(DEFAULT_BLOCK_SIZE);

        let mut obs = Vec::new();

        // allocate a sequence of numbers
        for i in 0..(DEFAULT_BLOCK_SIZE * 3) {
            match mem.alloc(i as usize) {
                Err(_) => assert!(false, "Allocation failed unexpectedly"),
                Ok(ptr) => obs.push(ptr),
            }
        }

        // check that all values of allocated words match the original
        // numbers written, that no heap corruption occurred
        for (i, ob) in obs.iter().enumerate() {
            assert!(i == unsafe { **ob })
        }
    }
}


use std::ptr::write;

use blockalloc::{Block as RawBlock,
                 BlockError as RawBlockError};

use allocator::{AllocError, alloc_size_of};
use blockmeta::BlockMeta;
use constants;


impl From<RawBlockError> for AllocError {
    fn from(error: RawBlockError) -> AllocError {
        match error {
            RawBlockError::BadRequest => AllocError::BadRequest,
            RawBlockError::OOM => AllocError::OOM,
        }
    }
}


/// A block of heap. This maintains the bump cursor and limit per block
/// and the mark flags in a separate `meta` struct.  A pointer to the
/// `meta` struct is placed in the very first word of the block memory
/// to provide fast access when in the object marking phase.
/// Thus allocation in the first line of the block doesn't begin at
/// offset 0 but after this `meta` pointer.
pub struct Block {
    cursor: usize,
    limit: usize,
    block: RawBlock,
    meta: Box<BlockMeta>,
}


impl Block {
    /// Create a new block of heap space and it's metadata, placing a
    /// pointer to the metadata in the first word of the block.
    pub fn new() -> Result<Block, AllocError> {
        let mut block = Block {
            cursor: alloc_size_of::<usize>(),
            limit: constants::BLOCK_SIZE,
            block: RawBlock::new(constants::BLOCK_SIZE)?,
            meta: BlockMeta::new_boxed(),
        };

        let meta_ptr: *const BlockMeta = &*block.meta;
        unsafe { block.write(meta_ptr, 0) };

        Ok(block)
    }

    /// Write an object into the block at the given offset. The offset is not
    /// checked for overflow, hence this function is unsafe.
    pub unsafe fn write<T>(&mut self, object: T, offset: usize) -> *mut T {
        let p = self.block.as_ptr().offset(offset as isize) as *mut T;
        write(p, object);
        p
    }

    /// Write an object into the block at the internal bump-allocation offset,
    /// returning the object without allocating it if the result would
    /// overflow the block or available holes.
    pub fn inner_alloc<T>(&mut self, object: T, alloc_size: usize) -> Result<*mut T, T> {

        let next_bump = self.cursor + alloc_size;

        if next_bump > self.limit {

            if self.limit < constants::BLOCK_SIZE {
                if let Some((cursor, limit)) = self.meta.find_next_available_hole(self.limit) {
                    self.cursor = cursor;
                    self.limit = limit;
                    return self.inner_alloc(object, alloc_size);
                }
            }

            Err(object)

        } else {
            let offset = self.cursor;
            self.cursor = next_bump;
            Ok(unsafe { self.write(object, offset) })
        }
    }
}

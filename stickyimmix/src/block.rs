
use std::ptr::write;

use blockalloc::{Block as RawBlock,
                 BlockError as RawBlockError};

use allocator::AllocError;
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
            cursor: constants::FIRST_OBJECT_OFFSET,
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
    unsafe fn write<T>(&mut self, object: T, offset: usize) -> *mut T {
        let p = self.block.as_ptr().offset(offset as isize) as *mut T;
        write(p, object);
        p
    }

    /// Find a hole of at least the requested size and return Some(pointer) to it, or
    /// None if this block doesn't have a big enough hole.
    pub fn inner_alloc(&mut self, alloc_size: usize) -> Option<*mut u8> {

        let next_bump = self.cursor + alloc_size;

        if next_bump > self.limit {

            if self.limit < constants::BLOCK_SIZE {
                if let Some((cursor, limit)) = self.meta.find_next_available_hole(self.limit) {
                    self.cursor = cursor;
                    self.limit = limit;
                    return self.inner_alloc(alloc_size);
                }
            }

            None

        } else {
            let offset = self.cursor;
            self.cursor = next_bump;
            unsafe { Some(self.block.as_ptr().offset(offset as isize) as *mut u8) }
        }
    }

    /// Return the size of the hole we're positioned at
    pub fn current_hole_size(&self) -> usize {
        self.limit - self.cursor
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_block() {
        let mut b = Block::new().unwrap();

        b.inner_alloc(4); // TODO
    }
}

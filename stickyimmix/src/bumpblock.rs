
use std::ptr::write;

use blockalloc::{Block, BlockError};

use allocator::AllocError;
use blockmeta::BlockMeta;
use constants;


impl From<BlockError> for AllocError {
    fn from(error: BlockError) -> AllocError {
        match error {
            BlockError::BadRequest => AllocError::BadRequest,
            BlockError::OOM => AllocError::OOM,
        }
    }
}


/// A block of heap. This maintains the bump cursor and limit per block
/// and the mark flags in a separate `meta` struct.  A pointer to the
/// `meta` struct is placed in the very first word of the block memory
/// to provide fast access when in the object marking phase.
/// Thus allocation in the first line of the block doesn't begin at
/// offset 0 but after this `meta` pointer.
pub struct BumpBlock {
    cursor: usize,
    limit: usize,
    block: Block,
    meta: Box<BlockMeta>,
}


impl BumpBlock {
    /// Create a new block of heap space and it's metadata, placing a
    /// pointer to the metadata in the first word of the block.
    pub fn new() -> Result<BumpBlock, AllocError> {
        let mut block = BumpBlock {
            cursor: constants::FIRST_OBJECT_OFFSET,
            limit: constants::BLOCK_SIZE,
            block: Block::new(constants::BLOCK_SIZE)?,
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

    const TEST_UNIT_SIZE: usize = 8;

    // Helper function: given the Block, fill all holes with u32 values
    // and return the number of values allocated.
    // Also assert that all allocated values are unchanged as allocation
    // proceeds.
    fn loop_check_allocate(b: &mut BumpBlock) -> usize {
        let mut v = Vec::new();
        let mut index = 0;

        loop {
            println!("cursor={}, limit={}", b.cursor, b.limit);
            if let Some(ptr) = b.inner_alloc(TEST_UNIT_SIZE) {
                let u32ptr = ptr as *mut u32;

                assert!(!v.contains(&u32ptr));

                v.push(u32ptr);
                unsafe { *u32ptr = index }

                index += 1;
            } else {
                break;
            }
        }

        for (index, u32ptr) in v.iter().enumerate() {
            unsafe {
                assert!(**u32ptr == index as u32);
            }
        }

        index as usize
    }

    #[test]
    fn test_empty_block() {
        let mut b = BumpBlock::new().unwrap();

        let count = loop_check_allocate(&mut b);
        let expect = (constants::BLOCK_SIZE - constants::FIRST_OBJECT_OFFSET) / TEST_UNIT_SIZE;

        println!("expect={}, count={}", expect, count);
        assert!(count == expect);
    }

    #[test]
    fn test_half_block() {
        // This block has an available hole as the second half of the block
        let mut b = BumpBlock::new().unwrap();

        for i in 0..(constants::LINE_COUNT / 2) {
            b.meta.mark_line(i);
        }

        b.limit = b.cursor;  // block is recycled

        let count = loop_check_allocate(&mut b);
        let expect = (((constants::LINE_COUNT / 2) - 1) * constants::LINE_SIZE) / TEST_UNIT_SIZE;

        println!("expect={}, count={}", expect, count);
        assert!(count == expect);
    }

    #[test]
    fn test_conservatively_marked_block() {
        // This block has every other line marked, so the alternate lines are conservatively
        // marked. Nothing should be allocated in this block.

        let mut b = BumpBlock::new().unwrap();

        for i in 0..constants::LINE_COUNT {
            if i % 2 == 0 {
                b.meta.mark_line(i);
            }
        }

        b.limit = b.cursor;  // block is recycled

        let count = loop_check_allocate(&mut b);

        println!("count={}", count);
        assert!(count == 0);
    }
}

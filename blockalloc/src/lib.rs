/// A block allocator for blocks of memory that must be:
///  - powers of two in size
///  - aligned to their size
///
/// Internally this calls the stabilized std Alloc API.
/// https://doc.rust-lang.org/std/alloc/index.html
///
/// Usage:
/// ```
/// extern crate blockalloc;
/// use blockalloc::Block;
///
/// let size = 4096;  // must be a power of 2
/// let block = Block::new(size).unwrap();
/// ```
///
/// Normal scoping rules will call Block::drop() when `block` goes out of scope
/// causing the block to be fully deallocated.
use std::ptr::NonNull;

// ANCHOR: DefBlockComponents
pub type BlockPtr = NonNull<u8>;
pub type BlockSize = usize;
// ANCHOR_END: DefBlockComponents

/// Set of possible block allocation failures
// ANCHOR: DefBlockError
#[derive(Debug, PartialEq)]
pub enum BlockError {
    /// Usually means requested block size, and therefore alignment, wasn't a
    /// power of two
    BadRequest,
    /// Insufficient memory, couldn't allocate a block
    OOM,
}
// ANCHOR_END: DefBlockError

/// A block-size-aligned block of memory
// ANCHOR: DefBlock
pub struct Block {
    ptr: BlockPtr,
    size: BlockSize,
}
// ANCHOR_END: DefBlock

impl Block {
    /// Instantiate a new block of the given size. Size must be a power of two.
    // ANCHOR: BlockNew
    pub fn new(size: BlockSize) -> Result<Block, BlockError> {
        // validate that size is a power of two
        if !(size & (size - 1) == 0) {
            return Err(BlockError::BadRequest);
        }

        Ok(Block {
            ptr: internal::alloc_block(size)?,
            size,
        })
    }
    // ANCHOR_END: BlockNew

    /// Consume and return the pointer only
    pub fn into_mut_ptr(self) -> BlockPtr {
        self.ptr
    }

    /// Return the size in bytes of the block
    pub fn size(&self) -> BlockSize {
        self.size
    }

    /// Unsafely reassemble from pointer and size
    pub unsafe fn from_raw_parts(ptr: BlockPtr, size: BlockSize) -> Block {
        Block { ptr, size }
    }

    /// Return a bare pointer to the base of the block
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        internal::dealloc_block(self.ptr, self.size);
    }
}

/// The set of possible allocation sources
#[derive(Debug, PartialEq)]
pub enum BlockSource {
    RustAlloc,
    PosixMemalign,
    Windows,
}

pub fn block_source() -> BlockSource {
    internal::BLOCK_SOURCE
}

mod internal {
    use super::{BlockError, BlockPtr, BlockSize, BlockSource};
    use std::alloc::{alloc, dealloc, Layout};
    use std::ptr::NonNull;

    pub const BLOCK_SOURCE: BlockSource = BlockSource::RustAlloc;

    // ANCHOR: RustAllocBlock
    pub fn alloc_block(size: BlockSize) -> Result<BlockPtr, BlockError> {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, size);

            let ptr = alloc(layout);
            if ptr.is_null() {
                Err(BlockError::OOM)
            } else {
                Ok(NonNull::new_unchecked(ptr))
            }
        }
    }
    // ANCHOR_END: RustAllocBlock

    // ANCHOR: RustDeallocBlock
    pub fn dealloc_block(ptr: BlockPtr, size: BlockSize) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, size);

            dealloc(ptr.as_ptr(), layout);
        }
    }
    // ANCHOR_END: RustDeallocBlock
}

#[cfg(test)]
mod tests {

    use crate::{block_source, Block, BlockError, BlockSize, BlockSource};

    fn alloc_dealloc(size: BlockSize) -> Result<(), BlockError> {
        let block = Block::new(size)?;

        // ANCHOR: TestAllocPointer
        // the block address bitwise AND the alignment bits (size - 1) should
        // be a mutually exclusive set of bits
        let mask = size - 1;
        assert!((block.ptr.as_ptr() as usize & mask) ^ mask == mask);
        // ANCHOR_END: TestAllocPointer

        drop(block);
        Ok(())
    }

    #[test]
    fn test_bad_sizealign() {
        assert!(alloc_dealloc(999) == Err(BlockError::BadRequest))
    }

    #[test]
    fn test_4k() {
        assert!(alloc_dealloc(4096).is_ok())
    }

    #[test]
    fn test_32k() {
        assert!(alloc_dealloc(32768).is_ok())
    }

    #[test]
    fn test_16m() {
        assert!(alloc_dealloc(16 * 1024 * 1024).is_ok())
    }
}

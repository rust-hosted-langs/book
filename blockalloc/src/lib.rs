#![cfg_attr(feature = "alloc", feature(alloc, allocator_api))]

/// Turn on `--features "unstable"` for use of alloc crate and traits.
/// Otherwise, platform-specific (Unix or Windows) system calls will
/// be used to allocate block-aligned blocks.


use std::ptr::NonNull;


pub type BlockPtr = NonNull<u8>;
pub type BlockSize = usize;


/// Set of possible block allocation failures
#[derive(Debug, PartialEq)]
pub enum BlockError {
    /// Usually means requested block size, and therefore alignment, wasn't a power of two
    BadRequest,
    /// Insufficient memory, couldn't allocate a block
    OOM
}


/// A block-size-aligned block of memory
pub struct Block {
    ptr: BlockPtr,
    size: BlockSize,
}


impl Block {
    /// Instantiate a new block of the given size. Size must be a power of two.
    pub fn new(size: BlockSize) -> Result<Block, BlockError> {
        if !(size > 0 && (size & (size -1) == 0)) {
            return Err(BlockError::BadRequest)
        }

        Ok(Block {
            ptr: internal::alloc_block(size)?,
            size
        })
    }

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
        Block {
            ptr, size
        }
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


#[cfg(feature = "alloc")]
mod internal {

    use std::alloc::{Alloc, Global, Layout};
    use std::ptr::NonNull;
    use {BlockError, BlockPtr, BlockSize, BlockSource};


    pub const BLOCK_SOURCE: BlockSource = BlockSource::RustAlloc;


    pub fn alloc_block(size: BlockSize) -> Result<BlockPtr, BlockError> {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, size);

            match Global.alloc(layout) {
                Ok(ptr) => Ok(NonNull::new_unchecked(ptr.as_ptr() as *mut u8)),
                Err(_) => Err(BlockError::OOM)
            }
        }
    }

    pub fn dealloc_block(ptr: BlockPtr, size: BlockSize) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, size);

            Global.dealloc(ptr, layout);
        }
    }
}


#[cfg(all(unix, not(feature = "alloc")))]
mod internal {
    extern crate libc;

    use self::libc::{c_void, EINVAL, ENOMEM, free, posix_memalign};
    use std::ptr::{NonNull, null_mut};
    use {BlockError, BlockPtr, BlockSize, BlockSource};


    pub const BLOCK_SOURCE: BlockSource = BlockSource::PosixMemalign;


    pub fn alloc_block(size: BlockSize) -> Result<BlockPtr, BlockError> {
        unsafe {
            let mut address = null_mut();
            let rval = posix_memalign(&mut address, size, size);

            match rval {
                0 => Ok(NonNull::new_unchecked(address as *mut u8)),
                EINVAL => Err(BlockError::BadRequest),
                ENOMEM => Err(BlockError::OOM),
                _ => unreachable!()
            }
        }
    }

    pub fn dealloc_block(ptr: BlockPtr, _size: BlockSize) {
        unsafe {
            free(ptr.as_ptr() as *mut c_void);
        }
    }
}


#[cfg(all(windows, not(feature = "alloc")))]
mod internal {
    // maybe? https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/aligned-malloc

    use {Block, BlockError, BlockPtr, BlockSize, BlockSource};


    pub fn alloc_block(size: BlockSize) -> Result<BlockPtr, BlockError> {
        // TODO
    }

    pub fn dealloc_block(ptr: BlockPtr, size: BlockSize) {
        // TODO
    }
}


#[cfg(test)]
mod tests {

    use {Block, BlockError, BlockSize, BlockSource, block_source};

    #[test]
    fn test_block_source() {
        #[cfg(feature = "alloc")]
        assert!(block_source() == BlockSource::RustAlloc);

        #[cfg(all(unix, not(feature = "alloc")))]
        assert!(block_source() == BlockSource::PosixMemalign);

        #[cfg(all(windows, not(feature = "alloc")))]
        assert!(block_source() == BlockSource::Windows);
    }

    fn alloc_dealloc(size: BlockSize) -> Result<(), BlockError> {
        let block = Block::new(size)?;

        // the block address bitwise AND the alignment bits (size - 1) should
        // be a mutually exclusive set of bits
        let mask = size - 1;
        assert!((block.ptr.as_ptr() as usize & mask) ^ mask == mask);

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

#![cfg_attr(feature = "alloc", feature(alloc, allocator_api, global_allocator, heap_api))]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Turn on `--features "unstable"` for use of alloc crate and traits.
/// Otherwise, platform-specific (Unix or Windows) system calls will
/// be used to allocate block-aligned blocks.


pub type BlockPtr = *mut u8;
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

    pub fn new(size: BlockSize) -> Result<Block, BlockError> {
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

    /// Reassemble from pointer and size
    pub unsafe fn from_raw_parts(ptr: BlockPtr, size: BlockSize) -> Block {
        Block {
            ptr, size
        }
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

    use alloc::heap::{Alloc, AllocErr, Global, Layout};
    use std::ptr::NonNull;
    use {BlockError, BlockPtr, BlockSize, BlockSource};


    pub const BLOCK_SOURCE: BlockSource = BlockSource::RustAlloc;


    pub fn alloc_block(size: BlockSize) -> Result<BlockPtr, BlockError> {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, size);

            match Global.alloc(layout) {
                Ok(ptr) => Ok(ptr.as_ptr() as BlockPtr),
/*
                // TODO AllocErr API - how to use? The compiler complains
                // that the enum variants are ambiguous or don't exist
                // https://doc.rust-lang.org/std/heap/enum.AllocErr.html
                Err(AllocErr::Exhausted {..}) {
                    Err(BlockError::OOM)
                }
*/
                Err(_) => {
                     panic!("failed to allocate block!");
                }
            }
        }
    }

    pub fn dealloc_block(ptr: BlockPtr, size: BlockSize) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, size);

            let ptr = NonNull::new_unchecked(ptr as *mut u8).as_opaque();

            Global.dealloc(ptr, layout);
        }
    }
}


#[cfg(all(unix, not(feature = "alloc")))]
mod internal {
    extern crate libc;

    use {BlockError, BlockPtr, BlockSize, BlockSource};
    use self::libc::{c_void, EINVAL, ENOMEM, free, posix_memalign};
    use std::ptr;


    pub const BLOCK_SOURCE: BlockSource = BlockSource::PosixMemalign;


    pub fn alloc_block(size: BlockSize) -> Result<BlockPtr, BlockError> {
        unsafe {
            let mut address = ptr::null_mut();
            let rval = posix_memalign(&mut address, size, size);

            match rval {
                0 => Ok(address as BlockPtr),
                EINVAL => Err(BlockError::BadRequest),
                ENOMEM => Err(BlockError::OOM),
                _ => unreachable!()
            }
        }
    }

    pub fn dealloc_block(ptr: BlockPtr, _size: BlockSize) {
        unsafe {
            free(ptr as *mut c_void);
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
        assert!((block.ptr as usize & mask) ^ mask == mask);

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

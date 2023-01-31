use std::mem::size_of;

// ANCHOR: ConstBlockSize
pub const BLOCK_SIZE_BITS: usize = 15;
pub const BLOCK_SIZE: usize = 1 << BLOCK_SIZE_BITS;
// ANCHOR_END: ConstBlockSize
pub const BLOCK_PTR_MASK: usize = !(BLOCK_SIZE - 1);

// ANCHOR: ConstLineSize
pub const LINE_SIZE_BITS: usize = 7;
pub const LINE_SIZE: usize = 1 << LINE_SIZE_BITS;

// How many total lines are in a block
pub const LINE_COUNT: usize = BLOCK_SIZE / LINE_SIZE;

// We need LINE_COUNT number of bytes for marking lines, so the capacity of a block
// is reduced by that number of bytes.
pub const BLOCK_CAPACITY: usize = BLOCK_SIZE - LINE_COUNT;

// The first line-mark offset into the block is here.
pub const LINE_MARK_START: usize = BLOCK_CAPACITY;
// How many bytes do we actually need for marking lines, given that the last bytes of
// the block are not for objects but for marking lines. This _should_ divide cleanly.
pub const LINE_MARK_COUNT: usize = BLOCK_CAPACITY / LINE_SIZE;
// ANCHOR_END: ConstLineSize

pub const OBJECT_LIMIT: usize = 0;

pub const MAX_ALLOC_SIZE: usize = std::u32::MAX as usize;
pub const ALLOC_ALIGN_MASK: usize = !(size_of::<usize>() - 1);

/// Object size ranges
pub const SMALL_OBJECT_MIN: usize = 1;
pub const SMALL_OBJECT_MAX: usize = LINE_SIZE;
pub const MEDIUM_OBJECT_MIN: usize = SMALL_OBJECT_MAX + 1;
pub const MEDIUM_OBJECT_MAX: usize = BLOCK_CAPACITY;
pub const LARGE_OBJECT_MIN: usize = MEDIUM_OBJECT_MAX + 1;
pub const LARGE_OBJECT_MAX: usize = MAX_ALLOC_SIZE;

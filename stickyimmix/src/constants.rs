use std::mem::size_of;

pub const BLOCK_SIZE_BITS: usize = 15;
pub const BLOCK_SIZE: usize = 1 << BLOCK_SIZE_BITS;
pub const BLOCK_PTR_MASK: usize = !(BLOCK_SIZE - 1);

pub const LINE_SIZE_BITS: usize = 7;
pub const LINE_SIZE: usize = 1 << LINE_SIZE_BITS;
pub const LINE_COUNT: usize = BLOCK_SIZE / LINE_SIZE;

pub const MAX_ALLOC_SIZE: usize = std::u32::MAX as usize;

/// The first object in a block is not at offset 0 - that location is reserved
/// for a pointer to the BlockMeta struct for the Block - but at the next
/// double-word offset.
pub const FIRST_OBJECT_OFFSET: usize = size_of::<usize>() * 2;
pub const BLOCK_CAPACITY: usize = BLOCK_SIZE - FIRST_OBJECT_OFFSET;

/// Object size ranges
pub const SMALL_OBJECT_MIN: usize = 1;
pub const SMALL_OBJECT_MAX: usize = LINE_SIZE;
pub const MEDIUM_OBJECT_MIN: usize = SMALL_OBJECT_MAX + 1;
pub const MEDIUM_OBJECT_MAX: usize = BLOCK_CAPACITY;
pub const LARGE_OBJECT_MIN: usize = MEDIUM_OBJECT_MAX + 1;
pub const LARGE_OBJECT_MAX: usize = MAX_ALLOC_SIZE;

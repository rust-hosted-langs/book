
use std::mem::size_of;

use object::Header;


pub const BLOCK_SIZE_BITS: usize = 15;
pub const BLOCK_SIZE: usize = 1 << BLOCK_SIZE_BITS;
pub const BLOCK_PTR_MASK: usize = !(BLOCK_SIZE - 1);

pub const LINE_SIZE_BITS: usize = 7;
pub const LINE_SIZE: usize = 1 << LINE_SIZE_BITS;
pub const LINE_COUNT: usize = BLOCK_SIZE / LINE_SIZE;

pub const OBJECT_HEADER_SIZE: usize = size_of::<Header>();


use std::mem::size_of;

use rawptr::RawPtr;


/// An allocation error type
#[derive(Debug, PartialEq)]
pub enum AllocError {
    /// Some attribute of the allocation, most likely the size requested,
    /// could not be fulfilled or a block line size is not a divisor of
    /// the requested block size
    BadRequest,
    /// Out of memory - allocating the space failed
    OOM,
}


/// A type that describes allocation of an object into a heap space, returning
/// a bare pointer type on success
pub trait AllocRaw {
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>;
}


/// Return the allocated size of an object as it's size_of::<T>() value rounded
/// up to a double-word boundary
pub fn alloc_size_of<T>() -> usize {
    let align = size_of::<usize>() * 2;
    (size_of::<T>() & !(align - 1)) + align
}


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
    type Header: AllocHeader;

    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>;

    fn get_header(*const ()) -> Self::Header;
}


/// Object size class.
/// - Small objects fit inside a line
/// - Medium objects span more than one line
/// - Large objects span multiple blocks
#[repr(u8)]
pub enum SizeClass {
    Small,
    Medium,
    Large,
}


/// TODO Object mark bit.
/// Every object is `Allocated` on creation.
#[repr(u8)]
pub enum Mark {
    Allocated,
    Black,
    White,
}


/// An allocation header struct must provide an implementation of this trait,
/// providing appropriate information to the garbage collector.
pub trait AllocHeader {
    /// Initialize a new header with the given attributes and return it
    fn new(size_class: SizeClass, mark_bit: Mark) -> Self;

    /// Set the Mark value to "marked"
    fn mark(&mut self);

    /// Get the current Mark value
    fn is_marked(&self) -> bool;

    /// Get the size class of the object
    fn size_class(&self) -> SizeClass;

    // TODO tracing information
    // e.g. fn tracer(&self) -> Fn()
}


/// Return the allocated size of an object as it's size_of::<T>() value rounded
/// up to a double-word boundary
pub fn alloc_size_of<T>() -> usize {
    let align = size_of::<usize>() * 2;
    (size_of::<T>() & !(align - 1)) + align
}

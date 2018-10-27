
use std::mem::size_of;

use rawptr::RawPtr;


/// An allocation error type
#[derive(Copy, Clone, Debug, PartialEq)]
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
    /// An implementation of an object header type
    type Header: AllocHeader;

    /// Allocate a single object of type T
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>;

    /// Given a bare pointer to an object, return the expected header address
    fn get_header(*const ()) -> *const Self::Header;

    /// Given a bare pointer to an object's header, return the expected object address
    fn get_object(header: *const Self::Header) -> *const ();
}


/// Object size class.
/// - Small objects fit inside a line
/// - Medium objects span more than one line
/// - Large objects span multiple blocks
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SizeClass {
    Small,
    Medium,
    Large,
}


/// TODO Object mark bit.
/// Every object is `Allocated` on creation.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Mark {
    Allocated,
    Unmarked,
    Marked,
}


/// A managed-type type-identifier type should implement this!
pub trait AllocTypeId {}


/// All managed object types must implement this trait in order to be allocatable
pub trait AllocObject<T: AllocTypeId> {
   const TYPE_ID: T;
}


/// An object header struct must provide an implementation of this trait,
/// providing appropriate information to the garbage collector.
pub trait AllocHeader {
    /// Associated type that identifies the allocated object type
    type TypeId: AllocTypeId;

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

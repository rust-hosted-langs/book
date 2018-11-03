
use std::mem::size_of;

use constants;
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
    //fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>;
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>
        where T: AllocObject<<Self::Header as AllocHeader>::TypeId>;

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


impl SizeClass {
    pub fn get_for_size(object_size: usize) -> Result<SizeClass, AllocError> {
        match object_size {
            0...constants::LINE_SIZE => Ok(SizeClass::Small),
            constants::LINE_SIZE...constants::BLOCK_CAPACITY => Ok(SizeClass::Medium),
            constants::BLOCK_CAPACITY...constants::MAX_ALLOC_SIZE => Ok(SizeClass::Large),
            _ => Err(AllocError::BadRequest)
        }
    }
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

    /// Create a new header
    fn new<O: AllocObject<Self::TypeId>>(size: u32, size_class: SizeClass, mark: Mark) -> Self;

    /// Set the Mark value to "marked"
    fn mark(&mut self);

    /// Get the current Mark value
    fn is_marked(&self) -> bool;

    /// Get the size class of the object
    fn size_class(&self) -> SizeClass;

    /// Get the size of the object in bytes
    fn size(&self) -> u32;

    // TODO tracing information
    // e.g. fn tracer(&self) -> Fn()
}


/// Return the allocated size of an object as it's size_of::<T>() value rounded
/// up to a double-word boundary
///
/// TODO this isn't currently implemented, as aligning the object to a double-word
/// boundary while considering header size (which is not known to this libarary
/// until compile time) means touching numerous bump-allocation code points with
/// some math and bitwise ops I haven't worked out yet
pub fn alloc_size_of(object_size: usize) -> usize {
    let align = size_of::<usize>(); // * 2;
    (object_size + (align - 1)) & !(align - 1)
}

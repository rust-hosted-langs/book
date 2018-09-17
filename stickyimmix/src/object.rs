/// NOT YET

/// The garbage collection mark bit
#[repr(u8)]
pub enum Mark {
    Free,
    Allocated,
    Marked,
}


/// Object size class.
/// - Small objects fit inside a line
/// - Medium objects span more than one line
/// - Large objects span multiple blocks
#[repr(u8)]
pub enum SizeClass {
    Small,
    Medium,
    Large
}


/// Providing a type identification shorthand
#[repr(u16)]
pub enum TypeId {
    Symbol,
}


#[repr(C)]
pub struct Header {
    type_id: TypeId,
    sz_class: SizeClass,
    mark: Mark,
    size: u32
}


#[repr(C)]
pub struct HeapObject<T> {
    header: Header,
    object: T
}

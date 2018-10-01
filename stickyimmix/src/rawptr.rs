

/// A container for a bare pointer to an object of type `T`.
/// At this level, compile-time type information is still
/// part of the type.
pub struct RawPtr<T: Sized> {
    ptr: *const T
}


impl<T: Sized> RawPtr<T> {
    /// Create a new RawPtr from a bare pointer
    pub fn new(ptr: *const T) -> RawPtr<T> {
        RawPtr {
            ptr
        }
    }

    pub fn get(&self) -> *const T {
        self.ptr
    }

    pub fn get_mut(&mut self) -> *mut T {
        self.ptr as *mut T
    }
}


impl<T> Clone for RawPtr<T> {
    fn clone(&self) -> RawPtr<T> {
        RawPtr {
            ptr: self.ptr
        }
    }
}


impl<T> Copy for RawPtr<T> {}


impl<T: Sized> PartialEq for RawPtr<T> {
    fn eq(&self, other: &RawPtr<T>) -> bool {
        self.ptr == other.ptr
    }
}

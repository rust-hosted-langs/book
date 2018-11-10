

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

    /// Get a `*const` copy of the bare pointer
    pub fn get(&self) -> *const T {
        self.ptr
    }

    /// Get a `*mut` copy of the bare pointer
    pub fn get_mut(&mut self) -> *mut T {
        self.ptr as *mut T
    }

    /// Get a `&` reference to the object. Unsafe because there are no guarantees at this level
    /// about the internal pointer's validity.
    pub unsafe fn as_ref(&self) -> &T {
        &*self.get() as &T
    }

    /// Get a `&mut` reference to the object. Unsafe because there are no guarantees at this level
    /// about the internal pointer's validity.
    /// In addition, there can be no compile-time guarantees of mutable aliasing prevention.
    /// Use with caution!
    pub unsafe fn as_mut_ref(&mut self) -> &mut T {
        &mut *(self.get_mut()) as &mut T
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

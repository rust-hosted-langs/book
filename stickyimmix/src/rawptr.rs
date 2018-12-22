use std::ptr::NonNull;

/// A container for a bare pointer to an object of type `T`.
/// At this level, compile-time type information is still
/// part of the type.
pub struct RawPtr<T: Sized> {
    ptr: NonNull<T>,
}

impl<T: Sized> RawPtr<T> {
    /// Create a new RawPtr from a bare pointer
    pub fn new(ptr: *const T) -> RawPtr<T> {
        RawPtr {
            ptr: unsafe { NonNull::new_unchecked(ptr as *mut T) },
        }
    }

    /// Get the pointer value as a word-sized integer
    pub fn as_word(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    pub fn as_untyped(&self) -> NonNull<()> {
        self.ptr.cast()
    }

    /// Get a `&` reference to the object. Unsafe because there are no guarantees at this level
    /// about the internal pointer's validity.
    pub unsafe fn as_ref(&self) -> &T {
        self.ptr.as_ref()
    }

    /// Get a `&mut` reference to the object. Unsafe because there are no guarantees at this level
    /// about the internal pointer's validity.
    /// In addition, there can be no compile-time guarantees of mutable aliasing prevention.
    /// Use with caution!
    pub unsafe fn as_mut_ref(&mut self) -> &mut T {
        self.ptr.as_mut()
    }
}

impl<T> Clone for RawPtr<T> {
    fn clone(&self) -> RawPtr<T> {
        RawPtr { ptr: self.ptr }
    }
}

impl<T> Copy for RawPtr<T> {}

impl<T: Sized> PartialEq for RawPtr<T> {
    fn eq(&self, other: &RawPtr<T>) -> bool {
        self.ptr == other.ptr
    }
}

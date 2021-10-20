use std::mem::size_of;
use std::ptr::NonNull;
use std::slice::from_raw_parts_mut;

pub use stickyimmix::ArraySize;

use crate::error::{ErrorKind, RuntimeError};
use crate::memory::MutatorView;

/// Arrays start out at this size by default
pub const DEFAULT_ARRAY_SIZE: ArraySize = 8;

/// Arrays grow at this rate by default
pub fn default_array_growth(capacity: ArraySize) -> Result<ArraySize, RuntimeError> {
    if capacity == 0 {
        Ok(DEFAULT_ARRAY_SIZE)
    } else {
        capacity
            .checked_add(capacity / 2)
            .ok_or(RuntimeError::new(ErrorKind::BadAllocationRequest))
    }
}

/// Fundamental array type on which other variable-length types are built.
/// Analagous to RawVec.
// ANCHOR: DefRawArray
pub struct RawArray<T: Sized> {
    /// Count of T-sized objects that can fit in the array
    capacity: ArraySize,
    ptr: Option<NonNull<T>>,
}
// ANCHOR_END: DefRawArray

/// Since this base array type needs to be used in an interior-mutable way by the containers
/// built on top of it, the Copy+Clone traits need to be implemented for it so that it can
/// be used in a Cell
impl<T: Sized> Clone for RawArray<T> {
    fn clone(&self) -> Self {
        RawArray {
            capacity: self.capacity,
            ptr: self.ptr,
        }
    }
}

impl<T: Sized> Copy for RawArray<T> {}

impl<T: Sized> RawArray<T> {
    /// Return a RawArray of capacity 0 with no array bytes allocated
    pub fn new() -> RawArray<T> {
        RawArray {
            capacity: 0,
            ptr: None,
        }
    }

    /// Return a RawArray of the given capacity number of bytes allocated
    // ANCHOR: DefRawArrayWithCapacity
    pub fn with_capacity<'scope>(
        mem: &'scope MutatorView,
        capacity: u32,
    ) -> Result<RawArray<T>, RuntimeError> {
        // convert to bytes, checking for possible overflow of ArraySize limit
        let capacity_bytes = capacity
            .checked_mul(size_of::<T>() as ArraySize)
            .ok_or(RuntimeError::new(ErrorKind::BadAllocationRequest))?;

        Ok(RawArray {
            capacity,
            ptr: NonNull::new(mem.alloc_array(capacity_bytes)?.as_ptr() as *mut T),
        })
    }
    // ANCHOR_END: DefRawArrayWithCapacity

    /// Resize the array to the new capacity
    /// TODO the inner implementation of this should live in the allocator API to make
    /// better use of optimizations
    pub fn resize<'scope>(
        &mut self,
        mem: &'scope MutatorView,
        new_capacity: u32,
    ) -> Result<(), RuntimeError> {
        // If we're reducing the capacity to 0, simply detach the array pointer
        if new_capacity == 0 {
            self.capacity = 0;
            self.ptr = None;
            return Ok(());
        }

        match self.ptr {
            // If we have capacity, create new capacity and copy over all bytes from the old
            // to the new array
            Some(old_ptr) => {
                // Convert existing capacity to bytes
                let old_capacity_bytes = size_of::<T>() as ArraySize * self.capacity;
                let old_ptr = old_ptr.as_ptr();

                // Convert new capacity to bytes but check that the number of bytes isn't
                // outside of ArraySize range
                let new_capacity_bytes = new_capacity
                    .checked_mul(size_of::<T>() as ArraySize)
                    .ok_or(RuntimeError::new(ErrorKind::BadAllocationRequest))?;

                let new_ptr = mem.alloc_array(new_capacity_bytes)?.as_ptr() as *mut T;

                // create a pair of slices from the raw pointers and byte sizes
                let (old_slice, new_slice) = unsafe {
                    (
                        from_raw_parts_mut(old_ptr as *mut u8, old_capacity_bytes as usize),
                        from_raw_parts_mut(new_ptr as *mut u8, new_capacity_bytes as usize),
                    )
                };

                // Copy content from old to new array
                for (src, dest) in old_slice.iter().zip(new_slice) {
                    *dest = *src;
                }

                self.ptr = NonNull::new(new_ptr);
                self.capacity = new_capacity;

                Ok(())
            }

            // If we have no capacity, create new blank capacity
            None => {
                *self = Self::with_capacity(mem, new_capacity)?;
                Ok(())
            }
        }
    }

    /// Return the capacity of the array in the count of objects it can hold
    // ANCHOR: DefRawArrayCapacity
    pub fn capacity(&self) -> ArraySize {
        self.capacity
    }
    // ANCHOR_END: DefRawArrayCapacity

    /// Return a pointer to the array
    // ANCHOR: DefRawArrayAsPtr
    pub fn as_ptr(&self) -> Option<*const T> {
        match self.ptr {
            Some(ptr) => Some(ptr.as_ptr()),
            None => None,
        }
    }
    // ANCHOR_END: DefRawArrayAsPtr
}

use std::cell::Cell;
use std::fmt;
use std::ops::Deref;

use stickyimmix::{AllocObject, RawPtr};

use crate::headers::TypeList;
use crate::pointerops::ScopedRef;
use crate::printer::Print;
use crate::taggedptr::{FatPtr, TaggedPtr, Value};

/// Type that provides a generic anchor for mutator timeslice lifetimes
// ANCHOR: DefMutatorScope
pub trait MutatorScope {}
// ANCHOR_END: DefMutatorScope

// Copy On Write semantics? Maybe the below...
// TODO, add MutatorView methods that can return MutScopedPtr?
//
// pub trait CopyOnWrite {
//     fn copy_mut<'guard>(&self, _guard: &'guard MutatorView) -> MutScopedPtr<'guard, Self>;
// }
//
// pub struct MutScopedPtr<'guard, T: Sized> {
//     value: &mut 'guard T
// }
//
// impl Deref, DerefMut for MutScopedPtr
//
// impl<'guard, T: Sized> MutScopedPtr<'guard, T> {
//    pub fn into_immut(self) -> ScopedPtr<'guard, T> {}
// }

/// An untagged compile-time typed pointer with scope limited by `MutatorScope`
// ANCHOR: DefScopedPtr
pub struct ScopedPtr<'guard, T: Sized> {
    value: &'guard T,
}
// ANCHOR_END: DefScopedPtr

impl<'guard, T: Sized> ScopedPtr<'guard, T> {
    pub fn new(_guard: &'guard dyn MutatorScope, value: &'guard T) -> ScopedPtr<'guard, T> {
        ScopedPtr { value }
    }

    /// Convert the compile-time type pointer to a runtime type pointer
    pub fn as_tagged(&self, guard: &'guard dyn MutatorScope) -> TaggedScopedPtr<'guard>
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>,
    {
        TaggedScopedPtr::new(
            guard,
            TaggedPtr::from(FatPtr::from(RawPtr::new(self.value))),
        )
    }
}

/// Anything that _has_ a scope lifetime can pass as a scope representation
impl<'scope, T: Sized> MutatorScope for ScopedPtr<'scope, T> {}

impl<'guard, T: Sized> Clone for ScopedPtr<'guard, T> {
    fn clone(&self) -> ScopedPtr<'guard, T> {
        ScopedPtr { value: self.value }
    }
}

impl<'guard, T: Sized> Copy for ScopedPtr<'guard, T> {}

impl<'guard, T: Sized> Deref for ScopedPtr<'guard, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<'guard, T: Sized + Print> fmt::Display for ScopedPtr<'guard, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.print(self, f)
    }
}

impl<'guard, T: Sized + Print> fmt::Debug for ScopedPtr<'guard, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.print(self, f)
    }
}

impl<'guard, T: Sized + PartialEq> PartialEq for ScopedPtr<'guard, T> {
    fn eq(&self, rhs: &ScopedPtr<'guard, T>) -> bool {
        self.value == rhs.value
    }
}

/// A wrapper around untagged raw pointers for storing compile-time typed pointers in data
/// structures with interior mutability, allowing pointers to be updated to point at different
/// target objects.
#[derive(Clone)]
pub struct CellPtr<T: Sized> {
    inner: Cell<RawPtr<T>>,
}

impl<T: Sized> CellPtr<T> {
    /// Construct a new CellPtr from a ScopedPtr
    pub fn new_with(source: ScopedPtr<T>) -> CellPtr<T> {
        CellPtr {
            inner: Cell::new(RawPtr::new(source.value)),
        }
    }

    pub fn get<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, T> {
        ScopedPtr::new(guard, self.inner.get().scoped_ref(guard))
    }

    // the explicit 'guard lifetime bound to MutatorScope is omitted here since the ScopedPtr
    // carries this lifetime already so we can assume that this operation is safe
    pub fn set(&self, source: ScopedPtr<T>) {
        self.inner.set(RawPtr::new(source.value))
    }
}

impl<T: Sized> From<ScopedPtr<'_, T>> for CellPtr<T> {
    fn from(ptr: ScopedPtr<T>) -> CellPtr<T> {
        CellPtr::new_with(ptr)
    }
}

/// A _tagged_ runtime typed pointer type with scope limited by `MutatorScope` such that a `Value`
/// instance can safely be derived and accessed. This type is neccessary to derive `Value`s from.
#[derive(Copy, Clone)]
pub struct TaggedScopedPtr<'guard> {
    ptr: TaggedPtr,
    value: Value<'guard>,
}

impl<'guard> TaggedScopedPtr<'guard> {
    pub fn new(guard: &'guard dyn MutatorScope, ptr: TaggedPtr) -> TaggedScopedPtr<'guard> {
        TaggedScopedPtr {
            ptr,
            value: FatPtr::from(ptr).as_value(guard),
        }
    }

    pub fn value(&self) -> Value<'guard> {
        self.value
    }

    pub fn get_ptr(&self) -> TaggedPtr {
        self.ptr
    }
}

/// Anything that _has_ a scope lifetime can pass as a scope representation. `Value` also implements
/// `MutatorScope` so this is largely for consistency.
impl<'scope> MutatorScope for TaggedScopedPtr<'scope> {}

impl<'guard> Deref for TaggedScopedPtr<'guard> {
    type Target = Value<'guard>;

    fn deref(&self) -> &Value<'guard> {
        &self.value
    }
}

impl<'guard> fmt::Display for TaggedScopedPtr<'guard> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<'guard> fmt::Debug for TaggedScopedPtr<'guard> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<'guard> PartialEq for TaggedScopedPtr<'guard> {
    fn eq(&self, rhs: &TaggedScopedPtr<'guard>) -> bool {
        self.ptr == rhs.ptr
    }
}

/// A wrapper around the runtime typed `TaggedPtr` for storing pointers in data structures with
/// interior mutability, allowing pointers to be updated to point at different target objects.
#[derive(Clone)]
pub struct TaggedCellPtr {
    inner: Cell<TaggedPtr>,
}

impl TaggedCellPtr {
    /// Construct a new Nil TaggedCellPtr instance
    pub fn new_nil() -> TaggedCellPtr {
        TaggedCellPtr {
            inner: Cell::new(TaggedPtr::nil()),
        }
    }

    /// Construct a new TaggedCellPtr from a TaggedScopedPtr
    pub fn new_with(source: TaggedScopedPtr) -> TaggedCellPtr {
        TaggedCellPtr {
            inner: Cell::new(TaggedPtr::from(source.ptr)),
        }
    }

    pub fn new_ptr(source: TaggedPtr) -> TaggedCellPtr {
        TaggedCellPtr {
            inner: Cell::new(source),
        }
    }

    /// Return the pointer as a `TaggedScopedPtr` type that carries a copy of the `TaggedPtr` and
    /// a `Value` type for both copying and access convenience
    pub fn get<'guard>(&self, guard: &'guard dyn MutatorScope) -> TaggedScopedPtr<'guard> {
        TaggedScopedPtr::new(guard, self.inner.get())
    }

    /// Set this pointer to point at the same object as a given `TaggedScopedPtr` instance
    /// The explicit 'guard lifetime bound to MutatorScope is omitted here since the TaggedScopedPtr
    /// carries this lifetime already so we can assume that this operation is safe
    pub fn set(&self, source: TaggedScopedPtr) {
        self.inner.set(TaggedPtr::from(source.ptr))
    }

    /// Take the pointer of another `TaggedCellPtr` and set this instance to point at that object too
    pub fn copy_from(&self, other: &TaggedCellPtr) {
        self.inner.set(other.inner.get());
    }

    /// Return true if the pointer is nil
    pub fn is_nil(&self) -> bool {
        self.inner.get().is_nil()
    }

    /// Set this pointer to nil
    pub fn set_to_nil(&self) {
        self.inner.set(TaggedPtr::nil())
    }

    /// Set this pointer to another TaggedPtr
    pub fn set_to_ptr(&self, ptr: TaggedPtr) {
        self.inner.set(ptr)
    }

    /// Return the raw TaggedPtr from within
    pub fn get_ptr(&self) -> TaggedPtr {
        self.inner.get()
    }
}

impl From<TaggedScopedPtr<'_>> for TaggedCellPtr {
    fn from(ptr: TaggedScopedPtr) -> TaggedCellPtr {
        TaggedCellPtr::new_with(ptr)
    }
}

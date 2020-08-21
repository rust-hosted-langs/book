/// Container traits
///
/// TODO iterators/views
use stickyimmix::ArraySize;

use crate::error::RuntimeError;
use crate::memory::MutatorView;
use crate::safeptr::{MutatorScope, ScopedPtr, TaggedCellPtr, TaggedScopedPtr};

/// Base container-type trait. All container types are subtypes of `Container`.
///
/// All container operations _must_ follow interior mutability only rules.
/// Because there are no compile-time mutable aliasing guarantees, there can be no references
/// into arrays at all, unless there can be a guarantee that the array memory will not be
/// reallocated.
///
/// `T` cannot be restricted to `Copy` because of the use of `Cell` for interior mutability.
pub trait Container<T: Sized + Clone>: Sized {
    /// Create a new, empty container instance.
    fn new() -> Self;
    /// Create a new container instance with the given capacity.
    // TODO: this may not make sense for tree types
    fn with_capacity<'guard>(
        mem: &'guard MutatorView,
        capacity: ArraySize,
    ) -> Result<Self, RuntimeError>;

    /// Reset the size of the container to zero - empty
    fn clear<'guard>(&self, mem: &'guard MutatorView) -> Result<(), RuntimeError>;

    /// Count of items in the container
    fn length(&self) -> ArraySize;
}

/// If implemented, the container can be filled with a set number of values in one operation
pub trait FillContainer<T: Sized + Clone>: Container<T> {
    /// The `item` is an object to copy into each container memory slot.
    fn fill<'guard>(
        &self,
        mem: &'guard MutatorView,
        size: ArraySize,
        item: T,
    ) -> Result<(), RuntimeError>;
}

/// If implemented, the container can be filled with a set number of values in one operation
pub trait FillAnyContainer: FillContainer<TaggedCellPtr> {
    /// The `item` is an object to copy into each container memory slot.
    fn fill<'guard>(
        &self,
        mem: &'guard MutatorView,
        size: ArraySize,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;
}

/// Generic stack trait. If implemented, the container can function as a stack
// ANCHOR: DefStackContainer
pub trait StackContainer<T: Sized + Clone>: Container<T> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<(), RuntimeError>;

    /// Pop returns a bounds error if the container is empty, otherwise moves the last item of the
    /// array out to the caller.
    fn pop<'guard>(&self, _guard: &'guard dyn MutatorScope) -> Result<T, RuntimeError>;

    /// Return the value at the top of the stack without removing it
    fn top<'guard>(&self, _guard: &'guard dyn MutatorScope) -> Result<T, RuntimeError>;
}
// ANCHOR_END: DefStackContainer

/// Specialized stack trait. If implemented, the container can function as a stack
// ANCHOR: DefStackAnyContainer
pub trait StackAnyContainer: StackContainer<TaggedCellPtr> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(
        &self,
        mem: &'guard MutatorView,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;

    /// Pop returns a bounds error if the container is empty, otherwise moves the last item of the
    /// array out to the caller.
    fn pop<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;

    /// Return the value at the top of the stack without removing it
    fn top<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;
}
// ANCHOR_END: DefStackAnyContainer

/// Generic indexed-access trait. If implemented, the container can function as an indexable vector
pub trait IndexedContainer<T: Sized + Clone>: Container<T> {
    /// Return a copy of the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<T, RuntimeError>;

    /// Move an object into the array at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<(), RuntimeError>;
}

/// A trait that is implemented for containers that can represent their contents as a slice.
pub trait SliceableContainer<T: Sized + Clone>: IndexedContainer<T> {
    /// This function allows access to the interior of a container as a slice by way of a
    /// function, permitting direct access to the memory locations of objects in the container
    /// for the lifetime of the closure call.
    ///
    /// It is important to understand that the 'guard lifetime is not the same safe duration
    /// as the slice lifetime - the slice may be invalidated during the 'guard lifetime
    /// by operations on the container that cause reallocation.
    ///
    /// To prevent the function from modifying the container outside of the slice reference,
    /// the implementing container must maintain a RefCell-style flag to catch runtime
    /// container modifications that would render the slice invalid or cause undefined
    /// behavior.
    fn access_slice<'guard, F, R>(&self, _guard: &'guard dyn MutatorScope, f: F) -> R
    where
        F: FnOnce(&mut [T]) -> R;
}

/// Specialized indexable interface for where TaggedCellPtr is used as T
pub trait IndexedAnyContainer: IndexedContainer<TaggedCellPtr> {
    /// Return a pointer to the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;

    /// Set the object pointer at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;
}

/// Hashable-indexed interface. Objects used as keys must implement Hashable.
pub trait HashIndexedAnyContainer {
    /// Return a pointer to to the object associated with the given key.
    /// Absence of an association should return an error.
    fn lookup<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;

    /// Associate a key with a value.
    fn assoc<'guard>(
        &self,
        mem: &'guard MutatorView,
        key: TaggedScopedPtr<'guard>,
        value: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;

    /// Remove an association by its key.
    fn dissoc<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;

    /// Returns true if the key exists in the container.
    fn exists<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        key: TaggedScopedPtr,
    ) -> Result<bool, RuntimeError>;
}

/// Convert a Pair list to a different container
pub trait AnyContainerFromPairList: Container<TaggedCellPtr> {
    fn from_pair_list<'guard>(
        &self,
        mem: &'guard MutatorView,
        pair_list: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError>;
}

/// Replace the contents of a container with the values in the slice
pub trait ContainerFromSlice<T: Sized + Clone>: Container<T> {
    fn from_slice<'guard>(
        mem: &'guard MutatorView,
        data: &[T],
    ) -> Result<ScopedPtr<'guard, Self>, RuntimeError>;
}

/// Replace the contents of a container with the values in the slice
pub trait AnyContainerFromSlice: Container<TaggedCellPtr> {
    fn from_slice<'guard>(
        mem: &'guard MutatorView,
        data: &[TaggedScopedPtr<'guard>],
    ) -> Result<ScopedPtr<'guard, Self>, RuntimeError>;
}

/// The implementor represents mutable changes via an internal version count
/// such that the use of any references to an older version return an error
pub trait VersionedContainer<T: Sized + Clone>: Container<T> {}

pub trait ImmutableContainer<T: Sized + Clone>: Container<T> {}

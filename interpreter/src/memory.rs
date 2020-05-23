/// VM-level memory abstraction
///
/// Defines Stack, Heap and Memory types, and a MemoryView type that gives a mutator a safe
/// view into the stack and heap.
use stickyimmix::{AllocObject, AllocRaw, ArraySize, RawPtr, StickyImmixHeap};

use crate::error::RuntimeError;
use crate::headers::{ObjectHeader, TypeList};
use crate::pointerops::ScopedRef;
use crate::safeptr::{MutatorScope, ScopedPtr, TaggedScopedPtr};
use crate::symbolmap::SymbolMap;
use crate::taggedptr::{FatPtr, TaggedPtr};

/// This type describes the mutator's view into memory - the heap and symbol name/ptr lookup.
///
/// It implements `MutatorScope` such that any `TaggedScopedPtr` or `Value` instances must be lifetime-
/// limited to the lifetime of this instance using `&'scope dyn MutatorScope`;
pub struct MutatorView<'memory> {
    heap: &'memory Heap,
}

impl<'memory> MutatorView<'memory> {
    fn new(mem: &'memory Memory) -> MutatorView<'memory> {
        MutatorView { heap: &mem.heap }
    }

    /// Get a Symbol pointer from its name
    pub fn lookup_sym(&self, name: &str) -> TaggedScopedPtr<'_> {
        TaggedScopedPtr::new(self, self.heap.lookup_sym(name))
    }

    /// Write an object into the heap and return a scope-limited pointer to it
    pub fn alloc<T>(&self, object: T) -> Result<ScopedPtr<'_, T>, RuntimeError>
    where
        T: AllocObject<TypeList>,
    {
        Ok(ScopedPtr::new(
            self,
            self.heap.alloc(object)?.scoped_ref(self),
        ))
    }

    /// Write an object into the heap and return a scope-limited runtime-tagged pointer to it
    pub fn alloc_tagged<T>(&self, object: T) -> Result<TaggedScopedPtr<'_>, RuntimeError>
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>,
    {
        Ok(TaggedScopedPtr::new(self, self.heap.alloc_tagged(object)?))
    }

    /// Make space for an array of bytes
    pub fn alloc_array(&self, capacity: ArraySize) -> Result<RawPtr<u8>, RuntimeError> {
        self.heap.alloc_array(capacity)
    }

    /// Return a nil-initialized runtime-tagged pointer
    pub fn nil(&self) -> TaggedScopedPtr<'_> {
        TaggedScopedPtr::new(self, TaggedPtr::nil())
    }
}

impl<'memory> MutatorScope for MutatorView<'memory> {}

/// The heap implementation
pub type HeapStorage = StickyImmixHeap<ObjectHeader>;

// Heap memory types.
struct Heap {
    heap: HeapStorage,
    syms: SymbolMap,
}

impl Heap {
    fn new() -> Heap {
        Heap {
            heap: HeapStorage::new(),
            syms: SymbolMap::new(),
        }
    }

    /// Get a Symbol pointer from its name
    fn lookup_sym(&self, name: &str) -> TaggedPtr {
        TaggedPtr::symbol(self.syms.lookup(name))
    }

    /// Write an object to the heap and return the raw pointer to it
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, RuntimeError>
    where
        T: AllocObject<TypeList>,
    {
        Ok(self.heap.alloc(object)?)
    }

    /// Write an object into the heap and return a tagged pointer to it
    fn alloc_tagged<T>(&self, object: T) -> Result<TaggedPtr, RuntimeError>
    where
        FatPtr: From<RawPtr<T>>,
        T: AllocObject<TypeList>,
    {
        Ok(TaggedPtr::from(FatPtr::from(self.heap.alloc(object)?)))
    }

    fn alloc_array(&self, capacity: ArraySize) -> Result<RawPtr<u8>, RuntimeError> {
        Ok(self.heap.alloc_array(capacity)?)
    }
}

/// Wraps a heap and provides scope-limited access to the heap
pub struct Memory {
    heap: Heap,
}

impl Memory {
    /// Instantiate a new memory environment
    pub fn new() -> Memory {
        Memory { heap: Heap::new() }
    }

    /// Run a mutator process
    pub fn mutate<M: Mutator>(&self, m: &M, input: M::Input) -> Result<M::Output, RuntimeError> {
        let mut guard = MutatorView::new(self);
        m.run(&mut guard, input)
    }
}

/// Defines the interface a heap-mutating type must use to be allowed access to the heap
pub trait Mutator: Sized {
    type Input;
    type Output;

    fn run(&self, mem: &MutatorView, input: Self::Input) -> Result<Self::Output, RuntimeError>;
}

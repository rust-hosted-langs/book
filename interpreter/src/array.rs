/// Basic mutable array type:
///
///  Array<T>
///  ArrayU32 = Array<u32>
///  ArrayU16 = Array<u16>
///  ArrayU8 = Array<u8>
use std::cell::Cell;
use std::fmt;
use std::ptr::{read, write};
use std::slice::from_raw_parts_mut;

pub use stickyimmix::{AllocObject, ArraySize};

use crate::containers::{
    AnyContainerFromPairList, AnyContainerFromSlice, Container, ContainerFromSlice,
    FillAnyContainer, FillContainer, IndexedAnyContainer, IndexedContainer, SliceableContainer,
    StackAnyContainer, StackContainer,
};
use crate::error::{ErrorKind, RuntimeError};
use crate::headers::TypeList;
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::rawarray::{default_array_growth, RawArray, DEFAULT_ARRAY_SIZE};
use crate::safeptr::{MutatorScope, ScopedPtr, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

// For a RefCell-style interior mutability pattern
type BorrowFlag = isize;
const INTERIOR_ONLY: isize = 0;
const EXPOSED_MUTABLY: isize = 1;

/// An array, like Vec, but applying an interior mutability pattern.
///
/// Implements Container traits, including SliceableContainer.
/// Since SliceableContainer allows mutable access to the interior
/// of the array, RefCell-style runtime semantics are employed to
/// prevent the array being modified outside of the slice borrow.
// ANCHOR: DefArray
#[derive(Clone)]
pub struct Array<T: Sized + Clone> {
    length: Cell<ArraySize>,
    data: Cell<RawArray<T>>,
    borrow: Cell<BorrowFlag>,
}
// ANCHOR_END: DefArray

/// Internal implementation
impl<T: Sized + Clone> Array<T> {
    /// Allocate a new instance on the heap
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
    ) -> Result<ScopedPtr<'guard, Array<T>>, RuntimeError>
    where
        Array<T>: AllocObject<TypeList>,
    {
        mem.alloc(Array::new())
    }

    /// Clone the contents of an existing Array
    pub fn alloc_clone<'guard>(
        mem: &'guard MutatorView,
        from_array: ScopedPtr<'guard, Array<T>>,
    ) -> Result<ScopedPtr<'guard, Array<T>>, RuntimeError>
    where
        Array<T>: AllocObject<TypeList> + ContainerFromSlice<T>,
    {
        from_array.access_slice(mem, |items| ContainerFromSlice::from_slice(mem, items))
    }

    /// Allocate a new instance on the heap with pre-allocated capacity
    pub fn alloc_with_capacity<'guard>(
        mem: &'guard MutatorView,
        capacity: ArraySize,
    ) -> Result<ScopedPtr<'guard, Array<T>>, RuntimeError>
    where
        Array<T>: AllocObject<TypeList>,
    {
        mem.alloc(Array::with_capacity(mem, capacity)?)
    }

    /// Return a bounds-checked pointer to the object at the given index
    fn get_offset(&self, index: ArraySize) -> Result<*mut T, RuntimeError> {
        if index >= self.length.get() {
            Err(RuntimeError::new(ErrorKind::BoundsError))
        } else {
            let ptr = self
                .data
                .get()
                .as_ptr()
                .ok_or(RuntimeError::new(ErrorKind::BoundsError))?;

            let dest_ptr = unsafe { ptr.offset(index as isize) as *mut T };

            Ok(dest_ptr)
        }
    }

    /// Bounds-checked write
    fn write<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<&T, RuntimeError> {
        unsafe {
            let dest = self.get_offset(index)?;
            write(dest, item);
            Ok(&*dest as &T)
        }
    }

    /// Bounds-checked read
    fn read<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<T, RuntimeError> {
        unsafe {
            let dest = self.get_offset(index)?;
            Ok(read(dest))
        }
    }

    /// Bounds-checked reference-read
    pub fn read_ref<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<&T, RuntimeError> {
        unsafe {
            let dest = self.get_offset(index)?;
            Ok(&*dest as &T)
        }
    }

    /// Represent the array as a slice. This is necessarily unsafe even for the 'guard lifetime
    /// duration because while a slice is held, other code can cause array internals to change
    /// that might cause the slice pointer and length to become invalid. Interior mutability
    /// patterns such as RefCell-style should be used in addition.
    pub unsafe fn as_slice<'guard>(&self, _guard: &'guard dyn MutatorScope) -> &mut [T] {
        if let Some(ptr) = self.data.get().as_ptr() {
            from_raw_parts_mut(ptr as *mut T, self.length.get() as usize)
        } else {
            &mut []
        }
    }

    /// Represent the full capacity of the array, however initialized, as a slice.
    /// This is necessarily unsafe even for the 'guard lifetime
    /// duration because while a slice is held, other code can cause array internals to change
    /// that might cause the slice pointer and length to become invalid. Interior mutability
    /// patterns such as RefCell-style should be used in addition.
    pub unsafe fn as_capacity_slice<'guard>(&self, _guard: &'guard dyn MutatorScope) -> &mut [T] {
        if let Some(ptr) = self.data.get().as_ptr() {
            from_raw_parts_mut(ptr as *mut T, self.data.get().capacity() as usize)
        } else {
            &mut []
        }
    }
}

impl<T: Sized + Clone> Container<T> for Array<T> {
    fn new() -> Array<T> {
        Array {
            length: Cell::new(0),
            data: Cell::new(RawArray::new()),
            borrow: Cell::new(INTERIOR_ONLY),
        }
    }

    fn with_capacity<'guard>(
        mem: &'guard MutatorView,
        capacity: ArraySize,
    ) -> Result<Array<T>, RuntimeError> {
        Ok(Array {
            length: Cell::new(0),
            data: Cell::new(RawArray::with_capacity(mem, capacity)?),
            borrow: Cell::new(INTERIOR_ONLY),
        })
    }

    fn clear<'guard>(&self, _guard: &'guard MutatorView) -> Result<(), RuntimeError> {
        if self.borrow.get() != INTERIOR_ONLY {
            Err(RuntimeError::new(ErrorKind::MutableBorrowError))
        } else {
            self.length.set(0);
            Ok(())
        }
    }

    fn length(&self) -> ArraySize {
        self.length.get()
    }
}

impl<T: Sized + Clone> FillContainer<T> for Array<T> {
    fn fill<'guard>(
        &self,
        mem: &'guard MutatorView,
        size: ArraySize,
        item: T,
    ) -> Result<(), RuntimeError> {
        let length = self.length();

        if length > size {
            Ok(())
        } else {
            let mut array = self.data.get(); // Takes a copy

            let capacity = array.capacity();

            if size > capacity {
                if capacity == 0 {
                    array.resize(mem, DEFAULT_ARRAY_SIZE)?;
                } else {
                    array.resize(mem, default_array_growth(capacity)?)?;
                }
                // Replace the struct's copy with the resized RawArray object
                self.data.set(array);
            }

            self.length.set(size);

            for index in length..size {
                self.write(mem, index, item.clone())?;
            }

            Ok(())
        }
    }
}

impl<T: Sized + Clone> StackContainer<T> for Array<T> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(&self, mem: &'guard MutatorView, item: T) -> Result<(), RuntimeError> {
        if self.borrow.get() != INTERIOR_ONLY {
            return Err(RuntimeError::new(ErrorKind::MutableBorrowError));
        }

        let length = self.length.get();
        let mut array = self.data.get(); // Takes a copy

        let capacity = array.capacity();

        if length == capacity {
            if capacity == 0 {
                array.resize(mem, DEFAULT_ARRAY_SIZE)?;
            } else {
                array.resize(mem, default_array_growth(capacity)?)?;
            }
            // Replace the struct's copy with the resized RawArray object
            self.data.set(array);
        }

        self.length.set(length + 1);
        self.write(mem, length, item)?;
        Ok(())
    }

    /// Pop returns None if the container is empty, otherwise moves the last item of the array
    /// out to the caller.
    fn pop<'guard>(&self, guard: &'guard dyn MutatorScope) -> Result<T, RuntimeError> {
        if self.borrow.get() != INTERIOR_ONLY {
            return Err(RuntimeError::new(ErrorKind::MutableBorrowError));
        }

        let length = self.length.get();

        if length == 0 {
            Err(RuntimeError::new(ErrorKind::BoundsError))
        } else {
            let last = length - 1;
            let item = self.read(guard, last)?;
            self.length.set(last);
            Ok(item)
        }
    }

    /// Return the value at the top of the stack without removing it
    fn top<'guard>(&self, guard: &'guard dyn MutatorScope) -> Result<T, RuntimeError> {
        let length = self.length.get();

        if length == 0 {
            Err(RuntimeError::new(ErrorKind::BoundsError))
        } else {
            let last = length - 1;
            let item = self.read(guard, last)?;
            Ok(item)
        }
    }
}

impl<T: Sized + Clone> IndexedContainer<T> for Array<T> {
    /// Return a copy of the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<T, RuntimeError> {
        self.read(guard, index)
    }

    /// Move an object into the array at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: T,
    ) -> Result<(), RuntimeError> {
        self.write(guard, index, item)?;
        Ok(())
    }
}

impl<T: Sized + Clone> SliceableContainer<T> for Array<T> {
    fn access_slice<'guard, F, R>(&self, guard: &'guard dyn MutatorScope, f: F) -> R
    where
        F: FnOnce(&mut [T]) -> R,
    {
        self.borrow.set(EXPOSED_MUTABLY);
        let slice = unsafe { self.as_slice(guard) };
        let result = f(slice);
        self.borrow.set(INTERIOR_ONLY);
        result
    }
}

/// Array of u8
pub type ArrayU8 = Array<u8>;

impl Print for ArrayU8 {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "ArrayU8[...]")
    }
}

/// Array of u16
pub type ArrayU16 = Array<u16>;

impl Print for ArrayU16 {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "ArrayU16[...]")
    }
}

/// Array of u32
pub type ArrayU32 = Array<u32>;

impl Print for ArrayU32 {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "ArrayU32[...]")
    }
}

impl FillAnyContainer for Array<TaggedCellPtr> {
    fn fill<'guard>(
        &self,
        mem: &'guard MutatorView,
        size: ArraySize,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        let length = self.length();

        if length > size {
            Ok(())
        } else {
            let mut array = self.data.get(); // Takes a copy

            let capacity = array.capacity();

            if size > capacity {
                if capacity == 0 {
                    array.resize(mem, DEFAULT_ARRAY_SIZE)?;
                } else {
                    array.resize(mem, default_array_growth(capacity)?)?;
                }
                // Replace the struct's copy with the resized RawArray object
                self.data.set(array);
            }

            self.length.set(size);

            for index in length..size {
                self.write(mem, index, TaggedCellPtr::new_with(item))?;
            }

            Ok(())
        }
    }
}

impl StackAnyContainer for Array<TaggedCellPtr> {
    /// Push can trigger an underlying array resize, hence it requires the ability to allocate
    fn push<'guard>(
        &self,
        mem: &'guard MutatorView,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        Ok(StackContainer::<TaggedCellPtr>::push(
            self,
            mem,
            TaggedCellPtr::new_with(item),
        )?)
    }

    /// Pop returns None if the container is empty, otherwise moves the last item of the array
    /// out to the caller.
    fn pop<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        Ok(StackContainer::<TaggedCellPtr>::pop(self, guard)?.get(guard))
    }

    /// Return the value at the top of the stack without removing it
    fn top<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        Ok(StackContainer::<TaggedCellPtr>::top(self, guard)?.get(guard))
    }
}

impl IndexedAnyContainer for Array<TaggedCellPtr> {
    /// Return a pointer to the object at the given index. Bounds-checked.
    fn get<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        Ok(self.read_ref(guard, index)?.get(guard))
    }

    /// Set the object pointer at the given index. Bounds-checked.
    fn set<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        index: ArraySize,
        item: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        self.read_ref(guard, index)?.set(item);
        Ok(())
    }
}

impl AnyContainerFromPairList for Array<TaggedCellPtr> {
    fn from_pair_list<'guard>(
        &self,
        mem: &'guard MutatorView,
        pair_list: TaggedScopedPtr<'guard>,
    ) -> Result<(), RuntimeError> {
        self.length.set(0);

        let mut head = pair_list;
        while let Value::Pair(p) = *head {
            StackAnyContainer::push(self, mem, p.first.get(mem))?;
            head = p.second.get(mem);
        }

        Ok(())
    }
}

impl<T: Clone + Sized> ContainerFromSlice<T> for Array<T>
where
    Array<T>: AllocObject<TypeList>,
{
    fn from_slice<'guard>(
        mem: &'guard MutatorView,
        data: &[T],
    ) -> Result<ScopedPtr<'guard, Array<T>>, RuntimeError> {
        let array = Array::alloc_with_capacity(mem, data.len() as ArraySize)?;
        let slice = unsafe { array.as_capacity_slice(mem) };
        slice.clone_from_slice(data);
        array.length.set(data.len() as ArraySize);
        Ok(array)
    }
}

impl AnyContainerFromSlice for Array<TaggedCellPtr> {
    fn from_slice<'guard>(
        mem: &'guard MutatorView,
        data: &[TaggedScopedPtr<'guard>],
    ) -> Result<ScopedPtr<'guard, Self>, RuntimeError> {
        let array = Array::<TaggedCellPtr>::alloc_with_capacity(mem, data.len() as ArraySize)?;
        let slice = unsafe { array.as_capacity_slice(mem) };

        // probably slow
        for index in 0..data.len() {
            slice[index] = TaggedCellPtr::new_with(data[index])
        }

        array.length.set(data.len() as ArraySize);
        Ok(array)
    }
}

impl Print for Array<TaggedCellPtr> {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "[")?;

        for i in 0..self.length() {
            if i > 1 {
                write!(f, ", ")?;
            }

            let ptr =
                IndexedAnyContainer::get(self, guard, i).expect("Failed to read ptr from array");

            fmt::Display::fmt(&ptr.value(), f)?;
        }

        write!(f, "]")
    }
}

#[cfg(test)]
mod test {
    use super::{
        AnyContainerFromPairList, Array, Container, IndexedAnyContainer, IndexedContainer,
        StackAnyContainer, StackContainer,
    };
    use crate::error::{ErrorKind, RuntimeError};
    use crate::memory::{Memory, Mutator, MutatorView};
    use crate::pair::Pair;
    use crate::safeptr::TaggedCellPtr;
    use crate::taggedptr::Value;

    #[test]
    fn array_generic_push_and_pop() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let array: Array<i64> = Array::new();

                // TODO StickyImmixHeap will only allocate up to 32k at time of writing
                // test some big array sizes
                for i in 0..1000 {
                    array.push(view, i)?;
                }

                for i in 0..1000 {
                    assert!(array.pop(view)? == 999 - i);
                }

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn array_generic_indexing() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let array: Array<i64> = Array::new();

                for i in 0..12 {
                    array.push(view, i)?;
                }

                assert!(array.get(view, 0) == Ok(0));
                assert!(array.get(view, 4) == Ok(4));

                for i in 12..1000 {
                    match array.get(view, i) {
                        Ok(_) => panic!("Array index should have been out of bounds!"),
                        Err(e) => assert!(*e.error_kind() == ErrorKind::BoundsError),
                    }
                }

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn arrayany_tagged_pointers() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let array: Array<TaggedCellPtr> = Array::new();
                let array = view.alloc(array)?;

                for _ in 0..12 {
                    StackAnyContainer::push(&*array, view, view.nil())?;
                }

                // or by copy/clone
                let pair = view.alloc_tagged(Pair::new())?;

                IndexedAnyContainer::set(&*array, view, 3, pair)?;

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn array_with_capacity_and_realloc() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let array: Array<TaggedCellPtr> = Array::with_capacity(view, 256)?;

                let ptr_before = array.data.get().as_ptr();

                // fill to capacity
                for _ in 0..256 {
                    StackAnyContainer::push(&array, view, view.nil())?;
                }

                let ptr_after = array.data.get().as_ptr();

                // array storage shouldn't have been reallocated
                assert!(ptr_before == ptr_after);

                // overflow capacity, requiring reallocation
                StackAnyContainer::push(&array, view, view.nil())?;

                let ptr_realloc = array.data.get().as_ptr();

                // array storage should have been reallocated
                assert!(ptr_before != ptr_realloc);

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }

    #[test]
    fn arrayany_from_pair_list() {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = ();
            type Output = ();

            fn run(
                &self,
                view: &MutatorView,
                _input: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                let array: Array<TaggedCellPtr> = Array::new();
                let array = view.alloc(array)?;

                let pair = Pair::new();
                pair.first.set(view.lookup_sym("thing0"));

                let head = view.alloc_tagged(pair)?;
                let mut tail = head;

                for n in 1..12 {
                    if let Value::Pair(pair) = *tail {
                        tail = pair.append(view, view.lookup_sym(&format!("thing{}", n)))?;
                    } else {
                        panic!("expected pair!")
                    }
                }

                array.from_pair_list(view, head)?;

                for n in 0..12 {
                    let thing = IndexedAnyContainer::get(&*array, view, n)?;

                    match *thing {
                        Value::Symbol(s) => assert!(s.as_str(view) == format!("thing{}", n)),
                        _ => panic!("expected symbol!"),
                    }
                }

                Ok(())
            }
        }

        let test = Test {};
        mem.mutate(&test, ()).unwrap();
    }
}

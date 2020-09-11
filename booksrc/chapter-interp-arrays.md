# Arrays

Before we get to the basics of compilation, we need another data structure:
the humble array. The first use for arrays will be to store the bytecode
sequences that the compiler generates.

Rust already provides `Vec` but as we're implementing everything in terms of our
memory management abstraction, we cannot directly use `Vec`. Rust does not
(yet) expose the ability to specify a custom allocator type as part of `Vec`,
nor are we interested in replacing the global allocator.

Our only option is to write our own version of `Vec`! Fortunately we can
learn a lot from `Vec` itself and it's underlying implementation. Jump over to
the [Rustonomicon][1] for a primer on the internals of `Vec`.

The first thing we'll learn is to split the implementation into a `RawArray<T>`
type and an `Array<T>` type. `RawArray<T>` will provide an unsafe abstraction
while `Array<T>` will make a safe layer over it.


## RawArray

If you've just come back from _Implementing Vec_ in the Nomicon, you'll
recognize what we're doing below with `RawArray<T>`:

```rust,ignore
{{#include ../interpreter/src/rawarray.rs:DefRawArray}}
```

Instead of `Unique<T>` for the pointer, we're using `Option<NonNull<T>>`.
One simple reason is that `Unique<T>` is likely to be permanently unstable and
only available internally to `std` collections. The other is that we can
avoid allocating the backing store if no capacity is requested yet, setting
the value of `ptr` to `None`.

For when we _do_ know the desired capacity, there is
`RawArray<T>::with_capacity()`. This method, because it allocates, requires
access to the `MutatorView` instance. If you'll recall from the chapter on
the allocation API, the API provides an array allocation method with
signature:

```rust,ignore
AllocRaw::alloc_array(&self, size_bytes: ArraySize) -> Result<RawPtr<u8>, AllocError>;
```

This method is wrapped on the interpreter side by `Heap` and `MutatorView` and
in both cases the return value remains, simply, `RawPtr<u8>` in the success
case. It's up to `RawArray<T>` to receive the `RawPtr<u8>` value and maintain
it safely. Here's `with_capcity()`, now:

```rust,ignore
{{#include ../interpreter/src/rawarray.rs:DefRawArrayWithCapacity}}
```

### Resizing

If a `RawArray<T>`'s content will exceed it's capacity, there is
`RawArray<T>::resize()`. It allocates a new backing array using the
`MutatorView` method `alloc_array()` and copies the content of the old
over to the new, finally swapping in the new backing array for the old.

The code for this is straightforward but a little longer, go check it out
in `interpreter/src/rawarray.rs`.

### Accessing

Since `RawArray<T>` will be wrapped by `Array<T>`, we need a couple more
methods to access the raw memory:

```rust,ignore
impl<T: Sized> RawArray<T> {
{{#include ../interpreter/src/rawarray.rs:DefRawArrayCapacity}}

{{#include ../interpreter/src/rawarray.rs:DefRawArrayAsPtr}}
}
```

And that's it! Now for the safe wrapper.


## Array

The definition of the struct wrapping `RawArray<T>` is as follows:

```rust,ignore
{{#include ../interpreter/src/array.rs:DefArray}}
```

Here we have three members:

* `length` - the length of the array
* `data` - the `RawArray<T>` being wrapped
* `borrow` - a flag serving as a runtime borrow check, allowing `RefCell`
  runtime semantics, since we're in a world of interior mutability patterns

We have a method to create a new array - `Array::alloc()`

```rust,ignore
impl<T: Sized + Clone> Array<T> {
{{#include ../interpreter/src/array.rs:DefArrayAlloc}}
}
```

In fact we'll extend this pattern of a method named "alloc" to any data
structure for convenience sake.

There are many more methods for `Array<T>` and it would be exhausting to be
exhaustive. Let's go over the core methods used to read and write elements
and then an example use case.

### Reading and writing

First of all, we need a function that takes an array index and returns a
pointer to a memory location, if the index is within bounds:

```rust,ignore
impl<T: Sized + Clone> Array<T> {
{{#include ../interpreter/src/array.rs:DefArrayGetOffset}}
}
```

There are two bounds checks here - firstly, the index should be within the
(likely non-zero) length values; secondly, the `RawArray<T>` instance
should have a backing array allocated. If either of these checks fail, the
result is an error. If these checks pass, we can be confident that there
is array backing memory and that we can return a valid pointer to somewhere
inside that memory block.

For reading a value in an array, we need two methods:

1. one that handles move/copy semantics and returns a value
2. one that handles reference semantics and returns a reference to the original
   value in it's location in the backing memory

First, then:

```rust,ignore
impl<T: Sized + Clone> Array<T> {
{{#include ../interpreter/src/array.rs:DefArrayRead}}
}
```

and secondly:

```rust,ignore
impl<T: Sized + Clone> Array<T> {
{{#include ../interpreter/src/array.rs:DefArrayReadRef}}
}
```

Writing, or copying, an object to an array is implemented as simply as follows:

```rust,ignore
impl<T: Sized + Clone> Array<T> {
{{#include ../interpreter/src/array.rs:DefArrayReadRef}}
}
```

These simple functions should only be used internally by `Array<T>` impl
methods. We have numerous methods that wrap the above in more appropriate
semantics for values of `T` in `Array<T>`.

### The Array interfaces

To define the interfaces to the Array, and other collection types, we define a
number of traits. For example, a collection that behaves as a stack implements
`StackContainer`; a numerically indexable type implements `IndexedContainer`,
and so on. As we'll see, there is some nuance, though, when it comes to a
difference between collections of non-pointer types and collections of pointer
types.

For our example, we will describe the stack interfaces of `Array<T>`.

First, the general case trait, with methods for accessing values stored in the
array (non-pointer types):

```rust,ignore
{{#include ../interpreter/src/containers.rs:DefStackContainer}}
```

These are unremarkable functions, by now we're familiar with the references to
`MutatorScope` and `MutatorView` in method parameter lists.

In any instance of `Array<T>`, `T` need only implement `Clone` and cannot be
dynamically sized. Thus `T` can be any primitive type or any straightforward
struct.

What if we want to store pointers to other objects? For example, if we want a
heterogenous array, such as Python's `List` type, what would we provide in
place of `T`? The answer is to use the `TaggedCellPtr` type. However,
an `Array<TaggedCellPtr`, because we want to interface with pointers and
use the memory access abstractions provided, can be made a little more
ergonomic. For that reason, we have separate traits for containers of type
`Container<TaggedCellPtr`. In the case of the stack interface this looks like:

```rust,ignore
{{#include ../interpreter/src/containers.rs:DefStackAnyContainer}}
```

As you can see, these methods, while for `T = TaggedCellPtr`, provide an
interface based on passing and returning `TaggedScopedPtr`.

Let's look at the implementation of one of these methods - `push()`  - for
both `StackContainer` and `StackAnyContainer`.

Here's the code for `StackContainer::push()`:

```rust,ignore
impl<T: Sized + Clone> StackContainer<T> for Array<T> {
{{#include ../interpreter/src/array.rs:DefStackContainerArrayPush}}
}
```

In summary, the order of operations is:

1. Check that a runtime borrow isn't in progress. If it is, return an error.
1. Since we must implement interior mutability, the member `data` of the
   `Array<T>` struct is a `Cell`. We have to `get()` the content in order
   to use it.
1. We then ask whether the array backing store needs to be grown. If so,
   we resize the `RawArray<T>` and, since it's kept in a `Cell` on `Array<T>`,
   we have to `set()` value back into `data` to save the change.
1. Now we have an `RawArray<T>` that has enough capacity, the length is
   incremented and the object to be pushed is written to the next memory
   location using the internal `Array<T>::write()` method detailed earlier.

Fortunately we can implement `StackAnyContainer::push()` in terms of
`StackContainer::push()`:

```rust,ignore
impl StackAnyContainer for Array<TaggedCellPtr> {
{{#include ../interpreter/src/array.rs:DefStackAnyContainerArrayPush}}
}
```

### One last thing

To more easily differentiate arrays of type `Array<T>` from arrays of type
`Array<TaggedCellPtr>`, we make a type alias `List` where:

```rust,ignore
pub type List = Array<TaggedCellPtr>;
```


## In conclusion

We referenced how `Vec` is implemented internally and followed the same pattern
of defining a `RawArray<T>` unsafe layer with a safe `Array<T>` wrapper. Then
we looked into the stack interface for `Array<T>` and the implementation of
`push()`.

There is more to arrays, of course - indexed access the most obvious, and also
a few convenience methods. See the source code in `interpreter/src/array.rs`
for the full detail.

In the next chapter we'll put `Array<T>` to use in a `Bytecode` type!


[1]: https://doc.rust-lang.org/nomicon/vec.html

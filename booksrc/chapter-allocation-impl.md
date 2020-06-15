# Implementing the Allocation API

In this final chapter of the allocation part of the book, we'll cover the
`AllocRaw` trait implementation.

This trait is implemented on the `StickyImmixHeap` struct:

```rust
impl<H: AllocHeader> AllocRaw for StickyImmixHeap<H> {
    type Header = H;

    ...
}
```

Here the associated header type is provided as the generic type `H`, leaving it
up to the interpreter to define.

## Allocating objects

The first function to implement is `AllocRaw::alloc<T>()`. This function must:
* calculate how much space in bytes is required by the object and header
* allocate that space
* instantiate an object header and write it to the first bytes of the space
* copy the object itself to the remaining bytes of the space
* return a pointer to where the object lives in this space

Let's look at the implementation.

```rust
impl<H: AllocHeader> AllocRaw for StickyImmixHeap<H> {
{{#include ../stickyimmix/src/heap.rs:DefAlloc}}
}
```

This, hopefully, is easy enough to follow after the previous chapters -
* `self.find_space()` is the function described in the chapter
  [Allocating into multiple blocks](./chapter-managing-blocks.md#allocating-into-the-head-block)
* `Self::Header::new()` will be implemented by the interpreter
* `write(space as *mut Self::Header, header)` calls the std function
  `std::ptr::write`

## Allocating arrays

We need a similar (but awkwardly different enough) implementation for array
allocation. The key differences are that the type is fixed to a `u8` pointer
and the array is initialized to zero bytes. It is up to the interpreter to
write into the array itself.

```rust
impl<H: AllocHeader> AllocRaw for StickyImmixHeap<H> {
{{#include ../stickyimmix/src/heap.rs:DefAllocArray}}
}
```

## Switching between header and object

As stated in the previous chapter, these functions are essentially pointer
operations that do not dereference the pointers. Thus they are not unsafe
to call, but the types they operate _on_ should have a suitably unsafe API.

`NonNull` is the chosen parameter and return type and the pointer arithmetic
for obtaining the header from an object pointer of unknown type is shown
below.

For our Immix implementation, since headers are placed immediately
ahead of an object, we simply subtract the header size from the object
pointer.

```rust
impl<H: AllocHeader> AllocRaw for StickyImmixHeap<H> {
{{#include ../stickyimmix/src/heap.rs:DefGetHeader}}
}
```

Getting the object from a header is the reverse - adding the header size
to the header pointer results in the object pointer:

```rust
impl<H: AllocHeader> AllocRaw for StickyImmixHeap<H> {
{{#include ../stickyimmix/src/heap.rs:DefGetObject}}
}
```

## Conclusion

Thus ends the first part of our Immix implementation. In the next part of the
book we will jump over the fence to the interpreter and begin using the
interfaces we've defined in this part.

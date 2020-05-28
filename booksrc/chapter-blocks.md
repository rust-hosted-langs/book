# Obtaining Blocks of Memory

When requesting blocks of memory at a time, one of the questions is *what
is the desired block alignment?*

* In deciding, one factor is that using an alignment that is a multiple of the
  page size can make it easier to return memory to the operating system.
* Another factor is that if the block is aligned to it's size, it is fast to
  do bitwise arithmetic on a pointer to an object in a block to compute the
  block boundary and therefore the location of any block metadata.

With both these in mind we'll look at how to allocate blocks that are
aligned to the size of the block.


## A basic crate interface

A block of memory is defined as a base address and a size, so we need a struct
that contains these elements.

To wrap the base address pointer, we'll use the recommended type for building
collections, [`std::ptr::NonNull<T>`](https://doc.rust-lang.org/std/ptr/struct.NonNull.html),
which is available on stable.

```rust
{{#include ../blockalloc/src/lib.rs:DefBlock}}
```

Where `BlockPtr` and `BlockSize` are defined as:

```rust
{{#include ../blockalloc/src/lib.rs:DefBlockComponents}}
```

To obtain a `Block`, we'll create a `Block::new()` function which, along with
`Block::drop()`, is implemented internally by wrapping the stabilized Rust alloc 
routines:

```rust
{{#include ../blockalloc/src/lib.rs:BlockNew}}
```

Where parameter `size` must be a power of two, which is validated on the first
line of the function.  Requiring the block size to be a power of two means
simple bit arithmetic can be used to find the beginning and end of a block in
memory, if the block size is always the same.

Errors take one of two forms, an invalid block-size or out-of-memory, both
of which may be returned by `Block::new()`.

```rust
{{#include ../blockalloc/src/lib.rs:DefBlockError}}
```

Now on to the platform-specific implementations.


## Custom aligned allocation on stable Rust

On the stable rustc channel we have access to some features of the
[Alloc](https://doc.rust-lang.org/std/alloc/index.html) API. 

This is the ideal option since it abstracts platform specifics for us, we do
not need to write different code for Unix and Windows ourselves.

Fortunately there is enough stable functionality to 
fully implement what we need.

With an appropriate underlying implementation this code should compile and 
execute for any target. The allocation function, implemented in the `internal` 
mod, reads:

```rust
{{#include ../blockalloc/src/lib.rs:AllocBlock}}
```

Once a block has been allocated, there is no safe abstraction at this level
to access the memory. The `Block` will provide a bare pointer to the beginning
of the memory and it is up to the user to avoid invalid pointer arithmetic
and reading or writing outside of the block boundary.

```rust
{{#include ../blockalloc/src/lib.rs:BlockAsPtr}}
```


## Deallocation

Again, using the stable Alloc functions:

```rust
{{#include ../blockalloc/src/lib.rs:DeallocBlock}}
```

The implementation of `Block::drop()` calls the deallocation function
for us so we can create and drop `Block` instances without leaking memory.


## Testing

We want to be sure that the system level allocation APIs do indeed return
block-size-aligned blocks. Checking for this is straightforward.

A correctly aligned block should have it's low bits
set to `0` for a number of bits that represents the range of the block
size - that is, the block size minus one. A bitwise XOR will highlight any
bits that shouldn't be set:

```rust
{{#include ../blockalloc/src/lib.rs:TestAllocPointer}}
```

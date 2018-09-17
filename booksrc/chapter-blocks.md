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
{{#include ../blockalloc/src/lib.rs:26:29}}
```

Where `BlockPtr` and `BlockSize` are defined as:

```rust
{{#include ../blockalloc/src/lib.rs:11:12}}
```

To obtain a `Block`, we'll create a `Block::new()` function which, along with
`Block::drop()`, is implemented in terms of platform-specific allocation
routines:

```rust
{{#include ../blockalloc/src/lib.rs:34:43}}
```

Where parameter `size` must be a power of two, which is validated on the first
line of the function.

Errors take one of two forms, an invalid block-size or out-of-memory, both
of which may be returned by `Block::new()`.

```rust
{{#include ../blockalloc/src/lib.rs:17:22}}
```

Now on to the platform-specific implementations.


## Custom aligned allocation on unstable Rust

On the unstable rustc channel we have access to the
[Alloc](https://doc.rust-lang.org/alloc/allocator/trait.Alloc.html) API. This
is the ideal option since it abstracts platform specifics for us - with an
appropriate underlying implementation this code should compile and execute
for any target.

The allocation function, implemented in the `internal` mod, reads:

```rust
{{#include ../blockalloc/src/lib.rs:101:110}}
```

And deallocation:

```rust
{{#include ../blockalloc/src/lib.rs:112:120}}
```


## Custom aligned allocation on stable Rust on Unix-like platforms

As of writing, the stable Rust channel does not provide access directly to the
allocation APIs in the previous section.  In order to get block-size
aligned blocks of memory on stable on Unix-like platforms, we'll use
the
[posix_memalign()](http://man7.org/linux/man-pages/man3/posix_memalign.3.html)
standard library function call which we can access in the
[libc](https://docs.rs/libc/0.2.40/libc/fn.posix_memalign.html) crate.

```rust
{{#include ../blockalloc/src/lib.rs:134:146}}
```

Deallocation is done with the `free()` libc function:

```rust
{{#include ../blockalloc/src/lib.rs:148:152}}
```


## Custom aligned allocation in stable Rust on Windows

Allocation:

```rust
{{#include ../blockalloc/src/lib.rs:163:165}}
```

Deallocation:

```rust
{{#include ../blockalloc/src/lib.rs:167:169}}
```


## Testing

We want to be sure that the system level allocation APIs do indeed return
block-size-aligned blocks. Checking for this is straightforward.

A correctly aligned block should have it's low bits
set to `0` for a number of bits that represents the range of the block
size - that is, the block size minus one. A bitwise XOR will highlight any
bits that shouldn't be set:

```rust
{{#include ../blockalloc/src/lib.rs:195:196}}
```

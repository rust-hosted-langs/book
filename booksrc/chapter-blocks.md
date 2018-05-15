# Blocks of Memory

When requesting blocks of memory at a time, one of the questions is *what
is the desired block alignment?*

* One factor is that using an alignment that is a multiple of the page size
  can make it easier to return memory to the operating system.
* Another factor is that if the block is aligned to it's size, it is fast to
  do bitwise arithmetic on a pointer to an object in a block to compute the
  block boundary and therefore the location of any block metadata.

With both these in mind we'll look at how to allocate blocks that are
aligned to the size of the block.


## When block alignment is not important

There are plenty of case where we will not care about block alignment.
For example, providing arenas for immutable interned objects. In such a
case, the only guarantee needed is that the objects don't ever move
and can be reliably freed at the end of the arena life.

The simplest option is to use `Vec` and reinterpret it as a `*mut u8` that
can be used as raw bytes of memory.

In using a `Vec`, we must be careful never to resize it as that could cause
reallocation of the backing array, causing the objects in it to be moved.

If we manage our own objects independently of compile-time lifetime
information, we can easily end up with broken pointers something other than
the garbage collector can move objects.

Using `Vec` looks like this:

```rust
fn alloc(block_size: usize) -> *mut u8 {
    let mut vec = Vec::with_capcity(block_size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}
```

And deallocation:

```rust
// unsafe because the caller must pass in the correct block size value and
// a ptr that originated from a Vec
unsafe fn dealloc(ptr: *mut u8, block_size: usize) {
    Vec::from_raw_parts(ptr, 0, block_size);
}
```


## A basic library interface

A block of memory, as evidenced in the above `Vec` example, is comprised of
two pieces of information:

* the address in memory of the block
* the size of the block

It's worth taking a moment to look at how `Vec`
[is implemented](https://doc.rust-lang.org/stable/nomicon/vec.html) and how
what we want is slightly different. Two obvious points are

* Since `Unique<T>` is still unstable, we won't use it here
* We aren't concerned with being able to change the capacity of a block

Similarly to `RawVec`, we can build on a struct that contains the two basic
items of information we need:

```rust
{{#include ../blockalloc/src/lib.rs:26:29}}
```

Where `BlockPtr` and `BlockSize` are defined as:

```rust
{{#include ../blockalloc/src/lib.rs:11:12}}
```

To obtain a `Block`, we have the `Block::new()` function which, along with
`Block::drop()`, is implemented in terms of platform-specific allocation
routines.

```rust
{{#include ../blockalloc/src/lib.rs:34:39}}
```

Where:

* parameter `size` must be a power of two
* and errors take one of two forms, an invalid block-size or out-of-memory:

```rust
{{#include ../blockalloc/src/lib.rs:17:22}}
```


## Block-aligned allocation on unstable Rust

On the unstable rustc channel we have access to the
[Alloc](https://doc.rust-lang.org/alloc/allocator/trait.Alloc.html) API. This
is the ideal option since it abstracts platform specifics for us - with an
appropriate underlying implementation this code should compile and execute
for any target.

The allocation function, implemented in the `internal` mod, reads:

```rust
{{#include ../blockalloc/src/lib.rs:92:111}}
```

And deallocation:

```rust
{{#include ../blockalloc/src/lib.rs:113:121}}
```


## Block-aligned allocation on stable Rust on Unix-like platforms

As of writing, the stable Rust channel does not provide access directly to the
allocation APIs in the previous section.  In order to get block-size
aligned blocks of memory on stable, we'll use platform-specific library calls.

On all Unix platforms, we'll use `posix_memalign(**ptr, size, align)` for
block allocation.

```rust
{{#include ../blockalloc/src/lib.rs:137:149}}
```

And deallocation:

```rust
{{#include ../blockalloc/src/lib.rs:151:155}}
```


## Block-aligned allocation in stable Rust on Windows

Allocation:

```rust
{{#include ../blockalloc/src/lib.rs:165:167}}
```

And deallocation:

```rust
{{#include ../blockalloc/src/lib.rs:169:171}}
```


## Testing

We want to be sure that the system level allocation APIs do indeed return
block-size-aligned blocks. Checking for this is straightforward.

A correctly aligned block should have it's low bits
set to `0` for a number of bits that represents the range of the block
size - that is, the block size minus one. A bitwise XOR will highlight any
bits that shouldn't be set:

```rust
{{#include ../blockalloc/src/lib.rs:197:198}}
```

# Block-size-aligned Blocks of Memory

## The simplest case

In normal Rust code we'd write code such as

```rust
let shared_vec = Rc::new(vec![3, 4, 5]);
```

In the case of both `Vec` and `Rc`, memory is allocated on the global heap
using the system allocator. In order to efficiently allocate and
garbage collect managed objects, we need to manage our own heap and
so need somewhere to allocate any object type into.

The simplest option is to use `Vec` and reinterpret it as a `*mut u8` that
can be used as raw bytes of memory.

In using a `Vec` we must be careful never to resize it as that could cause
reallocation of the backing array, possibly causing the objects in it to move.
If we manage our own objects independently of compile-time lifetime
information, we can easily end up with broken pointers if objects can be
moved by something other than the garbage collector.

Using `Vec` this way looks like:

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

In many cases this might be entirely sufficient, for example, providing arenas
for interned objects that last the lifetime of a virtual machine. In such
a case, the only guarantee needed *is* that the objects don't ever move.


## A basic library interface

A block of memory, as evidenced in the `Vec` example, is comprised of two pieces
of information:

* the address in memory of the block
* the size of the block

It's worth taking a moment to look at how `Vec`
[is implemented](https://doc.rust-lang.org/stable/nomicon/vec.html) and how
what we want is slightly different. Two obvious points are

*  Since `Unique<T>` is still unstable, we won't use it here
*  We aren't concerned with being able to change the capacity of a block

Similarly to `RawVec`, we can build on a struct that contains the two basic
items of information we need:

```rust
{{#include ../blockalloc/src/lib.rs:26:29}}
```

Where `BlockPtr` and `BlockSize` are defined as:

```rust
{{#include ../blockalloc/src/lib.rs:11:12}}
```

To obtain a `Block`, functions are provided:

```rust
{{#include ../blockalloc/src/lib.rs:45:52}}
```

Where:

* the `internal` mod contains the platform-specific implementations
* deallocation consumes the `Block`
* and errors take one of two forms, an invalid block-size or out-of-memory:

```rust
{{#include ../blockalloc/src/lib.rs:17:22}}
```


## Allocation on unstable Rust

On the unstable rustc channel we have access to the
[Alloc](https://doc.rust-lang.org/alloc/allocator/trait.Alloc.html) API. This
is the ideal option since it abstracts platform specifics for us - with an
appropriate underlying implementation this code should compile and execute
for any target.

The allocation function, implemented in the `internal` mod and feature gated
reads:

```rust
{{#include ../blockalloc/src/lib.rs:80:102}}
```

And deallocation:

```rust
{{#include ../blockalloc/src/lib.rs:104:114}}
```


## Stable Rust on Unix-like platforms

As of writing, the stable Rust channel does not provide access directly to the
allocation APIs in the previous section.  In order to get block-size
aligned blocks of memory on stable, we'll use platform-specific library calls.

On all Unix platforms, we'll use `posix_memalign(**ptr, size, align)` for
block allocation.

```rust
{{#include ../blockalloc/src/lib.rs:130:152}}
```

And deallocation:

```rust
{{#include ../blockalloc/src/lib.rs:147:152}}
```


## Stable Rust on Windows

Allocation:

```rust
{{#include ../blockalloc/src/lib.rs:162:164}}
```

And deallocation:

```rust
{{#include ../blockalloc/src/lib.rs:166:168}}
```


## Testing

The most crucial test is that the block we allocate is indeed aligned to it's
size.

Testing this is straightforward - the block address should have it's low bits
set to `0` for a number of bits that represents the size of the block minus
one. A bitwise XOR will highlight any bits that shouldn't be set:

```rust
{{#include ../blockalloc/src/lib.rs:194:195}}
```

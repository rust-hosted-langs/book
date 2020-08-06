# Arrays

Before we get to the basics of compilation, we need another data structure:
the humble array. The primary use for arrays will be to store the bytecode
sequences that the compiler generates.

Rust already has `Vec` but as we're implementing everything in terms of our
memory management abstraction, we cannot directly use `Vec`. Rust does not
(yet) expose the ability to specify a custom allocator type as part of `Vec`,
nor are we interested in replacing the global allocator.

Our only option is to write our own version of `Vec`! Fortunately we can
learn a lot from `Vec` and it's underlying implementation. Jump over to the
[Nomicon][1] for the Rust-oriented approach to arrays.

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


[1]: https://doc.rust-lang.org/nomicon/vec.html

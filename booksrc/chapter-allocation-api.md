# Defining the allocation API

Let's look back at the allocator prototype API we defined in the introductory
chapter.

```rust
trait AllocRaw {
    fn alloc<T>(&self, object: T) -> *const T;
}
```

This will quickly prove to be inadequate and non-idiomatic. For starters, there
is no way to report that allocation failed except for perhaps returning a null
pointer. That is certainly a workable solution but is not going to feel
idiomatic or ergonomic for how we want to use the API. Let's make a couple
changes:

```rust
trait AllocRaw {
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>;
}
```

Now we're returning a `Result`, the failure side of which is an error type
where we can distinguish between different allocation failure modes. This is
often not _that_ useful but working with `Result` is far more ergonomic than
checking a pointer for being null. We'll allow for distinguishing between
Out Of Memory and an allocation request that for whatever reason is invalid.

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocError}}
```

The second change is that instead of a `*const T` value in the success
discriminant we'll wrap a pointer in a new struct: `RawPtr<T>`. This wrapper
will amount to little more than containing a `std::ptr::NonNull` instance
and some functions to access the instance.

```rust
{{#include ../stickyimmix/src/rawptr.rs:DefRawPtr}}
```

This'll be better to work with on the user-of-the-crate side.

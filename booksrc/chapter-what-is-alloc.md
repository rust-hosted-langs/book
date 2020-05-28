# The type of allocation

Before we start writing objects into `Block`s, we need to know the nature of
the interface in Rust terms.

If we consider the global allocator in Rust, implicitly available via
`Box::new()`, `Vec::new()` and so on, we'll notice that since the global
allocator is available on every thread and allows the creation of new
objects on the heap (that is, mutation of the heap) from any code location
without needing to follow the rules of borrowing and mutable aliasing,
it is essentially a container that implements `Sync` and the interior
mutability pattern.

We need to follow suit, but we'll leave `Sync` until later chapters.

An interface that satisfies the interior mutability property, by borrowing
the allocator instance immutably, might look like:

```rust
trait AllocRaw {
    fn alloc<T>(&self, object: T) -> *const T;
}
```

naming it `AllocRaw` because when layering on top of `Block` we'll
work with raw pointers and not concern ourselves with the lifetime of
allocated objects.

It will become a little more complex than this but for now, this captures
the essence of the interface.

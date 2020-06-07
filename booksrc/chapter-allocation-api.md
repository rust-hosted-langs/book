# Defining the allocation API

Let's look back at the allocator prototype API we defined in the introductory
chapter.

```rust
trait AllocRaw {
    fn alloc<T>(&self, object: T) -> *const T;
}
```

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
often not _that_ useful but working with `Result` is far more idiomatic Rust
than checking a pointer for being null. We'll allow for distinguishing between
Out Of Memory and an allocation request that for whatever reason is invalid.

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocError}}
```

The second change is that instead of a `*const T` value in the success
discriminant we'll wrap a pointer in a new struct: `RawPtr<T>`. This wrapper
will amount to little more than containing a `std::ptr::NonNull` instance
and some functions to access the pointer.

```rust
{{#include ../stickyimmix/src/rawptr.rs:DefRawPtr}}
```

This'll be better to work with on the user-of-the-crate side.

It'll also make it easier to modify internals or even swap out entire
implementations. This is a motivating factor for the design of this interface
as we'll see as we continue to amend it to account for object headers now.

## Object headers

The purpose of an object header is to provide the allocator, the language
runtime and the garbage collector with information about the object that
is needed at runtime. Typical data points that are stored might include:

* object size
* some kind of type identifier
* garbage collection information such as a mark flag

We want to create a flexible interface to a language while also ensuring that
the language _will_ provide the information that the allocator and garbage
collector in _this_ crate need.

We'll define a trait for the user to implement.

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocHeader}}
```

Now we have a bunch more questions. Some of these trait methods are
straightforward - `fn size(&self) -> u32` returns the object size; `mark()`
and `is_marked()` must be GC related.

But this struct references some more types that must be defined and explained.

### Type identification

_First, a note: what follows is a set of design trade-offs made for the
purposes of this book; there are many ways this could be implemented._

The types described next are all about the _object_ type.

That is, the problem to solve is that certain values in an object header and
certain actions on objects are strongly associated with the type of the object.

We ideally want to make it difficult for the user to make mistakes with this
and leak undefined behavior through what we would prefer to be a safe-Rust
interface, while at the same time being flexible enough for the user to make
_language_ appropriate decisions about the header design.

First up, an object header implementation must define an associated type
`type TypeId: AllocTypeId` where `AllocTypeId` is define simply as:

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocTypeId}}
```

This means the user is free to implement a type idntifier type however they
please, the only constraint is that it implements this trait.

Next, the definition of the header constructor,

```rust
    fn new<O: AllocObject<Self::TypeId>>(
        size: u32,
        size_class: SizeClass,
        mark: Mark
    ) -> Self;
```

refers to a type `O` that must implement `AllocObject` which in turn must refer
to the common `AllocTypeId`. The generic type `O` is the object for which the
header is being instantiated for.

And what is `AllocObject`? Simply:

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocObject}}
```

### An example

To demonstrate...

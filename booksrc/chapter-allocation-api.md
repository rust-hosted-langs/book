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

Now we have a bunch more questions to answer. Some of these trait methods are
straightforward - `fn size(&self) -> u32` returns the object size; `mark()`
and `is_marked()` must be GC related. Some are less obvious, such as
`new_array()` which we'll cover at the end of this chapter.

But this struct references some more types that must be defined and explained.

### Type identification

_First, a note: what follows is a set of design trade-offs made for the
purposes of this book; there are many ways this could be implemented._

The types described next are all about the _object_ type.

That is, the problem to solve is that certain values in an object header and
certain actions on objects are strongly associated with the type of the object.

We ideally want to make it difficult for the user to make mistakes with this
and leak undefined behavior. We would also prefer this to be a safe-Rust
interface, while at the same time being flexible enough for the user to make
interpreter-appropriate decisions about the header design.

First up, an object header implementation must define an associated type
`type TypeId: AllocTypeId` where `AllocTypeId` is define simply as:

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocTypeId}}
```

This means the user is free to implement a type identifier type however they
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

In summary, we have:

* `AllocHeader`: a trait that the header type must implement
* `AllocTypeId`: a trait that a type identifier must implement
* `AllocObject`: a trait that objects that can be allocated must implement

### An example

Let's implement a couple of traits to make it more concrete.

The simplest form of type identifier is an enum. Each discriminant describes
a type that the interpreter will use at runtime.

```rust
#[derive(PartialEq, Copy, Clone)]
enum MyTypeId {
    Number,
    String,
    Array,
}

impl AllocTypeId for TestTypeId {}
```

A hypothetical numeric type for our interpreter with the type identifier as
associated constant:

```rust
struct Number {
    value: i64
}

impl AllocObject<TestTypeId> for Big {
    const TYPE_ID: MyTypeId = MyTypeId::Number;
}
```

And finally, here is an overly simplistic object header struct and the `new()`
function from `AllocHeader`.

```rust
struct NyHeader {
    size: u32,
    size_class: SizeClass,
    mark: Mark,
    type_id: MyTypeId,
}

impl AllocHeader for MyHeader {
    type TypeId = MyTypeId;

    fn new<O: AllocObject<Self::TypeId>>(
        size: u32,
        size_class: SizeClass,
        mark: Mark
    ) -> Self {
        MyHeader {
            size,
            size_class,
            mark,
            type_id: O::TYPE_ID,
        }
    }

    ...
}
```

The types `SizeClass` and `Mark` are provided by the allocator API and are just
enums. The `new()` function is designed to be called internally by the
allocator itself, not on the interpreter side of the implementation.

The one drawback to this scheme is that it's possible to associate an incorrect
type id constant with an object. This would result in objects being misidentified
at runtime and accessed incorrectly, likely leading to panics.

Fortunately, this kind of trait implementation boilerplate is ideal for derive
macros. Since the language side will be implementing these structs and traits,
we'll defer until the relevant interpreter chapter to go over that.


## Back to AllocRaw

Now that we have some object and header definitions and constraints, we need to
apply them to the `AllocRaw` API. We can't allocate an object unless it
implements `AllocObject` and has an associated constant that implements
`AllocTypeId`.  We also need to expand the interface with functions that the
interpreter can use to reliably get the header for an object and the object
for a header.

We will add an associated type to tie the allocator
API to the header type and indirectly to the type identification that will be
used.

```rust
pub trait AllocRaw {

    type Header: AllocHeader;

    ...
```

Then we can update the `alloc()` function definition to constrain the types
that can be allocated to only those that implement the appropriate traits.

```rust
    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>
    where
        T: AllocObject<<Self::Header as AllocHeader>::TypeId>;
```

We need the user and the garbage collector to be able to access the header,
so we need a function that will return the header given an object pointer.

The garbage collector does not know about concrete types, it will need to
be able to get the header without knowing the object type. It's possible
that an interpreter will, at times, also not know the type at runtime.

The function signature cannot refer to the type, therefore. That is,
we can't write

```rust
    // looks good but won't work
    fn get_header<T>(object: RawPtr<T>) -> NonNull<Self::Header>
    where
        T: AllocObject<<Self::Header as AllocHeader>::TypeId>;
```

even though it seems this would be good and right. Instead this function will
be much simpler:

```rust
    fn get_header(object: NonNull<()>) -> NonNull<Self::Header>;
```

We also need a function to get the object _from_ the header:

```rust
    fn get_object(header: NonNull<Self::Header>) -> NonNull<()>;
```

Now we have an object allocation function, traits that constrain what can be
allocated, allocation header definitions and functions for switching
between an object and it's header.


## Dynamically sized types

TODO

Our complete definition now looks like this:

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocRaw}}
```

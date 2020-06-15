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
often not that useful but working with `Result` is far more idiomatic Rust
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
the interpreter will provide the information that the allocator and garbage
collector in _this_ crate need.

We'll define a trait for the user to implement.

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocHeader}}
```

Now we have a bunch more questions to answer! Some of these trait methods are
straightforward - `fn size(&self) -> u32` returns the object size; `mark()`
and `is_marked()` must be GC related. Some are less obvious, such as
`new_array()` which we'll cover at the end of this chapter.

But this struct references some more types that must be defined and explained.

### Type identification

What follows is a set of design trade-offs made for the purposes of this book;
there are many ways this could be implemented.

The types described next are all about sharing compile-time and runtime object
type information between the allocator, the GC and the interpreter.

We ideally want to make it difficult for the user to make mistakes with this
and leak undefined behavior. We would also prefer this to be a safe-Rust
interface, while at the same time being flexible enough for the user to make
interpreter-appropriate decisions about the header design.

First up, an object header implementation must define an associated type
```rust
pub trait AllocHeader: Sized {
    type TypeId: AllocTypeId;
}
```
where `AllocTypeId` is define simply as:

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocTypeId}}
```

This means the interpreter is free to implement a type identifier type however
it pleases, the only constraint is that it implements this trait.

Next, the definition of the header constructor,

```rust
pub trait AllocHeader: Sized {
    ...

    fn new<O: AllocObject<Self::TypeId>>(
        size: u32,
        size_class: SizeClass,
        mark: Mark
    ) -> Self;

    ...
}
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

And finally, here is a possible object header struct and the implementation of
`AllocHeader::new()`:

```rust
struct MyHeader {
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

These would all be defined and implemented in the interpreter and are not
provided by the Sticky Immix crate, while all the functions in the trait
`AllocHeader` are intended to be called internally by the allocator itself,
not on the interpreter side.

The types `SizeClass` and `Mark` _are_ provided by this crate and are enums.

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
}
```

Then we can update the `alloc()` function definition to constrain the types
that can be allocated to only those that implement the appropriate traits.

```rust
pub trait AllocRaw {
    ...

    fn alloc<T>(&self, object: T) -> Result<RawPtr<T>, AllocError>
    where
        T: AllocObject<<Self::Header as AllocHeader>::TypeId>;

    ...
}
```

We need the user and the garbage collector to be able to access the header,
so we need a function that will return the header given an object pointer.

The garbage collector does not know about concrete types, it will need to
be able to get the header without knowing the object type. It's likely
that the interpreter will, at times, also not know the type at runtime.

Indeed, one of the functions of an object header is to, at runtime, given
an object pointer, derive the type of the object.

The function signature therefore cannot refer to the type. That is,
we can't write

```rust
pub trait AllocRaw {
    ...

    // looks good but won't work in all cases
    fn get_header<T>(object: RawPtr<T>) -> NonNull<Self::Header>
    where
        T: AllocObject<<Self::Header as AllocHeader>::TypeId>;

    ...
}
```

even though it seems this would be good and right. Instead this function will
have to be much simpler:

```rust
pub trait AllocRaw {
    ...

    fn get_header(object: NonNull<()>) -> NonNull<Self::Header>;

    ...
}
```

We also need a function to get the object _from_ the header:

```rust
pub trait AllocRaw {
    ...

    fn get_object(header: NonNull<Self::Header>) -> NonNull<()>;

    ...
}
```

These functions are not unsafe but they do return `NonNull` which implies that
dereferencing the result should be considered unsafe - there is no protection
against passing in garbage and getting garbage out.

Now we have an object allocation function, traits that constrain what can be
allocated, allocation header definitions and functions for switching
between an object and it's header.

There's one missing piece: we can allocate objects of type `T`, but
such objects always have compile-time defined size. `T` is constrained to
`Sized` types in the `RawPtr` definition. So how do we allocate dynamically
sized objects, such as arrays?


## Dynamically sized types

Since we can allocate objects of type `T`, and each `T` must derive
`AllocObject` and have an associated const of type `AllocTypeId`, dynamically
sized allocations must fit into this type identification scheme.

Allocating dynamically sized types, or in short, arrays, means there's some
ambiguity about the type at compile time as far as the allocator is concerned:

* Are we allocating one object or an array of objects? If we're allocating an
  array of objects, we'll have to initialize them all. Perhaps we don't want to
  impose that overhead up front?
* If the allocator knows how many objects compose an array, do we want to bake
  fat pointers into the interface to carry that number around?

In the same way, then, that the underlying implementation of `std::vec::Vec` is
backed by an array of `u8`, we'll do the same. We shall define the return type
of an array allocation to be of type `RawPtr<u8>` and the size requested to be
in bytes. We'll leave it to the interpreter to build layers on top of this to
handle the above questions.

As the definition of `AllocTypeId` is up to the interpreter, this crate can't
know the type id of an array. Instead, we will require the interpreter to
implement a function on the `AllocHeader` trait:

```rust
pub trait AllocHeader: Sized {
    ...

    fn new_array(size: ArraySize, size_class: SizeClass, mark: Mark) -> Self;

    ...
}
```

This function should return a new object header for an array of u8 with the
appropriate type identifier.

We will also add a function to the `AllocRaw` trait for allocating arrays that
returns the `RawPtr<u8>` type.

```rust
pub trait AllocRaw {
    ...

    fn alloc_array(&self, size_bytes: ArraySize) -> Result<RawPtr<u8>, AllocError>;

    ...
}
```

Our complete `AllocRaw` trait definition now looks like this:

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocRaw}}
```

In the next chapter we'll build out the `AllocRaw` trait implementation.

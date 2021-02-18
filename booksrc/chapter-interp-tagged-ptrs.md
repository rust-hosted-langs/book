# Tagged pointers and object headers

Since our virtual machine will support a dynamic language where the compiler
does no type checking, all the type information will be managed at runtime.

In the previous chapter, we introduced a pointer type `ScopedPtr<T>`. This
pointer type has compile time knowledge of the type it is pointing at.

We need an alternative to `ScopedPtr<T>` that can represent all the
runtime-visible types so they can be resolved _at_ runtime.

As we'll see, carrying around type information or looking it up in the
header on every access will be inefficient space and performance-wise.

We'll implement a common optimization: tagged pointers.


## Runtime type identification

The object header can always give us the type id for an object, given a pointer
to the object. However, it requires us to do some arithmetic on the pointer
to get the location of the type identifier, then dereference the pointer to get
the type id value. This dereference can be expensive if the object being
pointed at is not in the CPU cache. Since getting an object type is a very
common operation in a dynamic language, these lookups become expensive,
time-wise.

Rust itself doesn't have runtime type _identification_ but does have runtime
dispatch through trait objects. In this scheme a pointer consists of two words:
the pointer to the object itself and a second pointer to the vtable where the
concrete object type's methods can be looked up. The generic name for this form
of pointer is a _fat_ pointer.

We could easily use a fat pointer type for runtime type identification
in our interpreter. Each pointer could carry with it an additional word with
the type id in it, or we could even just use trait objects!

A dynamically typed language will manage many pointers that must be type
identified at runtime. Carrying around an extra word per pointer is expensive,
space-wise, however.


## Tagged pointers

Many runtimes implement [tagged pointers][1] to avoid the space overhead, while
partially improving the time overhead of the header type-id lookup.

In a pointer to any object on the heap, the least most significant bits turn out
to always be zero due to word or double-word alignment.

On a 64 bit platform, a pointer is a 64 bit word. Since objects are
at least word-aligned, a pointer is always be a multiple of 8 and
the 3 least significant bits are always 0. On 32 bit platforms, the 2 least
significant bits are always 0.

      64..............48..............32..............16...........xxx
    0b1111111111111111111111111111111111111111111111111111111111111000
                                                                   / |
                                                                  /  |
                                                                unused

When dereferencing a pointer, these bits must always be zero. But we _can_ use
them in pointers at rest to store a limited type identifier! We'll limit
ourselves to 2 bits of type identifier so as to not complicate our code in
distinguishing between 32 and 64 bit platforms[^1].

Given we'll only have 4 possible types we can id directly from a pointer,
we'll still need to fall back on the object header for types that don't fit
into this range.


## Encoding this in Rust

Flipping bits on a pointer directly definitely constitutes a big Unsafe. We'll
need to make a tagged pointer type that will fundamentally be `unsafe` because
it won't be safe to dereference it. Then we'll need a safe abstraction over
that type to make it safe to dereference.

But first we need to understand the object header and how we get an object's
type from it.


### The object header

We introduced the object header traits in the earlier chapter
[Defining the allocation API](./chapter-allocation-api.md). The chapter
explained how the object header is the responsibility of the interpreter to
implement.

Now that we need to implement type identification, we need the object header.

The allocator API requires that the type identifier implement the
`AllocTypeId` trait. We'll use an `enum` to identify for all our runtime types:

```rust,ignore
{{#include ../interpreter/src/headers.rs:DefTypeList}}
```

Given that the allocator API requires every object that can be allocated to
have an associated type id `const`, this `enum` represents every type that
can be allocated and that we will go on to describe in this book.

It is a member of the `ObjectHeader` struct along with a few other members
that our Immix implementation requires:

```rust,ignore
{{#include ../interpreter/src/headers.rs:DefObjectHeader}}
```

The rest of the header members will be the topic of the later garbage
collection part of the book.


### A safe pointer abstraction

A type that can represent one of multiple types at runtime is obviously the
`enum`. We can wrap possible `ScopedPtr<T>` types like so:

```rust,ignore
{{#include ../interpreter/src/taggedptr.rs:DefValue}}
```

Note that this definition does _not_ include all the same types that were
listed above in `TypeList`. Only the types that can be passed dynamically at
runtime need to be represented here. The types not included here are always
referenced directly by `ScopedPtr<T>` and are therefore known types at
compile and run time.

You probably also noticed that `Value` _is_ the fat pointer we discussed
earlier. It is composed of a set of `ScopedPtr<T>`s, each of which should
only require a single word, and an `enum` discriminant integer, which will
also, due to alignment, require a word.

This `enum`, since it wraps `ScopedPtr<T>` and has the same requirement
for an explicit lifetime, is Safe To Dereference.

As this type occupies the same space as a fat pointer, it isn't the type
we want for storing pointers at rest, though. For that type, let's look at
the compact tagged pointer type now.


### What lies beneath

Below we have a `union` type, making this an unsafe representation of a pointer.
The `tag` value will be constrained to the values 0, 1, 2 or 3, which will
determine which of the next four possible members should be accessed. Members
will have to be bit-masked to access their correct values.

```rust,ignore
{{#include ../interpreter/src/taggedptr.rs:DefTaggedPtr}}
```

As you can see, we've allocated a tag for a `Symbol` type, a `Pair` type and
one for a numeric type. The fourth member indicates an object whose type
must be determined from the type id in the object header.

> ***Note:*** Making space for an inline integer is a common use of a tag. It
> means any integer arithmetic that fits within the available bits will not
> require memory lookups into the heap to retrieve operands. In our case we've
> defined the numeric type as an `isize`. Since the 2 least significant bits
> are used for the tag, we will have to right-shift the value by 2 to extract
> the correct integer value. We'll go into this implementation in more depth
> in a later chapter.

The tags and masks are defined as:

```rust,ignore
{{#include ../interpreter/src/pointerops.rs:TaggedPtrTags}}
```

Thus you can see from the choice of embedded tag values, we've optimized for
fast identification of `Pair`s and `Symbol`s and integer math. If we decide to,
it will be easy to switch to other types to represent in the 2 tag bits.

### Connecting into the allocation API

Translating between `Value` and `TaggedPtr` will be made easier by creating
an intermediate type that represents all types as an `enum` but doesn't require
a valid lifetime. This type will be useful because it is most closely
ergonomic with the allocator API and the object header type information.

```rust.ignore
{{#include ../interpreter/src/taggedptr.rs:DefFatPtr}}
```

We'll extend `Heap` (see previous chapter) with a method to return a tagged
pointer on request:

```rust,ignore
impl Heap {
{{#include ../interpreter/src/memory.rs:DefHeapAllocTagged}}
}
```

In this method it's clear that we implemented `From<T>` to convert
between pointer types. Next we'll look at how these conversions are
implemented.


## Type conversions

We have three pointer types: `Value`, `FatPtr` and `TaggedPtr`, each which
has a distinct flavor. We need to be able to convert from one to the other:

    TaggedPtr <-> FatPtr -> Value


### FatPtr to Value

We can implement `From<FatPtr>` for `TaggedPtr` and `Value`
to convert to the final two possible pointer representations.
Well, not exactly - the function signature

```rust,ignore
impl From<FatPtr> for Value<'guard> {
    fn from(ptr: FatPtr) -> Value<'guard> { ... }
}
```

is not able to define the `'guard` lifetime, so we have to implement a
similar method that can:

```rust,ignore
impl FatPtr {
{{#include ../interpreter/src/taggedptr.rs:DefFatPtrAsValue}}
}
```


### FatPtr to TaggedPtr

For converting down to a single-word `TaggedPtr` type we will introduce a helper
trait and methods to work with tag values and `RawPtr<T>` types from the
allocator:

```rust,ignore
{{#include ../interpreter/src/pointerops.rs:DefTagged}}
```

This will help convert from `RawPtr<T>` values in `FatPtr` to the `NonNull<T>`
based `TaggedPtr` discriminants.

Because `TaggedPtr` is a `union` type and because it has to apply the
appropriate tag value inside the pointer itself, we can't work with it as
ergnomically as an `enum`. We'll create some more helper functions for
instantiating `TaggedPtr`s appropriately.

Remember that for storing an integer in the pointer we have to left-shift it 2
bits to allow for the tag. We'll apply proper range checking in a later chapter.

```rust,ignore
impl TaggedPtr {
{{#include ../interpreter/src/taggedptr.rs:DefTaggedPtrNil}}

{{#include ../interpreter/src/taggedptr.rs:DefTaggedPtrNumber}}

{{#include ../interpreter/src/taggedptr.rs:DefTaggedPtrSymbol}}

{{#include ../interpreter/src/taggedptr.rs:DefTaggedPtrPair}}
}
```

Finally, we can use the above methods to implement `From<FatPtr` for `TaggedPtr`:

```rust,ignore
{{#include ../interpreter/src/taggedptr.rs:DefFromFatPtrForTaggedPtr}}
```


### TaggedPtr to FatPtr

To convert from a `TaggedPtr` to the intermediate type is implemented in two
parts: identifying object types from the tag; identifying object types from the
header where the tag is insufficient.

Part the first, which requires `unsafe` due to accessing a `union` type and
dereferencing the object header for the `TAG_OBJECT` discriminant:

```rust,ignore
{{#include ../interpreter/src/taggedptr.rs:FromTaggedPtrForFatPtr}}

impl TaggedPtr {
{{#include ../interpreter/src/taggedptr.rs:DefTaggedPtrIntoFatPtr}}
}
```

And part two, the object header method `get_object_fatptr()` as seen in the
code above:

```rust,ignore
impl ObjectHeader {
{{#include ../interpreter/src/headers.rs:DefObjectHeaderGetObjectFatPtr}}
}
```

This method contains no unsafe code and yet we've declared it unsafe!

Manipulating pointer types is not unsafe in of itself, only dereferencing them
is unsafe and we are not dereferencing them here.

While we have the safety rails of the `enum` types to prevent
_invalid_ types from being returned, we could easily mismatch a `TypeList` value
with an incorrect `FatPtr` value and return an _incorrect_ type. Additionally
we could forget to untag a pointer, leaving it as an invalid pointer value.

These possible mistakes could cause undefined behavior and quite likely crash
the interpreter.

The compiler will not catch these cases and so this is an area for critical
scrutiny of correctness! Hence the method is marked unsafe to draw attention.


## Using tagged pointers in data structures

Finally, we need to see how to use these types in data structures that we'll
create.

In the previous chapter, we defined a `CellPtr` type that wrapped a `RawPtr<T>`
in a `Cell<T>` so that data structures can contain mutable pointers to other
objects. Similarly, we'll want something to wrap tagged pointers:

```rust,ignore
{{#include ../interpreter/src/safeptr.rs:DefTaggedCellPtr}}
```

We'll also wrap `Value` in a type `TaggedScopedPtr` that we'll use similarly
to `ScopedPtr<T>`.

```rust,ignore
{{#include ../interpreter/src/safeptr.rs:DefTaggedScopedPtr}}
```

This `TaggedScopedPtr` carries an instance of `TaggedPtr` _and_ a `Value`.
This tradeoff means that while this type has three words to heft around,
the `TaggedPtr` member can be quickly accessed for copying into a
`TaggedCellPtr` without needing to down-convert from `Value`.

The type is only suitable for handling pointers that actively need to be
dereferenced due to it's size.

> ***Note:*** Redundancy: TaggedScopedPtr and Value are almost
> identical in requirement and functionality.
> TODO: consider merging into one type.
> See issue <https://github.com/rust-hosted-langs/book/issues/30>

A `TaggedScopedPtr` can be obtained by:

* calling `TaggedCellPtr::get()`
* or the `MutatorView::alloc_tagged()` method

The `get()` method on `TaggedCellPtr` returns a `TaggedScopedPtr`:

```rust,ignore
impl TaggedCellPtr {
{{#include ../interpreter/src/safeptr.rs:DefTaggedCellPtrGet}}
}
```

The `MutatorView` method to allocate a new object and get back a tagged
pointer (a `TaggedScopedPtr`) looks simply like this:

```rust,ignore
impl MutatorView {
{{#include ../interpreter/src/memory.rs:DefMutatorViewAllocTagged}}
}
```


## Quick recap

In summary, what we created here was a set of pointer types:

* types suitable for storing a pointer at rest - `TaggedPtr` and `TaggedCellPtr`
* types suitable for dereferencing a pointer - `Value` and `TaggedScopedPtr`
* a type suitable for intermediating between the two - `FatPtr` - that the
  heap allocation interface can return

We now have the basic pieces to start defining data structures for our
interpreter, so that is what we shall do next!

----

[^1]: There are other pointer tagging schemes, notably the use of "spare" NaN
bit patterns in 64 bit floating point values. Further, _which_ types are
best represented by the tag bits is highly language dependent. Some languages
use them for garbage collection information while others may use them for
still other types hidden from the language user. In the interest of clarity,
we'll stick to a simple scheme.


[1]: https://en.wikipedia.org/wiki/Tagged_pointer

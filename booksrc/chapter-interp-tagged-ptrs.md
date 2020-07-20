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
the type id in it, or we could even just use trait objects directly!

A dynamically typed language will manage many pointers that must be type
identified at runtime. Carrying around an extra word per pointer is expensive,
space-wise.


## Tagged pointers

Many runtimes implement [tagged pointers](1) to avoid the space overhead, while
partially improving the time overhead of the header type-id lookup.

In a pointer to any object on the heap, the least most significant bits turn out
to always be zero due to word or double-word alignment.

On a 64 bit platform, a pointer will be a 64 bit word. Since objects will be
at least word-aligned - a pointer will always be a multiple of 8 - that means
that there are 3 bits that are always 0. On 32 bit platforms, the 2 least
significant bits are always 0.

      64..............48..............32..............16...........xxx
    0b1000001111011101101100101010101010010101010011110110101111101000
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

But first we need to understand the object header.


### The allocation object header

We introduced the object header traits in the earlier chapter
[Defining the allocation API](./chapter-allocation-api.md). The chapter
explained how the object header is the responsibility of the interpreter to
implement.

Now that we need to implement type identification, we need the object header
implementation first.

The allocator API requires that the type identifier implement the
`AllocTypeId` trait. We'll use an `enum` to identify for all our runtime types:

```rust,ignore
{{#include ../interpreter/src/headers.rs:DefTypeList}}
```

Given that the allocator API requires every object that can be allocated to
have an associated type id `const`, this enum represents every type that
can be allocated and that we will go on to describe in this book.

This type identifier `enum` is a member of the `ObjectHeader` struct along
with a few other members that our Immix implementation requires:

```rust,ignore
{{#include ../interpreter/src/headers.rs:DefObjectHeader}}
```

The rest of the header members will be the topic of the later garbage
collection part of the book.

> ***Note:*** While we are using an `enum` for clarity, we could choose other
> means of type identification that would be more flexible. For example,
> we could generate a lookup table of compile-time generated ids to
> trait objects.


### A safe pointer abstraction

Starting with the safe abstraction, a type that can represent one of multiple
types at runtime is obviously the `enum`.
We can wrap possible `ScopedPtr<T>` types in an `enum`:

```rust,ignore
{{#include ../interpreter/src/taggedptr.rs:DefValue}}
```

Notice that this `enum` does _not_ include all the same types that were
listed above in `TypeList`. Only the types that can be passed dynamically at
runtime need to be represented here. Other types not listed here are known
at runtime.

You probably also noticed that `Value` _is_ the fat pointer we discussed
earlier. It is composed of a set of `ScopedPtr<T>`s, each of which should
only require a single word, and an enum discriminant integer, which will
also, due to alignment, require a word.

This enum, since it wraps `ScopedPtr<T>` and has the same requirement
for an explicit lifetime, is Safe To Dereference.

As this type occupies the same space as a fat pointer, it isn't the type
we want for storing pointers at rest, though.

For that type, let's look at the compact tagged pointer type now.


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

Translating between `Value` and `TaggedPtr` will be made easier by creating
an intermediate type that represents all types as an enum but doesn't require
a valid lifetime. This type will be useful because it is most closely
ergonomic with the allocator API and the object header type information.

```rust.ignore
{{#include ../interpreter/src/taggedptr.rs:DefFatPtr}}
```

Next we'll look at how to convert between `FatPtr`, `TaggedPtr` and `Value`.


## Type conversions

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

To convert from a `TaggedPtr` to the intermediate type, we need to access the
object header. The header object itself will own the method for returning a
`FatPtr`.

```rust,ignore
impl ObjectHeader {
{{#include ../interpreter/src/headers.rs:DefObjectHeaderGetObjectFatPtr}}
}
```

## Tagged pointers in data structures

TODO: TaggedCellPtr, TaggedScopedPtr

----

[^1]: There are other pointer tagging schemes, notably the use of "spare" NaN
bit patterns in 64 bit floating point values. Further, _which_ types are
best represented by the tag bits is highly language dependent. Some languages
use them for garbage collection information while others may use them for
still other types hidden from the language user. In the interest of clarity,
we'll stick to a simple scheme.


[1]: https://en.wikipedia.org/wiki/Tagged_pointer

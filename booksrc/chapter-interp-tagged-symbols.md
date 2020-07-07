# Tagged pointers and symbols

In the previous chapter, we introduced a pointer type `ScopedPtr<T>`. This
pointer type has compile time knowledge of the type it is pointing at.

In our interpreter we won't always have that. As a dynamic language
interpreter, our compiler won't do type checking. We'll depend on runtime
type identification in our virtual machine.

In Python, for example, the following code does not have compile time
protection against passing in strings:

```python
def multiply(a, b):
    return a * b

multiply("bob", "alice")
```

This script will result in a runtime error and not a compile time error.
As a dynamically typed interpreter, our language will behave similarly.

For this to work, we need an alternative to `ScopedPtr<T>` that does not
care about compile time types _but_ from which the type can be resolved
at runtime.

We'll spend some time now inventing some new pointer types to support this.

## Runtime type identification

The object header can always give us the type id for an object, given a pointer
to the object. However, it requires us to dereference the pointer, do some
arithmetic on the pointer to get the header, then further arithmetic to get
the type id in the header.

Rust itself doesn't have runtime type _identification_ but does have runtime
dispatch through trait objects. In this scheme a pointer consists of two words:
the pointer to the object itself and a second pointer to the vtable where the
concrete object type's methods can be looked up. The generic name for this form
of pointer is a _fat_ pointer.

We could easily use a fat pointer type for runtime type identification
in our interpreter. Each pointer could carry with it an additional word with
the type id in it, or we could even just use trait objects directly!

A dynamically typed language will manage many pointers that must be type
identified at runtime. Carrying around an extra word per pointer is expensive!
A common optimization in many runtimes is to use [tagged pointers](1).

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

### Tagged pointer types

Flipping bits on a pointer directly definitely constitutes a big Unsafe. We'll
need to make a tagged pointer type that will fundamentally be `unsafe` because
it won't be safe to dereference it. Then we'll need a safe abstraction over
that type to make it safe to dereference.

In the previous chapter we defined a pointer type that retained compile time
type knowledge. We will still need to be able to resolve to concrete types at
runtime. We can use our previously defined pointer type, though:

```rust,ignore
#[derive(Copy, Clone)]
pub enum Value<'guard> {
    Nil,
    Pair(ScopedPtr<'guard, Pair>),
    Symbol(ScopedPtr<'guard, Symbol>),
}
```

Here we wrapped possible variants of `ScopedPtr<T>` in an enum, and we've
started by defining two more new types, `Pair` and `Symbol`, that we'll go
on to explain shortly.

You probably noticed that `Value` is essentially a fat pointer. It is composed
of a set of `ScopedPtr<T>` values, each of which should only require a single
word, and an enum discriminant value, which will also, due to alignment,
require a word. We'll end up with a lot more discriminants so at this point,
we can't do tagged pointer trickery, the discriminant value will not fit into
2 bits.

This enum, however, since it wraps `ScopedPtr<T>` and has the same requirement
for an explicit lifetime, is Safe To Dereference.

Since this type occupies the same space as a fat pointer, it isn't the type
we want for storing pointers at rest. Let's look at the compact tagged pointer
type now:

```rust,ignore
{{#include ../interpreter/src/taggedptr.rs:DefTaggedPtr}}
```

Here we have a `union` type, making this an unsafe representation of a pointer.
The `tag` value will be constrained to the values 0, 1, 2 or 3, which will
determine which of the next four possible members should be accessed. Members
will have to be bit-masked to access their correct values.

These tags and masks are defined as:

```rust,ignore
{{#include ../interpreter/src/pointerops.rs:TaggedPtrTags}}
```

As you can see, we've allocated a tag for a `Symbol` type, a `Pair` type and
one for a numeric type. The fourth member indicates an object whose type
must be determined from the type id in the object header.

Making space for an inline integer type is a not-uncommon use of a tag. It
means any integer arithmetic that fits within the available bits will not
require memory lookups into the heap to retrieve operands. In our case we've
defined the numeric type as an `isize`. Since the 2 least significant bits
are used for the tag, we will have to right-shift the value by 2 to extract
the correct integer value.

Thus you can see from the choice of embedded tag values, we've optimized for
identifying `Pair`s and `Symbol`s and integer math.

Translating between `Value` and `TaggedPtr` will be made easier by creating
an intermediate type that represents all types as an enum but doesn't require
a valid lifetime.

```rust.ignore
#[derive(Copy, Clone)]
pub enum FatPtr {
    Nil,
    Pair(RawPtr<Pair>),
    Symbol(RawPtr<Symbol>),
}
```

In this representation we encapsulate the `RawPtr<T>` type that the allocator
API gives us. We will also implement `From<FatPtr>` for `TaggedPtr` and `Value`
to convert to the final two possible pointer representations.


----

[^1]: There are other pointer tagging schemes, notably the use of "spare" NaN
bit patterns in 64 bit floating point values. Further, _which_ types are
best represented by the tag bits is highly language dependent. Some languages
use them for garbage collection information while others may use them for
still other types hidden from the language user. In the interest of clarity,
we'll stick to a simple scheme.

[1]: https://en.wikipedia.org/wiki/Tagged_pointer

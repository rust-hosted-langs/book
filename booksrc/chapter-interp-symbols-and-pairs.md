# Symbols and Pairs

To bootstrap our compiler, we'll parse s-expressions into `Symbol` ad `Pair`
types, where a `Pair` is essentially a Lisp cons cell.

The definition of `Symbol` is just the raw components of a `&str`:

```rust,ignore
{{#include ../interpreter/src/symbol.rs:DefSymbol}}
```

How we handle these raw components will be covered in just a bit.
First though, we'll delve into the `Pair` type.


## Pairs of pointers

The definition of `Pair` is

```rust,ignore
{{#include ../interpreter/src/pair.rs:DefPair}}
```

The type of `first` and `second` is `TaggedCellPtr`, as seen in the previous
chapter. This pointer type can point at any runtime supported type. By the
end of this chapter we'll be able to build a nested linked list of `Pair`s
and `Symbol`s.

Since this structure will be used for parsing and compiling, the `Pair`
`struct` has a couple of extra members that optionally describe the source
code line and character number of the values pointed at by `first` and
`second`. We'll come back to these in the chapter on parsing.

To instantiate a `Pair` function with `first` and `second` set to `Nil`, let's
create a `new()` function:

```rust,ignore
impl Pair {
{{#include ../interpreter/src/pair.rs:DefPairNew}}
}
```

That function, as it's not being allocated into the heap, doesn't require the
lifetime guard. Let's look at a more interesting function: `cons()`, which
assigns a value to `first` and `second` and puts the `Pair` on to the heap:

```rust,ignore
{{#include ../interpreter/src/pair.rs:DefCons}}
```

Here we have the lifetime `'guard` associated with the `MutatorView` instance
which grants access to the `alloc_tagged()` method.

The other two args, `head` and `rest` are required to share the same `'guard`
lifetime as the `MutatorView` instance, or rather, at least `'guard` must be
a subtype of their lifetimes. Their values, of type `TaggedScopedPtr<'guard>`,
can be written directly to the `first` and `second` members of `Pair` with
`TaggedCellPtr::set()`.

The only other piece to add, since `Pair` must be able to be passed into
our allocator API, is the `AllocObject` impl for `Pair`:

```rust,ignore
impl AllocObject<TypeList> for Pair {
    const TYPE_ID: TypeList = TypeList::Pair;
}
```

This impl will repeat for every type in `TypeList` so it'll be a great candidate
for a macro.

And that's it! We have a cons-cell style `Pair` type and some elementary
methods for creating and allocating them.

Now, back to `Symbol`, which seems like it should be even simpler, but as we'll
see has some nuance to it.


## Symbols and pointers

TODO

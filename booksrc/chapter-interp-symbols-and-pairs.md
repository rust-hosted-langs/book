# Symbols and Pairs

To bootstrap our compiler, we'll parse s-expressions into `Symbol` ad `Pair`
types, where a `Pair` is essentially a Lisp cons cell.

The definition of `Symbol` is just the raw components of a `&str`:

```rust,ignore
{{#include ../interpreter/src/symbol.rs:DefSymbol}}
```

Why this is how `Symbol` is defined and how we handle these raw components will
be covered in just a bit. First though, we'll delve into the `Pair` type.


## Pairs of pointers

The definition of `Pair` is

```rust,ignore
{{#include ../interpreter/src/pair.rs:DefPair}}
```

The type of `first` and `second` is `TaggedCellPtr`, as seen in the previous
chapter. This pointer type can point at any dynamic type. By the
end of this chapter we'll be able to build a nested linked list of `Pair`s
and `Symbol`s.

Since this structure will be used for parsing and compiling, the `Pair`
`struct` has a couple of extra members that optionally describe the source
code line and character number of the values pointed at by `first` and
`second`. These will be useful for reporting error messages. We'll come back
to these in the chapter on parsing.

To instantiate a `Pair` function with `first` and `second` set to nil, let's
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
which grants access to the allocator `alloc_tagged()` method and the getter
and setter on `TaggedScopedPtr`.

The other two args, `head` and `rest` are required to share the same `'guard`
lifetime as the `MutatorView` instance, or rather, `'guard` must at least be
a subtype of their lifetimes. Their values, of type `TaggedScopedPtr<'guard>`,
can be written directly to the `first` and `second` members of `Pair` with
the setter `TaggedCellPtr::set()`.

We'll also add a couple `impl` methods for appending an object to a `Pair`
in linked-list fashion:

```rust,ignore
impl Pair {
{{#include ../interpreter/src/pair.rs:DefPairAppend}}
}
```

This method, given a value to append, creates a new `Pair` whose member `first`
points at the value, then sets the `second` of the `&self` `Pair` to that new
`Pair` instance. This is in support of s-expression notation `(a b)` which
describes a linked-list of `Pair`s arranged, in pseudo-Rust:

```
Pair {
    first: a,
    second: Pair {
        first: b,
        second: nil,
    },
}
```

The second method is for directly setting the value of the `second` for
s-expression dot-notation style: `(a . b)` is represented by `first` pointing
at `a`, dotted with `b` which is pointed at by `second`. In our pseudo
representation:

```
Pair {
    first: a,
    second: b,
}
```

The implementation is simply:

```rust,ignore
impl Pair {
{{#include ../interpreter/src/pair.rs:DefPairDot}}
}
```

The only other piece to add, since `Pair` must be able to be passed into
our allocator API, is the `AllocObject` impl for `Pair`:

```rust,ignore
impl AllocObject<TypeList> for Pair {
    const TYPE_ID: TypeList = TypeList::Pair;
}
```

This impl pattern will repeat for every type in `TypeList` so it'll be a great
candidate for a macro.

And that's it! We have a cons-cell style `Pair` type and some elementary
methods for creating and allocating them.

Now, back to `Symbol`, which seems like it should be even simpler, but as we'll
see has some nuance to it.


## Symbols and pointers

Let's recap the definition of `Symbol` and that it is the raw members of a
`&str`:

```rust,ignore
{{#include ../interpreter/src/symbol.rs:DefSymbol}}
```

By this definition, a symbol has a name string, but does not own the string
itself. What means this?

Symbols are in fact pointers to interned strings. Since each symbol points
to a unique string, we can identify a symbol by it's pointer value rather than
needing to look up the string itself.

However, symbols do need to be discovered by their string name, and symbol
pointers must dereference to return their string form. i.e. a we need a
bidirectional mapping of string to pointer and pointer to string.

In our implementation, we use a `HashMap<String, RawPtr<Symbol>>` to map from
name strings to symbol pointers, while the `Symbol` object itself points back
to the name string.

This is encapsulated in a `SymbolMap` struct:

```rust,ignore
{{#include ../interpreter/src/symbolmap.rs:DefSymbolMap}}
```

where we use `RefCell` to wrap operations in interior mutability, just like
all other allocator functionality.

The second struct member `Arena` requires further explanation: since symbols are
unique strings that can be identified and compared by their pointer values,
these pointer values must remain static throughout the program lifetime.
Thus, `Symbol` objects cannot be managed by a heap that might perform object
relocation. We need a separate heap type for objects that are never
moved or freed unil the program ends, the `Arena` type.

The `Arena` type is simple. It, like `Heap`, wraps `StickyImmixHeap` but
unlike `Heap`, it will never run garbage collection.

```rust,ignore
{{#include ../interpreter/src/arena.rs:DefArena}}
```

The `ArenaHeader` is a simple object header type to fulfill the allocator
API requirements but whose methods will never be needed.

Allocating a `Symbol` will use the `Arena::alloc()` method which calls through
to the `StickyImmixHeap` instance.

We'll add a method for getting a `Symbol` from it's name string to the
`SymbolMap` at the allocator API level:

```rust,ignore
impl SymbolMap {
{{#include ../interpreter/src/symbolmap.rs:DefSymbolMapLookup}}
}
```

Then we'll add wrappers to the `Heap` and `MutatorView` impls to scope-restrict
access:

```rust,ignore
impl Heap {
{{#include ../interpreter/src/memory.rs:DefHeapLookupSym}}
}
```

and

```rust,ignore
impl<'memory> MutatorView<'memory> {
{{#include ../interpreter/src/memory.rs:DefMutatorViewLookupSym}}
}
```

This scope restriction is absolutely necessary, despite these objects never
being freed or moved during runtime. This is because `Symbol`, as a standalone
struct, remains unsafe to use with it's raw `&str` components. These components
can only safely be accessed when there is a guarantee that the backing
`Hashmap` is still in existence, which is only when the `MutatorView` is
accessible.

Two methods on `Symbol` guard access to the `&str`, one unsafe to reassemble
the `&str` from raw components, the other safe when given a `MutatorScope`
guard instance.

```rust,ignore
impl Symbol {
{{#include ../interpreter/src/symbol.rs:DefSymbolUnguardedAsStr}}

{{#include ../interpreter/src/symbol.rs:DefSymbolAsStr}}
}
```

Finally, to make `Symbol`s allocatable in the Sticky Immix heap, we need to
implement `AllocObject` for it:

```rust,ignore
impl AllocObject<TypeList> for Symbol {
    const TYPE_ID: TypeList = TypeList::Symbol;
}
```


## Moving on swiftly

Now we've got the elemental pieces of s-expressions, lists and symbols, we can
move on to parsing s-expression strings.

Since the focus of this book is the underlying mechanisms of memory management
in Rust and the details of runtime implementation, parsing will receive less
attention. We'll make it quick!

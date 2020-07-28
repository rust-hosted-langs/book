# Symbols and Pairs

To bootstrap our compiler, we'll parse s-expressions into `Symbol` ad `Pair`
types, where a `Pair` is essentially a Lisp cons cell.

The definition of `Symbol` is just the raw components of a `&str`:

```rust,ignore
{{#include ../interpreter/src/symbol.rs:DefSymbol}}
```

How we handle these raw components will be covered later in this chapter.
First, we'll describe the `Pair` type.


## Pairs of pointers

The definition of `Pair` is

```rust,ignore
{{#include ../interpreter/src/pair.rs:DefPair}}
```

The type of `first` and `second` is `TaggedCellPtr` as seen in the previous
chapter. This pointer type can point at any runtime supported type. By the
end of this chapter we'll be able to build a nested linked list of `Pair`s
and `Symbol`s.

Since this structure will be used for parsing and compiling, the `Pair`
`struct` has a couple of extra members that optionally describe the source
code line and character number of the values pointed at by `first` and
`second`. We'll come back to these in the chapter on parsing.

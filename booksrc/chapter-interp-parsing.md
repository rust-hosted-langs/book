# Parsing s-expressions

We'll make this quick. It's not the main focus of this book and the topic is
better served by seeking out other resources that can do it justice.

In service of keeping it short, we're parsing s-expressions and we'll start
by considering only symbols and parentheses. We could hardly make it simpler.


## The interface

The interface we want should take a `&str` and return a `TaggedScopedPtr`.
We want the tagged version of the scoped ptr because the return value might
point to either a `Pair` or a `Symbol`. Examples of valid input are:

* `a-symbol`: a `Symbol` with name "a-symbol"
* `(this is a list)`: a linked list of `Pair`s, each with the `first` value
  pointing to a `Symbol`
* `(this (is a nested) list)`: a linked list, as above, containing a nested
  linked list
* `(this () is a nil symbol)`: the two characters `()` together are equivalent
  to the special symbol `nil`, also the value `0` in our `TaggedPtr` type
* `(one . pair)`: a single `Pair` instance with `first` pointing at the `Symbol`
  for "one" and `second` at the `Symbol` for "two"

Our internal implementation is split into tokenizing and then parsing the
token stream. Tokenizing takes the `&str` input and returns a `Vec<Token>`
on success:

```rust,ignore
fn tokenize(input: &str) -> Result<Vec<Token>, RuntimeError>;
```

The return `Vec<Token>` is an intermediate, throwaway value, and does not
interact with our Sticky Immix heap. Parsing takes the `Vec<Token>` and
returns a `TaggedScopedPtr` on success:

```rust,ignore
fn parse_tokens<'guard>(
    mem: &'guard MutatorView,
    tokens: Vec<Token>,
) -> Result<TaggedScopedPtr<'guard>, RuntimeError>;
```


## Tokens, a short description

The full set of tokens we will consider parsing is:

```rust,ignore
{{#include ../interpreter/src/lexer.rs:DefTokenType}}
```

We combine this enum with a source input position indicator to compose the
`Token` type. This source position is defined as:

```rust,ignore
{{#include ../interpreter/src/error.rs:DefSourcePos}}
```

And whenever it is available to return as part of an error, error messages can
be printed with the relevant source code line.

The `Token` type;

```rust,ignore
{{#include ../interpreter/src/lexer.rs:DefToken}}
```


## Parsing, a short description

The key to quickly writing a parser in Rust is the `std::iter::Peekable`
iterator which can be obtained from the `Vec<Token>` instance with
`tokens.iter().peekable()`. This iterator has a `peek()` method that allows
you to look at the next `Token` instance without advancing the iterator.

Our parser, a hand-written recursive descent parser, uses this iterator type
to look ahead to the next token to identify primarily whether the next token
is valid in combination with the current token, or to know how to recurse
next without consuming the token yet.

For example, an open paren `(` followed by a symbol would start a new `Pair`
linked list, recursing into a new parser function call, but if it is
immediately followed by a close paren `)`, that is `()`, it is equivalent to
the symbol `nil`, while otherwise `)` _terminates_ a `Pair` linked list and
causes the current parsing function instance to return.

Another case is the `.` operator, which is only valid in the following pattern:
`(a b c . d)` where `a`, `b`, `c`, and `d` must be symbols or nested lists.
A `.` must be followed by a single expression followed by a `)`.

Tokenizing and parsing are wrapped in a function that takes the input `&str`
and gives back the `TaggedScopedPtr`:

```rust,ignore
{{#include ../interpreter/src/parser.rs:DefParse}}
```

Notice that this function and `parse_tokens()` require the
`mem: &'guard MutatorView` parameter. Parsing creates `Symbol` and `Pair`
instances in our Sticky Immix heap and so requires the scope-restricted
`MutatorView` instance.

This is all we'll say on parsing s-expressions. In the next chapter we'll do
something altogether more informative with regards to memory management
and it'll be necessary by the time we're ready to compile: arrays!

# Compiler: Design

Drawing from the [VM design](./chapter-interp-vm-design.md), the compiler must
support the following language constructs:

* function definitions
* anonymous functions
* function calls
* lexical scoping
* closures
* local variables
* global variables
* expressions

This is a minimally critical set of features that any further language
constructs can be built on while ensuring that our compiler remains easy to
understand for the purposes of this book.

[Our parser, recall](./chapter-interp-parsing.md), reads in s-expression syntax
and produces a nested `Pair` and `Symbol` based data structure. Adding other
types - integers, strings, arrays etc - is mostly a matter of expanding the
parser.  The compiler as described here, being for a dynamically typed
language, will support them without refactoring.

## Eval/apply

Our compiler design is fundamentally based on the eval/apply pattern.

In this pattern we recursively descend into the `Pair` tree, beginning with
calling eval with the root node.

The simplest understanding of eval/apply is that eval looks at leaves of
the tree while apply looks at compositions of leaves.

### Eval

Eval looks at the given node and attempts to generate an instruction for it
that would resolve the node to a value - that is, evaluate it.

#### Symbols

If the node is a special symbol, such as `nil` or `true`, then it is treated as
a literal and an instruction is generated to load that literal symbol into the
next available register.

Otherwise if the node is any other symbol, it is assumed to be bound to a value
(it must be a variable) and an instruction is generated for fetching the value
into a register.

Variables come in three kinds: local, non-local or global.

**Local**: the symbol has been declared earlier in the expression using `let` and
the compiler already has a record of that - the symbol is associated with a
local register index and a simple lookup instruction is generated.

**Non-local**: the symbol has been bound in a parent function. Again, the
compiler has a record of the declaration, which register is associated with the
symbol and which relative call frame will contain that register. An upvalue
lookup instruction is generated.

**Global**: if the symbol isn't found as a local binding or a non-local binding,
it is assumed to be a global, and a late-binding global lookup instruction is
generated. In the event the programmer has misspelled a variable name, this is
possibly the instruction that will be generated and the programmer will see an
unknown-variable error at runtime.

#### Expressions and function calls

When eval is passed a `Pair`, this represents the beginning of an expression,
or a function call. A composition of things.

In s-expression syntax, a function call looks like `(function_name arg1 arg2)`.
That is parsed into a `Pair` tree, which takes the form:

```
Pair(
  Symbol(function_name),
  Pair(
    Symbol(arg1),
    Pair(
      Symbol(arg2),
      nil
    )
  )
)
```

It is apply's job to handle this case, so eval extracts the first and second
values from the outermost `Pair` and passes them into apply. 

### Apply

## Register allocation

# Compiler: Implementation

Before we get into eval and apply let's consider how we will support variables
and lexical scoping.

## Variables and Scopes

As seen in the previous chapter, variable accesses come in three types, as far
as the compiler and VM are concerned: local, nonlocal and global. Each access
uses a different bytecode operation, and so the compiler must be able to
determine what operations to emit at compile time.

Given that we have named function parameters and `let`, we have syntax for
explicit variable declaration within function definitions. This means that we
can easily keep track of whether a variable reference is local, nonlocal or
global.

If a variable wasn't declared as a parameter or in a `let` block, it
must be global. Global variables are accessed dynamically by name.

As far as local and nonlocal variables are concerned, the VM does not care
about or consider their names. At the VM level, local and nonlocal variables
are numbered registers. That is, each function's local variables are mapped to
a register numbered between 2 and 255. The compiler must generate the mapping
from variable names to register numbers.

For generating and maintaining mappings, we need data structures for keeping
track of:

- function local variables and their mappings to register numbers
- references to nonlocal variables and their relative stack offsets
- nested scopes within functions

### Named variables

Our first data structure will define a register based variable:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefVariable}}
```

For every named, non-global variable (created by defining parameters and `let`
blocks) a `Variable` instance is created in the compiler.

The member `closed_over` defaults to `false`. If the compiler detects that the
variable must escape the stack as part of a closure, this flag will be flipped
to `true`.

### Scope structure

The data structures that manage nesting of scopes and looking up a `Variable`
by name are defined here.

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefScope}}

{{#include ../interpreter/src/compiler.rs:DefNonlocal}}

{{#include ../interpreter/src/compiler.rs:DefVariables}}
```

For every function defined, the compiler maintains an instance of the type
`Variables`.

Each function's `Variables` has a stack of `Scope` instances, each of which has
it's own set of name to `Variable` register number mappings.  The outermost
`Scope` contains the mapping of function parameters to registers.

A nested function's `Variables`, when the function refers to a nesting
function's variable, builds a mapping of nonlocal variable name to relative
stack position of that variable. This is a `NonLocal` - a relative stack frame
offset and the register number within that stack frame of the variable.

In summary, under these definitions:

- `Scope` manages the mapping of a variable name to the `Variable` register
  number within a single scope
- A `Nonlocal` instance references a relative stack location of a nonlocal
  variable for compiling upvalues
- `Variables` maintains all the nested scopes for a function during compilation
  and caches all the nonlocal references. It also keeps a reference to a parent
  nesting function if there is one, in order to handle lexically scoped
  lookups.

### Retrieving named variables

Whenever a variable is referenced in source code, the mapping to it's register
must be looked up. The result of a lookup is a `Binding` instance:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefBinding}}
```

The lookup process checks the local function scopes first.

If the variable is found to be declared there, the `Local` enum variant is
returned. In terms of bytecode, this will translate to a direct register
reference.

Next, any outer function scopes are searched. If the variable is found in any
outer scope, an `Upvalue` variant is returned. The compiler will emit instructions
to copy the value refered to by the upvalue into a function-local temporary
register.

If the lookup for the variable returns nothing, a global lookup instruction is
emitted that will, if the name exists as a globally bound value, copy the
result of the lookup into a function-local temporary register.

## Eval/apply

Recall that:

_Eval looks at the given node and attempts to generate an instruction for it
that would resolve the node to a value - that is, evaluate it;_

while:

_apply takes a function name and a list of arguments. First it recurses into
eval for each argument expression, then generates instructions to call the
function with the argument results._

Let's look at some examples of eval.

```rust,ignore
        match *ast_node {
            ...

{{#include ../interpreter/src/compiler.rs:DefCompileEvalPair}}

            ...
        }
```

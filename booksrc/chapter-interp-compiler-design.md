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
and produces a nested `Pair` and `Symbol` based abstract syntax tree. Adding
other types - integers, strings, arrays etc - is mostly a matter of expanding
the parser.  The compiler as described here, being for a dynamically typed
language, will support them without refactoring.

## Eval/apply

Our compiler design is based on the _eval/apply_ pattern.

In this pattern we recursively descend into the `Pair` AST, beginning with
calling _eval_ on the root node of the expression to be compiled.

_Eval_ is, of course, short for "evaluate" - we want to evaluate the given
expression. In our case, the immediate requirement is not the result but the
sequence of instructions that will generate the result, that will evaluate the
expression.

More concretely, _eval_ looks at the node in the AST it is given and if it
resolves to fetching a value for a variable, it generates that instruction;
otherwise if it is a function call, the arguments are evaluated and then the
function and arguments are passed to _apply_, which generates appropriate
function call instructions.

### Implementing Eval

_Eval_ looks at the given node and attempts to generate an instruction for it
that would resolve the node to a value - that is, evaluate it.

#### Symbols

If the node is a special symbol, such as `nil` or `true`, then it is treated as
a literal and an instruction is generated to load that literal symbol into the
next available register.

Otherwise if the node is any other symbol, it is assumed to be bound to a value
(it must be a variable) and an instruction is generated for fetching the value
into a register.

Variables come in three kinds: local, nonlocal or global.

**Local**: the symbol has been declared earlier in the expression using `let` and
the compiler already has a record of that - the symbol is associated with a
local register index and a simple lookup instruction is generated.

**Nonlocal**: the symbol has been bound in a parent nesting function. Again,
the compiler has a record of the declaration, which register is associated with
the symbol and which relative call frame will contain that register. An upvalue
lookup instruction is generated.

**Global**: if the symbol isn't found as a local binding or a nonlocal binding,
it is assumed to be a global, and a late-binding global lookup instruction is
generated. In the event the programmer has misspelled a variable name, this is
possibly the instruction that will be generated and the programmer will see an
unknown-variable error at runtime.

#### Expressions and function calls

When _eval_ is passed a `Pair`, this represents the beginning of an expression,
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

It is _apply_'s job to handle this case, so _eval_ extracts the first and
second values from the outermost `Pair` and passes them into apply. In more
general terms, _eval_ calls _apply_ with the function name and the argument
list and leaves the rest up to _apply_.

### Implementing Apply

_Apply_ takes a function name and a list of arguments. It generates
instructions to first evaluate each argument, then call the function with the
argument results.

#### Calling functions

Functions are either built into to the language and VM or are
library/user-defined functions composed of other functions.

In every case, the simplified pattern for function calls is:

* allocate a register to write the return value into
* _eval_ each of the arguments in sequence, allocating their resulting values 
  into consequent registers
* compile the function call opcode, giving it the number of argument registers
  it should expect

Compiling a call to a builtin function might translate directly to a dedicated
bytecode operation. For example, querying whether a value is `nil` with builtin
function `nil?` compiles 1:1 to a bytecode operation that directly represents
that query.

Compiling a call to a user defined function is a more involved. In it's more
general form, supporting first class functions and closures, a function call
requires two additional pointers to be placed in registers. The complete
function call register allocation looks like this:

| Register | Use |
|----------|-----|
| 0 | reserved for return value |
| 1 | reserved for closure environment pointer |
| 2 | first argument |
| 3 | second argument |
| ... | |
| n | function pointer |

If a closure is called, the closure object itself contains a pointer to it's
environment and the function to call and those pointers can be copied over to
registers. Otherwise, the closure environment pointer will be a `nil` pointer.

The VM, when entering a new function, will represent the return value register
always as the zeroth register.

When the function call returns, all registers except the return value are
discarded.

#### Compiling functions

Let's look at a simple function definition:

```
(def is_true (x) 
  (is? x true))
```

This function has a name `is_true`, takes one argument `x` and evaluates one
expression `(is? x true)`.

The same function may be written without a name:

```
(lambda (x) (is? x true))
```

Compiling a function requires a few inputs:

* an optional reference to a parent nesting function
* an optional function name
* a list of argument names
* a list of expressions that will compute the return value

The desired output is a data structure that combines:

* the optional function name
* the argument names
* the compiled bytecode

First, a scope structure is established. A scope is a lexical nesting block in
which variables are bound and unbound. In the compiler, this structure is
simply a mapping of variable name to the register number that contains the
value.

The first variables to be bound in the function's scope are the argument names.
The compiler, given the list of argument names to the function and the order in
which the arguments are given, associates each argument name with the register
number that will contain it's value. As we saw above, these are predictably and
reliably registers 2 and upward, one for each argument.

A scope may have a parent scope if the function is defined within another
function. This is how nonlocal variable references will be looked up. We will
go further into that when we discuss closures.

The second step is to _eval_ each expression in the function, assigning the
result to register 0, the preallocated return value register. The result of
compiling each expression via _eval_ is bytecode.

Thirdly and finally, a function object is instantiated, given it's name, the
argument names and the bytecode.

#### Compiling closures

During compilation of the expressions within a function, if any of those
expressions reference nonlocal variables (that is, variables not declared
within the scope of the function) then the function object needs additional
data to describe how to access those nonlocal variables at runtime.

In the below example, the anonymous inner function references the parameter
`n` to the outer function, `n`. When the inner function is returned, the value
of `n` must be carried with it even after the stack scope of the outer function
is popped and later overwritten with values for other functions.

```
(def make_adder (n) 
  (lambda (x) (+ x n))
)
```

_Eval_, when presented with a symbol to evaluate that has not been declared in
the function scope, searches outer scopes next. If a binding is found in an
outer scope, a nonlocal reference is added to the function's _local_ scope
that points to the outer scope and a `GetUpvalue` instruction is compiled.

This nonlocal reference is a combination of two values: a count of stack
frames to skip over to find the outer scope variable and the register offset in
that stack frame.

Non-local references are added to the function object that is returned by the
function compiler. The VM will use these to identify the absolute location on
the stack where a nonlocal variable should be read from and create upvalue
objects at runtime when a variable is closed over.

#### Compiling let

Let is the declaration of variables and assigning values: the binding of 
values, or the results of expressions, to symbols. Secondly, it provides
space to evaluate expressions that incorporate those variables.

Here we bind the result of `(make_adder 3)` - a function - to the symbol
`add_3` and then call `add_3` with argument `4`.

```
(let ((add_3 (make_adder 3)))
  (add_3 4))
```

The result of the entire `let` expression should be `7`.

Compiling `let` simply introduces additional scopes within a function scope.
That is, instead of a function containing a single scope for all it's
variables, scopes are nested. A stack of scopes is needed, with the parameters
occupying the outermost scope.

First a new scope is pushed on to the scope stack and each symbol being bound
is added to the new scope.

To generate code, a result register is reserved and a register for each binding
is reserved.

Finally, each expression is evaluated and the scope is popped, removing the
bindings from view.

## Register allocation

A function call may make use of no more than 256 registers. Recall from earlier
that the 0th register is reserved for the function return value and subsequent
registers are reserved for the function arguments.

Beyond these initial registers the compiler uses a simple strategy in register
allocation: if a variable (a parameter or a `let` binding) is declared, it is
allocated a register based on a stack discipline. Thus, variables are
essentially pushed and popped off the register stack as they come into and out
of scope.

This strategy primarily ensures code simplicity - there is no register
allocation optimization.

## C'est tout!

That covers the VM and compiler design at an overview level. We've glossed over
a lot of detail but the next chapters will expose the implementation detail.
Get ready!

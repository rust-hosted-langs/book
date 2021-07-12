# Virtual Machine: Implementation

In this chapter we'll dive into some of the more interesting and important
implementation details of our virtual machine.

To begin with, we'll lay out a struct for a single thread of execution. This
struct should contain everything needed to execute the output of the compiler.

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefThread}}
```

Here we see every data structure needed to represent:

- function call frames
- stack values
- closed-over stack values (Upvalues)
- global values
- bytecode to execute

The VM's primary operation is to iterate through instructions, executing each
in sequence. The outermost control struture is, therefore, a loop containing
a `match` expression.

Here is a code extract of the opening lines of this match operation. The
function shown is a member of the `Thread` struct. It evaluates the next
instruction and is called in a loop by an outer function. We'll look at several
extracts from this function in this chapter.

```rust,ignore
{{#include ../interpreter/src/vm.rs:ThreadEvalNextInstr}}

                ...
```

The function obtains a slice view of the register stack, then narrows that down
to a 256 register window for the current function.

Then it fetches the next opcode and using `match`, decodes it.

Let's take a closer look at the stack.


## The stack

While some runtimes and compilers, particularly low-level languages, have a
single stack that represents both function call information and local variables,
our high-level runtime splits the stack into:

1. a stack of `CallFrame` objects containing function call and return
   information
2. and a register stack for local variables.

Let's look at each in turn.

### The register stack

In our `Thread` struct, the register stack is represented by the two members:

```rust,ignore
pub struct Thread {
    ...
    stack: CellPtr<List>,
    stack_base: Cell<ArraySize>,
    ...
}
```

Remember that the `List` type is defined as `Array<TaggedCellPtr>` and is
therefore an array of tagged pointers. Thus, the register stack is a homogenous
array of word sized values that are pointers to objects on the heap or values
that can be inlined in the tagged pointer word.

We also have a `stack_base` variable to quickly retrieve the offset into `stack`
that indicates the beginning of the window of 256 registers that the current
function has for it's local variables.

### The call frame stack

In our `Thread` struct, the call frame stack is represented by the members:

```rust,ignore
pub struct Thread {
    ...
    frames: CellPtr<CallFrameList>,
    instr: CellPtr<InstructionStream>,
    ...
}
```

A `CallFrame` and an array of them are defined as:

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefCallFrame}}

{{#include ../interpreter/src/vm.rs:DefCallFrameList}}
```

A `CallFrame` contains all the information needed to resume a function when
a nested function call returns:

* a `Function` object, which references the `Bytecode` comprising the
  function
* the return instruction pointer
* the stack base index for the function's stack register window

On every function call, a `CallFrame` instance is pushed on to the `Thread`'s
`frames` stack and on every return from a function, the top `CallFrame` is
popped off the stack.

Additionally, we keep a pointer to the current executing function (the function
represented by the top `CallFrame`) with the member `instr:
CellPtr<InstructionStream>`.

For a review of the definition of `InstructionStream` see the
[bytecode](./chapter-interp-bytecode.md) chapter where we defined it as
a pair of values - a `ByteCode` reference and a pointer to the next `Opcode`
to fetch.

The VM keeps the `InstructionStream` object pointing at the same `ByteCode`
object as is pointed at by the `Function` in the `CallFrame` at the top of
the call frame stack. Thus, when a call frame is popped off the stack, the
`InstructionStream` is updated with the `ByteCode` and instruction pointer
from the `CallFrame` at the new stack top; and similarly when a function
is called _into_ and a new `CallFrame` is pushed on to the stack.


## Functions and function calls

### Function objects

Since we've mentioned `Function` objects above, let's now have a look at the
definition.

```rust,ignore
{{#include ../interpreter/src/function.rs:DefFunction}}
```

Instances of `Function` are produced by the compiler, one for each function
definition that is compiled, including nested function definitions.

A `Function` object is a simple collection of values, some of which may be
`nil`. Any member represented by a `TaggedCellPtr` may, of course, contain
a `nil` value.

Thus the function may be anonymous, represented by a `nil` name value.

While the function name is optional, the parameter names are always included.
Though they do not need to be known in order to execute the function, they are
useful for representing the function in string form if the programmer needs to
introspect a function object.

Members that are _required_ to execute the function are the arity, the
`ByteCode` and any nonlocal references.

Nonlocal references are an optional list of `(relative_stack_frame, register)`
tuples, provided by the compiler, that are needed to locate nonlocal variables
on the register stack. These are, of course, a key component of implementing
closures.

We'll talk about closures shortly, but before we do, we'll extend `Function`s
with partial application of arguments.


### Partial functions

A partial function application takes a subset of the arguments required to
make a function call. These arguments must be stored for later.

Thus, a `Partial` object references the `Function` to be called and a list
of arguments to give it when the call is finally executed.

Below is the definition of `Partial`. Note that it also contains a possible
closure environment which, again, we'll arrive at momentarily.

```rust,ignore
{{#include ../interpreter/src/function.rs:DefPartial}}
```

The `arity` and `used` members indicate how many arguments are expected and how
many have been given. These are provided directly in this struct rather than
requiring dereferencing the `arity` on the `Function` object and the length of
the `args` list. This is for convenience and performance.

Each time more arguments are added to a `Partial`, a new `Partial` instance must
be allocated and the existing arguments copied over. A `Partial` object, once
created, is immutable.


### Closures

Closures and partial applications have, at an abstract level, something in
common: they both reference values that the function will need when it is
finally called.

It's also possible, of course, to have a partially applied closure.

We can extend the `Partial` definition with a closure environment so that we can
use the same object type everywhere to represent a function pointer, applied
arguments and closure environment as needed.

#### Compiling a closure

The compiler, because it keeps track of variable names and scopes, knows when a
`Function` references nonlocal variables. After such a function is defined, the
compiler emits a `MakeClosure` instruction.

#### Referencing the stack with upvalues

The VM, when it executes `MakeClosure`, creates a new `Partial` object.  It
then iterates over the list of nonlocal references and allocates an `Upvalue`
object for each, which are added to the `env` member on the `Partial` object.

The below code extract is from the function `Thread::eval_next_instr()` in
the `MakeClosure` instruction decode and execution block.

The two operands of the `MakeClosure` operation - `dest` and `function` - are
registers. `function` points at the `Function` to be given an environment and
made into a closure `Partial` instance; the pointer to this instance will be
written to the `dest` register.

```rust,ignore
{{#include ../interpreter/src/vm.rs:OpcodeMakeClosure}}
```

The `Upvalue` struct itself is defined as:

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefUpvalue}}
```

An `Upvalue` is an object that references an absolute register stack location
(that is the `location` member.)

The initial value of `closed` is `false`. In this state, the location on the
stack that contains the variable _must_ be a valid location. That is, the stack
can not have been unwound yet. If the closure is called, `Upvalue`s in this
state are simply an indirection between the function and the variable on the
register stack.

The compiler is able to keep track of variables and whether they are closed
over. It emits bytecode instructions to close `Upvalue` objects when variables
on the stack go out of scope.

This instruction, `CloseUpvalues`, copies the variable from the register stack
to the `value` member of the `Upvalue` object and sets `closed` to `true`.

From then on, when the closure reads or writes to this variable, the value on
the `Upvalue` object is modified rather than the location on the register stack.


## Global values

```rust,ignore
pub struct Thread {
    ...
    globals: CellPtr<Dict>,
    ...
}
```

The outermost scope of a program's values and functions are the global values.
We can manage these with an instance of a `Dict`. While a `Dict` can use any
hashable value as a key, internally the VM will only allow `Symbol`s to be
keys. That is, globals must be named objects.

# Virtual Machine: Implementation

This chapter will explain some of the more important implementation details
of our virtual machine.

We'll begin by laying out a struct for a single thread of execution:

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefThread}}
```

This struct contains every data structure needed to represent global values,
stack values, closed-over stack values, function call frames and instructions
to execute.

Let's look at each one of these.


## The stack

The stack is separated into a stack of `CallFrame` instances and a register
stack.

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

### The call frame stack

In our `Thread` struct, the call frame stack is represented by the member:

```rust,ignore
pub struct Thread {
    ...
    frames: CellPtr<CallFrameList>,
    instr: CellPtr<InstructionStream>,
    ...
}
```

Where `CallFrameList` is defined as

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefCallFrameList}}
```

and a `CallFrame` struct is defined as:

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefCallFrame}}
```

A `CallFrame` contains all the information needed to resume a function when
a nested function call returns:

* the `Function` object, which references the `Bytecode` comprising the
  function
* the return instruction pointer
* the stack base index for the function's stack register window

On every function call, a `CallFrame` instance is pushed on to the `Thread`'s
frames list.

Directly related to the call frame stack is the current instruction pointer.
This is the `Thread` member `instr: CellPtr<InstructionStream>`,

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


## Function objects

Since we've mentioned `Function` objects above, let's now have a look at the
definition.

```rust,ignore
{{#include ../interpreter/src/function.rs:DefFunction}}
```

Instances of `Function` are produced by the compiler, one for each function
definition that is compiled.

A `Function` object is a simple collection of values, some of which may be
`nil`. Any member represented by a `TaggedCellPtr` may, of course, contain
a `nil` value.

Thus the function may be anonymous, represented by a `nil` name member value.
While the function name is optional, the parameter names are always included.
While they do not need to be known in order to execute the function, they are
useful for representing the function if the programmer needs to introspect a
function object.

Members that are required to execute the function are the arity, the `ByteCode`
and any nonlocal references.

Nonlocal references are an optional list of `(relative_stack_frame, register)`
values provided by the compiler that are needed to locate nonlocal variables on
the register stack. These are, of course, a key component of implementing
closures. We'll talk about closures shortly, but before we do, we'll extend
`Function`s with partial application of arguments.


## Partial functions

A partial function application takes a subset of the arguments required to
make a function call. These arguments must be stored for later and can be
added to over multiple partial applications of the same function.

Thus, a `Partial` object references the `Function` to be called and a list
of arguments to give it when the call happens. Below is the definition
of `Partial`. Note that it also contains a possible closure environment.
We'll discuss that in the next section.

```rust,ignore
{{#include ../interpreter/src/function.rs:DefPartial}}
```

The `arity` and `used` members indicate how many arguments are expected and how
many have been given. These are provided directly in this struct rather than
requiring dereferencing the `arity` on the `Function` object and the length of
the `args` list. This is for convenience and performance.

Each time more arguments are added to a `Partial`, a new `Partial` instance must
be allocated and the existing arguments copied over. Essentially, a `Partial`
object, once created, is immutable.


## Closures

Closures and partial applications have, at an abstract level, something in
common: they both reference values that the function will need when it is
finally called and need to carry these references around with them.

We can extend the `Partial` definition with a closure environment so that we
can use the same object type everywhere to represent a function pointer,
applied arguments and closure environment as needed. This will maximize
flexibility and simplicity in our language and VM design.

The compiler, because it keeps track of variable names and scopes, knows when a
`Function` references nonlocal variables. When such a function is going to be
referenced to be called next or at some later time, the compiler emits a
`MakeClosure` instruction.

The VM, when it executes `MakeClosure`, creates a new `Partial` object.  It
then iterates over the list of nonlocal references and allocates an `Upvalue`
object for each, which are added to the `env` member on the `Partial` object.
The `Upvalue` struct is defined as:

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefUpvalue}}
```

An `Upvalue` is an object that references a register stack location (that is
the `location` member.) The initial value of `closed` is `false`.  In this
state, the location on the stack that contains the variable _must_ be a valid
location. That is, the stack can not have been unwound yet.  If the closure is
called, `Upvalue`s in this state are simply an indirection between the function
and the variable on the register stack.

The compiler is able to keep track of variables and whether they are closed
over. It emits bytecode instructions to close `Upvalue` objects when
closed-over variables go out of scope. This instruction, `CloseUpvalues`,
copies the variable from the register stack to the `value` member of the
`Upvalue` object and sets `closed` to `true`. From now on, when the closure
reads or writes to this variable, the value on the `Upvalue` object is modified
rather than the location on the register stack.


## Global values


## Tying it all together

<include VM code snippets here>

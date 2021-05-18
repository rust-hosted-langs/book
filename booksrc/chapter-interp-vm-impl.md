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
closures.


## Closures and upvalues

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefUpvalue}}
```

## Partial functions

## Global values


## Tieing it all together

<include VM code snippets here>

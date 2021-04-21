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

Let's look at each one of these next.

## The stack

The stack is separated into a stack of `CallFrame` instances and a register
stack.

### The register stack

```rust,ignore
pub struct Thread {
    ...
    stack: CellPtr<List>,
    stack_base: Cell<ArraySize>
    ...
}
```

Remember that the `List` type is defined as `Array<TaggedCellPtr>` and is
therefore an array of tagged pointers. Thus, the register stack is a homogenous
array of word sized values that are pointers to objects on the heap or values
that can be inlined in the tagged pointer word.

### The call frame stack

```rust,ignore
pub struct Thread {
    ...
    frames: CellPtr<CallFrameList>
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

### Function objects

Since we've mentioned `Function` objects above, let's have a look at the
definition.

```rust,ignore
{{#include ../interpreter/src/function.rs:DefFunction}}
```

## Global values

## Closures and upvalues

```rust,ignore
{{#include ../interpreter/src/vm.rs:DefUpvalue}}
```

## Partial functions

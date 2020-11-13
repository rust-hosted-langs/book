# Virtual Machine: Architecture and Design

In this short chapter we will outline our virtual machine design choices. These
are substantially a matter of pragmatic dynamic language implementation points
and as such, borrow heavily from uncomplicated prior work such as Lua 5 and 
Crafting Interpreters.


## Bytecode

We already discussed our Lua-inspired bytecode in a [previous
chapter](./chapter-interp-bytecode.md). To recap, we are using 32 bit
fixed-width opcodes with space for 8 bit register numbers and/or 16 bit
literals.


## The stack

Just like in [Crafting Interpreters][1] we'll maintain two separate stack data
structures:

* the register stack for storing values
* the call frame stack

These are separated out because the register stack will be composed entirely of
`TaggedCellPtr`s. We don't want to coerce a call frame struct into a set of 
tagged pointers or have to allocate each frame on the heap.

### The register stack

The register stack is an array of `TaggedCellPtr`s. Thus each stack value can
point to a heap object or contain a literal integer.

As bytecode operands are limited to 8 bit register indexes, a function is
limited to a maximum of 256 registers, and therefore can address a maximum of
256 contiguous stack slots.

This requires us to implement a sliding window into the register stack which
will move as functions are called and return. The call frame stack will contain
the stack base pointer for each function call. and we can use a Rust slice to
implement the window of 256 contiguous stack slots which a function call is
limited to.

### The call frame stack

A call frame needs to store three critical data points:

* a pointer to the function being executed
* the return instruction pointer when a nested function is called
* the stack base pointer for the function call

These three items can form a simple struct and we can define an
`Array<CallFrame>` type for optimum performance.


## Global values

To store global values, we have all we need: the `Dict` type that maps `Symbol`s
to another value. The VM will, of course, have an abstraction over the internal
`Dict` to enforce `Symbol`s only as keys.


## Closures

We'll implement closures using upvalues, just as in Lua 5 and [Crafting
Interpreters][2].

In the classic implementation from Lua 5, followed also by Crafting
Interpreters, a linked list of upvalues is used to connect stack locations to a
shared value.  In our implementation, we'll use the `Dict` type that we already
have available to do this mapping.  In every other respect, our implementation
will be very similar.


## Partial functions

Here is one point where we will introduce a less common construct in our virtual
machine. Functions will be first class, that is they are objects that can be
passed around as values and arguments. On top of that, we'll allow passing
insufficient arguments to a function when it is called. The return value of
such an operation will, instead of an error, be a `Partial` instance. This value
must carry with it the arguments given and a pointer to the function waiting to
be called.

This is far from sufficient for a fully featured currying implementation but is
an interesting extension to first class functions, especially as it allows us to
not _require_ lambdas to be constructed syntactically every time they might be
used.

By that we mean the following: if we have a function `(def mul (x y) (* x y))`,
to turn that into a function that multiplies a number by 3 we'd normally have to
define a second function, or lambda, `(def mul3 (x) (mul x 3))` and call it
instead. However, with a simple partial function implementation we can avoid the
second function definition and call `(mul 3)`, which will collect the function
`mul` and argument `3` into a `Partial` and wait for the final argument before
calling into the function `mul` with both required arguments.


## Instruction execution

TODO: match on opcode See
https://github.com/rust-hosted-langs/runtimes-WG/issues/3


[1]: http://craftinginterpreters.com/calls-and-functions.html#call-frames
[2]: http://craftinginterpreters.com/closures.html

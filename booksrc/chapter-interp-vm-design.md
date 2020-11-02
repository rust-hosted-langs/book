# Virtual Machine: Architecture and Design

In this chapter we will outline some Virtual Machine design choices. 


## Bytecode

We already discussed our Lua-inspired bytecode in a
[previous chapter](./chapter-interp-bytecode.md). To recap: 32 bit fixed-width 
opcodes with space for 8 bit register numbers and/or 16 bit literals.


## The stack

We'll maintain two separate stack data structures:

* the register stack
* the call frame stack

These are separated out because the register stack will be composed entirely of
`TaggedCellPtr`s. We don't want to coerce a call frame struct into a set of 
tagged pointers or have to allocate each frame on the heap.

## The register stack

The register stack is an array of `TaggedCellPtr`s. Thus each stack value can
point to a heap object or contain a literal integer.

As operands are limited to 8 bit register numbers, we will interface with the
stack on a sliding window basis. While each call frame will know it's 
stack base pointer, the function itself will see only 256 contiguous stack
locations starting from zero through 255. We can make use of Rust slices to
create this window into the stack array for each call frame.

## The call frame stack

TODO


## Global values

TODO: dict, only symbols can be keys


## Closures

We will borrow one more thing from the Lua 5 compiler/VM, which is also well
documented in Crafting Interpreters: upvalues.


## Partial functions


## Instruction execution

TODO: match on opcode See
https://github.com/rust-hosted-langs/runtimes-WG/issues/3

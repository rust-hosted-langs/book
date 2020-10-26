# Virtual Machine: Architecture and Design

In this chapter we will outline some Virtual Machine design choices. 


## Bytecode

To begin with, we already have a specification for bytecode from the earlier
chapter. To recap: 32 bit fixed opcodes with space for operands that are
registers. 

That design choice was borrowed primarily from the Lua 5 implementation.


## The stack

We'll maintain two separate stack data structures:

* the register stack
* the call frame stack

These are separated out because the register stack will be composed entirely
of `TaggedCellPtr`s - we don't want to coerce a call frame into a set of
tagged pointers or allocate each frame on the heap.

## The register stack

Each call frame will have a stack base pointer. This pointer will indicate the
base position in the register stack at which the called function will see a 
window of 256 registers. We can make use of Rust slices to enforce bounds 
checking on this window.

## The call frame stack

TODO


## Global values

TODO: dict, only symbols can be keys


## Closures

We will borrow one more thing from the Lua 5 compiler/VM, which is also well
documented in Crafting Interpreters: upvalues.


## Partial functions


## Instruction execution

TODO: match on opcode
See https://github.com/rust-hosted-langs/runtimes-WG/issues/3

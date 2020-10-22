# Virtual Machine: Architecture and Design

In this chapter we will outline some Virtual Machine design choices. 


## Bytecode

To begin with, we already have a specification for bytecode from the earlier
chapter. To recap: 32 bit fixed opcodes with space for operands that are
registers. 

That design choice was borrowed primarily from the Lua 5 implementation.


## The stack

As in Crafting Interpreters, we'll maintain two separate stack data
structures:

* the register stack
* the call frame stack

We'll separate these out because the register stack will be composed entirely
of `TaggedCellPtr`s and we don't want to have to allocate every call frame on 
the heap and store a pointer to it in the register stack or do some very
unsafe and error prone casting of memory on the register stack itself to store
call frame entries.

## The register stack

Each call frame will have a stack base pointer. This pointer will indicate the
base position in the register stack at which the called function will see a 
window of 256 registers. We can make use of Rust slices to enforce bounds 
checking on this window.


## Global values

TODO: dict, only symbols can be keys


## Closures

We will borrow one more thing from the Lua 5 compiler/VM, which is also well
documented in Crafting Interpreters: upvalues.


## Instruction execution

TODO: match on opcode
https://github.com/rust-hosted-langs/runtimes-WG/issues/3

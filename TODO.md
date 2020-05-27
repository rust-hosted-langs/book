# TODO

FOCUS: GC/mutator interface
  How is it implemented?
  How is it used?

## StickyImmix

* keep some empty blocks in the free list
* allocate medium objects into overflow if no hole in current block
* keep track of large objects individually

Later:
* order blocks by base address, maybe use BTreeMap

* GC traits:
  * Trace
  * RootsIterMut

Need a roots abstraction where stacks and their roots etc are visible from
the GC
* RootsIterMut::Item = &mut TaggedPtr


## Interpreter

* Clean up unused functions
* Clean up TODOs
* Fix any glaring problems
* Garbage collection


## Chapters

* Bump allocator
* An allocator API
* Object headers and the allocator API

* A safe lifetime-limited-pointer abstraction allocator API
* Runtime types, object headers and pointers
* The virtual machine

* Symbols and Pairs
* Lexing and parsing
* Arrays
* Dictionaries
* Bytecode
* Compiling simple expressions
* Functions and partial-applications

* Executing... stuff

* Garbage collection

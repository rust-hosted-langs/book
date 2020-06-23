# Summary

- [Introduction](./introduction.md)
- [Allocators](./part-allocators.md)
  - [Alignment](./chapter-alignment.md)
  - [Obtaining blocks of memory](./chapter-blocks.md)
  - [The type of allocation](./chapter-what-is-alloc.md)
- [Sticky Immix: Allocation](./part-stickyimmix.md)
  - [Bump allocation](./chapter-simple-bump.md)
  - [Allocating into multiple blocks](./chapter-managing-blocks.md)
  - [Defining the allocation API](./chapter-allocation-api.md)
  - [Implementing the API](./chapter-allocation-impl.md)
- [The Eval-rs](./part-interpreter.md)
  - [Allocating safely](./chapter-interp-alloc.md)

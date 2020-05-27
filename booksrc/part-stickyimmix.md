# Sticky Immix

For our first allocator, we'll create a Sticky Immix implementation.

Full Immix implements object moving for defragmentation of memory blocks.
We'll leave out this evacuation operation in this guide.
We'll also stick to single-threaded operation for added simplicity.

_What this is not: custom memory management to replace the global Rust allocator
or allocating and freeing standard library collections and types._

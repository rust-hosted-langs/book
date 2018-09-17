# Sticky Immix

For our first allocator, we'll create a Sticky Immix implementation.

Full Immix implements object moving for defragmentation of memory blocks.
We'll leave out this evacuation operation in this first pass.
We'll also stick to single-threaded operation for added simplicity.

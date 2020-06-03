# Allocating into Multiple Blocks

Let's now zoom out of the fractal code soup one level and begin arranging multiple
blocks so we can allocate - in theory - indefinitely.

We'll need a new struct for wrapping multiple blocks:

```rust
{{#include ../stickyimmix/src/heap.rs:DefBlockList}}
```

Immix maintains several lists of blocks. We won't include them all in the first
iteration but in short they are:

* `free`: a list of blocks that contain no objects. These blocks are held at the
  ready to allocate into on demand.
* `recycle`: a list of blocks that contain some objects but also gaps that can
  be allocated into. These blocks would also undergo defragmentation.
* `large`: not a list of blocks, necessarily, but a list of objects larger than
  the block size, or some other method of accounting for large objects
* `rest`: the rest of the blocks that have been allocated into but are not
  ready for recycling

In our first iteration we'll keep the `rest` list of blocks and two blocks to
immediately allocate into. Why two? To understand why, we need to understand
how Immix thinks about object sizes.

## Immix and object sizes

We've seen that there are two numbers that define granularity in Immix: the
block size and the line size.  These numbers give us the ability to categorize
object sizes:

* small: those that (with object header and alignment overhead) fit inside a
  line
* medium: those that are larger than one line but smaller than a block
* large: those that are larger than a block

In the previous chapter we described the basic allocation algorithm: when
an object is being allocated, the current block is scanned for a hole between
marked lines large enough to allocate into. This does seem like it could
be inefficient. We could spend a lot of CPU cycles looking for a big enough
hole, especially for a medium sized object.

To avoid this, Immix maintains a second block, an overflow block, to allocate
medium objects into that don't fit the first available hole in the
main block being allocated into.

Thus two blocks to immediately allocate into:

* `head`: the current block being allocated into
* `overflow`: a block kept handly for writing medium objects into that don't
  fit the `head` block's current hole

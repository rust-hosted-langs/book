# Allocating into Multiple Blocks

Let's now zoom out of the fractal code soup one level and begin arranging multiple
blocks so we can allocate - in theory - indefinitely.

## Lists of blocks

We'll need a new struct for wrapping multiple blocks:

```rust
{{#include ../stickyimmix/src/heap.rs:DefBlockList}}
```

Immix maintains several lists of blocks. We won't include them all in the first
iteration but in short they are:

* `free`: a list of blocks that contain no objects. These blocks are held at the
  ready to allocate into on demand
* `recycle`: a list of blocks that contain some objects but also at least one
  line that can be allocated into
* `large`: not a list of blocks, necessarily, but a list of objects larger than
  the block size, or some other method of accounting for large objects
* `rest`: the rest of the blocks that have been allocated into but are not
  suitable for recycling

In our first iteration we'll only keep the `rest` list of blocks and two blocks
to immediately allocate into. Why two? To understand why, we need to understand
how Immix thinks about object sizes.

### Immix and object sizes

We've seen that there are two numbers that define granularity in Immix: the
block size and the line size.  These numbers give us the ability to categorize
object sizes:

* small: those that (with object header and alignment overhead) fit inside a
  line
* medium: those that (again with object header and alignment overhead) are
  larger than one line but smaller than a block
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
* `overflow`: a block kept handy for writing medium objects into that don't
  fit the `head` block's current hole

We'll be ignoring large objects for now and attending only to allocating small
and medium objects into blocks.

Instead of recycling blocks with holes, or maintaining a list of pre-allocated
free blocks, we'll allocate a new block on demand whenever we need more space.
We'll get to identifying holes and recyclable blocks in a later chapter.

### Managing the overflow block

Generally in our code for this book, we will try to default to not allocating
memory unless it is needed. For example, when an array is instantiated,
the backing storage will remain unallocated until a value is pushed on to
it.

Thus in the definition of `BlockList`, `head` and `overflow` are `Option`
types and won't be instantiated except on demand.

For allocating into the overflow block we'll define a function in the
`BlockList` impl:

```rust
fn overflow_alloc(&mut self, alloc_size: usize) -> Result<*const u8, AllocError>
```

The input constraint is that, since overflow is for medium objects, `alloc_size`
must be less than the block size.

The logic inside will divide into three branches:

1. We haven't got an overflow block yet - `self.overflow` is `None`. In this
   case we have to instantiate a new block (since we're not maintaining
   a list of preinstantiated free blocks yet) and then, since that block
   is empty and we have a medium sized object, we can expect the allocation
   to succeed.
   ```rust
       match self.overflow {
           Some ...,
           None => {
                let mut overflow = BumpBlock::new()?;

                // object size < block size means we can't fail this expect
                let space = overflow
                    .inner_alloc(alloc_size)
                    .expect("We expected this object to fit!");

                self.overflow = Some(overflow);

                space
            }
       }
   ```
2. We _have_ an overflow block and the object fits. Easy.
   ```rust
        match self.overflow {
            // We already have an overflow block to try to use...
            Some(ref mut overflow) => {
                // This is a medium object that might fit in the current block...
                match overflow.inner_alloc(alloc_size) {
                    // the block has a suitable hole
                    Some(space) => space,
                    ...
                }
            },
            None => ...
        }
   ```
3. We have an overflow block but the object does not fit. Now we simply
   instantiate a _new_ overflow block, adding the old one to the `rest`
   list (in future it will make a good candidate for recycing!). Again,
   since we're writing a medium object into a block, we can expect allocation
   to succeed.
   ```rust
        match self.overflow {
            // We already have an overflow block to try to use...
            Some(ref mut overflow) => {
                // This is a medium object that might fit in the current block...
                match overflow.inner_alloc(alloc_size) {
                    Some ...,
                    // the block does not have a suitable hole
                    None => {
                        let previous = replace(overflow, BumpBlock::new()?);

                        self.rest.push(previous);

                        overflow.inner_alloc(alloc_size).expect("Unexpected error!")
                    }
                }
            },
            None => ...
        }
   ```

In this logic, the only error can come from failing to create a new block.

On success, at this level of interface we continue to return a `*const u8`
pointer to the available space as we're not yet handling the type of the
object being allocated.

You may have noticed that the function signature for `overflow_alloc` takes a
`&mut self`.  This isn't compatible with the interior mutability model
of allocation.  We'll have to wrap the `BlockList` struct in another struct
that handles this change of API model.

## The heap struct

This outer struct will provide the external crate interface and some further
implementation of block management.

The crate interface will require us to consider object headers and so in the
struct definition below there is reference to a generic type `H` that
the _user_ of the heap will define as the object header.

```rust
{{#include ../stickyimmix/src/heap.rs:DefStickyImmixHeap}}
```

Since object headers are not owned directly by the heap struct, we need a
`PhantomData` instance to associate with `H`.  We'll discuss object headers
in a later chapter.

Now let's focus on the use of the `BlockList`.

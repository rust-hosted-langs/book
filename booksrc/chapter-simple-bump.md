# Bump allocation

Now that we can get blocks of raw memory, we need to write objects into it. The
simplest way to do this is to write objects into a block one after the other
in consecutive order. This is bump allocation - we have a pointer, the bump
pointer, which points at the space in the block after the last object that
was written. When the next object is written, the bump pointer is incremented
to point to the space after _that_ object [^1].

We will used a fixed power-of-two block size. The benefit of this is that
given a pointer to an object, by zeroing the bits of the pointer that represent
the block size, the result points to the beginning of the block. This will
be useful later when implementing garbage collection.

Our block size will be 32k, a reasonably optimal size arrived at in the
original [Immix][1] paper. This size can be any power of two though and
different use cases may show different optimal sizes.

```rust,ignore
{{#include ../stickyimmix/src/constants.rs:ConstBlockSize}}
```

Next, we'll define a struct that wraps the block with a bump pointer and other
metadata.

```rust,ignore
{{#include ../stickyimmix/src/bumpblock.rs:DefBumpBlock}}
```

## Bump allocation basics

In this struct definition, there are two members that we are interested in
for this section. The other two, `limit` and `meta`, will be discussed in the
next section.

* `cursor`: this is the bump pointer. In our implementation it is the index
  into the block where the next object can be written.
* `block`: this is the `Block` itself in which objects will be written.

For this bump allocation function, the `alloc_size` parameter should be a number
of bytes of memory requested. We'll assume that the value provided is equivalent
to an exact number of words so that we don't end up with badly aligned object
placement.

```rust,ignore
impl BumpBlock {
    pub fn inner_alloc(&mut self, alloc_size: usize) -> Option<*const u8> {
        let next_bump = self.cursor + alloc_size;

        if next_bump > constants::BLOCK_SIZE {
            None
        } else {
            let offset = self.cursor;
            self.cursor = next_bump;
            unsafe { Some(self.block.as_ptr().add(offset) as *const u8) }
        }
    }
}
```

In this overly simplistic initial implementation, allocation will simply return
`None` if the block is full. If there _is_ space, it will be returned as a
`Some(*const u8)` pointer.

Note that this function does not _write_ the object to memory, it merely
returns a pointer to an available space.  Writing the object will simply
require invoking the `std::ptr::write` function. We will do that in a separate
module but for completeness of this chapter, this might look something like:

```rust,ignore
use std::ptr::write;

unsafe fn write<T>(dest: *const u8, object: T) {
    write(dest as *mut T, object);
}
```


## Some time passes...

After allocating and freeing objects, we will have gaps between objects in a
block that can be reused. The above bump allocation algorithm is unaware of
these gaps so we'll have to modify it before it can allocate into fragmented
blocks.

Remember that in Immix, only whole lines are considered for reuse. To recap,
a block is divided into lines. When objects are marked as live, so are the
lines that an object occupies. Therefore, only lines that are _not_ marked
as live are usable for allocation into.

We'll need a data structure to represent this. we'll call it `BlockMeta`,
but first some constants that we need in order to know how big a line is
and how many are in a block:

```rust,ignore
{{#include ../stickyimmix/src/constants.rs:ConstLineSize}}
```

And now the definition of `BlockMeta`:

```rust,ignore
{{#include ../stickyimmix/src/blockmeta.rs:DefBlockMeta}}
```

* `line_mark` is an array of boolean flags, one for each line in a
block, to indicate whether it has been marked or not.
* `block_mark` simply says whether the entire block has
marked objects in it. If this is ever `false`, the entire block can be
deallocated.

This struct contains one function we will study:

```rust,ignore
{{#include ../stickyimmix/src/blockmeta.rs:DefFindNextHole}}
```

* The input to this function, `starting_at`, is the offset into the block at
which we are looking for a large enough consecutive set of unmarked lines
to write an object into. The value passed in will be the bump pointer, of
course, since that is where we last successfully wrote to the block at.
* The return value is `None` if no unmarked lines are found.
* If there _are_ unmarked lines after the `starting_at` point, the return
value will be a pair of numbers - `(cursor, limit)` - where `cursor` will
be the new bump pointer value and `limit` will be the upper bound of the
available hole.

The first thing this function does is convert from block byte offset math
to line count math:

```rust,ignore
         let starting_line = starting_at / constants::LINE_SIZE;
```

And then iterate over the lines starting with the line that the requested
byte offset starting point corresponds with:

```rust,ignore
         for (index, marked) in self.line_mark[starting_line..].iter().enumerate() {
             let abs_index = starting_line + index;
```

We're looking for unmarked lines to allocate into, so we'll count how many
we get so we can later calculate the start and end offsets of a hole:

```rust,ignore
            // count unmarked lines
            if !*marked {
                count += 1;
```

Up next are a couple lines of code that need longer explanation:

```rust,ignore
                if count == 1 && abs_index > 0 {
                    continue;
                }
```

The Immix authors found that marking _every_ line that contains a live object
could be expensive. For example, many small objects might cross line boundaries,
requiring two lines to be marked as live. This would require looking up the
object size and calculating whether the object crosses the boundary into the
next line. To save CPU cycles, they simplified the algorithm by saying that
any object that fits in a line _might_ cross into the next line so we will
conservatively _consider_ the next line marked just in case. This sped up
marking at little fragmentation expense.

So the three lines of code above simply say: if we've so-far only found one
unmarked block, consider that it might be a conservatively-marked line and
ignore it.

Once that condition has passed and we're clear of any conservatively-marked
line, we can consider the next unmarked line as totally available. Here we
save the index of this line in the variable `start`:

```rust,ignore
                if start.is_none() {
                    start = Some(abs_index);
                }
```

Now we have a starting line for the overall hole between marked objects. Next
we'll close the `if *marked` scope by setting the end of the hole:

```rust,ignore
                stop = abs_index + 1;
            }
```

The loop will continue and while there are consecutive unmarked lines, `stop`
will continue to be updated to a later line boundary.

As soon as we hit a marked line or the end of the block, and we have a nonzero
number of unmarked lines, we'll test whether we have a valid hole to allocate
into:

```rust,ignore
            if count > 0 && (*marked || stop >= constants::LINE_COUNT) {
                if let Some(start) = start {
                    let cursor = start * constants::LINE_SIZE;
                    let limit = stop * constants::LINE_SIZE;

                    return Some((cursor, limit));
                }
            }
```

Here we convert line-based math back into block byte-offset values and return
the new bump-pointer and upper limit.

Otherwise, if the above conditions failed but we've still reached a marked
line, reset the state:

```rust,ignore
            if *marked {
                count = 0;
                start = None;
            }
```

Finally, if the whole loop terminates without returning a new
`Some((cursor, limit))` pair, return `None` as our way of saying this block
has no usable holes to allocate into.

We'll return to the `BumpBlock::inner_alloc()` function now to make use of
`BlockMeta` and it's hole finding operation.

The `BumpBlock` struct contains two more members: `limit` and `meta`. These
should now be obvious - `limit` is the known byte offset limit into which
we can allocate, and `meta` is the `BlockMeta` instance associated with the
block.

We need to update `inner_alloc()` with a new condition:

* the size being requested must fit between `self.cursor` and `self.limit`

(Note that for a fresh, new block, `self.limit` is set to the block size.)

If the above condition is not met, we will call
`BlockMeta::find_next_available_hole()` to get a new `cursor` and `limit`
to try, and repeat that until we've either _found_ a big enough hole or
reached the end of the block, exhausting our options.

The new definition of `BumpBlock::inner_alloc()` reads as follows:

```rust,ignore
{{#include ../stickyimmix/src/bumpblock.rs:DefBumpBlockAlloc}}
```

and as you can see, this implementation is recursive.


## Wrapping this up

At the beginning of this chapter I stated that given a pointer to an object,
by zeroing the bits of the pointer that represent the block size, the result
points to the beginning of the block.

We'll make use of that now.

During the mark phase of garbage collection, we will need to know which line
or lines to mark, in addition to marking the object itself. We will make a
copy of the `BlockMeta` instance pointer in the 0th word of the memory block
so that given any object pointer, we can obtain the `BlockMeta` instance.

In the next chapter we'll handle multiple `BumpBlock`s so that we can keep
allocating objects after one block is full.

----

[^1]: Note that objects can be written from the end of the block down to the beginning
too, decrementing the bump pointer. This is usually [slightly simpler and more
efficient to implement](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html).

[1]: http://www.cs.utexas.edu/users/speedway/DaCapo/papers/immix-pldi-2008.pdf

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

```rust
{{#include ../stickyimmix/src/constants.rs:ConstBlockSize}}
```

Next, we'll define a struct that wraps the block with a bump pointer and other
metadata.

```rust
{{#include ../stickyimmix/src/bumpblock.rs:DefBumpBlock}}
```

## Pointers and writing to memory

In this struct definition, there are two members that we are interested in
for this chapter. The other two, `limit` and `meta`, will be discussed in the 
next chapter.

* `cursor`: this is the bump pointer. In our implementation it is the index
  into the block where the next object can be written.
* `block`: this is the `Block` itself in which objects will be written.

For this bump allocation function, the `alloc_size` parameter should be a number
of bytes of memory requested. We'll assume that the value provided is equivalent
to an exact number of words so that we don't end up with badly aligned object
placement.

```rust
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

```rust
use std::ptr::write;

unsafe fn write<T>(dest: *const u8, object: T) {
    let ptr = dest as *mut T;
    write(p, object);
}
```


## Preparing for garbage collection

When objects written to our blocks 

The `BumpBlock` struct contains two more members: `limit` and `meta`. These
are ... TBC



[^1]: Note that objects can be written from the end of the block down to the beginning
too, decrementing the bump pointer. This is usually [slightly simpler and more
efficient to implement](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html).

[1]: http://www.cs.utexas.edu/users/speedway/DaCapo/papers/immix-pldi-2008.pdf


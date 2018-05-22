# The nature of an allocator

Before we start writing objects into `Block`s, we need to know the nature of
the interface in Rust terms.

If we consider the global allocator in Rust, implicitly available via
`Box::new()`, `Vec::new()` and so on, we'll notice that since the global
allocator is available on every thread and allows the creation of new
objects on the heap (that is, mutation of the heap) from any code location
without needing to follow the rules of borrowing and mutable aliasing,
it is essentially a container that implements `Sync` and the interior
mutability pattern.

We need to follow suit, but we'll leave `Sync` until later chapters.

An interface that satisfies the interior mutability property might look
like

```rust
trait AllocBare {
    fn alloc<T>(object: T) -> *const T;
}
```

naming it `AllocBare` because when building on top of `Block` level we'll
work with bare pointers.


## Notes on alignment

There be subtleties in alignment. On x86 architectures, general access
can be unaligned but will probably incur an access penalty. SIMD types must
typically be aligned.

The values that `std::mem::align_of<T>()` will return, on x86_64 for example,
are:

- `u8`: 1
- `u16`: 2
- `u32`: 4
- `u64`: 8
- any bigger struct: 8

and that is not taking SIMD types into account. The story on 32bit ARM and
aarch64 is sufficiently similar but there is a higher chance that an ARM core
is configured to raise a bus error on a misaligned access.

[Intel recommends](https://software.intel.com/sites/default/files/managed/9e/bc/64-ia-32-architectures-optimization-manual.pdf?wapkw=248966)
objects larger than 64 bits be aligned to 16 bytes. Apparently system
`malloc()` implementations
[tend to comply](http://www.erahm.org/2016/03/24/minimum-alignment-of-allocation-across-platforms/),
probably to accommodate SIMD types.

Another consideration is atomic access which does not work on non-word aligned
accesses.

With all that in mind, to keep things simple, we'll align everything to a
word. This will mean we won't allocate anything smaller than a word-sized
object.

(When we get to prepending an object _header_, the minimum memory required for
an object will be two words.)

Thus, the allocated size of an object will be determined by

```rust
let word_size = size_of::<usize>();
// mask out the least significant bits that correspond to the word size - 1
// then add the full word size
let size = (size_of::<T>() & !(word_size - 1)) + word_size;
```

which rounds up the result of `size_of::<T>` to the nearest word size.

For a more detailed explanation of the alignment adjustment calculation, see
[phil-opp](https://github.com/phil-opp)'s kernel
[heap allocator](https://os.phil-opp.com/kernel-heap/#alignment).

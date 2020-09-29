# Dicts

The implementation of dicts, or hash tables, is going to combine a reuse of the
[RawArray](./chapter-interp-arrays.md)
type and closely follow the [Crafting Interpreters][1] design:

* open addressing
* linear probing
* FNV hashing

Go read the corresponding chapter in Crafting Interpreters and then come
back here. We won't duplicate much of Bob's excellent explanation of the above
terms and we'll assume you are familiar with his chapter when reading
ours.


## Code design

A `Dict` in our interpreter will allow any hashable value as a key and any
type as a value. We'll store pointers to the key and the value together in
a struct `DictItem`.

Here, we'll also introduce the single diversion from
Crafting Interpreters' implementation in that we'll cache the hash value and
use it as part of a tombstone indicator. This adds an extra 64 bits storage
requirement per item but we will also take the stance that if two keys have
the same hash value then the keys are equal. This simplifies our implementation
as we won't need to implement object equality comparisons just yet.

```rust,ignore
{{#include ../interpreter/src/dict.rs:DefDictItem}}
```

The `Dict` itself mirrors Crafting Interpreters' implementation of a count of
used entries and an array of entries. Since tombstones are counted as used
entries, we'll add a separate `length` that excludes tombstones.

```rust,ignore
{{#include ../interpreter/src/dict.rs:DefDict}}
```


## Hashing

Since our only language supported types for now are `Symbol`s, `Pair`s and
inline integers in our tagged pointer, we'll take the step of least complexity
and implement hashing for `Symbol`s and tagged integers only to begin with.
This is all we _need_ support for to implement the compiler and virtual machine.

The Rust standard library defines trait `std::hash::Hash` that must be
implemented by types that want to be hashed. This trait requires the type to
implement method `fn hash<H>(&self, state: &mut H) where H: Hasher`.

This signature requires a reference to the type `&self` to access it's data.
In our world, this is insufficient: we also require a `&MutatorScope`
lifetime to access an object. We will have to wrap `std::hash::Hash` in our
own trait that extends, essentially the same signature, with this scope
guard parameter. This trait is named `Hashable`:


```rust,ignore
{{#include ../interpreter/src/hashable.rs:DefHashable}}
```

We can implement this trait for `Symbol` - it's a straightforward wrap of
calling `Hash::hash()`:

```rust,ignore
{{#include ../interpreter/src/symbol.rs:DefImplHashableForSymbol}}
```

Then finally, because this is all for a dynamically typed interpreter, we'll
write a function that can take any type - a `TaggedScopedPtr` - and attempt
to return a 64 bit hash value from it:

```rust,ignore
{{#include ../interpreter/src/dict.rs:DefHashKey}}
```

Now we can take a `Symbol` or a tagged integer and use them as keys in our
`Dict`.


## Finding an entry

The methods that a dictionary typically provides, lookup, insertion and
deletion, all hinge around one internal function, `find_entry()`.

This function scans the internal `RawArray<DictItem>` array for a slot that
matches the hash value argument. It may find an exact match for an existing
key-value entry; if it does not, it will return the first available slot for
the hash value, whether an empty never-before used slot or the tombstone
entry of a formerly used slot.

A tombstone, remember, is a slot that previously held a key-value pair but
has been deleted. These slots must be specially marked so that when searching
for an entry that generated a hash for an earlier slot but had to be inserted
at a later slot, we know to keep looking rather than stop searching at the
empty slot of a deleted entry.

Slot  | Content
------|--------
n - 1 | empty
n     | X: hash % capacity == n
n + 1 | tombstone
n + 2 | Y: hash % capacity == n
n + 3 | empty

For example, in the above table:

* Key `X`'s hash maps to slot `n`.
* At some point another entry was inserted at slot `n + 1`.
* Then `Y`, with hash mapping also to slot `n`, was inserted, but had to be
  bumped to slot `n + 2` because the previous two slots were occupied.
* Then the entry at slot `n + 1` was deleted and marked as a tombstone.

If slot `n + 1` was simply marked as `empty` after it's occupant was deleted,
then when searching for `Y` we wouldn't know to keep searching and find `Y` in
slot `n + 2`. Hence, deleted entries are marked differently to empty slots.

Here is the code for the Find Entry function:

```rust,ignore
{{#include ../interpreter/src/dict.rs:DefFindEntry}}
```

To begin with, it calculates the index in the array from which to start
searching. Then it iterates over the internal array, examining each entry's
hash and key as it goes.

* The first tombstone that is encountered is saved. This may turn out to be the
  entry that should be returned if an exact hash match isn't found by the time
  a never-before used slot is reached. We want to reuse tombstone entries, of
  course.
* If no tombstone was found and we reach a never-before used slot, return
  that slot.
* If an exact match is found, return that slot of course.


[1]: http://craftinginterpreters.com/hash-tables.html

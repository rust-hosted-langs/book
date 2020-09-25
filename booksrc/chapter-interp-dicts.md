# Dicts

The implementation of dicts is going to combine a reuse of the
[RawArray](./chapter-interp-arrays.md)
type and closely follow the [Crafting Interpreters][1] design:

* open addressing
* linear probing
* FNV hashing

Go read the corresponding chapter in Crafting Interpreters and then come
back here. We won't duplicate Bob's excellent explanation of the above
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




[1]: http://craftinginterpreters.com/hash-tables.html

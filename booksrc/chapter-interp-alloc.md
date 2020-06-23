# Allocating safely

In this chapter we'll build some safe Rust abstractions over the allocation API
defined in the Sticky Immix crate.

Let's first recall this interface:

```rust
{{#include ../stickyimmix/src/allocator.rs:DefAllocRaw}}
```

These are the functions we'll be calling. When we allocate an object, we'll get
back a `RawPtr<T>` which has no safe way to dereference it. This is impractical,
we very much do not want to wrap every dereferencing in `unsafe { ... }`.
We'll need a layer over `RawPtr<T>` where we can guarantee safe dereferencing.

## Pointers

In safe Rust, mutable (`&mut`) and immutable (`&`) references are passed around
to access objects. These reference types are compile-time constrained pointers
where the constraints are

1. the mutability of the access
2. the lifetime of the access

For our layer over `RawPtr<T>` we'll have to consider both these constraints.

### Mutability

This constraint is concerned with shared access to an object. In other words,
it cares about how many pointers there are to an object at any time and whether
they allow mutable or immutable access.

The short of it is:

* Either only one `&mut` reference may be held in a scope
* Or many `&` immutable references may be held in a scope

The compiler must be able to determine that a `&mut` reference is the only
live reference in it's scope that points at an object in order
for mutable access to that object to be safe of data races.

In a runtime memory managed language such as the interpreter we are building,
we will not have compile time knowledge of shared access to objects. We
won't know at compile time how many pointers to an object we may have at
any time. This is the normal state of things in languages such as Python,
Ruby or Javascript.

This means that we can't allow `&mut` references in our safe layer at all!

If we're restricted to `&` immutable references everywhere, that then means
we must apply the interior mutability pattern everywhere in our design in
order to comply with the laws of safe Rust.

### Lifetime

The second aspect to references is their lifetime. This concerns the
duration of the reference, from inception until it goes out of scope.

The key concept to think about now is "scope."

In an interpreted language there are two major operations on the objects
in memory:

```rust
fn run_mutator() {
    parse_source_code();
    compile();
    execute_bytecode();
}
```

and

```rust
fn run_garbage_collection() {
   trace_objects();
   free_dead_objects();
}
```

A few paragraphs earlier we determined that we can't have `&mut` references
to allocated objects in our interpreter.

By extension, we can't safely hold a mutable reference to the entire heap
as a data structure.

Except, that is exactly what garbage collection requires. The nature of
garbage collection is that it views the entire heap as a single data structure
in it's own right that it needs to traverse and modify. It wants the
heap to be `&mut`.

Consider, especially, that some garbage collectors _move_ objects, so that
pointers to moved objects, wherever they may be, must be modified by the
garbage collector without breaking the mutator! The garbage collector must
be able to reliably discover _every single pointer to moved objects_ to avoid
leaving invalid pointers scattered around[^1].

Thus we have two mutually exclusive interface requirements, one that must
only hold `&` object references and applies _interior_ mutability to the heap
and the other that wants the whole heap to be `&mut`.

For this part of the book, we'll focus on the use of the allocator and save
garbage collection for a later part.

This mutual exclusivity constraint on the allocator results in the statements:

* When garbage collection is running, it is not safe to allocate[^2]
* When garbage collection is not running, it is safe to allocate

Thus our abstraction must encapsulate a concept of a time when "it is safe to
allocate" and since we're working with the Rust compiler, this must be a
compile time concept.

Scopes and lifetimes are perfect for this abstraction. What we'll need is
some way to define a lifetime (that is, a scope) within which access to the
allocator is safe.

### A pointer type

First, let's define a simple pointer type that can wrap an allocated type `T`
in a lifetime:

```rust
{{#include ../interpreter/src/safeptr.rs:DefScopedPtr}}
```

As you can see we have a lifetime `'guard` that we'll use to restrict the
scope in which this pointer can be accessed.


[^1]: This is the topic of discussion in Felix Klock's series
[GC and Rust](http://blog.pnkfx.org/blog/categories/gc/) which is recommended
reading.

[^2]: while this distinction exists at the interface level, in reality there
are multiple phases in garbage collection and not all of them require
exclusive access to the heap. This is an advanced topic that we won't
bring into consideration yet.

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
to objects in our interpreter.

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

* When garbage collection is running, it is not safe to run the mutator[^2]
* When garbage collection is not running, it is safe to run the mutator

Thus our abstraction must encapsulate a concept of a time when "it is safe to
run the mutator" and since we're working with safe Rust, this must be a
compile time concept.

Scopes and lifetimes are perfect for this abstraction. What we'll need is
some way to define a lifetime (that is, a scope) within which access to the
heap by the mutator is safe.

### Some pointer types

First, let's define a simple pointer type that can wrap an allocated type `T`
in a lifetime:

```rust
{{#include ../interpreter/src/safeptr.rs:DefScopedPtr}}
```

This type will implement `Clone`, `Copy` and `Deref` - it can be passed around
freely within the scope and safely dereferenced.

As you can see we have a lifetime `'guard` that we'll use to restrict the
scope in which this pointer can be accessed. We need a mechanism to restrict
this scope.

The guard pattern is what we'll use, if the hint wasn't strong enough.

We'll construct some types that ensure that safe pointers such as
`ScopedPtr<T>`, and access to the heap at in any way, are mediated by an
instance of a guard type that can provide access.

We will end up passing a reference to the guard instance around everywhere. In
most cases we won't care about the instance type itself so much as the lifetime
that it carries with it. As such, we'll define a trait for this type to
implement that so that we can refer to the guard instance by this trait rather
than having to know the concrete type. This'll also allow other types to
proxy the main scope-guarding instance.

```rust
{{#include ../interpreter/src/safeptr.rs:DefMutatorScope}}
```

You may have noticed that we've jumped from `RawPtr<T>` to `ScopedPtr<T>` with
seemingly nothing to bridge the gap. How do we _get_ a `ScopedPtr<T>`?

We'll create a wrapper around `RawPtr<T>` that will complete the picture. This
wrapper type is what will hold pointers at rest inside any data structures.

```rust
{{#include ../interpreter/src/safeptr.rs:DefCellPtr}}
```

This is straightforwardly a `RawPtr<T>` in a `Cell` to allow for modifying the
pointer. We won't allow dereferencing from this type either though.

Remember that dereferencing a heap object pointer is only safe when we are
in the right scope? We need to create a `ScopedPtr<T>` _from_ a `CellPtr<T>`
to be able to use it.

First we'll add a helper function to `RawPtr<T>` in our interpreter crate so
we can safely dereference a `RawPtr<T>`. This code says that, given an instance
of a `MutatorScope`-implementing type, give me back a reference type with
the same lifetime as the guard that I can safely use. Since the `_guard`
parameter is never used except to define a lifetime, it should be optimized
out by the compiler!

```rust
{{#include ../interpreter/src/pointerops.rs:DefScopedRef}}
```

We'll use this in our `CellPtr<T>` to obtain a `ScopedPtr<T>`:

```rust
impl<T: Sized> CellPtr<T> {
{{#include ../interpreter/src/safeptr.rs:DefCellPtrGet}}
}
```

Thus, anywhere (structs, enums) that needs to store a pointer to something on
the heap will use `CellPtr<T>` and any code that accesses these pointers
during the scope-guarded mutator code will obtain `ScopedPtr<T>` instances
that can be safely dereferenced.


## The heap and the mutator

The next question is: where do we get an instance of `MutatorScope` from?

The lifetime of an instance of a `MutatorScope` will define the lifetime
of any safe object accesses. By following the guard pattern, we will find
we have:

* a heap struct that contains an instance of the Sticky Immix heap
* a guard struct that proxies the heap struct for the duration of a scope
* a mechanism to enforce the scope limit

### A heap struct

Let's make a type alias for the Sticky Immix heap so we aren't referring
to it as such throughout the interpreter:

```rust
{{#include ../interpreter/src/memory.rs:DefHeapStorage}}
```

The let's put that into a heap struct, along with any other
interpreter-global storage:

```rust
{{#include ../interpreter/src/memory.rs:DefHeap}}
```

We'll discuss the `SymbolMap` type in the next chapter.

Now, since we've wrapped the Sticky Immix heap in our own `Heap` struct,
we'll need to `impl` an `alloc()` method to proxy the Sticky Immix
allocation function.

```rust
impl Heap {
{{#include ../interpreter/src/memory.rs:DefHeapAlloc}}
}
```

A couple things to note about this function:

* It returns `RuntimeError` in the error case, this type converts `From` the
  Sticky Immix crate's error type.
* The `where` constraint is similar to that of `AllocRaw::alloc()` but in now
  we have a concrete `TypeList` type to bind to. We'll look at `TypeList`
  in the next chapter along with `SymbolMap`.

### A guard struct

This next struct will be used as a scope-limited proxy for the `Heap` struct
with one major difference: function return types will no longer be `RawPtr<T>`
but `ScopedPtr<T>`.

```rust
{{#include ../interpreter/src/memory.rs:DefMutatorView}}
```

Here in this struct definition, it becomes clear that all we are doing is
borrowing the `Heap` instance for a limited lifetime. Thus, the lifetime of
the `MutatorView` instance _will be_ the lifetime that all safe object
access is constrained to.

A look at the `alloc()` function now:

```rust
impl<'memory> MutatorView<'memory> {
{{#include ../interpreter/src/memory.rs:DefMutatorViewAlloc}}
}
```

Very similar to `Heap::alloc()` but the return type is now a `ScopedPtr<T>`
whose lifetime is the same as the `MutatorView` instance.

### Enforcing a scope limit

We now have a `Heap` and a guard, `MutatorView`, but we want one more thing:
to prevent an instance of `MutatorView` from being returned from anywhere -
that is, enforcing a scope within which an instance of `MutatorView` will
live and die. This will make it easier to separate mutator operations and
garbage collection operations.

First we'll apply a constraint on how a mutator _gains_ heap access: through
a trait.

```rust
{{#include ../interpreter/src/memory.rs:DefMutator}}
```

If a piece of code wants to access the heap, it _must_ implement this trait!

Secondly, we'll apply another wrapper struct, this time to the `Heap` type.
This is so that we can borrow the `heap` member instance.

```rust
{{#include ../interpreter/src/memory.rs:DefMemory}}
```

This `Memory` struct and the `Mutator` trait are now tied together with a
function:

```rust
impl Memory {
{{#include ../interpreter/src/memory.rs:DefMemoryMutate}}

}
```

The key to the scope limitation mechanism is that this `mutate` function is
the only way to gain access to the heap. It creates an instance of
`MutatorView` that goes out of scope at the end of the function and thus
can't leak outside of the call stack.


## An example

Let's construct a simple example to demonstrate these many parts. This
will omit defining a `TypeId` and any other types that we didn't discuss
above.

```rust
struct Stack {}

impl Stack {
    fn say_hello(&self) {
        println!("I'm the stack!");
    }
}

struct Roots {
    stack: CellPtr<Stack>
}

impl Roots {
    fn new(stack: ScopedPtr<'_, Stack>) -> Roots {
        Roots {
            stack: CellPtr::new_with(stack)
        }
    }
}

struct Interpreter {}

impl Mutator for Interpreter {
    type Input: ();
    type Output: Roots;

    fn run(&self, mem: &MutatorView, input: Self::Input) -> Result<Self::Output, RuntimeError> {
        let stack = mem.alloc(Stack {})?;   // returns a ScopedPtr<'_, Stack>
        stack.say_hello();

        let roots = Roots::new(stack);

        let stack_ptr = roots.stack.get(mem);  // returns a ScopedPtr<'_, Stack>
        stack_ptr.say_hello();

        Ok(roots)
    }
}

fn main() {
    ...
    let interp = Interpreter {};

    let result = memory.mutate(&interp, ());

    let roots = result.unwrap();

    // no way to do this - compile error
    let stack = roots.stack.get();
    ...
}
```

In this simple, contrived example, we instantiated a `Stack` on the heap.
An instance of `Roots` is created on the native stack and given a pointer
to the `Stack` instance. The mutator returns the `Roots` object, which
continues to hold a pointer to a heap object. However, outside of the `run()`
function, the `stack` member can't be safely accesed.

Up next: using this framework to implement parsing!

----

[^1]: This is the topic of discussion in Felix Klock's series
[GC and Rust](http://blog.pnkfx.org/blog/categories/gc/) which is recommended
reading.

[^2]: while this distinction exists at the interface level, in reality there
are multiple phases in garbage collection and not all of them require
exclusive access to the heap. This is an advanced topic that we won't
bring into consideration yet.

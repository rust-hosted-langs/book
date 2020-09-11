# Bytecode

In this chapter we will look at a bytecode compilation target. We'll combine
this with a section on the virtual machine interface to the bytecode data
structure.

We won't go much into detail on each bytecode operation, that will be more
usefully covered in the compiler and virtual machine chapters. Here, we'll
describe the data structures involved. As such, this will be one of our
shorter chapters. Let's go!


## Design questions

Now that we're talking bytecode, we're at the point of choosing what type of
virtual machine we will be compiling for. The most common type is stack-based
where operands are pushed and popped on and off the stack. This requires
instructions for pushing and popping, with instructions in-between for operating
on values on the stack.

We'll be implementing a register-based VM though. The inspiration for this
comes from Lua 5[^1] which implements a fixed-width bytecode register VM. While
stack based VMs are typically claimed to be simpler, we'll see that the Lua
way of allocating registers per function also has an inherent simplicity and
has performance gains over a stack VM, at least for an interpreted
non jit-compiled VM.

Given register based, fixed-width bytecode, each opcode must reference the
register numbers that it operates on. Thus, for an (untyped) addition
operation `x = a + b`, each of `x`, `a` and `b` must be associated with a
register.

Following Lua, encoding this as a fixed width opcode typically looks like
encoding the operator and operands as 8 bit values packed into a 32 bit opcode
word. That implies, given 8 bits, that there can be a theoretical maximum of
256 registers for a function call. For the addition above, this encoding
might look like this:

```ignore
   32.....24......16.......8.......0
    [reg a ][reg b ][reg x ][Add   ]
```

where the first 8 bits contain the operator, in this case "Add", and the
other three 8 bit slots in the 32 bit word each contain a register number.

For some operators, we will need to encode values larger than 8 bits. As
we will still need space for an operator and a destination register, that
leaves a maximum of 16 bits for larger values.


## Opcodes

We have options in how we describe opcodes in Rust.

1. Each opcode represented by a u32
    * Pros: encoding flexibility, it's just a set of bits
    * Cons: bit shift and masking operations to encode and decode operator
      and operands. This isn't necessarily a big deal but it doesn't allow
      us to leverage the Rust type system to avoid encoding mistakes
1. Each opcode represented by an enum discriminant
    * Pros: operators and operands baked as Rust types at compile time, type
      safe encoding; no bit operations needed
    * Cons: encoding scheme limited to what an enum can represent

The ability to leverage the compiler to prevent opcode encoding errors is
attractive and we won't have any need for complex encodings. We'll use an enum
to represent all possible opcodes and their operands.

Since a Rust enum can contain named values within each variant, this is what
we use to most tightly define our opcodes.

### Opcode size

Since we're using `enum` instead of a directly size-controlled type such as u32
for our opcodes, we have to be more careful about making sure our opcode type
doesn't take up more space than is necessary.  32 bits is ideal for reasons
stated earlier (8 bits for the operator and 8 bits for three operands each.)

Let's do an experiment.

First, we need to define a register as an 8 bit value. We'll also define an
inline literal integer as 16 bits.

```rust,ignore
type Register = u8;
type LiteralInteger = i16;
```

Then we'll create an opcode enum with a few variants that might be typical:

```rust,ignore
#[derive(Copy, Clone)]
enum Opcode {
    Add {
        dest: Register,
        a: Register,
        b: Register
    },
    LoadLiteral {
        dest: Register,
        value: LiteralInteger
    }
}
```

It should be obvious that with an enum like this we can safely pass compiled
bytecode from the compiler to the VM. It should also be clear that this, by
allowing use of `match` statements, will be very ergonomic to work with.

Theoretically, if we never have more than 256 variants, our variants never have
more than 3 `Register` values (or one `Register` and one `LiteralInteger` sized
value), the compiler should be able to pack `Opcode` into 32 bits.

Our test: we hope the output of the following code to be `4` - 4 bytes or 32
bits.

```rust,ignore
use std::mem::size_of;

fn main() {
    println!("Size of Opcode is {}", size_of::<Opcode>());
}
```

And indeed when we run this, we get `Size of Opcode is 4`!

To keep an eye on this situation, we'll put this check into a unit test:

```rust,ignore
{{#include ../interpreter/src/bytecode.rs:DefTestOpcodeIs32Bits}}
```

Now, let's put these `Opcode`s into an array.


## An array of Opcode

We can define this array easily, given that `Array<T>` is a generic type:

```rust,ignore
{{#include ../interpreter/src/bytecode.rs:DefArrayOpcode}}
```

Is this enough to define bytecode? Not quite. We've accommodated 16 bit
literal signed integers, but all kinds of other types can be literals.
We need some way of referencing any literal type in bytecode. For that
we add a `Literals` type, which is just:

```rust,ignore
{{#include ../interpreter/src/bytecode.rs:DefLiterals}}
```

Any opcode that loads a literal (other than a 16 bit signed integer) will
need to reference an object in the `Literals` list. This is easy enough:
just as there's a `LiteralInteger`, we have `LiteralId` defined as

```rust,ignore
pub type LiteralId = u16;
```

This id is an index into the `Literals` list.  This isn't the most efficient
scheme or encoding, but given a preference for fixed 32 bit opcodes, it will
also keep things simple.

The `ByteCode` type, finally, is a composition of `ArrayOpcode` and `Literals`:

```rust,ignore
{{#include ../interpreter/src/bytecode.rs:DefByteCode}}
```


## Bytecode compiler support

There are a few methods implemented for `ByteCode`:

1. `fn push<'guard>(&self, mem: &'MutatorView, op: Opcode) -> Result<(), RuntimeError>`
   This function pushes a new opcode into the `ArrayOpcode` instance.
1. ```rust,ignore
   fn update_jump_offset<'guard>(
       &self,
       mem: &'guard MutatorView,
       instruction: ArraySize,
       offset: JumpOffset,
   ) -> Result<(), RuntimeError>
   ```
   This function, given an instruction index into the `ArrayOpcode` instance,
   and given that the instruction at that index is a type of jump instruction,
   sets the relative jump offset of the instruction to the given offset.
   This is necessary because forward jumps cannot be calculated until all the
   in-between instructions have been compiled first.
1. ```rust,ignore
   fn push_lit<'guard>(
       &self,
       mem: &'guard MutatorView,
       literal: TaggedScopedPtr
   ) -> Result<LiteralId, RuntimeError>
   ```
   This function pushes a literal on to the `Literals` list and returns the
   index - the id - of the item.
1. ```rust,ignore
   fn push_loadlit<'guard>(
       &self,
       mem: &'guard MutatorView,
       dest: Register,
       literal_id: LiteralId,
   ) -> Result<(), RuntimeError>
   ```
   After pushing a literal into the `Literals` list, the corresponding load
   instruction should be pushed into the `ArrayOpcode` list.

`ByteCode` and it's functions combined with the `Opcode` enum are enough to
build a compiler for.


## Bytecode execution support

The previous section described a handful of functions for our compiler to use
to build a `ByteCode` structure.

We'll need a different set of functions for our virtual machine to access
`ByteCode` from an execution standpoint.

The execution view of bytecode is of a contiguous sequence of instructions and
an instruction pointer. We're going to create a separate `ByteCode` instance
for each function that gets compiled, so our execution model will have to
be able to jump between `ByteCode` instances. We'll need a new struct to
represent that:

```rust,ignore
{{#include ../interpreter/src/bytecode.rs:DefInstructionStream}}
```

In this definition, the pointer `instructions` can be updated to point at any
`ByteCode` instance. This allows us to switch between functions by managing
different `ByteCode` pointers as part of a stack of call frames. In support
of this we have:

```rust,ignore
impl InstructionStream {
{{#include ../interpreter/src/bytecode.rs:DefInstructionStreamSwitchFrame}}
}
```

Of course, the main function needed during execution is to retrieve the next
opcode. Ideally, we can keep a pointer that points directly at the next opcode
such that only a single dereference and pointer increment is needed to get
the opcode and advance the instruction pointer. Our implementation is less
efficient for now, requiring a dereference of 1. the `ByteCode` instance and
then 2. the `ArrayOpcode` instance and finally 3. an indexing into the
`ArrayOpcode` instance:

```rust,ignore
{{#include ../interpreter/src/bytecode.rs:DefInstructionStreamGetNextOpcode}}
```


## Conclusion

The full `Opcode` definition can be found in `interpreter/src/bytecode.rs`.

As we work toward implementing a compiler, the next data structure we need is
a dictionary or hash map. This will also build on the foundational
`RawArray<T>` implementation. Let's go on to that now!


---

[^1]: Roberto Ierusalimschy et al, [The Implementation of Lua 5.0](https://www.lua.org/doc/jucs05.pdf)

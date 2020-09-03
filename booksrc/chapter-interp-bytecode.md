# Bytecode

In this chapter we will look at a bytecode compilation target. We'll combine
this with a section on the virtual machine interface to the bytecode data
structure.

We won't go much into detail on each bytecode operation, that will be more
usefully covered in the compiler and virtual machine chapters. Here, we'll
describe the data structures involved. As such, this will be a shorter
chapter. Let's go!


## Design questions

Now that we're talking bytecode, we're at the point of choosing what type of
virtual machine we will be compiling for. The most common type is stack-based.

We'll be implementing a register-based VM though. The inspiration for this
comes from Lua 5[^1] which implements a fixed-width bytecode register VM. While
stack based VMs are typically claimed to be simpler, we'll see that the Lua
way of allocating registers per function also has an inherent simplicity and
has performance gains over a stack VM, specifically for an interpreted
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

Let's do some experiments.

First, we need to define a register as an 8 bit value. We'll also define an
inline literal integer as 16 bits.

```rust,ignore
type Register = u8;
type LiteralInt = i16;
```

Then we'll create an opcode enum with a few variants that might be typical:

```rust,ignore
enum Opcode {
    Add {
        dest: Register,
        a: Register,
        b: Register
    },
    LoadLiteral {
        dest: Register,
        value: LiteralInt
    }
}
```

It should be obvious that with an enum like this we can safely pass compiled
bytecode from the compiler to the VM. It should also be clear that this, by
allowing use of `match` statements, will be very ergonomic to work with.

Theoretically, if our variants never have more than 3 `Register` values, or
one `Register` and one `LiteralInt` sized value, the compiler should be able
to pack `Opcode` into 32 bits.

Our test: we hope the output of the following code to be `4` - 4 bytes or 32
bits.

```rust,ignore
use std::mem::size_of;

fn main() {
    println!("Size of Opcode is {}", size_of::<Opcode>());
}
```

we get `Size of Opcode is 4`!

If we add more than 256 variants or values that sum up to greater than 24 bits,
this will not hold. To keep an eye on this situation, we'll put this check
into a unit test:

```rust,ignore
{{#include ../interpreter/src/bytecode.rs:DefTestOpcodeIs32Bits}}
```

---

[^1]: Roberto Ierusalimschy et al, [The Implementation of Lua 5.0](https://www.lua.org/doc/jucs05.pdf)

# Compiler: Implementation

Before we get into eval and apply let's consider how we will support variables
and lexical scoping.

## Variables and Scopes

As seen in the previous chapter, variable accesses come in three types, as far
as the compiler and VM are concerned: local, nonlocal and global. Each access
uses a different bytecode operation, and so the compiler must be able to
determine what operations to emit at compile time.

Given that we have named function parameters and `let`, we have syntax for
explicit variable declaration within function definitions. This means that we
can easily keep track of whether a variable reference is local, nonlocal or
global.

If a variable wasn't declared as a parameter or in a `let` block, it
must be global and global variables are accessed dynamically by name.

As far as local and nonlocal variables are concerned, the VM does not care
about or consider their names. At the VM level, local and nonlocal variables
are numbered registers. That is, each function's local variables are mapped to
a register numbered between 2 and 255. The compiler must generate the mapping
from variable names to register numbers.

For generating and maintaining mappings, we need data structures for keeping
track of:

- function local variables and their mappings to register numbers
- references to nonlocal variables and their relative stack offsets
- nested scopes within functions

### Named variables

Our first data structure will define a register based variable:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefVariable}}
```

For every named, non-global variable (created by defining function parameters
and `let` blocks) a `Variable` instance is created in the compiler.

The member `closed_over` defaults to `false`. If the compiler detects that the
variable must escape the stack as part of a closure, this flag will be flipped
to `true` (it cannot be set back to `false`.)

### Scope structure

The data structures that manage nesting of scopes and looking up a `Variable`
by name are defined here.

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefScope}}

{{#include ../interpreter/src/compiler.rs:DefNonlocal}}

{{#include ../interpreter/src/compiler.rs:DefVariables}}
```

For every function defined, the compiler maintains an instance of the type
`Variables`.

Each function's `Variables` has a stack of `Scope` instances, each of which has
it's own set of name to `Variable` register number mappings.  The outermost
`Scope` contains the mapping of function parameters to registers.

A nested function's `Variables`, when the function refers to a nesting
function's variable, builds a mapping of nonlocal variable name to relative
stack position of that variable. This is a `NonLocal` - a relative stack frame
offset and the register number within that stack frame of the variable.

In summary, under these definitions:

- A `Nonlocal` instance caches a relative stack location of a nonlocal variable
  for compiling upvalues
- `Scope` manages the mapping of a variable name to the `Variable` register
  number within a single scope
- `Variables` maintains all the nested scopes for a function during compilation
  and caches all the nonlocal references. It also keeps a reference to a parent
  nesting function if there is one, in order to handle lexically scoped
  lookups.

### Retrieving named variables

Whenever a variable is referenced in source code, the mapping to it's register
must be looked up. The result of a lookup is `Option<Binding>`.

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefBinding}}
```

The lookup process checks the local function scopes first.

If the variable is found to be declared there, `Some(Local)` enum variant is
returned. In terms of bytecode, this will translate to a direct register
reference.

Next, any outer function scopes are searched. If the variable is found in any
outer scope, `Some(Upvalue)` variant is returned. The compiler will emit
instructions to copy the value refered to by the upvalue into a function-local
temporary register.

If the lookup for the variable returns `None`, a global lookup instruction is
emitted that will dynamically look up the variable name in the global namespace
and copy the result into a function-local temporary register or raise an error
if the binding does not exist.

## Evaluation

We've just somewhat described what happens in the lower levels of _eval_. Let's
finish the job and put _eval_ in a code context. Here is the definition of a
function compilation data structure:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefCompiler}}
```

The two interesting members are
- `bytecode`, which is an instance of [ByteCode](./chapter-interp-bytecode.md)
- `vars`, an instance of `Variables` which we've described above. This instance
  will be the outermost scope of the `let` or function block being compiled.

The main entrypoint to this structure is the function `compile_function()`:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefCompilerCompileFunctionSig}}
        ...
    }
```

This function will set up a `Variables` scope with the given parameters and call
into function `compile_eval()` for each expression in the function. The full
definition of `compile_eval()` is below, and we'll go into the details of
`compile_function()` later.

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefCompileEval}}
```

Note that the return type is `Result<Register, RuntimeError>`. That is, a
successful _eval_ will return a register where the result will be stored at
runtime.

In the function body, the match branches fall into three categories:

- keywords literals (`nil`, `true`)
- all other literals
- named variables represented by `Symbol`s

What's in the evaluation of the `Symbol` AST type? Locals, nonlocals and
globals!

We can see the generation of special opcodes for retrieving nonlocal and global
values here, whereas a local will resolve directly to an existing register
without the need to generate any additional opcodes.

## Application

To evaluate a function call, we switch over to _apply_:

```rust,ignore
        match *ast_node {
            ...

{{#include ../interpreter/src/compiler.rs:DefCompileEvalPair}}

            ...
        }
```

This is the evaluation of the `Pair` AST type. This represents, visually, the
syntax `(function_name arg1 arg2 argN)` which is, of course, a function call.
_Eval_ cannot tell us the value of a function call, the function must be applied
to it's arguments first. Into _apply_ we recurse.

The first argument to `compile_apply()` is the function name `Symbol`, the
second argument is the list of function arguments.

Since we included the full `compile_eval()` function earlier, it wouldn't be
fair to leave out the definition of `compile_apply()`. Here it is:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefCompileApply}}
```

The `function` parameter is expected to be a `Symbol`, that is, have a _name_
represented by a `Symbol`. Thus, the function is `match`ed on the `Symbol`.

### Caling nil?

Let's follow the compilation of a simple function: `nil?`. This is where we'll
start seeing some of the deeper details of compilation, such as register
allocation and

```rust,ignore
                ...
{{#include ../interpreter/src/compiler.rs:DefCompileApplyIsNil}}
                ...
```

The function `nil?` takes a single argument and returns:

- the `Symbol` for `true` if the value of the argument is `nil`
- `nil` if the argument is _not_ `nil`.

In compiling this function call, a single bytecode opcode will be pushed on to
the `ByteCode` array. This is done in the `Compiler::push_op2()` function. It is
named `push_op2` because the opcode takes two operands: an argument register
and a result destination register. This function is used to compile all simple
function calls that follow the pattern of one argument, one result value. Here
is `push_op2()`:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefCompilerPushOp2}}
```

Let's break the function body down, line by line:

1. `let result = self.acquire_reg();`
    - `self.acquire_reg()`: is called to get an unused register. In this case, we
      need a register to store the result value in. This register acquisition
      follows a stack approach. Registers are acquired (pushed on to the stack
      window) as new variables are declared within a scope, and popped when the
      scope is exited.
    - The type of `result` is `Register` which is an alias for `u8` - an
      unsigned int from 0 to 255.

2. `let reg1 = self.compile_eval(mem, value_from_1_pair(mem, params)?)?;`
    - `value_from_1_pair(mem, params)?`: inspects the argument list and returns
      the argument if there is a single one, otherwise returns an error.
    - `self.compile_eval(mem, <arg>)?`: recurses into the argument to compile it
      down to a something that can be applied to the function call.
    - `let reg1 = <value>;`: where `reg1` will be the argument register to the
      opcode.

3. `self.bytecode.get(mem).push(mem, f(result, reg1))?;`
    - `f(result, reg1)`: calls function `f` that will return the opcode with
      operands applied in `ByteCode` format.
    - In the case of calling `nil?`, the argument `f` is:
        - `|dest, test| Opcode::IsNil { dest, test }`
    - `self.bytecode.get(mem).push(mem, <opcode>)?;`: gets the `ByteCode`
      reference and pushes the opcode on to the end of the bytecode array.

4. `Ok(result)`
    - the result register is returned to the `compile_apply()` function

... and `compile_apply()` itself returns the result register to _it's_ caller.

The pattern for compiling function application, more generally, is this:
- acquire a result register
- acquire any temporary intermediate result registers
- recurse into arguments to compile _them_ first
- emit bytecode for the function, pushing opcodes on to the bytecode array and
  putting the final result in the result register
- release any intermediate registers
- return the result register number

Compiling `nil?` was hopefully quite simple. Let's look at something much more
involved, now.

### Compiling anonymous functions

An anonymous function is defined, syntactically, as:

```ignore
(lambda (param1 param2)
  (expr1)
  (expr2)
  (return-expr))
```

There are 0 or more parameters and 1 or more expresssions in the body of the
function. The last expression of the body provides the return value.

Function compilation is initiated by _apply_. This is because a function is a
compound expression and cannot be reduced to a value by a single _eval_. Here's
the line in `compile_apply()` that calls anonymous function compilation:

```rust,ignore
                ...
{{#include ../interpreter/src/compiler.rs:DefCompileApplyLambda}}
                ...
```

Let's look at the type signature of `compile_anonymous_function()`:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefCompilerCompileAnonymousFunctionSig}}
```

The `params` parameter will be expected to be a `Pair` list: firstly, a list of
parameter names, followed by function body expressions.

The return value from is the same as all the other compilation functions so far:
`Result<Register>`. The compiled code will return a pointer to the function
object in a register.

Here is the function in full:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefCompilerCompileAnonymousFunction}}
```

After converting `Pair` lists to `Vec`s for convenience (wherein parameter names
and function body expressions are separated) the process calls into function
`compile_function()`, which brings us full circle to _eval_.

In `compile_function()`, below:

1. a `Scope` is instantiated and the parameters are pushed on to this outermost
   scope.
2. the function body expressions are iterated over, _eval_-ing each one
3. any upvalues that will be closed over as the compiled-function exits and goes
   out of scope have upvalue instructions generated
4. a `Function` object is returned with all details necessary to running the
   function in the VM environment

Here is `compile_function()`:

```rust,ignore
{{#include ../interpreter/src/compiler.rs:DefCompilerCompileFunction}}
```

Note that in addition to generating upvalue instructions as the
compiled-function goes out of scope, the calling compiler function
`compile_anonymous_function()` will issue a `MakeClosure` opcode such that a
closure object is put in the return register rather than a direct `Function`
object reference.

In our language, a closure object is represented by the `Partial` data structure
- a struct that represents a `Function` object pointer plus closed over values
and/or partially applied arguments. This data structure was described in the
chapter [Virtual Machine: Implementation](./chapter-interp-vm-impl.md).

Thus ends our tour of our interpreter.

## Concluding remarks

In this section, we've looked at a ground-up compiler and virtual machine
implementation within a memory-safe allocation system.

There is, of course, much more to explore in the VM and compiler source code.
The reader is encouraged to experiment with running and modifying the source.

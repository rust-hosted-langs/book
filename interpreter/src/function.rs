use itertools::join;
use std::fmt;

use crate::array::ArrayU16;
use crate::bytecode::ByteCode;
use crate::containers::{Container, ContainerFromSlice, SliceableContainer, StackContainer};
use crate::error::RuntimeError;
use crate::list::List;
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{CellPtr, MutatorScope, ScopedPtr, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// A function object type
// ANCHOR: DefFunction
#[derive(Clone)]
pub struct Function {
    /// name could be a Symbol, or nil if it is an anonymous fn
    name: TaggedCellPtr,
    /// Number of arguments required to activate the function
    arity: u8,
    /// Instructions comprising the function code
    code: CellPtr<ByteCode>,
    /// Param names are stored for introspection of a function signature
    param_names: CellPtr<List>,
    /// List of (CallFrame-index: u8 | Window-index: u8) relative offsets from this function's
    /// declaration where nonlocal variables will be found. Needed when creating a closure. May be
    /// nil
    nonlocal_refs: TaggedCellPtr,
}
// ANCHOR_END: DefFunction

impl Function {
    /// Allocate a Function object on the heap.
    ///
    /// The nonlocal_refs arg must contain a list of 16 bit values composed of two
    /// 8 bit values: CallFrame relative offset << 8 | Window offset
    /// These values should follow the same order as given in param_names
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
        name: TaggedScopedPtr<'guard>,
        param_names: ScopedPtr<'guard, List>,
        code: ScopedPtr<'guard, ByteCode>,
        nonlocal_refs: Option<ScopedPtr<'guard, ArrayU16>>,
    ) -> Result<ScopedPtr<'guard, Function>, RuntimeError> {
        // Store a nil ptr if no nonlocal references are given
        let nonlocal_refs = if let Some(refs_ptr) = nonlocal_refs {
            TaggedCellPtr::new_with(refs_ptr.as_tagged(mem))
        } else {
            TaggedCellPtr::new_nil()
        };

        mem.alloc(Function {
            name: TaggedCellPtr::new_with(name),
            arity: param_names.length() as u8,
            code: CellPtr::new_with(code),
            param_names: CellPtr::new_with(param_names),
            nonlocal_refs: nonlocal_refs,
        })
    }

    /// Return the Function's name as a string slice
    pub fn name<'guard>(&self, guard: &'guard dyn MutatorScope) -> &'guard str {
        let name = self.name.get(guard);
        match *name {
            Value::Symbol(s) => s.as_str(guard),
            _ => "<lambda>",
        }
    }

    /// Return the number of arguments the Function can take
    pub fn arity(&self) -> u8 {
        self.arity
    }

    /// Return the names of the parameters that the Function takes
    pub fn param_names<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, List> {
        self.param_names.get(guard)
    }

    /// Return the ByteCode object associated with the Function
    pub fn code<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, ByteCode> {
        self.code.get(guard)
    }

    /// Return true if the function is a closure - it has nonlocal variable references
    pub fn is_closure<'guard>(&self) -> bool {
        !self.nonlocal_refs.is_nil()
    }

    /// Return a list of nonlocal stack references referenced by the function. It is a panickable
    /// offense to call this when there are no nonlocals referenced by the function. This would
    /// indicate a compiler bug.
    pub fn nonlocals<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
    ) -> ScopedPtr<'guard, ArrayU16> {
        match *self.nonlocal_refs.get(guard) {
            Value::ArrayU16(nonlocals) => nonlocals,
            _ => unreachable!(),
        }
    }
}

impl Print for Function {
    /// Prints a string representation of the function
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let name = self.name.get(guard);
        let params = self.param_names.get(guard);

        let mut param_string = String::new();
        params.access_slice(guard, |items| {
            param_string = join(items.iter().map(|item| item.get(guard)), " ")
        });

        match *name {
            Value::Symbol(s) => write!(f, "(Function {} ({}))", s.as_str(guard), param_string),
            _ => write!(f, "(Function ({}))", param_string),
        }
    }

    /// Prints the disassembled bytecode
    fn debug<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        self.print(guard, f)?;
        write!(f, "\nbytecode follows:\n")?;
        self.code(guard).debug(guard, f)
    }
}

/// A partial function application object type
#[derive(Clone)]
pub struct Partial {
    /// Remaining number of arguments required to activate the function
    arity: u8,
    /// Number of arguments already applied
    used: u8,
    /// List of argument values already applied
    args: CellPtr<List>,
    /// Closure environment - must be either nil or a List of Upvalues
    env: TaggedCellPtr,
    /// Function that will be activated when all arguments are applied
    func: CellPtr<Function>,
}

impl Partial {
    /// Allocate a Partial application of a Function on the heap with the given set of arguments
    pub fn alloc<'guard>(
        mem: &'guard MutatorView,
        function: ScopedPtr<'guard, Function>,
        env: Option<ScopedPtr<'guard, List>>,
        args: &[TaggedCellPtr],
    ) -> Result<ScopedPtr<'guard, Partial>, RuntimeError> {
        let used = args.len() as u8;
        let arity = function.arity() - used;

        // Store a nil ptr if no closure env is given
        let env = if let Some(env_ptr) = env {
            TaggedCellPtr::new_with(env_ptr.as_tagged(mem))
        } else {
            TaggedCellPtr::new_nil()
        };

        // copy args to the Partial's own list
        let args_list: ScopedPtr<'guard, List> = ContainerFromSlice::from_slice(mem, &args)?;

        mem.alloc(Partial {
            arity,
            used,
            args: CellPtr::new_with(args_list),
            env,
            func: CellPtr::new_with(function),
        })
    }

    /// Clone an existing Partial application, appending the given arguments to the list
    pub fn alloc_clone<'guard>(
        mem: &'guard MutatorView,
        partial: ScopedPtr<'guard, Partial>,
        new_args: &[TaggedCellPtr],
    ) -> Result<ScopedPtr<'guard, Partial>, RuntimeError> {
        let used = partial.used() + new_args.len() as u8;
        let arity = partial.arity() - new_args.len() as u8;

        // clone the parent Partial's args
        let arg_list = List::alloc_clone(mem, partial.args(mem))?;
        // append any new args
        for arg in new_args {
            arg_list.push(mem, arg.clone())?
        }

        mem.alloc(Partial {
            arity,
            used,
            args: CellPtr::new_with(arg_list),
            env: partial.env.clone(),
            func: partial.func.clone(),
        })
    }

    /// Return the number of arguments this Partial needs before the function can be called
    pub fn arity(&self) -> u8 {
        self.arity
    }

    /// Return the count of arguments already applied
    pub fn used(&self) -> u8 {
        self.used
    }

    /// Return the arguments already supplied to the Partial
    pub fn args<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, List> {
        self.args.get(guard)
    }

    /// Return the closure environment. This will be nil if the Partial does not close over any
    /// variables.
    pub fn closure_env(&self) -> TaggedCellPtr {
        self.env.clone()
    }

    /// Return the Function object that the Partial will call
    pub fn function<'guard>(&self, guard: &'guard dyn MutatorScope) -> ScopedPtr<'guard, Function> {
        self.func.get(guard)
    }
}

impl Print for Partial {
    /// Prints a string representation of the Partial object
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let function = self.func.get(guard);
        let name = function.name.get(guard);
        let params = function.param_names.get(guard);

        let mut param_string = String::new();
        params.access_slice(guard, |items| {
            let start = self.used as usize;
            param_string = join(items[start..].iter().map(|item| item.get(guard)), " ")
        });

        match *name {
            Value::Symbol(s) => write!(f, "(Partial {} ({}))", s.as_str(guard), param_string),
            _ => write!(f, "(Partial ({}))", param_string),
        }
    }

    /// Prints the associated function's disassembled bytecode
    fn debug<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        self.print(guard, f)?;
        write!(f, "\nbytecode follows:\n")?;
        self.func.get(guard).code(guard).debug(guard, f)
    }
}

/// A list of arguments to apply to functions
pub struct CurriedArguments {
    // TODO
// not sure of the mechanics of this.
// The ghc runtime would push all these to the stack and then consume the stack with
// function continuations
}

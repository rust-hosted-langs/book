use crate::compiler::compile;
use crate::error::{ErrorKind, RuntimeError};
use crate::memory::{Mutator, MutatorView};
use crate::parser::parse;
use crate::safeptr::{CellPtr, TaggedScopedPtr};
use crate::vm::Thread;

/// A mutator that returns a Repl instance
pub struct RepMaker {}

impl Mutator for RepMaker {
    type Input = ();
    type Output = ReadEvalPrint;

    fn run(&self, mem: &MutatorView, _input: ()) -> Result<ReadEvalPrint, RuntimeError> {
        ReadEvalPrint::alloc(mem)
    }
}

/// Mutator that implements the VM
pub struct ReadEvalPrint {
    main_thread: CellPtr<Thread>,
}

impl ReadEvalPrint {
    pub fn alloc(mem: &MutatorView) -> Result<ReadEvalPrint, RuntimeError> {
        Ok(ReadEvalPrint {
            main_thread: CellPtr::new_with(Thread::alloc(mem)?),
        })
    }
}

impl Mutator for ReadEvalPrint {
    type Input = String;
    type Output = ();

    fn run(&self, mem: &MutatorView, line: String) -> Result<(), RuntimeError> {
        let thread = self.main_thread.get(mem);

        // If the first 2 chars of the line are ":d", then the user has requested a debug
        // representation
        let (line, debug) = if line.starts_with(":d ") {
            (&line[3..], true)
        } else {
            (line.as_str(), false)
        };

        match (|mem, line| -> Result<TaggedScopedPtr, RuntimeError> {
            let value = parse(mem, line)?;

            if debug {
                println!(
                    "# Debug\n## Input:\n```\n{}\n```\n## Parsed:\n```\n{:?}\n```",
                    line, value
                );
            }

            let function = compile(mem, value)?;

            if debug {
                println!("## Compiled:\n```\n{:?}\n```", function);
            }

            let value = thread.quick_vm_eval(mem, function)?;

            if debug {
                println!("## Evaluated:\n```\n{:?}\n```\n", value);
            }

            Ok(value)
        })(mem, &line)
        {
            Ok(value) => println!("{}", value),

            Err(e) => {
                match e.error_kind() {
                    // non-fatal repl errors
                    ErrorKind::LexerError(_) => e.print_with_source(&line),
                    ErrorKind::ParseError(_) => e.print_with_source(&line),
                    ErrorKind::EvalError(_) => e.print_with_source(&line),
                    _ => return Err(e),
                }
            }
        }

        Ok(())
    }
}

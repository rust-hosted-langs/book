use std::cell::Cell;
use std::fmt;

use crate::error::{err_eval, RuntimeError, SourcePos};
use crate::memory::MutatorView;
use crate::printer::Print;
use crate::safeptr::{MutatorScope, ScopedPtr, TaggedCellPtr, TaggedScopedPtr};
use crate::taggedptr::Value;

/// A Pair of pointers, like a Cons cell of old
// ANCHOR: DefPair
#[derive(Clone)]
pub struct Pair {
    pub first: TaggedCellPtr,
    pub second: TaggedCellPtr,
    // Possible source code positions of the first and second values
    pub first_pos: Cell<Option<SourcePos>>,
    pub second_pos: Cell<Option<SourcePos>>,
}
// ANCHOR_END: DefPair

impl Pair {
    /// Return a new empty Pair instance
    // ANCHOR: DefPairNew
    pub fn new() -> Pair {
        Pair {
            first: TaggedCellPtr::new_nil(),
            second: TaggedCellPtr::new_nil(),
            first_pos: Cell::new(None),
            second_pos: Cell::new(None),
        }
    }
    // ANCHOR_END: DefPairNew

    /// Set Pair.second to a new Pair with newPair.first set to the value
    pub fn append<'guard>(
        &self,
        mem: &'guard MutatorView,
        value: TaggedScopedPtr<'guard>,
    ) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
        let pair = Pair::new();
        pair.first.set(value);

        let pair = mem.alloc_tagged(pair)?;
        self.second.set(pair);

        Ok(pair)
    }

    /// Set Pair.second to the given value
    pub fn dot<'guard>(&self, value: TaggedScopedPtr<'guard>) {
        self.second.set(value);
    }

    pub fn set_first_source_code_pos(&self, pos: SourcePos) {
        self.first_pos.set(Some(pos));
    }

    pub fn set_second_source_code_pos(&self, pos: SourcePos) {
        self.second_pos.set(Some(pos));
    }
}

impl Print for Pair {
    fn print<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let mut tail = ScopedPtr::new(guard, self);

        write!(f, "({}", tail.first.get(guard))?;

        while let Value::Pair(next) = *tail.second.get(guard) {
            tail = next;
            write!(f, " {}", tail.first.get(guard))?;
        }

        // clunky way to print anything but nil
        let second = *tail.second.get(guard);
        match second {
            Value::Nil => (),
            _ => write!(f, " . {}", second)?,
        }

        write!(f, ")")
    }

    // In debug print, use dot notation
    fn debug<'guard>(
        &self,
        guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(
            f,
            "({:?} . {:?})",
            self.first.get(guard),
            self.second.get(guard)
        )
    }
}

/// Link the two values `head` and `rest` into a Pair instance
// ANCHOR: DefCons
pub fn cons<'guard>(
    mem: &'guard MutatorView,
    head: TaggedScopedPtr<'guard>,
    rest: TaggedScopedPtr<'guard>,
) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
    let pair = Pair::new();
    pair.first.set(head);
    pair.second.set(rest);
    mem.alloc_tagged(pair)
}
// ANCHOR_END: DefCons

/// Unpack a list of Pair instances into a Vec
pub fn vec_from_pairs<'guard>(
    guard: &'guard dyn MutatorScope,
    pair_list: TaggedScopedPtr<'guard>,
) -> Result<Vec<TaggedScopedPtr<'guard>>, RuntimeError> {
    match *pair_list {
        Value::Pair(pair) => {
            let mut result = Vec::new();

            result.push(pair.first.get(guard));

            let mut next = pair.second.get(guard);
            while let Value::Pair(next_pair) = *next {
                result.push(next_pair.first.get(guard));
                next = next_pair.second.get(guard);
            }

            // we've terminated the list, but correctly?
            match *next {
                Value::Nil => Ok(result),
                _ => Err(err_eval("Incorrectly terminated Pair list")),
            }
        }
        Value::Nil => Ok(Vec::new()),
        _ => Err(err_eval("Expected a Pair")),
    }
}

/// Unpack a list of Pair instances into a Vec, expecting n values
pub fn vec_from_n_pairs<'guard>(
    guard: &'guard dyn MutatorScope,
    pair_list: TaggedScopedPtr<'guard>,
    expect_length: usize,
) -> Result<Vec<TaggedScopedPtr<'guard>>, RuntimeError> {
    let result = vec_from_pairs(guard, pair_list)?;

    if result.len() != expect_length {
        return Err(err_eval(&format!(
            "Pair list has {} items, expected {}",
            result.len(),
            expect_length
        )));
    }

    Ok(result)
}

/// Convenience function for unpacking a list of Pair instances into one value
pub fn value_from_1_pair<'guard>(
    guard: &'guard dyn MutatorScope,
    pair_list: TaggedScopedPtr<'guard>,
) -> Result<TaggedScopedPtr<'guard>, RuntimeError> {
    let result = vec_from_pairs(guard, pair_list)?;

    match result.as_slice() {
        [first] => Ok(*first),
        _ => Err(err_eval(&format!(
            "Pair list has {} items, expected 1",
            result.len()
        ))),
    }
}

/// Convenience function for unpacking a list of Pair instances into two values
pub fn values_from_2_pairs<'guard>(
    guard: &'guard dyn MutatorScope,
    pair_list: TaggedScopedPtr<'guard>,
) -> Result<(TaggedScopedPtr<'guard>, TaggedScopedPtr<'guard>), RuntimeError> {
    let result = vec_from_pairs(guard, pair_list)?;

    match result.as_slice() {
        [first, second] => Ok((*first, *second)),
        _ => Err(err_eval(&format!(
            "Pair list has {} items, expected 2",
            result.len()
        ))),
    }
}

/// Convenience function for unpacking a list of Pair instances into three values
pub fn values_from_3_pairs<'guard>(
    guard: &'guard dyn MutatorScope,
    pair_list: TaggedScopedPtr<'guard>,
) -> Result<
    (
        TaggedScopedPtr<'guard>,
        TaggedScopedPtr<'guard>,
        TaggedScopedPtr<'guard>,
    ),
    RuntimeError,
> {
    let result = vec_from_pairs(guard, pair_list)?;

    match result.as_slice() {
        [first, second, third] => Ok((*first, *second, *third)),
        _ => Err(err_eval(&format!(
            "Pair list has {} items, expected 3",
            result.len()
        ))),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::RuntimeError;
    use crate::memory::{Memory, Mutator, MutatorView};

    fn test_helper(test_fn: fn(&MutatorView) -> Result<(), RuntimeError>) {
        let mem = Memory::new();

        struct Test {}
        impl Mutator for Test {
            type Input = fn(&MutatorView) -> Result<(), RuntimeError>;
            type Output = ();

            fn run(
                &self,
                mem: &MutatorView,
                test_fn: Self::Input,
            ) -> Result<Self::Output, RuntimeError> {
                test_fn(mem)
            }
        }

        let test = Test {};
        mem.mutate(&test, test_fn).unwrap();
    }

    #[test]
    fn unpack_pair_list_bad() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            // this is not a Pair, it's an error to convert it to a Vec
            let thing = mem.lookup_sym("nothing");

            let result = vec_from_pairs(mem, thing);

            assert!(result.is_err());

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn unpack_pair_list_n_values() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let mut head = cons(mem, mem.lookup_sym("alice"), mem.nil())?;
            head = cons(mem, mem.lookup_sym("bob"), head)?;
            head = cons(mem, mem.lookup_sym("carlos"), head)?;
            head = cons(mem, mem.lookup_sym("dave"), head)?;
            head = cons(mem, mem.lookup_sym("eve"), head)?;

            let result = vec_from_pairs(mem, head);

            assert!(result.is_ok());

            let inside = result.unwrap();
            assert!(
                inside
                    == vec![
                        mem.lookup_sym("eve"),
                        mem.lookup_sym("dave"),
                        mem.lookup_sym("carlos"),
                        mem.lookup_sym("bob"),
                        mem.lookup_sym("alice")
                    ]
            );

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn unpack_pair_list_bad_terminator() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let mut head = cons(
                mem,
                mem.lookup_sym("alice"),
                mem.lookup_sym("non-terminator"),
            )?;
            head = cons(mem, mem.lookup_sym("bob"), head)?;
            head = cons(mem, mem.lookup_sym("carlos"), head)?;
            head = cons(mem, mem.lookup_sym("dave"), head)?;
            head = cons(mem, mem.lookup_sym("eve"), head)?;

            let result = vec_from_pairs(mem, head);

            assert!(result.is_err());

            Ok(())
        }

        test_helper(test_inner)
    }

    #[test]
    fn unpack_pair_list_n_values_expected() {
        fn test_inner(mem: &MutatorView) -> Result<(), RuntimeError> {
            let mut head = cons(mem, mem.lookup_sym("alice"), mem.nil())?;
            head = cons(mem, mem.lookup_sym("bob"), head)?;
            head = cons(mem, mem.lookup_sym("carlos"), head)?;
            head = cons(mem, mem.lookup_sym("dave"), head)?;
            head = cons(mem, mem.lookup_sym("eve"), head)?;

            let result = vec_from_n_pairs(mem, head, 5);
            assert!(result.is_ok());

            let result = vec_from_n_pairs(mem, head, 3);
            assert!(result.is_err());

            let result = vec_from_n_pairs(mem, head, 6);
            assert!(result.is_err());

            Ok(())
        }

        test_helper(test_inner)
    }
}

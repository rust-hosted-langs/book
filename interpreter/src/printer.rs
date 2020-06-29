use std::fmt;
//use std::io;

use crate::safeptr::MutatorScope;
use crate::taggedptr::Value;

/// Trait for using a `Value` lifted pointer in the `Display` trait
pub trait Print {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result;

    fn debug<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        self.print(_guard, f)
    }

    //fn repr<'guard, F: fmt::Write>(&self, _guard: &'guard dyn MutatorScope, f: &mut F) -> fmt::Result;

    //fn output<'guard, F: io::Write>(
    //    &self,
    //    _guard: &'guard dyn MutatorScope,
    //    f: &mut F,
    //) -> io::Result<()>;
}

pub fn print(value: Value) -> String {
    format!("{}", value)
}

pub fn debug(value: Value) -> String {
    format!("{:?}", value)
}

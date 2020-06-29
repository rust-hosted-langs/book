/// An integer type - TODO
use std::fmt;

use crate::array::Array;
use crate::printer::Print;
use crate::safeptr::MutatorScope;

/// TODO A heap-allocated number
pub struct NumberObject {
    _value: Array<u64>,
}

impl Print for NumberObject {
    fn print<'guard>(
        &self,
        _guard: &'guard dyn MutatorScope,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        // TODO
        write!(f, "NumberObject(nan)")
    }
}

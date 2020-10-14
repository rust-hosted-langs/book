/// Scope-guard limited Hashable trait type
use std::hash::Hasher;

use crate::safeptr::MutatorScope;

// ANCHOR: DefHashable
/// Similar to Hash but for use in a mutator lifetime-limited scope
pub trait Hashable {
    fn hash<'guard, H: Hasher>(&self, _guard: &'guard dyn MutatorScope, hasher: &mut H);
}
// ANCHOR_END: DefHashable

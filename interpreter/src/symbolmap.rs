/// Implements str interning for mapping Symbol names to unique pointers
use std::cell::RefCell;
use std::collections::HashMap;

use stickyimmix::{AllocRaw, RawPtr};

use crate::arena::Arena;
use crate::symbol::Symbol;

/// A mapping of symbol names (Strings) to Symbol pointers. Only one copy of the symbol
/// name String is kept; a Symbol resides in managed memory with a raw pointer to the
/// String. Thus the lifetime of the SymbolMap must be at least the lifetime of the
/// managed memory. This is arranged here by maintaining Symbol memory alongside the
/// mapping HashMap.
///
/// No Symbol is ever deleted. Symbol name strings must be immutable.
// ANCHOR: DefSymbolMap
pub struct SymbolMap {
    map: RefCell<HashMap<String, RawPtr<Symbol>>>,
    arena: Arena,
}
// ANCHOR_END: DefSymbolMap

impl SymbolMap {
    pub fn new() -> SymbolMap {
        SymbolMap {
            map: RefCell::new(HashMap::new()),
            arena: Arena::new(),
        }
    }

    pub fn lookup(&self, name: &str) -> RawPtr<Symbol> {
        // Can't take a map.entry(name) without providing an owned String, i.e. cloning 'name'
        // Can't insert a new entry with just a reference without hashing twice, and cloning 'name'
        // The common case, lookups, should be fast, inserts can be slower.

        {
            if let Some(ptr) = self.map.borrow().get(name) {
                return *ptr;
            }
        }

        let name = String::from(name);
        let ptr = self.arena.alloc(Symbol::new(&name)).unwrap();
        self.map.borrow_mut().insert(name, ptr);
        ptr
    }
}

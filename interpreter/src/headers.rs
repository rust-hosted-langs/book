/// Defines an `ObjectHeader` type to immediately preceed each heap allocated
/// object, which also contains a type tag but with space for many more types.
use stickyimmix::{
    AllocHeader, AllocObject, AllocRaw, AllocTypeId, ArraySize, Mark, RawPtr, SizeClass,
};

use crate::array::{ArrayU16, ArrayU32, ArrayU8};
use crate::bytecode::{ArrayOpcode, ByteCode, InstructionStream};
use crate::dict::Dict;
use crate::function::{Function, Partial};
use crate::list::List;
use crate::memory::HeapStorage;
use crate::number::NumberObject;
use crate::pair::Pair;
use crate::pointerops::{AsNonNull, Tagged};
use crate::symbol::Symbol;
use crate::taggedptr::FatPtr;
use crate::text::Text;
use crate::vm::{CallFrameList, Thread, Upvalue};

/// Recognized heap-allocated types.
/// This should represent every type native to the runtime with the exception of tagged pointer inline value
/// types.
// ANCHOR: DefTypeList
#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeList {
    Pair,
    Symbol,
    NumberObject,
    Text,
    Array, // type id for array backing bytes
    List,
    ArrayU8,
    ArrayU16,
    ArrayU32,
    Dict,
    ArrayOpcode,
    ByteCode,
    InstructionStream,
    Function,
    Partial,
    CallFrameList,
    Thread,
    Upvalue,
}

// Mark this as a Stickyimmix type-identifier type
impl AllocTypeId for TypeList {}
// ANCHOR_END: DefTypeList

/// A heap-allocated object header
pub struct ObjectHeader {
    mark: Mark,
    size_class: SizeClass,
    type_id: TypeList,
    size_bytes: u32,
}

impl ObjectHeader {
    /// Convert the ObjectHeader address to a FatPtr pointing at the object itself
    pub fn get_object_fatptr(&self) -> FatPtr {
        let ptr_to_self = self.non_null_ptr();
        let object_addr = HeapStorage::get_object(ptr_to_self);

        // Only Object* types should be derived from the header.
        // Symbol, Pair and Number should have been derived from a pointer tag.
        //
        // NOTE any type that is a runtime dynamic type must be added to the below list
        match self.type_id {
            TypeList::NumberObject => {
                FatPtr::NumberObject(RawPtr::untag(object_addr.cast::<NumberObject>()))
            }
            TypeList::Text => FatPtr::Text(RawPtr::untag(object_addr.cast::<Text>())),
            TypeList::ArrayU8 => FatPtr::ArrayU8(RawPtr::untag(object_addr.cast::<ArrayU8>())),
            TypeList::ArrayU16 => FatPtr::ArrayU16(RawPtr::untag(object_addr.cast::<ArrayU16>())),
            TypeList::ArrayU32 => FatPtr::ArrayU32(RawPtr::untag(object_addr.cast::<ArrayU32>())),
            TypeList::List => FatPtr::List(RawPtr::untag(object_addr.cast::<List>())),
            TypeList::Dict => FatPtr::Dict(RawPtr::untag(object_addr.cast::<Dict>())),
            TypeList::Function => FatPtr::Function(RawPtr::untag(object_addr.cast::<Function>())),
            TypeList::Partial => FatPtr::Partial(RawPtr::untag(object_addr.cast::<Partial>())),
            TypeList::Upvalue => FatPtr::Upvalue(RawPtr::untag(object_addr.cast::<Upvalue>())),

            _ => panic!("Invalid ObjectHeader type tag {:?}!", self.type_id),
        }
    }
}

impl AsNonNull for ObjectHeader {}

impl AllocHeader for ObjectHeader {
    type TypeId = TypeList;

    fn new<O: AllocObject<Self::TypeId>>(
        size: u32,
        size_class: SizeClass,
        mark: Mark,
    ) -> ObjectHeader {
        ObjectHeader {
            mark,
            size_class,
            type_id: O::TYPE_ID,
            size_bytes: size,
        }
    }

    fn new_array(size: ArraySize, size_class: SizeClass, mark: Mark) -> ObjectHeader {
        ObjectHeader {
            mark,
            size_class,
            type_id: TypeList::Array,
            size_bytes: size as u32,
        }
    }

    fn mark(&mut self) {
        self.mark = Mark::Marked;
    }

    fn is_marked(&self) -> bool {
        self.mark == Mark::Marked
    }

    fn size_class(&self) -> SizeClass {
        self.size_class
    }

    fn size(&self) -> u32 {
        self.size_bytes
    }

    fn type_id(&self) -> TypeList {
        self.type_id
    }
}

/// Apply the type ID to each native type
macro_rules! declare_allocobject {
    ($T:ty, $I:tt) => {
        impl AllocObject<TypeList> for $T {
            const TYPE_ID: TypeList = TypeList::$I;
        }
    };
}

declare_allocobject!(Symbol, Symbol);
declare_allocobject!(Pair, Pair);
declare_allocobject!(NumberObject, NumberObject);
declare_allocobject!(Text, Text);
declare_allocobject!(List, List);
declare_allocobject!(ArrayU8, ArrayU8);
declare_allocobject!(ArrayU16, ArrayU16);
declare_allocobject!(ArrayU32, ArrayU32);
declare_allocobject!(Dict, Dict);
declare_allocobject!(ArrayOpcode, ArrayOpcode);
declare_allocobject!(ByteCode, ByteCode);
declare_allocobject!(InstructionStream, InstructionStream);
declare_allocobject!(Function, Function);
declare_allocobject!(Partial, Partial);
declare_allocobject!(CallFrameList, CallFrameList);
declare_allocobject!(Thread, Thread);
declare_allocobject!(Upvalue, Upvalue);

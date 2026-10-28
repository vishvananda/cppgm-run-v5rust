//! Host data-address policy from typed final or forward symbol facts. See NOTICE.
use super::{
    memory::{Base, Memory},
    object_compile::ObjectModel,
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    x86::Register as R,
};
use crate::lowir::{
    model::{GlobalInit, Program, SymbolDetail, SymbolId},
    vocabulary::{Binding, Storage, Type},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SymbolAccess {
    Direct,
    Got,
    TlsLocal,
    TlsInitial,
}

impl SymbolAccess {
    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Direct => "",
            Self::Got => "got",
            Self::TlsLocal => "tpoff",
            Self::TlsInitial => "gottpoff",
        }
    }
}

pub(super) fn host_symbol_access(program: &Program, symbol: SymbolId) -> SymbolAccess {
    let (binding, storage, defined, replaceable) = match program.symbol(symbol).detail {
        SymbolDetail::Global(_) => {
            let Some(global) = program.global(program.symbol(symbol)) else {
                return SymbolAccess::Direct;
            };
            (
                global.metadata.binding,
                global.metadata.storage,
                !matches!(global.init, GlobalInit::Declaration),
                global.metadata.flags & 64 != 0,
            )
        }
        SymbolDetail::Unresolved => {
            let Some(global) = program.forward_global(symbol) else {
                return SymbolAccess::Direct;
            };
            if global.binding.is_none() {
                return SymbolAccess::Direct;
            }
            (global.binding, global.storage, global.defined, false)
        }
        SymbolDetail::Function(_) => return SymbolAccess::Direct,
    };
    if storage == Some(Storage::ThreadLocal) {
        return if defined {
            SymbolAccess::TlsLocal
        } else {
            SymbolAccess::TlsInitial
        };
    }
    if if defined {
        binding == Some(Binding::Weak) && replaceable
    } else {
        binding != Some(Binding::Internal)
    } {
        SymbolAccess::Got
    } else {
        SymbolAccess::Direct
    }
}

impl Selection<'_> {
    pub(super) fn symbol_access(&self, symbol: SymbolId) -> SymbolAccess {
        if self.mir.object_model == ObjectModel::Host {
            host_symbol_access(self.program, symbol)
        } else {
            SymbolAccess::Direct
        }
    }
    pub(super) fn symbol_operand(&self, symbol: SymbolId, addend: i32) -> Operand {
        match self.symbol_access(symbol) {
            SymbolAccess::Direct if addend == 0 => Operand::Symbol(symbol),
            SymbolAccess::Direct => Operand::SymbolOffset(symbol, addend),
            access => Operand::AccessedSymbol(symbol, addend, access),
        }
    }
    pub(super) fn symbol_memory(&mut self, symbol: SymbolId, addend: i32, scratch: R) -> Memory {
        if self.symbol_access(symbol) == SymbolAccess::Direct {
            return Memory::new(Base::Symbol(symbol), addend);
        }
        self.emit(ScalarOp::Move(
            Type::Void,
            scratch,
            self.symbol_operand(symbol, 0),
        ));
        Memory::new(Base::Register(scratch), addend)
    }
}

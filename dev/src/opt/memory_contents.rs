//! Constant global reads and library identities from `function_ir.cpp`/`program.cpp`.
//! See NOTICE.
use crate::lowir::{
    literals::Constant,
    model::{DataItem, Global, GlobalInit, Linkage, Program, SymbolId},
    vocabulary::{Storage, Type},
};

pub(super) fn bytes(ty: Type) -> u64 {
    match ty {
        Type::Void => 0,
        Type::I1 | Type::I8 | Type::U8 => 1,
        Type::I16 | Type::U16 => 2,
        Type::I32 | Type::U32 | Type::F32 => 4,
        Type::I64 | Type::F64 | Type::Ptr => 8,
        Type::I128 | Type::F80 => 16,
        Type::Object { bytes, .. } => bytes,
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LibraryCall {
    Strlen,
    Memset,
    Memcpy,
    Memcmp,
}
pub(super) fn library(program: &Program, symbol: SymbolId, kind: LibraryCall) -> bool {
    let Some(function) = program.function(program.symbol(symbol)) else {
        return false;
    };
    let name = program.names.get(
        function
            .metadata
            .object
            .filter(|&id| !program.names.bytes(id).is_empty())
            .unwrap_or(function.name),
    );
    let (builtin, plain) = match kind {
        LibraryCall::Strlen => ("cppgm_builtin_strlen", "strlen"),
        LibraryCall::Memset => ("cppgm_builtin_memset", "memset"),
        LibraryCall::Memcpy => ("cppgm_builtin_memcpy", "memcpy"),
        LibraryCall::Memcmp => ("cppgm_builtin_memcmp", "memcmp"),
    };
    name == builtin || function.metadata.linkage == Linkage::C && name == plain
}
pub(super) fn contents<'a>(
    program: &'a Program,
    symbol: SymbolId,
    unwritten: &[bool],
) -> Option<&'a Global> {
    let global = program.global(program.symbol(symbol))?;
    if matches!(global.init, GlobalInit::Declaration)
        || global.metadata.flags & 64 != 0
        || !(global.metadata.storage == Some(Storage::ReadOnly)
            || unwritten.get(symbol.index()).copied().unwrap_or(false))
    {
        return None;
    }
    Some(global)
}
fn item_type(item: DataItem) -> Option<Type> {
    match item {
        DataItem::Zero(_) => None,
        DataItem::Constant(ty, _) => Some(ty),
        DataItem::Address(..) => Some(Type::Ptr),
    }
}
fn aligned(at: u64, ty: Type) -> u64 {
    let align = bytes(ty).clamp(1, 16);
    at.wrapping_add(align - 1) & !(align - 1)
}
pub(super) fn address(
    program: &Program,
    symbol: SymbolId,
    offset: i64,
    unwritten: &[bool],
    with_addend: bool,
) -> Option<(SymbolId, i64)> {
    let want = u64::try_from(offset).ok()?;
    let GlobalInit::Data(items) = &contents(program, symbol, unwritten)?.init else {
        return None;
    };
    if items.len() > 4096 {
        return None;
    }
    let mut at = 0_u64;
    for &item in items {
        if at > want {
            break;
        }
        let Some(ty) = item_type(item) else {
            if let DataItem::Zero(amount) = item {
                at = at.wrapping_add(u64::from_le_bytes(amount.to_le_bytes()));
            }
            continue;
        };
        at = aligned(at, ty);
        if want == at {
            if let DataItem::Address(_, target, addend) = item
                && (with_addend || addend == 0)
                && target.index() < program.symbols().len()
            {
                return Some((target, addend));
            }
            return None;
        }
        at = at.wrapping_add(bytes(ty));
    }
    None
}
pub(super) fn integer(
    program: &Program,
    symbol: SymbolId,
    offset: i64,
    ty: Type,
    unwritten: &[bool],
) -> Option<u128> {
    let want = u64::try_from(offset).ok()?;
    if ty.bits() == 0 {
        return None;
    }
    let global = contents(program, symbol, unwritten)?;
    let size = bytes(ty);
    match &global.init {
        GlobalInit::Zero => return Some(0),
        GlobalInit::Constant(Constant::Integer(value))
            if want == 0 && global.ty.is_some_and(|t| t.bits() != 0 && bytes(t) == size) =>
        {
            return Some(*value);
        }
        _ => (),
    }
    let GlobalInit::Data(items) = &global.init else {
        return None;
    };
    if items.len() > 4096 || size == 0 || size > 16 {
        return None;
    }
    let end = want.wrapping_add(size);
    let mut covered = 0_u64;
    let mut at = 0_u64;
    let mut out = [0_u8; 16];
    for &item in items {
        if at >= end {
            break;
        }
        let Some(item_ty) = item_type(item) else {
            if let DataItem::Zero(amount) = item {
                let stop = at.wrapping_add(u64::from_le_bytes(amount.to_le_bytes()));
                for _ in at.max(want)..stop.min(end) {
                    covered += 1;
                }
                at = stop;
            }
            continue;
        };
        let count = bytes(item_ty);
        at = aligned(at, item_ty);
        let stop = at.wrapping_add(count);
        if want < stop && end > at {
            let DataItem::Constant(t, Constant::Integer(bits)) = item else {
                return None;
            };
            if t.bits() == 0 {
                return None;
            }
            for b in at.max(want)..stop.min(end) {
                let k = u32::try_from(b - at).ok()?;
                out[usize::try_from(b - want).ok()?] =
                    u8::try_from((bits >> (8 * k)) & 255).ok()?;
                covered += 1;
            }
        }
        at = stop;
    }
    (covered == size).then(|| u128::from_le_bytes(out))
}

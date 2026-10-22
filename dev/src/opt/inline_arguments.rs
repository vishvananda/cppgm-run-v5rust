//! Call boundary compatibility and known/constant arguments from `opt/inliner.cpp`.
//! See NOTICE.
use super::memory_contents::bytes;
use crate::lowir::{
    model::{Body, Call, GlobalInit, Program, SlotId, SymbolId, Value},
    vocabulary::{Pass, Storage, Type},
};
pub(super) fn addressed_slot(body: &Body, mut value: Value) -> Option<SlotId> {
    for _ in 0..8 {
        let Value::Temp(t) = value else {
            return None;
        };
        if body.temp_is_parameter(t) {
            return None;
        }
        match body.instruction(body.temp(t).definition?).kind {
            crate::lowir::model::InstructionKind::Addr(Value::Slot(s)) => return Some(s),
            crate::lowir::model::InstructionKind::Index(_, _, base, _)
            | crate::lowir::model::InstructionKind::Copy(Type::Ptr, base) => value = base,
            _ => return None,
        }
    }
    None
}
pub(super) fn compatible(program: &Program, body: &Body, call: &Call, callee: SymbolId) -> bool {
    let Some(f) = program.function(program.symbol(callee)) else {
        return false;
    };
    let sig = program.signature(f.signature);
    let args = body.arguments(call.arguments);
    if args.len() != sig.parameters.len() || call.result != sig.result {
        return false;
    }
    if let Some(stated) = call.signature {
        let stated = program.signature(stated);
        if stated.parameters.len() != sig.parameters.len()
            || stated.result != sig.result
            || stated
                .parameters
                .iter()
                .zip(&sig.parameters)
                .any(|(a, b)| a.ty != b.ty)
        {
            return false;
        }
    }
    for (p, (parameter, &arg)) in sig.parameters.iter().zip(args).enumerate() {
        if parameter.pass.is_some()
            && !matches!(arg, Value::Symbol(_))
            && !matches!(arg,Value::Temp(t)if body.temp(t).ty==Some(Type::Ptr))
        {
            return false;
        }
        if matches!(parameter.ty, Type::Object { .. }) {
            if matches!(arg, Value::Symbol(_)) {
                return false;
            }
            if let Value::Slot(s) = arg
                && args
                    .iter()
                    .enumerate()
                    .any(|(q, &v)| q != p && addressed_slot(body, v) == Some(s))
            {
                return false;
            }
        }
        if let Value::Temp(t) = arg
            && let Some(have) = body.temp(t).ty
            && have != parameter.ty
            && (matches!(have, Type::Object { .. })
                || matches!(parameter.ty, Type::Object { .. })
                || (have.float_rank() != 0) != (parameter.ty.float_rank() != 0)
                || bytes(have) != bytes(parameter.ty))
        {
            return false;
        }
    }
    true
}
pub(super) fn constant(program: &Program, body: &Body, mut value: Value) -> Option<Value> {
    if matches!(value, Value::Integer(_)) {
        return Some(value);
    }
    if let Value::Temp(t) = value
        && !body.temp_is_parameter(t)
        && let Some(def) = body.temp(t).definition
        && let crate::lowir::model::InstructionKind::Addr(base) = body.instruction(def).kind
    {
        value = base;
    }
    let Value::Symbol(symbol) = value else {
        return None;
    };
    let global = program.global(program.symbol(symbol))?;
    (!matches!(global.init, GlobalInit::Declaration)
        && global.metadata.storage == Some(Storage::ReadOnly)
        && global.metadata.flags & 64 == 0)
        .then_some(value)
}
pub(super) fn known_target(program: &Program, body: &Body, call: &Call) -> bool {
    use crate::lowir::model::InstructionKind as I;
    let function = |mut value| {
        if let Value::Temp(t) = value
            && let Some(def) = body.temp(t).definition
            && let I::Addr(base) = body.instruction(def).kind
        {
            value = base;
        }
        matches!(value,Value::Symbol(s)if program.function(program.symbol(s)).is_some())
    };
    for &value in body.arguments(call.arguments) {
        if function(value) {
            return true;
        }
        let Value::Temp(t) = value else {
            continue;
        };
        let Some(def) = body.temp(t).definition else {
            continue;
        };
        let I::Addr(Value::Slot(slot)) = body.instruction(def).kind else {
            continue;
        };
        let mut writes = 0_u32;
        let mut stores = false;
        for &b in &body.layout {
            for &id in &body.block(b).instructions {
                let kind = &body.instruction(id).kind;
                let (I::Store(_, _, _, target)
                | I::CopyObject(_, _, _, target)
                | I::ZeroInit(_, _, target)) = *kind
                else {
                    continue;
                };
                let names = target == Value::Slot(slot)
                    || matches!(target,Value::Temp(t)if body.temp(t).definition.is_some_and(|d|matches!(body.instruction(d).kind,I::Addr(v)if v==Value::Slot(slot))));
                if names {
                    writes = writes.wrapping_add(1);
                    stores = matches!(*kind,I::Store(_,_,v,_)if function(v));
                }
            }
        }
        if writes == 1 && stores {
            return true;
        }
    }
    false
}
pub(super) fn partial_parameters(program: &Program, callee: SymbolId) -> bool {
    program.function(program.symbol(callee)).is_some_and(|f| {
        program
            .signature(f.signature)
            .parameters
            .iter()
            .all(|p| p.pass != Some(Pass::IndirectResult) && !matches!(p.ty, Type::Object { .. }))
    })
}

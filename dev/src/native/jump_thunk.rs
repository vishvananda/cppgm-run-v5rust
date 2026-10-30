//! Typed adjustment jump emission and explicit `LowIR` body recovery. See NOTICE.
use super::{
    image::{Image, ImageSymbol, Section},
    object_compile::ObjectModel,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{
    Error,
    model::{Arity, Body, Instruction, InstructionKind as I, JumpThunk, Program, SymbolId, Value},
};
fn defined(body: &Body, value: Value) -> Option<&Instruction> {
    let Value::Temp(temp) = value else {
        return None;
    };
    body.temp(temp).definition.map(|id| body.instruction(id))
}
fn offset(body: &Body, value: Value) -> Option<i64> {
    let Value::Integer(id) = value else {
        return None;
    };
    Some(
        u64::try_from(body.literal(id) & u128::from(u64::MAX))
            .ok()?
            .cast_signed(),
    )
}
fn recognize(body: &Body) -> Option<JumpThunk> {
    let [entry] = body.layout.as_slice() else {
        return None;
    };
    let mut call = None;
    let mut ret = None;
    for &id in &body.block(*entry).instructions {
        let inst = body.instruction(id);
        match inst.kind {
            I::Call(_) if call.is_none() => call = Some(inst),
            I::Return(_, _) => ret = Some(inst),
            I::Index(..) | I::Load(..) => {}
            _ => return None,
        }
    }
    let call = call?;
    let ret = ret?;
    let I::Call(callee) = &call.kind else {
        return None;
    };
    let Value::Symbol(target) = callee.callee else {
        return None;
    };
    let arguments = body.arguments(callee.arguments);
    if arguments.len() != body.parameter_count {
        return None;
    }
    let I::Return(_, result) = ret.kind else {
        return None;
    };
    if result.is_some() && result != call.result.map(Value::Temp) {
        return None;
    }
    let mut receiver = None;
    let mut adjustment = 0;
    let mut vcall = 0;
    for (index, &argument) in arguments.iter().enumerate() {
        let parameter = Value::Temp(body.temp_id(index)?);
        if argument == parameter {
            continue;
        }
        if receiver.is_some() || index > 1 {
            return None;
        }
        receiver = Some(u32::try_from(index).ok()?);
        let outer = defined(body, argument)?;
        let I::Index(_, _, base, amount) = outer.kind else {
            return None;
        };
        let mut fixed = Some(outer);
        if !matches!(amount, Value::Integer(_)) {
            let displacement = defined(body, amount)?;
            let I::Load(_, _, address) = displacement.kind else {
                return None;
            };
            let row = defined(body, address)?;
            let I::Index(_, _, table, slot) = row.kind else {
                return None;
            };
            let table = defined(body, table)?;
            let I::Load(_, _, object) = table.kind else {
                return None;
            };
            if object != base {
                return None;
            }
            vcall = offset(body, slot)?;
            if vcall == 0 {
                return None;
            }
            fixed = defined(body, base);
            if base == parameter {
                continue;
            }
        }
        let I::Index(_, _, base, amount) = fixed?.kind else {
            return None;
        };
        if base != parameter {
            return None;
        }
        adjustment = offset(body, amount)?;
    }
    Some(JumpThunk {
        target,
        this_index: receiver?,
        adjustment,
        vcall_offset: vcall,
    })
}
/// Explicit `LowIR` (and optimized bodies) has no authoritative lowering record.
/// Match the frozen adapter only when the program has no published thunk facts.
pub fn recover(program: &mut Program, model: ObjectModel) {
    if program.has_jump_thunks() {
        return;
    }
    for index in 0..program.publications().len() {
        let symbol = program.publications()[index];
        let Some(function) = program.function(program.symbol(symbol)) else {
            continue;
        };
        if model != ObjectModel::Host
            && program.signature(function.signature).boundary.arity != Arity::Variadic
        {
            continue;
        }
        let Some(object) = function.metadata.object else {
            continue;
        };
        let object = program.names.bytes(object);
        if !object.starts_with(b"_ZTh") && !object.starts_with(b"_ZTv") {
            continue;
        }
        if let Some(thunk) = function.body.as_ref().and_then(recognize) {
            program.set_jump_thunk(symbol, thunk);
        }
    }
}
pub(super) fn emit(symbol: SymbolId, thunk: JumpThunk, image: &mut Image) -> Result<(), Error> {
    let receiver = if thunk.this_index == 0 {
        R::RDI
    } else {
        R::RSI
    };
    let base = image.align_text()?;
    image.define(ImageSymbol::Program(symbol), Section::Text, base);
    let mut a = Assembler::default();
    if thunk.adjustment != 0 {
        a.alu_ri(0, Rm::Reg(receiver), thunk.adjustment, 8)?;
    }
    if thunk.vcall_offset != 0 {
        a.load(R::R10, Rm::mem(receiver, 0), 8, false)?;
        let displacement = i32::from_le_bytes(
            thunk.vcall_offset.to_le_bytes()[..4]
                .try_into()
                .map_err(|_| Error::Limit)?,
        );
        a.alu_rr(0, receiver, Rm::mem(R::R10, displacement), 8)?;
    }
    a.jump_symbol(ImageSymbol::Program(thunk.target))?;
    a.resolve()?;
    image.append(a)?;
    Ok(())
}

//! Recover typed variadic adjustment thunks before course object emission. See NOTICE.
use super::{
    image::{Image, ImageSymbol, Section},
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{
    Error,
    model::{Arity, Body, Function, Instruction, InstructionKind as I, Program, SymbolId, Value},
};
struct Thunk {
    target: SymbolId,
    receiver: R,
    adjustment: i64,
    vcall: i64,
}
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
fn recognize(body: &Body) -> Option<Thunk> {
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
        receiver = Some(if index == 0 { R::RDI } else { R::RSI });
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
    Some(Thunk {
        target,
        receiver: receiver?,
        adjustment,
        vcall,
    })
}
pub(super) fn emit(
    program: &Program,
    symbol: SymbolId,
    function: &Function,
    image: &mut Image,
) -> Result<bool, Error> {
    if program.signature(function.signature).boundary.arity != Arity::Variadic {
        return Ok(false);
    }
    let Some(object) = function.metadata.object else {
        return Ok(false);
    };
    let object = program.names.bytes(object);
    if !object.starts_with(b"_ZTh") && !object.starts_with(b"_ZTv") {
        return Ok(false);
    }
    let Some(body) = &function.body else {
        return Ok(false);
    };
    let Some(thunk) = recognize(body) else {
        return Ok(false);
    };
    let base = image.align_text()?;
    image.define(ImageSymbol::Program(symbol), Section::Text, base);
    let mut a = Assembler::default();
    if thunk.adjustment != 0 {
        a.alu_ri(0, Rm::Reg(thunk.receiver), thunk.adjustment, 8)?;
    }
    if thunk.vcall != 0 {
        a.load(R::R10, Rm::mem(thunk.receiver, 0), 8, false)?;
        let displacement = i32::from_le_bytes(
            thunk.vcall.to_le_bytes()[..4]
                .try_into()
                .map_err(|_| Error::Limit)?,
        );
        a.alu_rr(0, thunk.receiver, Rm::mem(R::R10, displacement), 8)?;
    }
    a.jump_symbol(ImageSymbol::Program(thunk.target))?;
    a.resolve()?;
    image.append(a)?;
    Ok(true)
}

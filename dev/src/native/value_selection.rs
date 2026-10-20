//! Typed constant aliases and terminal-value selection for one function. See NOTICE.
use super::{
    abi,
    mir::{Function as MirFunction, Instruction, Op},
    slot_analysis::{self, Slot},
    x86::Register as R,
};
use crate::{
    lowir::{
        Error,
        literals::{self, Constant, FloatKind},
        model::{
            Arity, Body, DebugLocation, Function, InstructionKind as I, Program, SymbolId, Value,
        },
        vocabulary::Type,
    },
    types::floating::FloatingScanner,
};
#[derive(Clone, Copy)]
enum Alias {
    Constant(Constant),
    Parameter(usize),
    Symbol(SymbolId),
}

pub(super) fn unsupported() -> Error {
    Error::NotImplemented("native instruction selection and placement")
}
pub(super) fn alias_compatible(to: Type, from: Type) -> bool {
    if to == from {
        return true;
    }
    if !matches!(to, Type::Ptr) && to.bits() == 0
        || !matches!(from, Type::Ptr) && from.bits() == 0
        || matches!(to, Type::I128)
        || matches!(from, Type::I128)
    {
        return false;
    }
    let tb = if to == Type::Ptr { 64 } else { to.bits() };
    let fb = if from == Type::Ptr { 64 } else { from.bits() };
    from == Type::I1
        || tb == 64
        || tb > fb
            && (!matches!(from, Type::I8 | Type::I16 | Type::I32)
                || matches!(to, Type::I8 | Type::I16 | Type::I32))
}
fn normalize(bits: u128, ty: Type) -> u128 {
    if ty == Type::I128 {
        return bits;
    }
    let width = if ty == Type::Ptr { 64 } else { ty.bits() };
    if width == 0 {
        return bits;
    }
    let mask = (1_u128 << width) - 1;
    let low = bits & mask;
    if matches!(ty, Type::I8 | Type::I16 | Type::I32 | Type::I64) && low & (1 << (width - 1)) != 0 {
        low | !mask
    } else {
        low
    }
}
fn resolve(body: &Body, aliases: &[Option<Alias>], value: Value) -> Result<Alias, Error> {
    match value {
        Value::Temp(temp) => aliases[temp.index()].ok_or_else(unsupported),
        Value::Integer(id) => Ok(Alias::Constant(Constant::Integer(body.literal(id)))),
        Value::Float(id, kind) => Ok(Alias::Constant(Constant::Float(kind, body.literal(id)))),
        Value::Symbol(id) => Ok(Alias::Symbol(id)),
        Value::Slot(_) => Err(unsupported()),
    }
}
fn copy_alias(
    body: &Body,
    aliases: &[Option<Alias>],
    ty: Type,
    value: Value,
) -> Result<Alias, Error> {
    let source = resolve(body, aliases, value)?;
    match source {
        Alias::Constant(Constant::Integer(bits))
            if (ty.bits() != 0 || ty == Type::Ptr) && ty != Type::I128 =>
        {
            Ok(Alias::Constant(Constant::Integer(normalize(bits, ty))))
        }
        Alias::Constant(Constant::Float(..)) if ty.float_rank() != 0 => Ok(source),
        Alias::Parameter(_) if matches!(value,Value::Temp(temp) if body.temp(temp).ty.is_some_and(|from|alias_compatible(ty,from))) => {
            Ok(source)
        }
        _ => Err(unsupported()),
    }
}
pub(super) fn select(
    program: &Program,
    symbol: SymbolId,
    function: &Function,
    level: u8,
) -> Result<MirFunction, Error> {
    let body = function.body.as_ref().ok_or(Error::Limit)?;
    if level != 0 || body.layout.len() != 1 {
        return Err(unsupported());
    }
    let signature = program.signature(function.signature);
    if signature.boundary.arity == Arity::Variadic
        || matches!(signature.result, Type::Object { .. })
    {
        return Err(unsupported());
    }
    let layout = abi::Layout::classify(signature)?;
    if layout.indirect_result {
        return Err(unsupported());
    }
    let slots = slot_analysis::analyze(body)?;
    if slots.iter().any(|s| matches!(s, Slot::Memory)) {
        return Err(unsupported());
    }
    let mut mir = MirFunction::new(
        symbol,
        function.signature,
        body.block(body.layout[0]).name,
        layout,
    );
    mir.lowir_instructions = body.instructions().len();
    mir.dead_slots = slots.iter().filter(|s| matches!(s, Slot::Dead)).count();
    mir.promoted_slots = slots
        .iter()
        .filter(|s| matches!(s, Slot::Promoted(_)))
        .count();
    let mut aliases = vec![None; body.temps().len()];
    for (index, alias) in aliases.iter_mut().take(body.parameter_count).enumerate() {
        *alias = Some(Alias::Parameter(index));
    }
    let mut floating = signature.result.float_rank() != 0
        || signature.parameters.iter().any(|p| p.ty.float_rank() != 0);
    let mut scanner = FloatingScanner::default();
    for &id in &body.block(body.layout[0]).instructions {
        let inst = body.instruction(id);
        let location = body.location(inst);
        body.visit_values(&inst.kind, |value| {
            floating |= matches!(value, Value::Float(..));
        });
        let alias = match inst.kind {
            I::Const(ty, value) => {
                floating |= ty.float_rank() != 0;
                let value = resolve(body, &aliases, value)?;
                Some(match value {
                    Alias::Constant(Constant::Integer(bits))
                        if ty.bits() != 0 && ty != Type::I128 =>
                    {
                        Alias::Constant(Constant::Integer(normalize(bits, ty)))
                    }
                    _ => value,
                })
            }
            I::Copy(ty, value) => {
                floating |= ty.float_rank() != 0;
                Some(copy_alias(body, &aliases, ty, value)?)
            }
            I::Store(_, ty, _, Value::Slot(slot))
                if matches!(slots[slot.index()], Slot::Dead | Slot::Promoted(_)) =>
            {
                floating |= ty.float_rank() != 0;
                None
            }
            I::Load(_, ty, Value::Slot(slot)) => {
                floating |= ty.float_rank() != 0;
                let Slot::Promoted(value) = slots[slot.index()] else {
                    return Err(unsupported());
                };
                let value = resolve(body, &aliases, value)?;
                Some(match value {
                    Alias::Constant(Constant::Integer(bits))
                        if ty.bits() != 0 && ty != Type::I128 =>
                    {
                        Alias::Constant(Constant::Integer(normalize(bits, ty)))
                    }
                    _ => value,
                })
            }
            I::Return(ty, value) => {
                floating |= ty.float_rank() != 0;
                emit_return(&mut mir, body, &aliases, ty, value, location, &mut scanner)?;
                None
            }
            I::Unreachable => {
                mir.instructions.push(Instruction {
                    op: Op::Trap,
                    location,
                });
                None
            }
            _ => return Err(unsupported()),
        };
        if let Some(result) = inst.result {
            aliases[result.index()] = alias;
        }
    }
    mir.scratch_bytes = if floating { 48 } else { 0 };
    Ok(mir)
}
fn emit_return(
    mir: &mut MirFunction,
    body: &Body,
    aliases: &[Option<Alias>],
    ty: Type,
    value: Option<Value>,
    location: Option<DebugLocation>,
    scanner: &mut FloatingScanner,
) -> Result<(), Error> {
    let mut emit = |op| mir.instructions.push(Instruction { op, location });
    let Some(value) = value else {
        emit(Op::Return(None));
        return Ok(());
    };
    let source = resolve(body, aliases, value)?;
    match source {
        Alias::Constant(value) => {
            if let Some(kind) = FloatKind::from_type(ty) {
                let bits = literals::native_float(scanner, value, kind)?;
                if kind == FloatKind::F80 {
                    emit(Op::FloatReturn(bits));
                } else {
                    let reg = R::xmm(0)?;
                    emit(Op::FloatMove(reg, kind, bits));
                    emit(Op::Return(Some(reg)));
                }
            } else {
                let bits = match value {
                    Constant::Integer(bits) => normalize(bits, ty),
                    Constant::Float(_, bits) => bits,
                };
                let bytes = bits.to_le_bytes();
                let mut word = [0; 8];
                word.copy_from_slice(&bytes[..8]);
                if ty == Type::I128 {
                    emit(Op::Move(R::RAX, u128::from(u64::from_le_bytes(word))));
                    word.copy_from_slice(&bytes[8..]);
                    emit(Op::Move(R::RDX, u128::from(u64::from_le_bytes(word))));
                    emit(Op::Return(None));
                } else {
                    let immediate = if matches!(value, Constant::Float(..)) {
                        bits
                    } else {
                        i128::from(i64::from_le_bytes(word)).cast_unsigned()
                    };
                    emit(Op::Move(R::RAX, immediate));
                    emit(Op::Return(Some(R::RAX)));
                }
            }
        }
        Alias::Parameter(index) => {
            let own = body
                .temps()
                .get(index)
                .and_then(|t| t.ty)
                .ok_or(Error::Limit)?;
            if own.float_rank() != 0 || matches!(own, Type::Object { .. } | Type::I128) {
                return Err(unsupported());
            }
            let argument = mir.layout.arguments.get(index).ok_or(Error::Limit)?;
            let [part] = mir.layout.parts(argument) else {
                return Err(unsupported());
            };
            let reg = part.register.ok_or_else(unsupported)?;
            if matches!(ty, Type::I128 | Type::F80) {
                return Err(unsupported());
            }
            // The course ABI keeps a scalar parameter in its incoming register.
            mir.instructions.push(Instruction {
                op: Op::Return(Some(reg)),
                location,
            });
        }
        Alias::Symbol(symbol) => {
            if ty == Type::I128 || ty.float_rank() != 0 {
                return Err(unsupported());
            }
            emit(Op::SymbolMove(R::RAX, symbol));
            emit(Op::Return(Some(R::RAX)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::builder::FunctionBuilder;
    #[test]
    fn wide_return_words_keep_their_zero_extended_immediate_facts() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "wide", &[], Type::I128)?;
        builder.block("entry")?;
        let literal = builder.integer(u128::MAX)?;
        let value = builder.value(I::Const(Type::I128, literal), "value")?;
        builder.effect(I::Return(Type::I128, Some(value)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let selected = select(&program, symbol, function, 0)?;
        let mut output = Vec::new();
        selected.write(&program, &mut output)?;
        let output = std::str::from_utf8(&output).map_err(|_| Error::Limit)?;
        assert!(output.contains("mov rax, 0x0000000000000000ffffffffffffffff\n"));
        assert!(output.contains("mov rdx, 0x0000000000000000ffffffffffffffff\n"));
        Ok(())
    }
}

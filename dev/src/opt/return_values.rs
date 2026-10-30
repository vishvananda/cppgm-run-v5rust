//! Return retyping and scalar/object merging from `opt/inliner.cpp`. See NOTICE.
use super::inline_shape::phi_type;
use crate::lowir::{
    Error,
    literals::{Constant, FloatKind},
    model::{
        BlockId, Body, Facts, InstId, Instruction, InstructionKind as I, OptimizerSlot, TempId,
        Value,
    },
    vocabulary::Type,
};
use crate::types::float_value::{BinaryFloat, Format};

pub(super) fn phi_input(body: &mut Body, ty: Type, value: Value) -> Result<Value, Error> {
    let (Some(kind), Value::Integer(id)) = (FloatKind::from_type(ty), value) else {
        return Ok(value);
    };
    let float =
        BinaryFloat::wide_integer(body.literal(id), true, Format::Extended).ok_or(Error::Limit)?;
    let format = match kind {
        FloatKind::F32 => Format::Single,
        FloatKind::F64 => Format::Double,
        FloatKind::F80 => Format::Extended,
    };
    let bits = float.stored(format).bits(format).ok_or(Error::Limit)?;
    body.constant(Constant::Float(kind, bits))
}
fn before_jump(body: &mut Body, block: BlockId, inst: Instruction) -> Result<InstId, Error> {
    let mut ids = body.block(block).instructions.clone();
    let jump = ids.pop().ok_or(Error::Limit)?;
    let id = body.append(block, inst)?;
    ids.push(id);
    ids.push(jump);
    body.replace_links(block, ids)?;
    Ok(id)
}
fn emit(body: &mut Body, block: BlockId, kind: I, ty: Type) -> Result<Value, Error> {
    let temp = body.generated_temp(ty)?;
    before_jump(body, block, Instruction::new(Some(temp), kind, 0))?;
    Ok(Value::Temp(temp))
}
fn result_copy(body: &Body, cont: BlockId, result: TempId, ty: Type) -> Option<InstId> {
    let Type::Object { bytes, .. } = ty else {
        return None;
    };
    let bytes = bytes.get();
    let id = *body.block(cont).instructions.first()?;
    let inst = body.instruction(id);
    let I::CopyObject(size, _, source, destination) = inst.kind else {
        return None;
    };
    if inst.copy_memory_facts().volatile()
        || source != Value::Temp(result)
        || destination == source
        || size != bytes
    {
        return None;
    }
    let mut uses = 0_usize;
    for inst in body.instructions() {
        body.visit_values(&inst.kind, |v| {
            uses += usize::from(v == Value::Temp(result));
        });
    }
    (uses == 1).then_some(id)
}
fn merge(
    body: &mut Body,
    result: TempId,
    ty: Type,
    returns: &[(BlockId, Value)],
    replacements: &mut Vec<Option<Value>>,
) -> Result<(), Error> {
    if replacements.len() <= result.index() {
        replacements.resize(body.temps().len(), None);
    }
    if returns.iter().all(|r| r.1 == returns[0].1) {
        replacements[result.index()] = Some(returns[0].1);
        return Ok(());
    }
    let I::Jump(cont) = body
        .instruction(
            *body
                .block(returns[0].0)
                .instructions
                .last()
                .ok_or(Error::Limit)?,
        )
        .kind
    else {
        return Err(Error::Limit);
    };
    let into = result_copy(body, cont, result, ty);
    let merge = if into.is_none() {
        Some(body.optimizer_slot(
            ty,
            if matches!(ty, Type::Object { .. }) {
                OptimizerSlot::ReturnMergeObject
            } else {
                OptimizerSlot::ReturnMerge
            },
        )?)
    } else {
        None
    };
    for &(block, value) in returns {
        if let Some(into) = into {
            let mut copy = body.instruction(into).clone();
            let I::CopyObject(_, _, source, _) = &mut copy.kind else {
                return Err(Error::Limit);
            };
            *source = value;
            before_jump(body, block, copy)?;
        } else if let Type::Object { bytes, align } = ty {
            let bytes = bytes.get();
            let address = emit(body, block, I::Addr(merge.ok_or(Error::Limit)?), Type::Ptr)?;
            before_jump(
                body,
                block,
                Instruction::new(None, I::CopyObject(bytes, align, value, address), 0),
            )?;
        } else {
            before_jump(
                body,
                block,
                Instruction::new(
                    None,
                    I::Store(Facts::default(), ty, value, merge.ok_or(Error::Limit)?),
                    0,
                ),
            )?;
        }
    }
    if matches!(ty, Type::Object { .. }) {
        if let Some(into) = into {
            let ids = body
                .block(cont)
                .instructions
                .iter()
                .copied()
                .filter(|&id| id != into)
                .collect();
            body.replace_links(cont, ids)?;
        } else {
            replacements[result.index()] = merge;
        }
    } else {
        let id = body.append(
            cont,
            Instruction::new(
                Some(result),
                I::Load(Facts::default(), ty, merge.ok_or(Error::Limit)?),
                0,
            ),
        )?;
        let mut ids = body.block(cont).instructions.clone();
        ids.pop();
        ids.insert(0, id);
        body.replace_links(cont, ids)?;
    }
    Ok(())
}
pub(super) fn finish(
    body: &mut Body,
    cont: BlockId,
    result: TempId,
    returns: &mut [(BlockId, Option<Value>)],
    replacements: &mut Vec<Option<Value>>,
) -> Result<(), Error> {
    let ty = body.temp(result).ty.ok_or(Error::Limit)?;
    for (block, value) in returns.iter_mut() {
        let mut v = value.ok_or(Error::Limit)?;
        if phi_type(ty) && matches!(v, Value::Slot(_) | Value::Symbol(_)) {
            let (kind, ty) = if matches!(v, Value::Slot(_)) {
                (I::Load(Facts::default(), ty, v), ty)
            } else {
                (I::Addr(v), Type::Ptr)
            };
            v = emit(body, *block, kind, ty)?;
        }
        if matches!(v,Value::Temp(t)if body.temp(t).ty!=Some(ty)) {
            v = emit(body, *block, I::Copy(ty, v), ty)?;
        }
        *value = Some(v);
    }
    if phi_type(ty) {
        let kind = if returns.is_empty() {
            let zero = if let Some(kind) = FloatKind::from_type(ty) {
                body.constant(Constant::Float(kind, 0))?
            } else {
                body.integer(0)?
            };
            I::Copy(ty, zero)
        } else {
            let mut pairs = Vec::with_capacity(returns.len());
            for &(block, value) in returns.iter() {
                pairs.push((block, phi_input(body, ty, value.ok_or(Error::Limit)?)?));
            }
            I::Phi(ty, pairs)
        };
        let id = body.append(cont, Instruction::new(Some(result), kind, 0))?;
        let mut ids = body.block(cont).instructions.clone();
        ids.pop();
        ids.insert(0, id);
        body.replace_links(cont, ids)?;
    } else if !returns.is_empty() {
        let values = returns
            .iter()
            .map(|&(b, v)| v.map(|v| (b, v)).ok_or(Error::Limit))
            .collect::<Result<Vec<_>, _>>()?;
        merge(body, result, ty, &values, replacements)?;
    }
    Ok(())
}

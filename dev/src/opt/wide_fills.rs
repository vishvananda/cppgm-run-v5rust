//! Sixteen-byte fill chunks and scalar tails from `opt/loop_passes.cpp`. See NOTICE.
use super::{
    alias_facts::{integral, signed_word},
    induction::{Exit, Induction},
    loop_memory::Emitter,
    loop_motion::invariant,
    memory_contents::bytes,
    natural_loops::{Forest, LoopId},
    numeric_fold::normalize,
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{Facts as Flags, InstId, InstructionKind as I, LinkEditor, OptimizerBlock, Value},
    vocabulary::{Binary, Compare, Convert, Type},
};
fn pattern(
    context: &Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    test: Exit,
    iv: Induction,
) -> Result<Option<InstId>, Error> {
    let l = forest.get(loop_);
    if context.level < 2
        || l.preheader.is_none()
        || test.exiting != l.header
        || !iv.pointer
        || test.compared != Value::Temp(iv.value)
        || !matches!(iv.step, 2 | 4 | 8)
    {
        return Ok(None);
    }
    let phis = context
        .body
        .block(l.header)
        .instructions
        .iter()
        .take_while(|&&id| matches!(context.body.instruction(id).kind, I::Phi(..)))
        .count();
    if phis != 1 {
        return Ok(None);
    }
    if let Some(Value::Temp(t)) = iv.init
        && let Some(def) = context.body.temp(t).definition
        && let I::Index(_, _, _, Value::Temp(count)) = context.body.instruction(def).kind
        && let Some(def) = context.body.temp(count).definition
        && let I::Binary(Binary::And, _, _, _, Value::Integer(k)) =
            context.body.instruction(def).kind
        && signed_word(context.body.literal(k)) == -16
    {
        return Ok(None);
    }
    let mut store = None;
    for &b in &l.blocks {
        for &id in &context.body.block(b).instructions {
            match context.body.instruction(id).kind {
                I::Store(flags, ..) => {
                    if flags.volatile() || store.replace(id).is_some() {
                        return Ok(None);
                    }
                }
                I::Jump(_) | I::Branch(..) => (),
                ref kind if super::pure(kind) => (),
                _ => return Ok(None),
            }
        }
    }
    let Some(store) = store else { return Ok(None) };
    let at = context.block_map()[store.index()].ok_or(Error::Limit)?;
    if at == l.header || !context.cfg()?.dominates(at, l.latches[0]) {
        return Ok(None);
    }
    let I::Store(_, ty, value, mut address) = context.body.instruction(store).kind else {
        return Err(Error::Limit);
    };
    if !integral(ty) || u64::try_from(iv.step).ok() != Some(bytes(ty)) {
        return Ok(None);
    }
    for _ in 0..4 {
        let Value::Temp(t) = address else { break };
        if t == iv.value {
            break;
        }
        let Some(def) = context.body.temp(t).definition else {
            return Ok(None);
        };
        let I::Copy(_, v) = context.body.instruction(def).kind else {
            return Ok(None);
        };
        address = v;
    }
    Ok((address == Value::Temp(iv.value)
        && (matches!(value, Value::Integer(_))
            || matches!(value, Value::Temp(_)) && invariant(context, forest, loop_, value)))
    .then_some(store))
}
pub(super) fn widen(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    test: Exit,
    ivs: &[Induction],
) -> Result<bool, Error> {
    let mut found = None;
    for &iv in ivs {
        if let Some(store) = pattern(context, forest, loop_, test, iv)? {
            found = Some((iv, store));
            break;
        }
    }
    let Some((iv, store)) = found else {
        return Ok(false);
    };
    let l = forest.get(loop_);
    let pre = l.preheader.ok_or(Error::Limit)?;
    let init = iv.init.ok_or(Error::Limit)?;
    let I::Store(_, ty, value, _) = context.body.instruction(store).kind else {
        return Err(Error::Limit);
    };
    let wide = context.body.optimizer_block(OptimizerBlock::WideLoop)?;
    let tail = context.body.optimizer_block(OptimizerBlock::WideTail)?;
    let links = LinkEditor::new(context.body)?;
    let last = *context
        .body
        .block(pre)
        .instructions
        .last()
        .ok_or(Error::Limit)?;
    let mut at = links.previous(last);
    let mut e = Emitter {
        body: context.body,
        links,
        location: Some(store),
    };
    let begin = e.value(pre, &mut at, I::Copy(Type::I64, init), Type::I64)?;
    let end = e.value(pre, &mut at, I::Copy(Type::I64, test.bound), Type::I64)?;
    let size = e.value(
        pre,
        &mut at,
        I::Binary(Binary::Sub, Flags::default(), Type::I64, end, begin),
        Type::I64,
    )?;
    let mask = e.literal(-16)?;
    let chunks = e.value(
        pre,
        &mut at,
        I::Binary(Binary::And, Flags::default(), Type::I64, size, mask),
        Type::I64,
    )?;
    let end16 = e.value(
        pre,
        &mut at,
        I::Index(Type::I8, None, init, chunks),
        Type::Ptr,
    )?;
    let replicator = match iv.step {
        8 => 1,
        4 => 0x0000_0001_0000_0001,
        _ => 0x0001_0001_0001_0001,
    };
    let v64 = if let Value::Integer(k) = value {
        let bits = normalize(ty, e.body.literal(k));
        let bits = u64::from_le_bytes(signed_word(bits).to_le_bytes());
        let mask = if iv.step == 8 {
            u64::MAX
        } else {
            (1_u64 << (8 * iv.step)) - 1
        };
        e.literal(i64::from_le_bytes(
            ((bits & mask).wrapping_mul(replicator)).to_le_bytes(),
        ))?
    } else if iv.step == 8 {
        if ty == Type::Ptr {
            e.value(pre, &mut at, I::Copy(Type::I64, value), Type::I64)?
        } else {
            value
        }
    } else {
        let v = e.value(
            pre,
            &mut at,
            I::Convert(Convert::ZeroExtend, Type::I64, ty, value),
            Type::I64,
        )?;
        let replicate = e.literal(i64::from_le_bytes(replicator.to_le_bytes()))?;
        e.value(
            pre,
            &mut at,
            I::Binary(Binary::Mul, Flags::default(), Type::I64, v, replicate),
            Type::I64,
        )?
    };
    let enter = e.value(
        pre,
        &mut at,
        I::Compare(Compare::Ne, Type::Ptr, init, end16),
        Type::I64,
    )?;
    let mut entry = e.body.instruction(last).clone();
    entry.kind = I::Branch(enter, wide, tail);
    e.body.update_instruction(last, entry);
    let mut at_wide = None;
    // The original empty chunk phi has no location; emitted instructions inherit the store.
    let q = e.body.generated_temp(Type::Ptr)?;
    let phi = e.body.append(
        wide,
        crate::lowir::model::Instruction::new(Some(q), I::Phi(Type::Ptr, Vec::new()), 0),
    )?;
    e.links.insert_after(wide, None, phi)?;
    at_wide.replace(phi);
    e.emit(
        wide,
        &mut at_wide,
        I::Store(Flags::default(), Type::I64, v64, Value::Temp(q)),
        Type::Void,
    )?;
    let eight = e.literal(8)?;
    let second = e.value(
        wide,
        &mut at_wide,
        I::Index(Type::I8, None, Value::Temp(q), eight),
        Type::Ptr,
    )?;
    e.emit(
        wide,
        &mut at_wide,
        I::Store(Flags::default(), Type::I64, v64, second),
        Type::Void,
    )?;
    let sixteen = e.literal(16)?;
    let q2 = e.value(
        wide,
        &mut at_wide,
        I::Index(Type::I8, None, Value::Temp(q), sixteen),
        Type::Ptr,
    )?;
    let again = e.value(
        wide,
        &mut at_wide,
        I::Compare(Compare::Ne, Type::Ptr, q2, end16),
        Type::I64,
    )?;
    e.emit(wide, &mut at_wide, I::Branch(again, wide, tail), Type::Void)?;
    let mut instruction = e.body.instruction(phi).clone();
    instruction.kind = I::Phi(Type::Ptr, vec![(pre, init), (wide, q2)]);
    e.body.update_instruction(phi, instruction);
    let mut at_tail = None;
    e.emit(tail, &mut at_tail, I::Jump(l.header), Type::Void)?;
    e.body.rename_phi_predecessor(l.header, pre, tail);
    let id = e.body.temp(iv.value).definition.ok_or(Error::Limit)?;
    let mut instruction = e.body.instruction(id).clone();
    let I::Phi(_, pairs) = &mut instruction.kind else {
        return Err(Error::Limit);
    };
    for (b, v) in pairs {
        if *b == tail {
            *v = end16;
        }
    }
    e.body.update_instruction(id, instruction);
    let at = e
        .body
        .layout
        .iter()
        .position(|&b| b == pre)
        .ok_or(Error::Limit)?;
    let insert = at + 1;
    e.body.layout.splice(insert..insert, [wide, tail]);
    e.links.commit(e.body)?;
    context.prepare_block_map(super::pass_context::Pass::RemoveLoops, false);
    context.loops_removed = context.loops_removed.checked_add(1).ok_or(Error::Limit)?;
    Ok(true)
}

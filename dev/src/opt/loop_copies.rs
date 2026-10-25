//! Overlap-guarded pointer copy loops from `opt/loop_passes.cpp`. See NOTICE.
use super::{
    induction::{Exit, Facts, Induction},
    loop_memory::Emitter,
    memory_contents::bytes,
    natural_loops::{Forest, LoopId},
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{
        Boundary, Call, Facts as Flags, Function, InstructionKind as I, LinkEditor, Metadata,
        OptimizerBlock, Parameter, Signature, SymbolDetail, SymbolId, Unwind, Value,
    },
    vocabulary::{Binary, Binding, Compare, Type},
};
fn pattern(
    context: &Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    test: Exit,
    ivs: &[Induction],
) -> Result<Option<(Induction, Induction)>, Error> {
    let l = forest.get(loop_);
    let mut store = None;
    let mut load = None;
    for &b in &l.blocks {
        for &id in &context.body.block(b).instructions {
            match context.body.instruction(id).kind {
                I::Store(flags, ..) => {
                    if flags.volatile() || store.replace(id).is_some() {
                        return Ok(None);
                    }
                }
                I::Load(flags, ..) => {
                    if flags.volatile() || load.replace(id).is_some() {
                        return Ok(None);
                    }
                }
                I::Jump(_) | I::Branch(..) => (),
                ref kind if super::pure(kind) => (),
                _ => return Ok(None),
            }
        }
    }
    let (Some(store), Some(load)) = (store, load) else {
        return Ok(None);
    };
    let I::Store(_, ty, value, dst) = context.body.instruction(store).kind else {
        return Err(Error::Limit);
    };
    let I::Load(_, ld_ty, src) = context.body.instruction(load).kind else {
        return Err(Error::Limit);
    };
    let result = context.body.instruction(load).result.ok_or(Error::Limit)?;
    if value != Value::Temp(result)
        || ty != ld_ty
        || !(ty.bits() != 0 || ty == Type::Ptr)
        || bytes(ty) == 0
    {
        return Ok(None);
    }
    for id in [load, store] {
        let b = context.block_map()[id.index()].ok_or(Error::Limit)?;
        if b == l.header || !context.cfg()?.dominates(b, l.latches[0]) {
            return Ok(None);
        }
    }
    let mut reads = 0;
    for &b in &l.blocks {
        for &id in &context.body.block(b).instructions {
            context
                .body
                .visit_values(&context.body.instruction(id).kind, |v| {
                    reads += u32::from(v == value);
                });
        }
    }
    if reads != 1 {
        return Ok(None);
    }
    let induction_of = |mut address| {
        for _ in 0..4 {
            let Value::Temp(t) = address else { break };
            if let Some(iv) = ivs.iter().copied().find(|iv| iv.value == t) {
                return Some(iv);
            }
            let def = context.body.temp(t).definition?;
            let I::Copy(_, v) = context.body.instruction(def).kind else {
                return None;
            };
            address = v;
        }
        None
    };
    let (Some(src), Some(dst)) = (induction_of(src), induction_of(dst)) else {
        return Ok(None);
    };
    Ok((src.value != dst.value
        && src.pointer
        && dst.pointer
        && u64::try_from(src.step).ok() == Some(bytes(ty))
        && src.step == dst.step
        && test.compared == Value::Temp(src.value))
    .then_some((src, dst)))
}
fn symbol(context: &mut Context<'_>) -> Result<Option<SymbolId>, Error> {
    let name = context.program.names.intern("__cppgm_builtin_memmove")?;
    if let Some(s) = context.program.find_symbol(name)
        && !matches!(
            context.program.symbol(s).detail,
            SymbolDetail::Unresolved | SymbolDetail::Function(_)
        )
    {
        return Ok(None);
    }
    let s = context.program.reference_from(name, context.symbol)?;
    if context.program.symbol(s).declared() {
        return Ok(Some(s));
    }
    let empty = context.program.names.intern("")?;
    let signature = context.program.add_signature(Signature {
        parameters: [Type::Ptr, Type::Ptr, Type::I64]
            .into_iter()
            .map(|ty| Parameter {
                name: empty,
                ty,
                pass: None,
                no_alias: false,
                object_bytes: 0,
                classes: None,
            })
            .collect(),
        result: Type::Ptr,
        boundary: Boundary {
            unwind: Unwind::No,
            ..Boundary::default()
        },
    })?;
    let object = context.program.names.intern("cppgm_builtin_memmove")?;
    context.program.add(
        s,
        Function {
            name,
            signature,
            metadata: Metadata {
                binding: Some(Binding::Strong),
                object: Some(object),
                ..Metadata::default()
            },
            body: None,
            location: None,
        },
    )?;
    Ok(Some(s))
}
pub(super) fn emit(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    test: Exit,
    ivs: &[Induction],
    private: bool,
) -> Result<bool, Error> {
    let l = forest.get(loop_);
    if context.level < 2 || test.exiting != l.header {
        return Ok(false);
    }
    let Some(pre) = l.preheader else {
        return Ok(false);
    };
    let Some((src, dst)) = pattern(context, forest, loop_, test, ivs)? else {
        return Ok(false);
    };
    let facts = Facts {
        body: context.body,
        cfg: context.cfg()?,
        forest,
        block_of: context.block_map(),
    };
    if !private {
        let mut ok = true;
        for &b in &context.body.layout {
            if !forest.contains(loop_, b) {
                for &id in &context.body.block(b).instructions {
                    context
                        .body
                        .visit_values(&context.body.instruction(id).kind, |v| {
                            ok &= !matches!(v, Value::Temp(_))
                                || facts.invariant(loop_, v)
                                || v == Value::Temp(src.value)
                                || v == Value::Temp(dst.value);
                        });
                }
            }
        }
        if !ok {
            return Ok(false);
        }
    }
    let Some(symbol) = symbol(context)? else {
        return Ok(false);
    };
    let copy = context.body.optimizer_block(OptimizerBlock::LoopCopy)?;
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
        location: None,
    };
    let src_init = src.init.ok_or(Error::Limit)?;
    let dst_init = dst.init.ok_or(Error::Limit)?;
    let end = e.value(pre, &mut at, I::Copy(Type::I64, test.bound), Type::I64)?;
    let begin = e.value(pre, &mut at, I::Copy(Type::I64, src_init), Type::I64)?;
    let size = e.value(
        pre,
        &mut at,
        I::Binary(Binary::Sub, Flags::default(), Type::I64, end, begin),
        Type::I64,
    )?;
    let dst_end = e.value(
        pre,
        &mut at,
        I::Index(Type::I8, None, dst_init, size),
        Type::Ptr,
    )?;
    let end = e.value(pre, &mut at, I::Copy(Type::I64, dst_init), Type::I64)?;
    let begin = e.value(pre, &mut at, I::Copy(Type::I64, src_init), Type::I64)?;
    let distance = e.value(
        pre,
        &mut at,
        I::Binary(Binary::Sub, Flags::default(), Type::I64, end, begin),
        Type::I64,
    )?;
    let whole = e.value(
        pre,
        &mut at,
        I::Compare(Compare::Uge, Type::I64, distance, size),
        Type::I64,
    )?;
    let mut at_copy = None;
    let arguments = e.body.add_arguments(&[dst_init, src_init, size])?;
    e.emit(
        copy,
        &mut at_copy,
        I::Call(Call {
            result: Type::Ptr,
            callee: Value::Symbol(symbol),
            arguments,
            copy_elision: false,
            argument_classes: Vec::new(),
            signature: None,
        }),
        Type::Ptr,
    )?;
    e.emit(copy, &mut at_copy, I::Jump(test.target), Type::Void)?;
    let mut term = e.body.instruction(last).clone();
    term.kind = I::Branch(whole, copy, l.header);
    e.body.update_instruction(last, term);
    let at = e
        .body
        .layout
        .iter()
        .position(|&b| b == pre)
        .ok_or(Error::Limit)?;
    e.body.layout.insert(at + 1, copy);
    e.links.commit(e.body)?;
    let leaving = |v| {
        if v == Value::Temp(src.value) {
            test.bound
        } else if v == Value::Temp(dst.value) {
            dst_end
        } else {
            v
        }
    };
    for id in context.body.block(test.target).instructions.clone() {
        let I::Phi(_, pairs) = &context.body.instruction(id).kind else {
            break;
        };
        let value = pairs
            .iter()
            .find(|&&(b, _)| b == test.exiting)
            .map(|&(_, v)| v)
            .ok_or(Error::Limit)?;
        context
            .body
            .append_phi_predecessor(id, copy, leaving(value))?;
    }
    for b in context.body.layout.clone() {
        if forest.contains(loop_, b) || b == copy {
            continue;
        }
        for id in context.body.block(b).instructions.clone() {
            let mut inst = context.body.instruction(id).clone();
            if b == test.target
                && let I::Phi(_, pairs) = &mut inst.kind
            {
                for (from, v) in pairs {
                    if *from != test.exiting {
                        *v = leaving(*v);
                    }
                }
                context.body.update_instruction(id, inst);
            } else {
                context.body.rewrite_values(id, |v| Ok(leaving(v)))?;
            }
        }
    }
    context.loops_removed = context.loops_removed.checked_add(1).ok_or(Error::Limit)?;
    Ok(true)
}

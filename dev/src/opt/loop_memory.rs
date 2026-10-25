//! Countdown range canonicalization and stored-value load motion from `opt/loop_passes.cpp`.
//! See NOTICE.
use super::{
    alias_facts::literal_integer,
    induction::Facts,
    loop_motion::{EntryObjects, invariant, move_before_jump},
    memory_contents::bytes,
    natural_loops::{Forest, LoopId},
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{Body, Facts as Flags, InstId, Instruction, InstructionKind as I, LinkEditor, Value},
    vocabulary::{Binary, Compare, Pass as ParameterPass, Type},
};

pub(super) struct Emitter<'a> {
    pub(super) body: &'a mut Body,
    pub(super) links: LinkEditor,
    pub(super) location: Option<InstId>,
}
impl Emitter<'_> {
    pub(super) fn emit(
        &mut self,
        block: crate::lowir::model::BlockId,
        after: &mut Option<InstId>,
        kind: I,
        ty: Type,
    ) -> Result<Option<Value>, Error> {
        let result = if ty == Type::Void {
            None
        } else {
            Some(self.body.generated_temp(ty)?)
        };
        let id = self.body.append(block, Instruction::new(result, kind, 0))?;
        if let Some(location) = self.location {
            self.body.inherit_location(id, location);
        }
        self.links.insert_after(block, *after, id)?;
        *after = Some(id);
        Ok(result.map(Value::Temp))
    }
    pub(super) fn value(
        &mut self,
        block: crate::lowir::model::BlockId,
        after: &mut Option<InstId>,
        kind: I,
        ty: Type,
    ) -> Result<Value, Error> {
        self.emit(block, after, kind, ty)?.ok_or(Error::Limit)
    }
    pub(super) fn literal(&mut self, value: i64) -> Result<Value, Error> {
        self.body.integer(literal_integer(value))
    }
}

pub(super) fn count_to_pointer(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
) -> Result<bool, Error> {
    let l = forest.get(loop_);
    if context.level < 2 || l.latches.len() != 1 {
        return Ok(false);
    }
    let Some(pre) = l.preheader else {
        return Ok(false);
    };
    let Some(&last) = context.body.block(l.header).instructions.last() else {
        return Ok(false);
    };
    let I::Branch(Value::Temp(mut counter), mut yes, mut no) = context.body.instruction(last).kind
    else {
        return Ok(false);
    };
    if yes == no || forest.contains(loop_, yes) == forest.contains(loop_, no) {
        return Ok(false);
    }
    let mut nonzero_true = true;
    if !context.body.temp_is_parameter(counter)
        && let Some(def) = context.body.temp(counter).definition
        && let I::Compare(pred, _, mut a, mut b) = context.body.instruction(def).kind
    {
        if !matches!(a, Value::Temp(_)) {
            std::mem::swap(&mut a, &mut b);
        }
        let (Value::Temp(t), Value::Integer(k)) = (a, b) else {
            return Ok(false);
        };
        if context.body.literal(k) != 0 || !matches!(pred, Compare::Ne | Compare::Eq) {
            return Ok(false);
        }
        nonzero_true = pred == Compare::Ne;
        counter = t;
    }
    if context.body.temp(counter).ty != Some(Type::I64) {
        return Ok(false);
    }
    let facts = Facts {
        body: context.body,
        cfg: context.cfg()?,
        forest,
        block_of: context.block_map(),
    };
    let mut count = None;
    let mut pointer = None;
    for &id in &context.body.block(l.header).instructions {
        if !matches!(context.body.instruction(id).kind, I::Phi(..)) {
            break;
        }
        let Some(iv) = facts.induction(loop_, id) else {
            continue;
        };
        if !iv.pointer && iv.value == counter && iv.step == -1 {
            count = Some(iv);
        } else if iv.pointer && pointer.is_none() {
            pointer = Some(iv);
        }
    }
    let (Some(count), Some(pointer)) = (count, pointer) else {
        return Ok(false);
    };
    if nonzero_true != forest.contains(loop_, yes) {
        return Ok(false);
    }
    context.body.recount_uses()?;
    context.invalidate_uses();
    if context.body.temp(count.value).uses != 2 || context.body.temp(count.next).uses != 1 {
        return Ok(false);
    }
    let links = LinkEditor::new(context.body)?;
    let mut at = links.previous(
        *context
            .body
            .block(pre)
            .instructions
            .last()
            .ok_or(Error::Limit)?,
    );
    let mut before = links.previous(last);
    let mut e = Emitter {
        body: context.body,
        links,
        location: Some(last),
    };
    let step = e.literal(pointer.step)?;
    let size = e.value(
        pre,
        &mut at,
        I::Binary(
            Binary::Mul,
            Flags::default(),
            Type::I64,
            count.init.ok_or(Error::Limit)?,
            step,
        ),
        Type::I64,
    )?;
    let bound = e.value(
        pre,
        &mut at,
        I::Index(Type::I8, None, pointer.init.ok_or(Error::Limit)?, size),
        Type::Ptr,
    )?;
    let test = e.value(
        l.header,
        &mut before,
        I::Compare(Compare::Ne, Type::Ptr, Value::Temp(pointer.value), bound),
        Type::I64,
    )?;
    if !nonzero_true {
        std::mem::swap(&mut yes, &mut no);
    }
    let mut term = e.body.instruction(last).clone();
    term.kind = I::Branch(test, yes, no);
    e.body.update_instruction(last, term);
    e.links.commit(e.body)?;
    context.prepare_block_map(super::pass_context::Pass::RemoveLoops, false);
    context.loops_removed = context.loops_removed.checked_add(1).ok_or(Error::Limit)?;
    Ok(true)
}
pub(super) fn hoist_stored(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
) -> Result<bool, Error> {
    let l = forest.get(loop_);
    if context.level < 2 || l.latches.len() != 1 {
        return Ok(false);
    }
    let Some(pre) = l.preheader else {
        return Ok(false);
    };
    let mut load = None;
    let mut store = None;
    for &b in &l.blocks {
        for &id in &context.body.block(b).instructions {
            match &context.body.instruction(id).kind {
                I::Load(..) => {
                    if load.replace(id).is_some() {
                        return Ok(false);
                    }
                }
                I::Store(..) => {
                    if store.replace(id).is_some() {
                        return Ok(false);
                    }
                }
                I::Jump(_) | I::Branch(..) => (),
                kind if super::pure(kind) => (),
                _ => return Ok(false),
            }
        }
    }
    let (Some(load), Some(store)) = (load, store) else {
        return Ok(false);
    };
    let ld = context.body.instruction(load);
    let I::Load(flags, ty, address) = ld.kind else {
        return Err(Error::Limit);
    };
    let I::Store(st_flags, st_ty, value, _) = context.body.instruction(store).kind else {
        return Err(Error::Limit);
    };
    if flags.volatile()
        || st_flags.volatile()
        || Some(value) != ld.result.map(Value::Temp)
        || ty != st_ty
        || !super::alias_facts::integral(ty)
        || !invariant(context, forest, loop_, address)
    {
        return Ok(false);
    }
    let lb = context.block_map()[load.index()].ok_or(Error::Limit)?;
    let sb = context.block_map()[store.index()].ok_or(Error::Limit)?;
    if lb == sb {
        if !context
            .body
            .block(lb)
            .instructions
            .iter()
            .take_while(|&&id| id != store)
            .any(|&id| id == load)
        {
            return Ok(false);
        }
    } else if !context.cfg()?.dominates(lb, sb) {
        return Ok(false);
    }
    let mut a = address;
    let mut offset = 0_u64;
    for _ in 0..4 {
        let Value::Temp(t) = a else { break };
        if context.body.temp_is_parameter(t) {
            break;
        }
        let Some(def) = context.body.temp(t).definition else {
            return Ok(false);
        };
        match context.body.instruction(def).kind {
            I::Addr(base @ (Value::Slot(_) | Value::Symbol(_))) => {
                a = base;
                break;
            }
            I::Index(ty, _, base, Value::Integer(k)) if context.body.literal(k) >> 64 == 0 => {
                offset = offset.wrapping_add(
                    u64::from_le_bytes(
                        super::alias_facts::signed_word(context.body.literal(k)).to_le_bytes(),
                    )
                    .wrapping_mul(bytes(ty)),
                );
                a = base;
            }
            _ => return Ok(false),
        }
    }
    let mut readable = matches!(a, Value::Slot(_) | Value::Symbol(_));
    if let Value::Temp(t) = a
        && context.body.temp_is_parameter(t)
    {
        let function = context
            .program
            .function(context.program.symbol(context.symbol))
            .ok_or(Error::Limit)?;
        let param = context
            .program
            .signature(function.signature)
            .parameters
            .get(t.index())
            .ok_or(Error::Limit)?;
        readable = param.pass == Some(ParameterPass::ByAddress)
            && offset.wrapping_add(bytes(ty)) <= param.object_bytes
            && EntryObjects::default().kept(context, forest, loop_, t.index())?;
    }
    if !readable {
        return Ok(false);
    }
    move_before_jump(context, load, pre)?;
    let mut inst = context.body.instruction(load).clone();
    inst.kind = I::Load(Flags::memory(false, false), ty, address);
    context.body.update_instruction(load, inst);
    context.loops_removed = context.loops_removed.checked_add(1).ok_or(Error::Limit)?;
    Ok(true)
}

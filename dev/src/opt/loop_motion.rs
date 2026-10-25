//! Loop memory locations, entry lifetimes and load motion from `opt/loop_passes.cpp`.
//! See NOTICE.
use super::{
    alias_facts::{self, signed_size, signed_word},
    memory_contents::bytes,
    natural_loops::{Forest, LoopId},
    pass_context::{Context, Pass},
};
use crate::lowir::{
    Error,
    model::{
        Body, Facts as Flags, InstId, Instruction, InstructionKind as I, LinkEditor,
        OptimizerBlock, Value,
    },
    vocabulary::{Binary, Convert, Pass as ParameterPass, Type},
};

#[derive(Clone, Copy)]
pub(super) struct Located {
    pub(super) root: Value,
    pub(super) offset: i64,
    pub(super) known: bool,
}
pub(super) fn locate(body: &Body, mut value: Value) -> Located {
    let mut at = Located {
        root: value,
        offset: 0,
        known: true,
    };
    for _ in 0..8 {
        let Value::Temp(temp) = value else { break };
        if body.temp_is_parameter(temp) {
            break;
        }
        let Some(def) = body.temp(temp).definition else {
            break;
        };
        match body.instruction(def).kind {
            I::Addr(base) | I::Copy(Type::Ptr, base) => value = base,
            I::Index(ty, _, base, index) => {
                if let Value::Integer(k) = index {
                    at.offset = at.offset.wrapping_add(
                        signed_word(body.literal(k)).wrapping_mul(signed_size(bytes(ty))),
                    );
                } else {
                    at.known = false;
                }
                value = base;
            }
            _ => break,
        }
    }
    at.root = value;
    at
}
pub(super) fn invariant(
    context: &Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    value: Value,
) -> bool {
    super::induction::invariant(context.body, forest, context.block_map(), loop_, value)
}
pub(super) fn make_preheader(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
) -> Result<bool, Error> {
    let header = forest.get(loop_).header;
    let entries = context
        .cfg()?
        .predecessors(header)
        .iter()
        .copied()
        .filter(|&p| !forest.contains(loop_, p))
        .collect::<Vec<_>>();
    let [entry] = entries.as_slice() else {
        return Ok(false);
    };
    if context.cfg()?.handler(*entry) {
        return Ok(false);
    }
    let last = *context
        .body
        .block(*entry)
        .instructions
        .last()
        .ok_or(Error::Limit)?;
    let pre = context
        .body
        .optimizer_block(OptimizerBlock::LoopPreheader)?;
    context
        .body
        .append(pre, Instruction::new(None, I::Jump(header), 0))?;
    let at = context
        .body
        .layout
        .iter()
        .position(|&b| b == header)
        .ok_or(Error::Limit)?;
    context.body.layout.insert(at, pre);
    context.body.redirect_successors(last, header, pre);
    context.body.rename_phi_predecessor(header, *entry, pre);
    Ok(true)
}
pub(super) fn move_before_jump(
    context: &mut Context<'_>,
    id: InstId,
    target: crate::lowir::model::BlockId,
) -> Result<(), Error> {
    let from = context
        .block_map()
        .get(id.index())
        .copied()
        .flatten()
        .ok_or(Error::Limit)?;
    let mut old = context.body.block(from).instructions.clone();
    old.retain(|&i| i != id);
    context.body.replace_links(from, old)?;
    let mut new = context.body.block(target).instructions.clone();
    new.insert(new.len().checked_sub(1).ok_or(Error::Limit)?, id);
    context.body.replace_links(target, new)?;
    context.record_instruction_block(id, target);
    Ok(())
}

#[derive(Default)]
pub(super) struct EntryObjects {
    live: Vec<u64>,
}
impl EntryObjects {
    pub(super) fn kept(
        &mut self,
        context: &Context<'_>,
        forest: &Forest,
        loop_: LoopId,
        parameter: usize,
    ) -> Result<bool, Error> {
        if parameter >= 64 {
            return Ok(false);
        }
        let cfg = context.cfg()?;
        let count = context.body.blocks().len();
        if self.live.len() != count {
            let function = context
                .program
                .function(context.program.symbol(context.symbol))
                .ok_or(Error::Limit)?;
            let params = &context.program.signature(function.signature).parameters;
            let n = context.body.parameter_count.min(params.len()).min(64);
            let mut calls = vec![false; count];
            let mut accessed = vec![0_u64; count];
            for &b in cfg.reverse_postorder() {
                for &id in &context.body.block(b).instructions {
                    let kind = &context.body.instruction(id).kind;
                    if matches!(kind, I::Call(_)) && alias_facts::writes(context.program, kind) {
                        calls[b.index()] = true;
                        accessed[b.index()] = 0;
                        continue;
                    }
                    let (I::Load(_, _, address) | I::Store(_, _, _, address)) = *kind else {
                        continue;
                    };
                    let at = locate(context.body, address);
                    let Value::Temp(t) = at.root else { continue };
                    if t.index() < n
                        && at.known
                        && at.offset >= 0
                        && params[t.index()].pass == Some(ParameterPass::ByAddress)
                        && at.offset.unsigned_abs() < params[t.index()].object_bytes
                    {
                        accessed[b.index()] |= 1_u64 << t.index();
                    }
                }
            }
            let all = if n == 64 { u64::MAX } else { (1_u64 << n) - 1 };
            self.live = vec![all; count];
            loop {
                let mut changed = false;
                for &b in cfg.reverse_postorder() {
                    let mut input = if cfg.reverse_postorder().first() == Some(&b)
                        || !cfg.handler(b) && !cfg.predecessors(b).is_empty()
                    {
                        all
                    } else {
                        0
                    };
                    for &p in cfg.predecessors(b) {
                        input &= self.live[p.index()];
                    }
                    let out = (if calls[b.index()] { 0 } else { input }) | accessed[b.index()];
                    changed |= out != self.live[b.index()];
                    self.live[b.index()] = out;
                }
                if !changed {
                    break;
                }
            }
        }
        Ok(cfg
            .predecessors(forest.get(loop_).header)
            .iter()
            .all(|&p| forest.contains(loop_, p) || self.live[p.index()] >> parameter & 1 != 0))
    }
}
struct Write {
    at: Located,
    size: u64,
    ty: Type,
    typed: bool,
}
impl Write {
    fn overlaps(&self, at: Located, size: u64, ty: Type, typed: bool) -> bool {
        if self.at.root == at.root {
            return !self.at.known
                || !at.known
                || self.at.offset < at.offset.wrapping_add(signed_size(size))
                    && at.offset < self.at.offset.wrapping_add(signed_size(self.size));
        }
        if matches!(self.at.root, Value::Slot(_) | Value::Symbol(_))
            && matches!(at.root, Value::Slot(_) | Value::Symbol(_))
        {
            return false;
        }
        !alias_facts::typed_apart(self.typed, self.ty, typed, ty)
    }
}
struct Motion<'a, 'p> {
    context: &'a mut Context<'p>,
    objects: EntryObjects,
    uses: Vec<u32>,
    links: Option<LinkEditor>,
}
impl Motion<'_, '_> {
    fn ids(&self, block: crate::lowir::model::BlockId) -> Vec<InstId> {
        self.links.as_ref().map_or_else(
            || self.context.body.block(block).instructions.clone(),
            |links| links.ids(block),
        )
    }
    fn move_to(&mut self, id: InstId, target: crate::lowir::model::BlockId) -> Result<(), Error> {
        let from = self
            .context
            .block_map()
            .get(id.index())
            .copied()
            .flatten()
            .ok_or(Error::Limit)?;
        let links = self.links.as_mut().ok_or(Error::Limit)?;
        links.unlink(from, id)?;
        let last = links.last(target).ok_or(Error::Limit)?;
        links.insert_after(target, links.previous(last), id)?;
        self.context.record_instruction_block(id, target);
        Ok(())
    }

    fn address_steps(
        &self,
        forest: &Forest,
        loop_: LoopId,
        mut address: Value,
    ) -> Option<[Option<InstId>; 4]> {
        let mut steps = [None; 4];
        let mut count = 0;
        while !invariant(self.context, forest, loop_, address) && count < steps.len() {
            let Value::Temp(t) = address else { break };
            let def = self.context.body.temp(t).definition?;
            let kind = &self.context.body.instruction(def).kind;
            let (I::Index(_, _, base, _) | I::Addr(base) | I::Copy(Type::Ptr, base)) = *kind else {
                break;
            };
            let mut all = true;
            self.context
                .body
                .visit_values(kind, |v| all &= invariant(self.context, forest, loop_, v));
            if !all {
                break;
            }
            steps[count] = Some(def);
            count += 1;
            address = base;
        }
        invariant(self.context, forest, loop_, address).then_some(steps)
    }
    fn readable(
        &mut self,
        forest: &Forest,
        loop_: LoopId,
        at: Located,
        size: u64,
    ) -> Result<bool, Error> {
        if !at.known {
            return Ok(false);
        }
        if matches!(at.root, Value::Slot(_) | Value::Symbol(_)) {
            return Ok(true);
        }
        let Value::Temp(t) = at.root else {
            return Ok(false);
        };
        let function = self
            .context
            .program
            .function(self.context.program.symbol(self.context.symbol))
            .ok_or(Error::Limit)?;
        let params = &self
            .context
            .program
            .signature(function.signature)
            .parameters;
        if at.offset < 0 || !self.context.body.temp_is_parameter(t) {
            return Ok(false);
        }
        let Some(param) = params.get(t.index()) else {
            return Ok(false);
        };
        if param.object_bytes == 0
            || at.offset.unsigned_abs().wrapping_add(size) > param.object_bytes
        {
            return Ok(false);
        }
        if self.objects.live.is_empty()
            && let Some(links) = &self.links
        {
            links.write_links(self.context.body)?;
        }
        self.objects.kept(self.context, forest, loop_, t.index())
    }
    fn hoist_from(&mut self, forest: &Forest, loop_: LoopId, apply: bool) -> Result<bool, Error> {
        let l = forest.get(loop_);
        if apply && l.preheader.is_none() {
            return Ok(false);
        }
        let mut writes = Vec::new();
        for &b in &l.blocks {
            for id in self.ids(b) {
                let kind = &self.context.body.instruction(id).kind;
                let write = match *kind {
                    I::Store(flags, ty, _, address) => Write {
                        at: locate(self.context.body, address),
                        size: bytes(ty),
                        ty,
                        typed: flags.typed() && !flags.volatile(),
                    },
                    I::CopyObject(size, _, _, address) | I::ZeroInit(size, _, address) => Write {
                        at: locate(self.context.body, address),
                        size,
                        ty: Type::Void,
                        typed: false,
                    },
                    _ => {
                        if alias_facts::writes(self.context.program, kind)
                            || matches!(kind,I::Load(f,..) if f.volatile())
                        {
                            return Ok(false);
                        }
                        continue;
                    }
                };
                writes.push(write);
            }
        }
        let mut seen = vec![false; self.context.body.temps().len()];
        let mut live = 0_usize;
        for &b in &l.blocks {
            for id in self.ids(b) {
                let kind = &self.context.body.instruction(id).kind;
                live += usize::from(b == l.header && matches!(kind, I::Phi(..)));
                self.context.body.visit_values(kind, |v| {
                    if let Value::Temp(t) = v
                        && !seen[t.index()]
                        && invariant(self.context, forest, loop_, v)
                    {
                        seen[t.index()] = true;
                        live += 1;
                    }
                });
            }
        }
        let limit = 10_usize.saturating_sub(live);
        let mut moved = 0;
        let mut changed = false;
        loop {
            let mut progress = false;
            if moved >= limit {
                break;
            }
            for &b in &l.blocks {
                let mut clear = b == l.header;
                for id in self.ids(b) {
                    let inst = self.context.body.instruction(id).clone();
                    if matches!(
                        inst.kind,
                        I::Call(_)
                            | I::Runtime(
                                crate::lowir::model::RuntimeInstruction::Throw(..)
                                    | crate::lowir::model::RuntimeInstruction::Resume
                            )
                    ) {
                        clear = false;
                    }
                    let I::Load(flags, ty, address) = inst.kind else {
                        continue;
                    };
                    if flags.volatile()
                        || matches!(ty, Type::Object { .. })
                        || inst.result.is_none()
                    {
                        continue;
                    }
                    let Some(steps) = self.address_steps(forest, loop_, address) else {
                        continue;
                    };
                    let at = locate(self.context.body, address);
                    let clobbered = writes
                        .iter()
                        .any(|w| w.overlaps(at, bytes(ty), ty, flags.typed()));
                    let on_entry = clear && b == l.header;
                    if clobbered
                        || !(on_entry || self.readable(forest, loop_, at, bytes(ty))?)
                        || moved >= limit
                    {
                        continue;
                    }
                    if !apply {
                        return Ok(true);
                    }
                    let pre = l.preheader.ok_or(Error::Limit)?;
                    for step in steps.into_iter().rev().flatten() {
                        self.move_to(step, pre)?;
                    }
                    self.move_to(id, pre)?;
                    if !on_entry {
                        let mut inst = self.context.body.instruction(id).clone();
                        inst.kind = I::Load(Flags::memory(false, false), ty, address);
                        self.context.body.update_instruction(id, inst);
                    }
                    moved += 1;
                    changed = true;
                    progress = true;
                }
            }
            if !progress {
                break;
            }
        }
        if self.uses.is_empty() {
            self.context.body.recount_uses()?;
            self.uses = self.context.body.temps().iter().map(|t| t.uses).collect();
        }
        loop {
            let mut progress = false;
            for &b in &l.blocks {
                for id in self.ids(b) {
                    let inst = self.context.body.instruction(id);
                    let arithmetic = inst.result.is_some()
                        && alias_facts::integral(inst.kind.result_type())
                        && matches!(inst.kind,
                        I::Binary(op,..) if !matches!(op,Binary::Div|Binary::Mod|Binary::UDiv|Binary::UMod))
                        || inst.result.is_some()
                            && alias_facts::integral(inst.kind.result_type())
                            && matches!(inst.kind,I::Convert(Convert::ZeroExtend|Convert::SignExtend|Convert::Truncate,_,source,_) if source.bits()!=0);
                    if !arithmetic {
                        continue;
                    }
                    let values = match inst.kind {
                        I::Binary(_, _, _, a, b) => [Some(a), Some(b)],
                        I::Convert(_, _, _, v) => [Some(v), None],
                        _ => return Err(Error::Limit),
                    };
                    let all = values
                        .iter()
                        .flatten()
                        .all(|&v| invariant(self.context, forest, loop_, v));
                    let dying = values.iter().flatten().any(|&v| {
                        matches!(v, Value::Temp(t) if usize::try_from(self.uses[t.index()]).ok() == Some(values.iter().flatten().filter(|&&w| w == v).count()))
                    });
                    if !all || !dying {
                        continue;
                    }
                    if !apply {
                        return Ok(true);
                    }
                    let mut kind = inst.kind.clone();
                    if let I::Binary(_, flags, ..) = &mut kind {
                        *flags = Flags::default();
                    }
                    let mut inst = self.context.body.instruction(id).clone();
                    inst.kind = kind;
                    self.context.body.update_instruction(id, inst);
                    self.move_to(id, l.preheader.ok_or(Error::Limit)?)?;
                    changed = true;
                    progress = true;
                }
            }
            if !progress {
                break;
            }
        }
        Ok(changed)
    }
}
pub(super) fn hoist(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg()?;
    let mut forest = Forest::default();
    forest.find(context.body.blocks().len(), context.cfg()?)?;
    if forest.len() == 0 {
        return Ok(false);
    }
    context.prepare_block_map(Pass::HoistLoads, true);
    let mut motion = Motion {
        context,
        objects: EntryObjects::default(),
        uses: Vec::new(),
        links: None,
    };
    let mut changed = false;
    for _ in 0..16 {
        let mut made = false;
        for loop_ in forest.ids() {
            if forest.get(loop_).preheader.is_none()
                && motion.hoist_from(&forest, loop_, false)?
                && make_preheader(motion.context, &forest, loop_)?
            {
                made = true;
                break;
            }
        }
        if !made {
            break;
        }
        changed = true;
        motion.context.invalidate_cfg();
        motion.context.ensure_cfg()?;
        motion.objects = EntryObjects::default();
        forest.find(motion.context.body.blocks().len(), motion.context.cfg()?)?;
        motion.context.prepare_block_map(Pass::HoistLoads, false);
    }
    motion.context.body.recount_uses()?;
    motion.uses = motion.context.body.temps().iter().map(|t| t.uses).collect();
    motion.links = Some(LinkEditor::new(motion.context.body)?);
    for loop_ in forest.ids().rev() {
        changed = motion.hoist_from(&forest, loop_, true)? || changed;
    }
    motion
        .links
        .take()
        .ok_or(Error::Limit)?
        .commit(motion.context.body)?;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_variable_index_keeps_its_root_but_loses_range_precision() {
        let p=crate::lowir::test_program(b"global @g : i64 = 0 function @f(%i:i64)->ptr {block ^entry:%a = index i64 @g,%i %b = index i8 %a,3 return ptr %b}").unwrap();
        let body = p
            .function(p.symbol(p.publications()[1]))
            .unwrap()
            .body
            .as_ref()
            .unwrap();
        let last = body
            .block(body.layout[0])
            .instructions
            .last()
            .copied()
            .unwrap();
        let I::Return(_, Some(value)) = body.instruction(last).kind else {
            panic!("return")
        };
        let at = locate(body, value);
        assert!(matches!(at.root, Value::Symbol(_)));
        assert_eq!(at.offset, 3);
        assert!(!at.known);
    }
}

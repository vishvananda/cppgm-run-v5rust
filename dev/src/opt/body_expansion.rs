//! Whole and partial call copying from `opt/inliner.cpp`. See NOTICE.
use super::{partial_inline::Cut, return_values};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, ImportMap, InstId, Instruction, InstructionKind as I, OptimizerBlock, Value,
    },
};

pub(super) struct Expansion<'a> {
    pub(super) body: &'a mut Body,
    pub(super) next: &'a mut Vec<Option<BlockId>>,
    pub(super) replacements: &'a mut Vec<Option<Value>>,
}
impl Expansion<'_> {
    fn prepare(
        &mut self,
        source: &Body,
        call_id: InstId,
        block: BlockId,
        partial: Option<&Cut>,
    ) -> Result<ImportMap, Error> {
        let inst = self.body.instruction(call_id).clone();
        let I::Call(call) = &inst.kind else {
            return Err(Error::Limit);
        };
        let args = self.body.arguments(call.arguments).to_vec();
        if args.len() < source.parameter_count {
            return Err(Error::Limit);
        }
        let mut map = ImportMap::new(source);
        for (index, &arg) in args.iter().take(source.parameter_count).enumerate() {
            let temp = source.temp_id(index).ok_or(Error::Limit)?;
            let ty = source.temp(temp).ty.ok_or(Error::Limit)?;
            map.temps[index] = Some(
                if matches!(arg,Value::Temp(t)if self.body.temp(t).ty!=Some(ty)) {
                    let temp = self.body.generated_temp(ty)?;
                    let id = self
                        .body
                        .append(block, Instruction::new(Some(temp), I::Copy(ty, arg), 0))?;
                    self.body.inherit_location(id, call_id);
                    Value::Temp(temp)
                } else {
                    arg
                },
            );
        }
        if let Some(partial) = partial {
            for &b in &source.layout {
                if !partial.copy[b.index()] {
                    continue;
                }
                for &id in &source.block(b).instructions {
                    if let Some(t) = source.instruction(id).result {
                        map.temps[t.index()] = Some(self.body.import_temp(source, t)?);
                    }
                }
            }
        } else {
            for index in source.parameter_count..source.temps().len() {
                let temp = source.temp_id(index).ok_or(Error::Limit)?;
                map.temps[index] = Some(self.body.import_temp(source, temp)?);
            }
        }
        let mut named = vec![partial.is_none(); source.slots().len()];
        if let Some(partial) = partial {
            for &b in &source.layout {
                if !partial.copy[b.index()] {
                    continue;
                }
                for &id in &source.block(b).instructions {
                    mark_slots(source, id, &mut named);
                }
            }
        }
        for (index, &named) in named.iter().enumerate() {
            if named {
                let slot = source.slot_id(index).ok_or(Error::Limit)?;
                map.slots[index] = Some(self.body.import_slot(source, slot)?);
            }
        }
        for &b in &source.layout {
            if partial.is_none_or(|p| p.copy[b.index()] || p.boundary[b.index()]) {
                map.blocks[b.index()] = Some(self.body.import_block(source, b)?);
            }
        }
        Ok(map)
    }
    pub(super) fn copy(
        &mut self,
        block: BlockId,
        call_id: InstId,
        source: &Body,
        partial: Option<&Cut>,
    ) -> Result<(), Error> {
        let call = self.body.instruction(call_id).clone();
        let ids = self.body.block(block).instructions.clone();
        let at = ids
            .iter()
            .position(|&id| id == call_id)
            .ok_or(Error::Limit)?;
        let cont = self
            .body
            .optimizer_block(OptimizerBlock::InlineContinuation)?;
        self.body.replace_links(block, ids[..at].to_vec())?;
        self.body.replace_links(cont, ids[at + 1..].to_vec())?;
        if let Some(&last) = self.body.block(cont).instructions.last() {
            let mut targets = Vec::new();
            self.body
                .instruction(last)
                .kind
                .visit_targets(|b| targets.push(b));
            for b in targets {
                self.body.rename_phi_predecessor(b, block, cont);
            }
        } else {
            self.body
                .append(cont, Instruction::new(None, I::Unreachable, 0))?;
        }
        let mut map = self.prepare(source, call_id, block, partial)?;
        let mut returns = Vec::new();
        for &b in &source.layout {
            let Some(nb) = map.blocks[b.index()] else {
                continue;
            };
            if partial.is_some_and(|p| p.boundary[b.index()]) {
                let mut again = call.clone();
                let I::Call(c) = &mut again.kind else {
                    return Err(Error::Limit);
                };
                let args = self.body.arguments(c.arguments).to_vec();
                c.arguments = self.body.add_arguments(&args)?;
                again.result = if let Some(result) = call.result {
                    Some(
                        self.body
                            .generated_temp(self.body.temp(result).ty.ok_or(Error::Limit)?)?,
                    )
                } else {
                    None
                };
                returns.push((nb, again.result.map(Value::Temp)));
                self.body.append(nb, again)?;
                let id = self
                    .body
                    .append(nb, Instruction::new(None, I::Jump(cont), 0))?;
                self.body.inherit_location(id, call_id);
                continue;
            }
            for &id in &source.block(b).instructions {
                let mut inst = self.body.import_instruction(
                    source,
                    id,
                    &mut map,
                    partial.map(|p| p.copy.as_slice()),
                )?;
                if let I::Return(_, value) = inst.kind {
                    returns.push((nb, value));
                    inst.kind = I::Jump(cont);
                }
                self.body.append(nb, inst)?;
            }
        }
        let entry =
            map.blocks[source.layout.first().ok_or(Error::Limit)?.index()].ok_or(Error::Limit)?;
        let jump = self
            .body
            .append(block, Instruction::new(None, I::Jump(entry), 0))?;
        self.body.inherit_location(jump, call_id);
        self.next.resize(self.body.blocks().len(), None);
        let after = self.next[block.index()];
        let mut tail = block;
        for &b in &source.layout {
            if let Some(nb) = map.blocks[b.index()] {
                self.next[tail.index()] = Some(nb);
                tail = nb;
            }
        }
        self.next[tail.index()] = Some(cont);
        self.next[cont.index()] = after;
        if let Some(result) = call.result {
            return_values::finish(self.body, cont, result, &mut returns, self.replacements)?;
        }
        Ok(())
    }
}

fn mark_slots(source: &Body, id: InstId, named: &mut [bool]) {
    source.visit_values(&source.instruction(id).kind, |value| {
        if let Value::Slot(slot) = value {
            named[slot.index()] = true;
        }
    });
}

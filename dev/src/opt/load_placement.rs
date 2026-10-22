//! Bounded phi/chain/PRE load placement from `memory_forwarding.cpp`. See NOTICE.
use super::{
    alias_facts::{self as facts, Address, Key},
    memory_contents::bytes,
    memory_forward::MemoryPass,
    pass_context::Pass,
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Facts, InstId, Instruction, InstructionKind as I, LinkEditor,
        RuntimeInstruction as R, TempId, Unwind, Value,
    },
    vocabulary::Type,
};

#[derive(Clone, Copy)]
enum Header {
    Unknown,
    Absent,
    Found(BlockId),
}
struct Placement<'a, 'b, 'p> {
    pass: &'a mut MemoryPass<'b, 'p>,
    links: LinkEditor,
    lists: Vec<Option<std::ops::Range<usize>>>,
    arena: Vec<InstId>,
    writes: Vec<u8>,
    headers: Vec<Header>,
}
impl MemoryPass<'_, '_> {
    pub(super) fn place_loads(&mut self) -> Result<(), Error> {
        self.context
            .prepare_block_map(Pass::ForwardMemory, !self.changed);
        let links = LinkEditor::new(self.context.body)?;
        let count = self.context.body.blocks().len();
        let mut placement = Placement {
            pass: self,
            links,
            lists: vec![None; count],
            arena: Vec::new(),
            writes: vec![0; count],
            headers: vec![Header::Unknown; count],
        };
        placement.run()?;
        placement.links.commit(placement.pass.context.body)
    }
}
impl Placement<'_, '_, '_> {
    fn may_clobber(&self, kind: &I, key: Key, want: Address, ty: Type, typed: bool) -> bool {
        match *kind {
            I::Store(flags, stored, _, address) => {
                if flags.volatile() || self.pass.key(address, stored) == key {
                    return true;
                }
                let at = self.pass.at(address);
                let apart = facts::typed_apart(flags.typed(), stored, typed, ty)
                    && !(at.base.is_some() && at.same_base(want));
                !apart
                    && self
                        .pass
                        .addresses
                        .may_alias(at, bytes(stored), want, bytes(ty))
            }
            I::CopyObject(size, _, _, address) | I::ZeroInit(size, _, address) => self
                .pass
                .addresses
                .may_alias(self.pass.at(address), size, want, bytes(ty)),
            _ => !self.pass.addresses.private(want),
        }
    }
    fn block_list(&mut self, block: BlockId) -> std::ops::Range<usize> {
        if let Some(list) = &self.lists[block.index()] {
            return list.clone();
        }
        let first = self.arena.len();
        self.arena.extend(self.links.ids(block));
        let list = first..self.arena.len();
        self.lists[block.index()] = Some(list.clone());
        list
    }
    fn value_at_end(
        &mut self,
        mut block: BlockId,
        key: Key,
        want: Address,
        ty: Type,
        typed: bool,
    ) -> Result<Option<Value>, Error> {
        let mut budget = 128_usize;
        for _ in 0..3 {
            let ids = self.block_list(block);
            for index in ids.rev() {
                let id = self.arena[index];
                if budget == 0 {
                    return Ok(None);
                }
                budget -= 1;
                let kind = &self.pass.context.body.instruction(id).kind;
                if let I::Load(flags, loaded, address) = *kind {
                    if !flags.volatile()
                        && loaded == key.ty
                        && self.pass.key(address, loaded) == key
                    {
                        return Ok(self
                            .pass
                            .context
                            .body
                            .instruction(id)
                            .result
                            .map(Value::Temp));
                    }
                    continue;
                }
                if !facts::writes(self.pass.context.program, kind) {
                    continue;
                }
                if let I::Store(flags, stored, value, address) = *kind
                    && !flags.volatile()
                    && stored == key.ty
                    && self.pass.key(address, stored) == key
                {
                    return Ok(facts::exact(self.pass.context.body, value, ty).then_some(value));
                }
                if self.may_clobber(kind, key, want, ty, typed) {
                    return Ok(None);
                }
            }
            let cfg = self.pass.context.cfg()?;
            if cfg.predecessors(block).len() == 1 {
                block = cfg.predecessors(block)[0];
                continue;
            }
            let Some(arms) = self.pass.diamond_arms(block)? else {
                return Ok(None);
            };
            for arm in arms {
                for id in self.links.ids(arm) {
                    if budget == 0 {
                        return Ok(None);
                    }
                    budget -= 1;
                    let kind = &self.pass.context.body.instruction(id).kind;
                    if facts::writes(self.pass.context.program, kind)
                        && self.may_clobber(kind, key, want, ty, typed)
                    {
                        return Ok(None);
                    }
                }
            }
            let Some(parent) = self.pass.context.cfg()?.dominator(block) else {
                return Ok(None);
            };
            block = parent;
        }
        Ok(None)
    }
    fn value_for(
        &mut self,
        block: BlockId,
        address: Value,
        ty: Type,
        typed: bool,
    ) -> Result<Option<Value>, Error> {
        self.value_at_end(
            block,
            self.pass.key(address, ty),
            self.pass.at(address),
            ty,
            typed,
        )
    }
    fn phi_input(&self, phi: InstId, pred: BlockId) -> Option<Value> {
        let I::Phi(_, pairs) = &self.pass.context.body.instruction(phi).kind else {
            return None;
        };
        pairs
            .iter()
            .find_map(|&(from, v)| (from == pred).then_some(v))
    }
    fn place_phi(
        &mut self,
        block: BlockId,
        result: TempId,
        ty: Type,
        pairs: Vec<(BlockId, Value)>,
    ) -> Result<(), Error> {
        let id = self
            .pass
            .context
            .body
            .append(block, Instruction::new(Some(result), I::Phi(ty, pairs), 0))?;
        self.links.insert_after(block, None, id)?;
        self.pass.context.record_instruction_block(id, block);
        self.pass.reused_load()
    }
    fn replace_phi(
        &mut self,
        block: BlockId,
        id: InstId,
        pairs: Vec<(BlockId, Value)>,
    ) -> Result<(), Error> {
        let I::Load(_, ty, _) = self.pass.context.body.instruction(id).kind else {
            return Err(Error::Limit);
        };
        let result = self
            .pass
            .context
            .body
            .instruction(id)
            .result
            .ok_or(Error::Limit)?;
        self.links.unlink(block, id)?;
        self.place_phi(block, result, ty, pairs)
    }
    fn phi_load(&mut self, block: BlockId, id: InstId, phi: InstId) -> Result<bool, Error> {
        let I::Load(flags, ty, _) = self.pass.context.body.instruction(id).kind else {
            return Err(Error::Limit);
        };
        let I::Phi(_, inputs) = &self.pass.context.body.instruction(phi).kind else {
            return Err(Error::Limit);
        };
        let inputs = inputs.clone();
        let mut pairs = Vec::new();
        for (pred, address) in inputs {
            let Some(value) = self.value_for(pred, address, ty, flags.typed())? else {
                return Ok(false);
            };
            pairs.push((pred, value));
        }
        self.replace_phi(block, id, pairs)?;
        Ok(true)
    }
    fn block_writes(&mut self, block: BlockId) -> u8 {
        if self.writes[block.index()] != 0 {
            return self.writes[block.index()];
        }
        let mut flags = 1;
        for id in self.links.ids(block) {
            let kind = &self.pass.context.body.instruction(id).kind;
            if matches!(kind, I::Store(..) | I::CopyObject(..) | I::ZeroInit(..)) {
                flags |= 2;
            } else if !matches!(kind, I::Load(..) | I::Phi(..))
                && facts::writes(self.pass.context.program, kind)
            {
                flags |= 4;
            }
        }
        self.writes[block.index()] = flags;
        flags
    }
    fn chain_above(&self, block: BlockId) -> Result<Option<Vec<BlockId>>, Error> {
        let cfg = self.pass.context.cfg()?;
        let Some(&first) = cfg.predecessors(block).first() else {
            return Ok(None);
        };
        let mut at = first;
        let mut chain = Vec::new();
        loop {
            if chain.len() == 3 || cfg.handler(at) || at == block {
                return Ok(None);
            }
            chain.push(at);
            let preds = cfg.predecessors(at);
            if preds.len() >= 2 {
                return Ok(Some(chain));
            }
            let Some(&parent) = preds.first() else {
                return Ok(None);
            };
            at = parent;
        }
    }
    fn chain_header(&mut self, block: BlockId) -> Result<Option<BlockId>, Error> {
        match self.headers[block.index()] {
            Header::Found(block) => return Ok(Some(block)),
            Header::Absent => return Ok(None),
            Header::Unknown => (),
        }
        let join = self.chain_above(block)?.and_then(|c| c.last().copied());
        let cfg = self.pass.context.cfg()?;
        let join = join.filter(|&join| {
            cfg.predecessors(join).len() <= 8
                && cfg.reverse_postorder().first() != Some(&join)
                && cfg
                    .predecessors(join)
                    .iter()
                    .any(|&p| cfg.dominates(join, p))
        });
        self.headers[block.index()] = join.map_or(Header::Absent, Header::Found);
        Ok(join)
    }
    fn mapped_definition(&self, temp: TempId) -> Option<(InstId, BlockId)> {
        let id = self.pass.context.body.temp(temp).definition?;
        let block = self
            .pass
            .context
            .block_map()
            .get(id.index())
            .copied()
            .flatten()?;
        Some((id, block))
    }
    fn chain_load(&mut self, block: BlockId, id: InstId) -> Result<bool, Error> {
        let Some(chain) = self.chain_above(block)? else {
            return Ok(false);
        };
        let join = *chain.last().ok_or(Error::Limit)?;
        let load = self.pass.context.body.instruction(id).clone();
        let I::Load(flags, ty, address) = load.kind else {
            return Err(Error::Limit);
        };
        let want = self.pass.at(address);
        let private = self.pass.addresses.private(want);
        let mut base_phi = None;
        if !matches!(want.base, Some(Value::Slot(_) | Value::Symbol(_))) {
            let base = if let Some(v @ Value::Temp(_)) = want.base {
                v
            } else {
                address
            };
            if let Value::Temp(temp) = base
                && self.pass.context.body.temp(temp).definition.is_some()
            {
                let Some((def, at)) = self.mapped_definition(temp) else {
                    return Ok(false);
                };
                if !self.pass.context.cfg()?.dominates(at, join) {
                    return Ok(false);
                }
                if at == join {
                    if !matches!(self.pass.context.body.instruction(def).kind, I::Phi(..))
                        || !matches!(want.base, Some(Value::Temp(_)))
                    {
                        return Ok(false);
                    }
                    base_phi = Some(def);
                }
            }
        }
        let key = self.pass.key(address, ty);
        let mut budget = 128;
        for at in chain {
            let writes = self.block_writes(at);
            if writes & 4 != 0 && !private {
                return Ok(false);
            }
            if writes & 6 == 0 {
                continue;
            }
            for id in self.links.ids(at) {
                if budget == 0 {
                    return Ok(false);
                }
                budget -= 1;
                let kind = &self.pass.context.body.instruction(id).kind;
                if facts::writes(self.pass.context.program, kind)
                    && self.may_clobber(kind, key, want, ty, flags.typed())
                {
                    return Ok(false);
                }
            }
        }
        let preds = self.pass.context.cfg()?.predecessors(join).to_vec();
        let mut known = vec![None; preds.len()];
        for back_edge in [false, true] {
            for (index, &pred) in preds.iter().enumerate() {
                if self.pass.context.cfg()?.dominates(join, pred) != back_edge {
                    continue;
                }
                known[index] =
                    self.value_from_edge(pred, address, ty, flags.typed(), want, base_phi)?;
                if known[index].is_none() {
                    return Ok(false);
                }
            }
        }
        let pairs = preds
            .into_iter()
            .zip(known)
            .map(|(pred, value)| Ok((pred, value.ok_or(Error::Limit)?)))
            .collect::<Result<Vec<_>, Error>>()?;
        self.links.unlink(block, id)?;
        self.place_phi(join, load.result.ok_or(Error::Limit)?, ty, pairs)?;
        self.lists[join.index()] = None;
        Ok(true)
    }
    fn value_from_edge(
        &mut self,
        pred: BlockId,
        address: Value,
        ty: Type,
        typed: bool,
        want: Address,
        base_phi: Option<InstId>,
    ) -> Result<Option<Value>, Error> {
        let Some(phi) = base_phi else {
            return self.value_for(pred, address, ty, typed);
        };
        let Some(input) = self.phi_input(phi, pred) else {
            return Ok(None);
        };
        let mut at = self.pass.at(input);
        if at.base.is_none() {
            if !matches!(input, Value::Temp(_)) {
                return Ok(None);
            }
            at.base = Some(input);
        }
        at = at.shifted(want.offset);
        self.value_at_end(pred, Key::new(at, Some(input), ty), at, ty, typed)
    }
    fn available(&self, value: Value, block: BlockId) -> Result<bool, Error> {
        let Value::Temp(temp) = value else {
            return Ok(matches!(value, Value::Slot(_) | Value::Symbol(_)));
        };
        if self.pass.context.body.temp(temp).definition.is_none() {
            return Ok(true);
        }
        let cfg = self.pass.context.cfg()?;
        Ok(self
            .mapped_definition(temp)
            .is_some_and(|(_, at)| at != block && cfg.dominates(at, block)))
    }
    fn append_before_jump(
        &mut self,
        block: BlockId,
        mut instruction: Instruction,
    ) -> Result<Value, Error> {
        let original = instruction.result.ok_or(Error::Limit)?;
        let ty = self
            .pass
            .context
            .body
            .temp(original)
            .ty
            .ok_or(Error::Limit)?;
        let result = self.pass.context.body.generated_temp(ty)?;
        instruction.result = Some(result);
        let id = self.pass.context.body.append(block, instruction)?;
        if let Some(&address) = self.pass.addresses.values.get(original.index()) {
            self.pass
                .addresses
                .values
                .resize(result.index() + 1, Address::default());
            self.pass.addresses.values[result.index()] = address;
        }
        let last = self.links.last(block).ok_or(Error::Limit)?;
        self.links
            .insert_after(block, self.links.previous(last), id)?;
        self.pass.context.record_instruction_block(id, block);
        self.lists[block.index()] = None;
        Ok(Value::Temp(result))
    }
    fn pre_load(&mut self, block: BlockId, id: InstId) -> Result<bool, Error> {
        let load = self.pass.context.body.instruction(id).clone();
        let I::Load(flags, ty, address) = load.kind else {
            return Err(Error::Limit);
        };
        let preds = self.pass.context.cfg()?.predecessors(block).to_vec();
        if preds.len() < 2
            || preds.len() > 8
            || self.pass.context.cfg()?.reverse_postorder().first() == Some(&block)
        {
            return Ok(false);
        }
        let mut step = None;
        let mut base_phi = None;
        if !self.available(address, block)? {
            let Value::Temp(temp) = address else {
                return Ok(false);
            };
            let Some(def) = self.pass.context.body.temp(temp).definition else {
                return Ok(false);
            };
            let kind = &self.pass.context.body.instruction(def).kind;
            let (I::Index(_, _, base, Value::Integer(_)) | I::Addr(base)) = *kind else {
                return Ok(false);
            };
            step = Some(def);
            if !self.available(base, block)? {
                let Value::Temp(base) = base else {
                    return Ok(false);
                };
                if self.pass.context.body.temp_is_parameter(base) || !matches!(kind, I::Index(..)) {
                    return Ok(false);
                }
                let Some((def, at)) = self.mapped_definition(base) else {
                    return Ok(false);
                };
                if at != block
                    || !matches!(self.pass.context.body.instruction(def).kind, I::Phi(..))
                {
                    return Ok(false);
                }
                base_phi = Some(def);
            }
        }
        let want = self.pass.at(address);
        if let Some(phi) = base_phi
            && want.base
                != self
                    .pass
                    .context
                    .body
                    .instruction(phi)
                    .result
                    .map(Value::Temp)
        {
            return Ok(false);
        }
        let mut pairs = Vec::new();
        let mut missing = 0;
        for &pred in &preds {
            let value = self.value_from_edge(pred, address, ty, flags.typed(), want, base_phi)?;
            if value.is_none() {
                let Some(last) = self.links.last(pred) else {
                    return Ok(false);
                };
                if self.pass.context.cfg()?.successors(pred).len() != 1
                    || self.pass.context.cfg()?.handler(pred)
                    || !matches!(self.pass.context.body.instruction(last).kind, I::Jump(_))
                {
                    return Ok(false);
                }
                missing += 1;
            }
            pairs.push((pred, value));
        }
        if missing == preds.len() {
            return Ok(false);
        }
        let mut speculates = false;
        let mut before = self.links.first(block);
        while let Some(at) = before {
            if at == id || speculates {
                break;
            }
            let kind = &self.pass.context.body.instruction(at).kind;
            speculates = match kind {
                I::Runtime(R::Throw(..) | R::Resume) => true,
                I::Call(call) => self
                    .pass
                    .context
                    .program
                    .call_boundary(call)
                    .is_none_or(|s| s.boundary.unwind != Unwind::No),
                _ => false,
            };
            before = self.links.next(at);
        }
        let mut inputs = Vec::new();
        for (pred, known) in pairs {
            let value = if let Some(value) = known {
                value
            } else {
                let mut at_address = address;
                if let Some(step) = step {
                    let mut recompute = self.pass.context.body.instruction(step).clone();
                    if let Some(phi) = base_phi {
                        let value = self.phi_input(phi, pred).ok_or(Error::Limit)?;
                        match &mut recompute.kind {
                            I::Index(_, _, base, _) => *base = value,
                            _ => return Err(Error::Limit),
                        }
                    }
                    at_address = self.append_before_jump(pred, recompute)?;
                }
                let mut copy = load.clone();
                copy.kind = I::Load(
                    Facts::memory(flags.volatile(), flags.typed() && !speculates),
                    ty,
                    at_address,
                );
                self.append_before_jump(pred, copy)?
            };
            inputs.push((pred, value));
        }
        self.replace_phi(block, id, inputs)?;
        Ok(true)
    }
    fn phi_address(&self, mut address: Value, block: BlockId) -> Option<InstId> {
        for _ in 0..3 {
            let Value::Temp(temp) = address else {
                break;
            };
            let Some(def) = self.pass.context.body.temp(temp).definition else {
                break;
            };
            let I::Copy(_, base) = self.pass.context.body.instruction(def).kind else {
                break;
            };
            address = base;
        }
        let Value::Temp(temp) = address else {
            return None;
        };
        let (def, at) = self.mapped_definition(temp)?;
        (at == block && matches!(self.pass.context.body.instruction(def).kind, I::Phi(..)))
            .then_some(def)
    }
    fn run(&mut self) -> Result<(), Error> {
        for block in self.pass.context.body.layout.clone() {
            let mut current = self.links.first(block);
            let mut written = false;
            while let Some(id) = current {
                let next = self.links.next(id);
                let kind = &self.pass.context.body.instruction(id).kind;
                let mut rewritten = false;
                if let I::Load(flags, ty, address) = *kind
                    && !flags.volatile()
                    && (facts::integral(ty) || matches!(ty, Type::F32 | Type::F64))
                    && !self.pass.context.cfg()?.handler(block)
                    && !written
                {
                    let phi = self.phi_address(address, block);
                    rewritten = if let Some(phi) = phi {
                        self.phi_load(block, id, phi)?
                    } else if self.pass.context.cfg()?.predecessors(block).len() == 1 {
                        self.chain_header(block)?.is_some() && self.chain_load(block, id)?
                    } else {
                        self.pre_load(block, id)?
                    };
                }
                if rewritten {
                    self.lists[block.index()] = None;
                } else {
                    let kind = &self.pass.context.body.instruction(id).kind;
                    written |= !matches!(kind, I::Phi(..) | I::Load(..))
                        && facts::writes(self.pass.context.program, kind);
                }
                current = next;
            }
        }
        Ok(())
    }
}

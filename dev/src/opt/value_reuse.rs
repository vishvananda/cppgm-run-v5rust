//! Dominator-scoped scalar simplification from `opt/value_simplify.cpp`. See NOTICE.
use super::{
    algebra::{self, Algebra, producer},
    definition_order, memory_calls, numeric_fold as fold,
    pass_context::{Context, Pass},
    pure,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{
        BlockId, Body, InstId, Instruction, InstructionKind as I, RuntimeInstruction as R, TempId,
        Value,
    },
    vocabulary::{Binary, Compare, Convert, Projection, Type, Unary},
};
use crate::support::hash::HashMap;
use crate::support::hash::Hasher as Fingerprint;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Operation {
    Copy(Type),
    Unary(Unary, Type),
    Binary(Binary, Type),
    Compare(Compare, Type),
    Convert(Convert, Type, Type),
    Select(Type),
    Addr,
    Index(Type, Option<Projection>),
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Operand {
    Identity(Value),
    Literal(Constant),
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Expression {
    op: Operation,
    values: [Option<Operand>; 3],
}
#[derive(Clone, Copy)]
struct Available {
    temp: TempId,
    block: BlockId,
    ordinal: usize,
}
#[derive(Clone, Copy, Hash)]
enum PhiOperand {
    SelfValue,
    Other(Value),
}

#[derive(Default)]
pub(super) struct Scratch {
    replacements: Vec<Option<Value>>,
    uses: Vec<u32>,
    alive: Vec<bool>,
    nonzero: Vec<bool>,
    equal: Vec<Option<Value>>,
    nonzero_undo: Vec<TempId>,
    equal_undo: Vec<TempId>,
    ordinal: Vec<usize>,
    first_opener: Vec<Option<usize>>,
    handler_reached: Vec<bool>,
    walked: Vec<bool>,
    table: HashMap<Expression, Available>,
}

pub(super) struct Simplifier<'a, 'p> {
    pub(super) context: &'a mut Context<'p>,
    pub(super) replacements: Vec<Option<Value>>,
    uses: Vec<u32>,
    pub(super) alive: Vec<bool>,
    pub(super) nonzero: Vec<bool>,
    pub(super) equal: Vec<Option<Value>>,
    nonzero_undo: Vec<TempId>,
    equal_undo: Vec<TempId>,
    ordinal: Vec<usize>,
    first_opener: Vec<Option<usize>>,
    handler_reached: Vec<bool>,
    walked: Vec<bool>,
    table: HashMap<Expression, Available>,
    pub(super) changed: bool,
    edited: bool,
}
fn resolve_value(replacements: &mut [Option<Value>], mut v: Value) -> Result<Value, Error> {
    let original = v;
    let mut count = 0;
    while let Value::Temp(t) = v {
        let Some(next) = replacements[t.index()] else {
            break;
        };
        v = next;
        count += 1;
        if count > replacements.len() {
            return Err(Error::Construction("cyclic scalar replacement".into()));
        }
    }
    let result = v;
    let mut v = original;
    while let Value::Temp(t) = v {
        let Some(next) = replacements[t.index()] else {
            break;
        };
        replacements[t.index()] = Some(result);
        v = next;
    }
    Ok(result)
}

impl<'a, 'p> Simplifier<'a, 'p> {
    fn new(context: &'a mut Context<'p>) -> Self {
        let temps = context.body.temps().len();
        let blocks = context.body.blocks().len();
        let instructions = context.body.instructions().len();
        let mut scratch = std::mem::take(&mut context.simplifier_scratch);
        scratch.uses.clear();
        scratch.uses.extend_from_slice(context.entry_uses());
        scratch.replacements.resize(temps, None);
        scratch.replacements.fill(None);
        scratch.alive.resize(instructions, true);
        scratch.alive.fill(true);
        scratch.nonzero.resize(temps, false);
        scratch.nonzero.fill(false);
        scratch.equal.resize(temps, None);
        scratch.equal.fill(None);
        scratch.ordinal.resize(instructions, 0);
        scratch.ordinal.fill(0);
        scratch.first_opener.resize(blocks, None);
        scratch.first_opener.fill(None);
        scratch.handler_reached.resize(blocks, false);
        scratch.handler_reached.fill(false);
        scratch.walked.resize(blocks, false);
        scratch.walked.fill(false);
        scratch.nonzero_undo.clear();
        scratch.equal_undo.clear();
        scratch.table.clear();
        Self {
            context,
            replacements: scratch.replacements,
            uses: scratch.uses,
            alive: scratch.alive,
            nonzero: scratch.nonzero,
            equal: scratch.equal,
            nonzero_undo: scratch.nonzero_undo,
            equal_undo: scratch.equal_undo,
            ordinal: scratch.ordinal,
            first_opener: scratch.first_opener,
            handler_reached: scratch.handler_reached,
            walked: scratch.walked,
            table: scratch.table,
            changed: false,
            edited: false,
        }
    }
    fn recycle(&mut self) {
        let scratch = &mut self.context.simplifier_scratch;
        std::mem::swap(&mut self.replacements, &mut scratch.replacements);
        std::mem::swap(&mut self.alive, &mut scratch.alive);
        std::mem::swap(&mut self.nonzero, &mut scratch.nonzero);
        std::mem::swap(&mut self.equal, &mut scratch.equal);
        std::mem::swap(&mut self.ordinal, &mut scratch.ordinal);
        std::mem::swap(&mut self.first_opener, &mut scratch.first_opener);
        std::mem::swap(&mut self.handler_reached, &mut scratch.handler_reached);
        std::mem::swap(&mut self.walked, &mut scratch.walked);
        std::mem::swap(&mut self.uses, &mut scratch.uses);
        std::mem::swap(&mut self.nonzero_undo, &mut scratch.nonzero_undo);
        std::mem::swap(&mut self.equal_undo, &mut scratch.equal_undo);
        std::mem::swap(&mut self.table, &mut scratch.table);
    }
    pub(super) fn resolve(&mut self, v: Value) -> Result<Value, Error> {
        resolve_value(&mut self.replacements, v)
    }
    fn index_openers(&mut self) -> Result<(), Error> {
        let mut work = Vec::new();
        let cfg = self.context.cfg()?;
        for &block in &self.context.body.layout {
            if cfg.has_eh(block) {
                for (ordinal, &id) in self
                    .context
                    .body
                    .block(block)
                    .instructions
                    .iter()
                    .enumerate()
                {
                    self.ordinal[id.index()] = ordinal;
                    if self.first_opener[block.index()].is_none()
                        && matches!(
                            self.context.body.instruction(id).kind,
                            I::Runtime(R::EhTry(_) | R::EhCleanup(_))
                        )
                    {
                        self.first_opener[block.index()] = Some(ordinal);
                    }
                }
            }
            if cfg.handler(block) {
                self.handler_reached[block.index()] = true;
                work.push(block);
            }
        }
        while let Some(block) = work.pop() {
            for &next in cfg.successors(block).iter().chain(cfg.handlers(block)) {
                if !std::mem::replace(&mut self.handler_reached[next.index()], true) {
                    work.push(next);
                }
            }
        }
        Ok(())
    }
    pub(super) fn available_in(&self, temp: TempId, block: BlockId) -> bool {
        let body = &*self.context.body;
        if !self.handler_reached[block.index()]
            || body.temp_is_parameter(temp)
            || body.temp(temp).definition.is_none()
        {
            return true;
        }
        let Some(def) = body.temp(temp).definition else {
            return true;
        };
        self.context
            .block_map()
            .get(def.index())
            .copied()
            .flatten()
            .is_some_and(|at| {
                self.first_opener[at.index()]
                    .is_none_or(|opener| self.ordinal[def.index()] < opener)
            })
    }
    fn substitute(&mut self, id: InstId) -> Result<(), Error> {
        // Resolve the one operand that queries producer facts before borrowing
        // the editable operand store. All remaining rewrites use disjoint facts.
        let callee = if let I::Call(call) = &self.context.body.instruction(id).kind {
            let original = call.callee;
            let signature = call.signature.is_some();
            let mut next = self.resolve(original)?;
            if matches!(next, Value::Integer(_) | Value::Float(..)) {
                next = original;
            } else if self.context.level >= 2
                && signature
                && let Some((_, def)) = producer(self.context.body, next)
                && let I::Addr(s @ Value::Symbol(symbol)) = def.kind
                && self
                    .context
                    .program
                    .function(self.context.program.symbol(symbol))
                    .is_some()
            {
                next = s;
            }
            Some(next)
        } else {
            None
        };
        let mut index = 0;
        let replacements = &mut self.replacements;
        let equal = &self.equal;
        let edited = &mut self.edited;
        self.context.body.rewrite_values(id, |was| {
            let is_callee = index == 0 && callee.is_some();
            index += 1;
            let next = if is_callee {
                callee.ok_or(Error::Limit)?
            } else if matches!(was, Value::Temp(_)) {
                let next = resolve_value(replacements, was)?;
                if let Value::Temp(t) = next {
                    equal[t.index()].unwrap_or(next)
                } else {
                    next
                }
            } else {
                was
            };
            *edited |= was != next;
            Ok(next)
        })
    }
    fn expression(&self, kind: &I) -> Option<Expression> {
        let (op, ty, mut values) = match *kind {
            I::Copy(t, a) => (Operation::Copy(t), t, [Some(a), None, None]),
            I::Unary(op, t, a) => (Operation::Unary(op, t), t, [Some(a), None, None]),
            I::Binary(op, _, t, a, b) => (Operation::Binary(op, t), t, [Some(a), Some(b), None]),
            I::Compare(mut pred, t, mut a, mut b) => {
                if matches!((a,b),(Value::Temp(a),Value::Temp(b)) if a.index()>b.index()) {
                    std::mem::swap(&mut a, &mut b);
                    pred = pred.swapped();
                }
                (Operation::Compare(pred, t), t, [Some(a), Some(b), None])
            }
            I::Convert(op, to, from, a) => (
                Operation::Convert(op, to, from),
                from,
                [Some(a), None, None],
            ),
            I::Select(t, a, b, c) => (Operation::Select(t), t, [Some(a), Some(b), Some(c)]),
            I::Addr(a) => (Operation::Addr, Type::Ptr, [Some(a), None, None]),
            I::Index(t, p, a, b) => (Operation::Index(t, p), Type::I64, [Some(a), Some(b), None]),
            _ => return None,
        };
        let values = values.each_mut().map(|v| {
            v.map(|value| match fold::literal(self.context.body, value) {
                Some(Constant::Integer(bits)) => Operand::Literal(fold::integer(ty, bits)),
                Some(c) => Operand::Literal(c),
                None => Operand::Identity(value),
            })
        });
        Some(Expression { op, values })
    }
    fn phi_value(&self, inst: &Instruction) -> Option<Value> {
        let I::Phi(ty, pairs) = &inst.kind else {
            return None;
        };
        let mut same = None;
        for &(_, v) in pairs {
            if Some(v) == inst.result.map(Value::Temp) {
                continue;
            }
            let Some(first) = same else {
                same = Some(v);
                continue;
            };
            if first == v {
                continue;
            }
            if !fold::literal(self.context.body, first)
                .zip(fold::literal(self.context.body, v))
                .is_some_and(|(a, b)| fold::same(*ty, a, b))
            {
                return None;
            }
        }
        same
    }
    fn phi_signature(inst: &Instruction) -> Result<(Type, usize, u64), Error> {
        let I::Phi(ty, pairs) = &inst.kind else {
            return Err(Error::Limit);
        };
        let mut sum = 0_u64;
        for &(block, v) in pairs {
            let mut h = Fingerprint::default();
            block.hash(&mut h);
            (if Some(v) == inst.result.map(Value::Temp) {
                PhiOperand::SelfValue
            } else {
                PhiOperand::Other(v)
            })
            .hash(&mut h);
            sum = sum.wrapping_add(h.finish());
        }
        Ok((*ty, pairs.len(), sum))
    }
    fn same_phi(&self, a: InstId, b: InstId) -> bool {
        let (a, b) = (
            self.context.body.instruction(a),
            self.context.body.instruction(b),
        );
        let (I::Phi(ta, pa), I::Phi(tb, pb)) = (&a.kind, &b.kind) else {
            return false;
        };
        if ta != tb || pa.len() != pb.len() {
            return false;
        }
        let mut inputs = HashMap::default();
        for &(pred, v) in pb {
            inputs.entry(pred).or_insert(v);
        }
        pa.iter().all(|&(pred, v)| {
            let input = inputs.get(&pred).copied();
            let self_a = Some(v) == a.result.map(Value::Temp);
            let self_b = input.is_some() && input == b.result.map(Value::Temp);
            self_a == self_b && (self_a || input == Some(v))
        })
    }
    fn prepare_phis(&mut self, block: BlockId) -> Result<(), Error> {
        let ids = self
            .context
            .body
            .block(block)
            .instructions
            .iter()
            .copied()
            .take_while(|&id| matches!(self.context.body.instruction(id).kind, I::Phi(..)))
            .collect::<Vec<_>>();
        let mut chains: HashMap<(Type, usize, u64), Vec<InstId>> = HashMap::default();
        for id in ids.iter().copied() {
            let mut instruction = self.context.body.instruction(id).clone();
            if let I::Phi(_, pairs) = &mut instruction.kind {
                for (_, v) in pairs {
                    *v = self.resolve(*v)?;
                }
            }
            self.context.body.update_instruction(id, instruction);
            if ids.len() < 2 {
                continue;
            }
            let key = Self::phi_signature(self.context.body.instruction(id))?;
            let result = self
                .context
                .body
                .instruction(id)
                .result
                .ok_or(Error::Limit)?;
            if let Some(chain) = chains.get(&key) {
                for &earlier in chain {
                    let previous = self
                        .context
                        .body
                        .instruction(earlier)
                        .result
                        .ok_or(Error::Limit)?;
                    if self.replacements[previous.index()].is_none() && self.same_phi(earlier, id) {
                        self.replacements[result.index()] = Some(Value::Temp(previous));
                        self.context.reused =
                            self.context.reused.checked_add(1).ok_or(Error::Limit)?;
                        self.changed = true;
                        break;
                    }
                }
            }
            chains.entry(key).or_default().push(id);
        }
        Ok(())
    }
    pub(super) fn edge_equality(
        &mut self,
        from: BlockId,
        to: BlockId,
    ) -> Result<Option<(Value, Value)>, Error> {
        let Some(&last) = self.context.body.block(from).instructions.last() else {
            return Ok(None);
        };
        let I::Branch(condition, yes, no) = self.context.body.instruction(last).kind else {
            return Ok(None);
        };
        if yes == no || yes != to && no != to {
            return Ok(None);
        }
        let taken = yes == to;
        let Some((_, def)) = producer(self.context.body, condition) else {
            return Ok(None);
        };
        let I::Compare(pred, ty, mut x, mut y) = def.kind else {
            return Ok(None);
        };
        if ty.float_rank() != 0 {
            return Ok(None);
        }
        if taken && pred == Compare::Eq || !taken && pred == Compare::Ne {
            if matches!(x, Value::Integer(_) | Value::Float(..)) {
                std::mem::swap(&mut x, &mut y);
            }
            return Ok(Some((x, y)));
        }
        if ty.bits() == 0 {
            return Ok(None);
        }
        let Some(c) = fold::literal(self.context.body, y) else {
            return Ok(None);
        };
        let bits = if !taken && pred == Compare::Ult && fold::all_ones(ty, c) {
            u128::MAX
        } else if !taken && pred == Compare::Ugt && fold::is_integer(ty, c, 0)
            || taken && pred == Compare::Ule && fold::is_integer(ty, c, 0)
            || taken && pred == Compare::Ult && fold::is_integer(ty, c, 1)
        {
            0
        } else {
            return Ok(None);
        };
        y = self.context.body.constant(fold::integer(ty, bits))?;
        Ok(Some((x, y)))
    }
    fn edge_facts(&mut self, block: BlockId) -> Result<(), Error> {
        let cfg = self.context.cfg()?;
        let preds = cfg.predecessors(block);
        let Some(parent) = cfg.dominator(block) else {
            return Ok(());
        };
        if preds != [parent] {
            return Ok(());
        }
        let Some(&last) = self.context.body.block(parent).instructions.last() else {
            return Ok(());
        };
        let I::Branch(condition, yes, no) = self.context.body.instruction(last).kind else {
            return Ok(());
        };
        if yes == no {
            return Ok(());
        }
        let taken = yes == block;
        if taken {
            self.mark_nonzero(condition);
        }
        if let Some((Value::Temp(x), y @ (Value::Integer(_) | Value::Float(..)))) =
            self.edge_equality(parent, block)?
            && self.equal[x.index()].is_none()
        {
            self.equal[x.index()] = Some(y);
            self.equal_undo.push(x);
        }
        if let Some((_, def)) = producer(self.context.body, condition)
            && let I::Compare(pred, ty, x, y) = def.kind
            && ty.bits() != 0
            && fold::literal(self.context.body, y).is_some_and(|v| fold::is_integer(ty, v, 0))
            && ((taken && pred == Compare::Ne) || (!taken && pred == Compare::Eq))
        {
            self.mark_nonzero(x);
        }
        Ok(())
    }
    fn mark_nonzero(&mut self, value: Value) {
        if let Value::Temp(t) = value
            && !self.nonzero[t.index()]
        {
            self.nonzero[t.index()] = true;
            self.nonzero_undo.push(t);
        }
    }
    fn select_identity(
        &mut self,
        inst: &Instruction,
        block: BlockId,
    ) -> Result<Option<Value>, Error> {
        let I::Phi(ty, pairs) = &inst.kind else {
            return Ok(None);
        };
        if pairs.len() != 2 {
            return Ok(None);
        }
        let equal = |body: &Body, a, b| {
            a == b
                || fold::literal(body, a)
                    .zip(fold::literal(body, b))
                    .is_some_and(|(a, b)| fold::same(*ty, a, b))
        };
        // Copy only this two-input phi: edge_equality can append a literal.
        let pairs = [pairs[0], pairs[1]];
        for &(_, u) in &pairs {
            if let Value::Temp(t) = u {
                if inst.result == Some(t) {
                    continue;
                }
                if !self.context.body.temp_is_parameter(t)
                    && let Some(def) = self.context.body.temp(t).definition
                    && def.index() < self.context.block_map().len()
                {
                    let available = self.context.block_map()[def.index()].is_some_and(|at| {
                        self.context.cfg().is_ok_and(|cfg| cfg.dominates(at, block))
                    }) && self.available_in(t, block);
                    if !available {
                        continue;
                    }
                }
            }
            let mut all = true;
            for &(pred, v) in &pairs {
                if equal(self.context.body, v, u) {
                    continue;
                }
                let arm = self.context.body.block(pred);
                let Some(&last) = arm.instructions.last() else {
                    all = false;
                    break;
                };
                let (from, to) =
                    if matches!(self.context.body.instruction(last).kind, I::Branch(..)) {
                        (Some(pred), block)
                    } else if arm.instructions.len() == 1
                        && matches!(self.context.body.instruction(last).kind, I::Jump(_))
                        && self.context.cfg()?.predecessors(pred).len() == 1
                    {
                        (
                            self.context.cfg()?.predecessors(pred).first().copied(),
                            pred,
                        )
                    } else {
                        (None, pred)
                    };
                let Some(from) = from else {
                    all = false;
                    break;
                };
                let Some((x, y)) = self.edge_equality(from, to)? else {
                    all = false;
                    break;
                };
                if !(equal(self.context.body, v, x) && equal(self.context.body, u, y)
                    || equal(self.context.body, v, y) && equal(self.context.body, u, x))
                {
                    all = false;
                    break;
                }
            }
            if all {
                return Ok(Some(u));
            }
        }
        Ok(None)
    }
    fn algebra(&mut self) -> Algebra<'_> {
        Algebra {
            program: self.context.program,
            body: self.context.body,
            unwritten: self.context.unwritten,
            uses: &mut self.uses,
            nonzero: &self.nonzero,
            changed: false,
        }
    }
    fn process_block(&mut self, block: BlockId) -> Result<(), Error> {
        self.walked[block.index()] = true;
        self.prepare_phis(block)?;
        let ids = self.context.body.block(block).instructions.clone();
        let mut kept = Vec::with_capacity(ids.len());
        for id in ids {
            let instruction = self.context.body.instruction(id);
            if matches!(instruction.kind, I::Phi(..))
                && instruction
                    .result
                    .is_some_and(|r| self.replacements[r.index()].is_some())
            {
                self.alive[id.index()] = false;
                continue;
            }
            self.substitute(id)?;
            if let Some(sequence) = memory_calls::equality_memcmp(
                self.context.program,
                self.context.body,
                self.context.level,
                block,
                id,
                &self.uses,
                &self.alive,
            )? {
                kept.extend(sequence);
                self.changed = true;
                let temps = self.context.body.temps().len();
                self.replacements.resize(temps, None);
                self.uses.resize(temps, 0);
                self.nonzero.resize(temps, false);
                self.equal.resize(temps, None);
                self.alive
                    .resize(self.context.body.instructions().len(), true);
                self.substitute(id)?;
            }
            let mut instruction = self.context.body.instruction(id).clone();
            {
                let edited = {
                    let mut algebra = self.algebra();
                    algebra.simplify_branch(&mut instruction.kind);
                    algebra.changed
                };
                self.changed |= edited;
            }
            algebra::canonicalize(&mut instruction.kind);
            let transformed = {
                let mut a = self.algebra();
                a.reassociate(&mut instruction.kind)?
                    || a.mask_or(&mut instruction.kind)
                    || a.merge_index(&mut instruction.kind)?
                    || a.narrow_compare(&mut instruction.kind)?
                    || a.exact_compare(&mut instruction.kind)?
            };
            let transformed = transformed
                || memory_calls::bulk(
                    self.context.program,
                    self.context.body,
                    &mut instruction,
                    &mut self.replacements,
                );
            let transformed = transformed
                || self
                    .algebra()
                    .readonly_table_address(&mut instruction.kind)?;
            self.changed |= transformed;
            let mut value = None;
            if instruction.result.is_some() {
                let (folded, edited) = {
                    let mut a = self.algebra();
                    let mut value = a.fold(&instruction.kind)?;
                    if value.is_none() {
                        value = a.identity(&mut instruction.kind)?;
                    }
                    (value, a.changed)
                };
                value = folded;
                self.changed |= edited;
                if value.is_none() && matches!(instruction.kind, I::Phi(..)) {
                    value = self.phi_value(&instruction);
                    if value.is_none() {
                        value = self.select_identity(&instruction, block)?;
                    }
                }
            }
            let result = instruction.result;
            let key = if result.is_some()
                && pure(&instruction.kind)
                && !matches!(instruction.kind, I::Phi(..) | I::Const(..))
            {
                self.expression(&instruction.kind)
            } else {
                None
            };
            self.context.body.update_instruction(id, instruction);
            if let (Some(result), Some(value)) = (result, value) {
                self.replacements[result.index()] = Some(value);
                self.alive[id.index()] = false;
                self.context.folded = self.context.folded.checked_add(1).ok_or(Error::Limit)?;
                self.changed = true;
                continue;
            }
            if let (Some(result), Some(key)) = (result, key) {
                if let Some(found) = self.table.get(&key).copied()
                    && self.context.cfg()?.dominates(found.block, block)
                {
                    if self.handler_reached[block.index()]
                        && self.first_opener[found.block.index()]
                            .is_some_and(|first| found.ordinal >= first)
                    {
                        kept.push(id);
                        continue;
                    }
                    self.replacements[result.index()] = Some(Value::Temp(found.temp));
                    self.alive[id.index()] = false;
                    self.context.reused = self.context.reused.checked_add(1).ok_or(Error::Limit)?;
                    self.changed = true;
                    continue;
                }
                self.table.insert(
                    key,
                    Available {
                        temp: result,
                        block,
                        ordinal: self.ordinal[id.index()],
                    },
                );
            }
            kept.push(id);
        }
        self.context.body.replace_links(block, kept)?;
        Ok(())
    }
    pub(super) fn finish(&mut self, full: bool) -> Result<(), Error> {
        let any =
            self.replacements.iter().any(Option::is_some) || self.equal.iter().any(Option::is_some);
        for index in 0..self.context.body.layout.len() {
            let block = self.context.body.layout[index];
            let walked = !full && self.walked[block.index()];
            let ids = self.context.body.block(block).instructions.clone();
            for id in ids {
                let kind = &self.context.body.instruction(id).kind;
                if walked && !matches!(kind, I::Phi(..)) {
                    break;
                }
                if any || matches!(kind, I::Call(_)) {
                    self.substitute(id)?;
                }
            }
        }
        Ok(())
    }
    fn run(&mut self) -> Result<bool, Error> {
        let Some(&entry) = self.context.cfg()?.reverse_postorder().first() else {
            return Ok(false);
        };
        self.index_openers()?;
        let mut stack = vec![(entry, 0_usize, 0_usize, 0_usize)];
        self.process_block(entry)?;
        while let Some(&(block, next, nonzero, equal)) = stack.last() {
            if let Some(&child) = self.context.cfg()?.children(block).get(next) {
                stack.last_mut().ok_or(Error::Limit)?.1 += 1;
                stack.push((child, 0, self.nonzero_undo.len(), self.equal_undo.len()));
                self.edge_facts(child)?;
                self.process_block(child)?;
            } else {
                for t in self.equal_undo.drain(equal..) {
                    self.equal[t.index()] = None;
                }
                for t in self.nonzero_undo.drain(nonzero..) {
                    self.nonzero[t.index()] = false;
                }
                stack.pop();
            }
        }
        self.finish(false)?;
        if self.context.level >= 3 && super::diamond_reuse::merge(self)? {
            self.changed = true;
            self.finish(true)?;
        }
        Ok(self.changed)
    }
}
pub(super) fn simplify(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg()?;
    context.prepare_uses(Pass::SimplifyValues, true)?;
    context.prepare_block_map(Pass::SimplifyValues, true);
    let (changed, edited) = {
        let mut simplifier = Simplifier::new(context);
        let changed = simplifier.run();
        let edited = simplifier.edited;
        simplifier.recycle();
        (changed?, edited)
    };
    if !changed && edited {
        context.invalidate_uses();
    }
    if changed {
        definition_order::restore(context)?;
    }
    if changed || edited {
        context.body.recount_uses()?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simplify_text(source: &str, level: u8) -> (String, bool, u64, u64) {
        let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
        let symbol = program
            .publications()
            .iter()
            .copied()
            .find(|&symbol| {
                program
                    .function(program.symbol(symbol))
                    .is_some_and(|function| function.body.is_some())
            })
            .unwrap();
        let unwritten = super::super::global_contents::find_unwritten(&program);
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let (changed, folded, reused) = {
            let mut context = Context::new(&mut program, symbol, &mut body, level, &[], &unwritten);
            let changed = context.run(Pass::SimplifyValues).unwrap();
            (changed, context.folded, context.reused)
        };
        body.compact().unwrap();
        program.restore_body(symbol, body).unwrap();
        let mut text = Vec::new();
        crate::lowir::writer::write(&program, &mut text).unwrap();
        (String::from_utf8(text).unwrap(), changed, folded, reused)
    }

    #[test]
    fn reuse_preserves_dominance_when_layout_visits_the_use_first() {
        let (text, changed, folded, reused) = simplify_text(
            r"
function @f(%a : i64, %b : i64) -> i64 {
block ^entry: jump ^first
block ^later: %y = binary sub i64 %a, %b return i64 %y
block ^first: %x = binary sub nowrap i64 %a, %b jump ^later
}",
            1,
        );
        assert!(changed);
        assert_eq!((folded, reused), (0, 1));
        assert!(text.contains("return i64 %x"));
        assert!(text.find("block ^first").unwrap() < text.find("block ^later").unwrap());
    }

    #[test]
    fn handler_reuse_requires_computation_before_the_first_opener() {
        for before in [false, true] {
            let prefix = if before {
                "%x = binary add i64 %a, %b"
            } else {
                ""
            };
            let suffix = if before {
                ""
            } else {
                "%x = binary add i64 %a, %b"
            };
            let source = format!(
                "declare function @g() -> void\nfunction @f(%a : i64, %b : i64) -> i64 {{block ^entry: {prefix} eh_try ^handler {suffix} call void @g() eh_end return i64 %x block ^handler: %y = binary add i64 %a, %b eh_end return i64 %y}}"
            );
            let (text, _, _, reused) = simplify_text(&source, 1);
            assert_eq!(reused, u64::from(before));
            assert_eq!(text.contains("%y = binary"), !before);
        }
    }

    #[test]
    fn sole_edge_equalities_do_not_change_the_report_or_escape_the_scope() {
        let (text, changed, folded, reused) = simplify_text(
            r"
function @f(%v : i64) -> i64 {
block ^entry: %test = cmp eq i64 %v, 7 branch %test, ^yes, ^no
block ^yes: return i64 %v
block ^no: return i64 %v
}",
            1,
        );
        assert!(!changed);
        assert_eq!((folded, reused), (0, 0));
        assert!(text.contains("return i64 7"));
        assert!(text.contains("return i64 %v"));
    }

    #[test]
    fn a_nonzero_edge_proves_the_unsigned_decrement_cannot_wrap() {
        let (text, _, folded, _) = simplify_text(
            r"
function @f(%v : i64) -> i64 {
block ^entry: %test = cmp ne i64 %v, 0 branch %test, ^yes, ^no
block ^yes: %d = binary sub i64 %v, 1 %r = cmp ult i64 %d, %v return i64 %r
block ^no: return i64 0
}",
            1,
        );
        assert_eq!(folded, 1);
        assert!(text.contains("return i64 1"));
    }

    #[test]
    fn duplicate_self_phis_are_reused_before_single_value_folding() {
        let (_, changed, folded, reused) = simplify_text(
            r"
function @f(%v : i64, %c : i64) -> i64 {
block ^entry: jump ^loop
block ^loop: %a = phi i64 [^entry: %v, ^loop: %a]
%b = phi i64 [^loop: %b, ^entry: %v]
branch %c, ^loop, ^exit
block ^exit: return i64 %b
}",
            1,
        );
        assert!(changed);
        assert_eq!((folded, reused), (1, 1));
    }

    #[test]
    fn unique_integer_load_retypes_only_at_equal_storage_width() {
        let (text, _, folded, _) = simplify_text(
            "function @f(%p : ptr) -> u8 {block ^entry: %a = load i8 %p %b = copy u8 %a return u8 %b}",
            1,
        );
        assert_eq!(folded, 1);
        assert!(text.contains("%a = load u8 %p"));
        let (text, _, folded, _) = simplify_text(
            "function @f(%p : ptr) -> u8 {block ^entry: %a = load i8 %p %b = copy u8 %a store i8 %a, %p return u8 %b}",
            1,
        );
        assert_eq!(folded, 0);
        assert!(text.contains("%a = load i8 %p"));
    }

    #[test]
    fn narrowed_comparisons_keep_unsigned_range_and_i1_bounds() {
        let (text, _, _, _) = simplify_text(
            "function @f(%v : u8) -> i64 {block ^entry: %w = convert zext i64 u8 %v %r = cmp lt i64 %w, 255 return i64 %r}",
            1,
        );
        assert!(text.contains("cmp ult u8 %v, 255"));
        let (text, _, _, _) = simplify_text(
            "function @f(%v : i1) -> i64 {block ^entry: %w = convert zext i64 i1 %v %r = cmp lt i64 %w, 2 return i64 %r}",
            1,
        );
        assert!(text.contains("cmp lt i64 %w, 2"));
    }

    #[test]
    fn exact_quotients_compare_original_pointer_dividends() {
        let (text, _, _, _) = simplify_text(
            "function @f(%v : ptr, %w : ptr) -> i64 {block ^entry: %a = binary div exact i64 %v, 8 %b = binary div exact i64 %w, 8 %r = cmp ne i64 %a, %b return i64 %r}",
            1,
        );
        assert!(text.contains("cmp ne ptr %v, %w"));
    }

    #[test]
    fn readonly_reads_cover_bytes_but_do_not_invent_padding() {
        let (text, _, folded, _) = simplify_text(
            "global @g [storage=readonly] = {u8 18 u8 52 zero 2}\nfunction @f() -> u16 {block ^entry: %r = load u16 @g return u16 %r}",
            1,
        );
        assert_eq!(folded, 1);
        assert!(text.contains("return u16 13330"));
        let (text, _, folded, _) = simplify_text(
            "global @g [storage=readonly] = {u8 18 i64 52}\nfunction @f() -> u16 {block ^entry: %r = load u16 @g return u16 %r}",
            1,
        );
        assert_eq!(folded, 0);
        assert!(text.contains("load u16 @g"));
    }

    #[test]
    fn bulk_calls_keep_the_frozen_size_limit_and_result_alias() {
        for size in [128, 129] {
            let source = format!(
                "declare function @memcpy(%d : ptr, %s : ptr, %n : i64) -> ptr [linkage=c]\nfunction @f(%a : ptr, %b : ptr) -> ptr {{block ^entry: %r = call ptr @memcpy(%a, %b, {size}) return ptr %r}}"
            );
            let (text, changed, _, _) = simplify_text(&source, 1);
            assert_eq!(changed, size == 128);
            assert_eq!(text.contains("copyobj"), size == 128);
            assert_eq!(text.contains("return ptr %a"), size == 128);
        }
    }

    #[test]
    fn memcmp_equality_loads_overlapping_chunks_only_from_o2() {
        let source = "declare function @memcmp(%a : ptr, %b : ptr, %n : i64) -> i32 [linkage=c]\nfunction @f(%a : ptr, %b : ptr) -> i64 {block ^entry: %r = call i32 @memcmp(%a, %b, 9) %c = cmp eq i32 %r, 0 return i64 %c}";
        let (o1, changed, _, _) = simplify_text(source, 1);
        assert!(!changed);
        assert!(o1.contains("call i32 @memcmp"));
        let (o2, changed, _, _) = simplify_text(source, 2);
        assert!(changed);
        assert!(!o2.contains("call i32 @memcmp"));
        assert_eq!(o2.matches("load i64").count(), 4);
        assert_eq!(o2.matches("index i8").count(), 2);
    }
    #[test]
    fn consecutive_diamonds_reuse_quiet_loads_only_at_o3() {
        let source = "function @f(%c : i64, %p : ptr) -> i64 {block ^entry: branch %c, ^a, ^b block ^a: %a0 = load i64 %p jump ^m block ^b: %b0 = load i64 %p jump ^m block ^m: %first = phi i64 [^a: %a0, ^b: %b0] branch %c, ^d, ^e block ^d: %d0 = load i64 %p jump ^z block ^e: %e0 = load i64 %p jump ^z block ^z: %second = phi i64 [^d: %d0, ^e: %e0] %r = binary add i64 %first, %second return i64 %r}";
        let (_, changed, _, _) = simplify_text(source, 2);
        assert!(!changed);
        let (text, changed, folded, reused) = simplify_text(source, 3);
        assert!(changed);
        assert_eq!((folded, reused), (0, 0));
        assert!(text.contains("binary add i64 %first, %first"));
        let source = source.replace("branch %c, ^d", "store i64 9, %p branch %c, ^d");
        let (_, changed, _, _) = simplify_text(&source, 3);
        assert!(!changed);
    }
    #[test]
    fn o3_preserves_the_frozen_symbol_index_alias_in_diamond_proofs() {
        let (text, _, _, _) = simplify_text(
            r"
declare global @dummy0 : i64
declare global @dummy1 : i64
declare global @dummy2 : i64
declare global @g : i64
declare global @h : i64
function @f(%c : i64) -> ptr {
block ^entry:
%pad1 = binary add i64 %c, 7
%pad2 = binary add i64 %c, 8
%pad3 = binary sub i64 %c, 9
%pad4 = binary sub i64 %c, 9
branch %c, ^a, ^b
block ^a: jump ^m
block ^b: jump ^m
block ^m: %first = phi ptr [^a: @g, ^b: @h] branch %c, ^d, ^e
block ^d: jump ^z
block ^e: jump ^z
block ^z: %second = phi ptr [^d: @h, ^e: @g] return ptr %second
}",
            3,
        );
        assert!(text.contains("return ptr %first"));
    }
}

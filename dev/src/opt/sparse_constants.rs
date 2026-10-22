//! Sparse executable-edge constant propagation from `opt/constant_propagation.cpp`.
//! See NOTICE.
use super::{
    control_flow::Cfg,
    numeric_fold as fold,
    pass_context::{Context, Pass},
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{BlockId, Body, GlobalInit, InstId, Instruction, InstructionKind as I, TempId, Value},
    vocabulary::{Binding, Storage, Type},
};

#[derive(Clone, Copy)]
enum State {
    Unknown,
    Known(Constant),
    Overdefined,
}
impl State {
    fn constant(self) -> Option<Constant> {
        if let Self::Known(value) = self {
            Some(value)
        } else {
            None
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct EdgeId(u32);
impl EdgeId {
    fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy)]
struct PhiInput {
    instruction: InstId,
    index: usize,
}
#[derive(Clone, Copy)]
struct User {
    instruction: InstId,
    input: Option<PhiInput>,
}
struct Users {
    first: Vec<usize>,
    entries: Vec<User>,
}
impl Users {
    fn build(body: &Body) -> Result<Self, Error> {
        let mut first = vec![0_usize; body.temps().len() + 1];
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                body.visit_values(&body.instruction(id).kind, |value| {
                    if let Value::Temp(temp) = value {
                        first[temp.index() + 1] += 1;
                    }
                });
            }
        }
        for t in 0..body.temps().len() {
            first[t + 1] = first[t + 1].checked_add(first[t]).ok_or(Error::Limit)?;
        }
        let mut entries = vec![None; first[body.temps().len()]];
        let mut fill = first[..body.temps().len()].to_vec();
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                let kind = &body.instruction(id).kind;
                let phi = matches!(kind, I::Phi(..));
                let mut index = 0;
                body.visit_values(kind, |value| {
                    if let Value::Temp(temp) = value {
                        entries[fill[temp.index()]] = Some(User {
                            instruction: id,
                            input: phi.then_some(PhiInput {
                                instruction: id,
                                index,
                            }),
                        });
                        fill[temp.index()] += 1;
                    }
                    index += 1;
                });
            }
        }
        Ok(Self {
            first,
            entries: entries
                .into_iter()
                .map(|entry| entry.ok_or(Error::Limit))
                .collect::<Result<_, _>>()?,
        })
    }
}
struct PhiEdges {
    successor_slots: Vec<EdgeId>,
    label_first: Vec<usize>,
    labels: Vec<Option<EdgeId>>,
    first: Vec<usize>,
    phis: Vec<InstId>,
    row_first: Vec<usize>,
    inputs: Vec<Option<PhiInput>>,
}
impl PhiEdges {
    fn build(body: &Body, cfg: &Cfg) -> Result<Self, Error> {
        let (succ_first, succs) = cfg.successor_edges();
        let (pred_first, preds) = cfg.predecessor_edges();
        let mut fill = pred_first[..body.blocks().len()].to_vec();
        let mut successor_slots = Vec::with_capacity(succs.len());
        for &target in succs {
            successor_slots.push(EdgeId(
                u32::try_from(fill[target.index()]).map_err(|_| Error::Limit)?,
            ));
            fill[target.index()] += 1;
        }
        // Successors are stored in numeric source-block order, matching preds.
        if successor_slots.len() != succ_first[body.blocks().len()] {
            return Err(Error::Limit);
        }
        let mut out = Self {
            successor_slots,
            label_first: vec![0; body.instructions().len() + 1],
            labels: Vec::new(),
            first: vec![0; body.blocks().len() + 1],
            phis: Vec::new(),
            row_first: vec![0; preds.len() + 1],
            inputs: Vec::new(),
        };
        let present = body.layout.iter().any(|&b| {
            body.block(b)
                .instructions
                .first()
                .is_some_and(|&i| matches!(body.instruction(i).kind, I::Phi(..)))
        });
        if !present {
            return Ok(out);
        }
        for (index, instruction) in body.instructions().iter().enumerate() {
            out.label_first[index] = out.labels.len();
            if let I::Phi(_, pairs) = &instruction.kind {
                out.labels.resize(out.labels.len() + pairs.len(), None);
            }
        }
        out.label_first[body.instructions().len()] = out.labels.len();
        let mut slot_of = vec![None; body.blocks().len()];
        for (index, block) in body.blocks().iter().enumerate() {
            out.first[index] = out.phis.len();
            if !block.placed {
                continue;
            }
            for &id in &block.instructions {
                if !matches!(body.instruction(id).kind, I::Phi(..)) {
                    break;
                }
                out.phis.push(id);
            }
            let count = out.phis.len() - out.first[index];
            for slot in pred_first[index]..pred_first[index + 1] {
                slot_of[preds[slot].index()] =
                    Some(EdgeId(u32::try_from(slot).map_err(|_| Error::Limit)?));
                out.row_first[slot] = out.inputs.len();
                out.inputs.resize(
                    out.inputs.len().checked_add(count).ok_or(Error::Limit)?,
                    None,
                );
            }
            for j in 0..count {
                let id = out.phis[out.first[index] + j];
                let I::Phi(_, pairs) = &body.instruction(id).kind else {
                    return Err(Error::Limit);
                };
                for (input, &(pred, _)) in pairs.iter().enumerate() {
                    let Some(slot) = slot_of.get(pred.index()).copied().flatten() else {
                        continue;
                    };
                    let slot = slot.index();
                    if slot < pred_first[index]
                        || slot >= pred_first[index + 1]
                        || preds[slot] != pred
                    {
                        continue;
                    }
                    out.labels[out.label_first[id.index()] + input] =
                        Some(EdgeId(u32::try_from(slot).map_err(|_| Error::Limit)?));
                    let cell = &mut out.inputs[out.row_first[slot] + j];
                    if cell.is_none() {
                        *cell = Some(PhiInput {
                            instruction: id,
                            index: input,
                        });
                    }
                }
            }
        }
        out.first[body.blocks().len()] = out.phis.len();
        out.row_first[preds.len()] = out.inputs.len();
        Ok(out)
    }
    fn label(&self, input: PhiInput) -> Option<EdgeId> {
        self.labels
            .get(self.label_first[input.instruction.index()] + input.index)
            .copied()
            .flatten()
    }
}
struct Propagator<'a, 'p> {
    context: &'a Context<'p>,
    cfg: &'a Cfg,
    states: Vec<State>,
    executable: Vec<bool>,
    edges: Vec<bool>,
    phis: PhiEdges,
    users: Users,
    block_work: Vec<BlockId>,
    temp_work: Vec<TempId>,
}
impl<'a, 'p> Propagator<'a, 'p> {
    fn new(context: &'a Context<'p>) -> Result<Self, Error> {
        let body = &*context.body;
        let cfg = context.cfg()?;
        let mut states = vec![State::Unknown; body.temps().len()];
        states[..body.parameter_count].fill(State::Overdefined);
        let entry = cfg.reverse_postorder().first().copied();
        let mut executable = vec![false; body.blocks().len()];
        if let Some(entry) = entry {
            executable[entry.index()] = true;
        }
        Ok(Self {
            context,
            cfg,
            states,
            executable,
            edges: vec![false; cfg.predecessor_edges().1.len()],
            phis: PhiEdges::build(body, cfg)?,
            users: Users::build(body)?,
            block_work: entry.into_iter().collect(),
            temp_work: Vec::new(),
        })
    }
    fn state(&self, value: Value) -> State {
        let body = &*self.context.body;
        match value {
            Value::Integer(id) => State::Known(Constant::Integer(body.literal(id))),
            Value::Float(id, kind) => State::Known(Constant::Float(kind, body.literal(id))),
            Value::Temp(id) => self.states[id.index()],
            _ => State::Overdefined,
        }
    }
    fn lower(&mut self, temp: TempId, to: State) -> Result<(), Error> {
        let from = self.states[temp.index()];
        let to = match (from, to) {
            (State::Overdefined, _) | (_, State::Unknown) => return Ok(()),
            (State::Known(a), State::Known(b)) => {
                let ty = self.context.body.temp(temp).ty.ok_or(Error::Limit)?;
                if fold::same(ty, a, b) {
                    return Ok(());
                }
                State::Overdefined
            }
            (_, to) => to,
        };
        self.states[temp.index()] = to;
        self.temp_work.push(temp);
        Ok(())
    }
    fn meet(&mut self, phi: TempId, value: Value) -> Result<(), Error> {
        if !matches!(self.states[phi.index()], State::Overdefined) {
            self.lower(phi, self.state(value))?;
        }
        Ok(())
    }
    fn readonly(&self, instruction: &Instruction) -> Option<Constant> {
        let I::Load(facts, ty, Value::Symbol(id)) = instruction.kind else {
            return None;
        };
        let symbol = self.context.program.symbol(id);
        let global = self.context.program.global(symbol)?;
        if facts.volatile()
            || global.metadata.binding != Some(Binding::Internal)
            || global.metadata.flags & 64 != 0
            || global.ty != Some(ty)
            || ty.bits() == 0
            || ty == Type::I128
            || !(global.metadata.storage == Some(Storage::ReadOnly)
                || self
                    .context
                    .unwritten
                    .get(id.index())
                    .copied()
                    .unwrap_or(false))
        {
            return None;
        }
        match global.init {
            GlobalInit::Zero => Some(fold::integer(ty, 0)),
            GlobalInit::Constant(Constant::Integer(bits)) => Some(fold::integer(ty, bits)),
            _ => None,
        }
    }
    fn evaluate(&mut self, id: InstId) -> Result<(), Error> {
        let instruction = self.context.body.instruction(id);
        if matches!(instruction.kind, I::Phi(..)) {
            return self.evaluate_phi(id);
        }
        let Some(result) = instruction.result else {
            if instruction.kind.terminator() {
                self.terminator(
                    self.context.block_map()[id.index()].ok_or(Error::Limit)?,
                    instruction,
                )?;
            }
            return Ok(());
        };
        let ready = State::Known(Constant::Integer(0));
        let (a, b, value) = match &instruction.kind {
            I::Const(ty, value) | I::Copy(ty, value) => {
                let a = self.state(*value);
                (a, ready, a.constant().and_then(|v| fold::copy(*ty, v)))
            }
            I::Unary(op, ty, value) => {
                let a = self.state(*value);
                (
                    a,
                    ready,
                    a.constant().and_then(|v| fold::unary(*op, *ty, v)),
                )
            }
            I::Binary(op, _, ty, a, b) => {
                let (a, b) = (self.state(*a), self.state(*b));
                (
                    a,
                    b,
                    a.constant()
                        .zip(b.constant())
                        .and_then(|(a, b)| fold::binary(*op, *ty, a, b)),
                )
            }
            I::Compare(op, ty, a, b) => {
                let (a, b) = (self.state(*a), self.state(*b));
                (
                    a,
                    b,
                    a.constant()
                        .zip(b.constant())
                        .and_then(|(a, b)| fold::compare(*op, *ty, a, b)),
                )
            }
            I::Convert(op, to, from, value) => {
                let a = self.state(*value);
                (
                    a,
                    ready,
                    a.constant().and_then(|v| fold::convert(*op, *to, *from, v)),
                )
            }
            I::Select(ty, condition, yes, no) => {
                let state = match self.state(*condition) {
                    State::Unknown => State::Unknown,
                    State::Known(Constant::Integer(bits)) => {
                        self.state(if bits != 0 { *yes } else { *no })
                    }
                    _ => State::Overdefined,
                };
                (
                    state,
                    ready,
                    state.constant().and_then(|v| fold::copy(*ty, v)),
                )
            }
            I::Load(..) => (State::Overdefined, ready, self.readonly(instruction)),
            _ => (State::Overdefined, ready, None),
        };
        if let Some(value) = value {
            self.lower(result, State::Known(value))?;
        } else if !matches!(a, State::Unknown) && !matches!(b, State::Unknown) {
            self.lower(result, State::Overdefined)?;
        }
        Ok(())
    }
    fn evaluate_phi(&mut self, id: InstId) -> Result<(), Error> {
        let instruction = self.context.body.instruction(id);
        let result = instruction.result.ok_or(Error::Limit)?;
        if matches!(self.states[result.index()], State::Overdefined) {
            return Ok(());
        }
        let I::Phi(ty, pairs) = &instruction.kind else {
            return Err(Error::Limit);
        };
        let mut state = State::Unknown;
        for (index, &(_, value)) in pairs.iter().enumerate() {
            if self
                .phis
                .label(PhiInput {
                    instruction: id,
                    index,
                })
                .is_none_or(|edge| !self.edges[edge.index()])
            {
                continue;
            }
            match (state, self.state(value)) {
                (_, State::Overdefined) => {
                    state = State::Overdefined;
                    break;
                }
                (State::Known(a), State::Known(b)) if !fold::same(*ty, a, b) => {
                    state = State::Overdefined;
                    break;
                }
                (State::Unknown, State::Known(value)) => state = State::Known(value),
                _ => (),
            }
        }
        self.lower(result, state)
    }
    fn mark_edge(&mut self, block: BlockId, target: BlockId) -> Result<(), Error> {
        let (first, targets) = self.cfg.successor_edges();
        for (k, &successor) in targets
            .iter()
            .enumerate()
            .take(first[block.index() + 1])
            .skip(first[block.index()])
        {
            if successor == target {
                return self.mark_successor(k);
            }
        }
        Ok(())
    }
    fn mark_successor(&mut self, k: usize) -> Result<(), Error> {
        let target = self.cfg.successor_edges().1[k];
        let slot = self.phis.successor_slots[k].index();
        if std::mem::replace(&mut self.edges[slot], true) {
            return Ok(());
        }
        if !std::mem::replace(&mut self.executable[target.index()], true) {
            self.block_work.push(target);
            return Ok(());
        }
        let count = self.phis.first[target.index() + 1] - self.phis.first[target.index()];
        for j in 0..count {
            if let Some(input) = self.phis.inputs[self.phis.row_first[slot] + j] {
                let instruction = self.context.body.instruction(input.instruction);
                let I::Phi(_, pairs) = &instruction.kind else {
                    return Err(Error::Limit);
                };
                self.meet(
                    instruction.result.ok_or(Error::Limit)?,
                    pairs[input.index].1,
                )?;
            }
        }
        Ok(())
    }
    fn terminator(&mut self, block: BlockId, instruction: &Instruction) -> Result<(), Error> {
        match &instruction.kind {
            I::Jump(target) => self.mark_edge(block, *target)?,
            I::Branch(condition, yes, no) => match self.state(*condition) {
                State::Unknown => (),
                State::Known(Constant::Integer(bits)) => {
                    self.mark_edge(block, if bits != 0 { *yes } else { *no })?;
                }
                _ => {
                    self.mark_edge(block, *yes)?;
                    self.mark_edge(block, *no)?;
                }
            },
            I::Switch(value, default, cases) => match self.state(*value) {
                State::Unknown => (),
                State::Known(c @ Constant::Integer(_)) => {
                    let ty = instruction
                        .switch_type()
                        .or_else(|| {
                            if let Value::Temp(temp) = value {
                                self.context.body.temp(*temp).ty
                            } else {
                                None
                            }
                        })
                        .unwrap_or(Type::I64);
                    let mut target = *default;
                    for &(label, block) in cases {
                        if let State::Known(label) = self.state(label)
                            && fold::same(ty, label, c)
                        {
                            target = block;
                            break;
                        }
                    }
                    self.mark_edge(block, target)?;
                }
                _ => {
                    let (first, _) = self.cfg.successor_edges();
                    for k in first[block.index()]..first[block.index() + 1] {
                        self.mark_successor(k)?;
                    }
                }
            },
            _ => (),
        }
        Ok(())
    }
    fn visit(&mut self, block: BlockId) -> Result<(), Error> {
        for &id in &self.context.body.block(block).instructions {
            self.evaluate(id)?;
            let kind = &self.context.body.instruction(id).kind;
            if let I::Runtime(runtime) = kind
                && let Some(target) = runtime.handler_target()
                && !std::mem::replace(&mut self.executable[target.index()], true)
            {
                self.block_work.push(target);
            }
            if matches!(kind, I::Call(_)) && self.context.program.may_end_block(kind) {
                break;
            }
        }
        Ok(())
    }
    fn visit_user(&mut self, user: User, temp: TempId) -> Result<(), Error> {
        let Some(block) = self.context.block_map()[user.instruction.index()] else {
            return Ok(());
        };
        if !self.executable[block.index()] {
            return Ok(());
        }
        if let Some(input) = user.input {
            if self
                .phis
                .label(input)
                .is_some_and(|slot| self.edges[slot.index()])
            {
                let result = self
                    .context
                    .body
                    .instruction(user.instruction)
                    .result
                    .ok_or(Error::Limit)?;
                self.meet(result, Value::Temp(temp))?;
            }
        } else {
            let instruction = self.context.body.instruction(user.instruction);
            if instruction.result.map_or_else(
                || instruction.kind.terminator(),
                |r| !matches!(self.states[r.index()], State::Overdefined),
            ) {
                self.evaluate(user.instruction)?;
            }
        }
        Ok(())
    }
    fn run(mut self) -> Result<Vec<Option<Constant>>, Error> {
        while !self.block_work.is_empty() || !self.temp_work.is_empty() {
            while let Some(block) = self.block_work.pop() {
                self.visit(block)?;
            }
            while let Some(temp) = self.temp_work.pop() {
                for k in self.users.first[temp.index()]..self.users.first[temp.index() + 1] {
                    self.visit_user(self.users.entries[k], temp)?;
                }
            }
        }
        Ok(self.states.into_iter().map(State::constant).collect())
    }
}
pub(super) fn propagate(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg()?;
    if context.cfg()?.reverse_postorder().is_empty() {
        return Ok(false);
    }
    context.prepare_block_map(Pass::Constants, true);
    let values = Propagator::new(context)?.run()?;
    let count = context.body.substitute_constants(&values)?;
    context.propagated = context.propagated.checked_add(count).ok_or(Error::Limit)?;
    Ok(count != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        model::{Program, SymbolId},
        writer,
    };
    fn run(source: &str) -> (Program, bool, u64) {
        let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
        let unwritten = super::super::global_contents::find_unwritten(&program);
        let symbol = program
            .publications()
            .iter()
            .copied()
            .find(|&s| {
                program
                    .function(program.symbol(s))
                    .is_some_and(|f| f.body.is_some())
            })
            .unwrap();
        let mut body = program.take_body(symbol).unwrap().unwrap();
        body.record_switch_types();
        let (changed, propagated) = {
            let mut context = Context::new(&program, symbol, &mut body, 1, &[], &unwritten);
            let changed = context.run(Pass::Constants).unwrap();
            (changed, context.propagated)
        };
        program.restore_body(symbol, body).unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        (program, changed, propagated)
    }
    fn text(program: &Program) -> String {
        let mut out = Vec::new();
        writer::write(program, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }
    #[test]
    fn executable_edges_meet_phi_inputs_and_keep_a_nonexecuted_arm_opaque() {
        let (program, changed, count) = run(r"
function @f(%unknown : i64) -> i64 {
 block ^entry: %c = const i64 1 branch %c, ^yes, ^no
 block ^no: %dead = binary add i64 %unknown, 9 jump ^join
 block ^yes: %v = binary mul i64 3, 7 jump ^join
 block ^join: %p = phi i64 [^yes: %v, ^no: %dead] %r = binary add i64 %p, 1 return i64 %r
}");
        assert!(changed);
        assert_eq!(count, 4);
        let out = text(&program);
        assert!(out.contains("branch 1, ^yes, ^no"));
        assert!(out.contains("%p = phi i64 [^yes: 21, ^no: %dead]"));
        assert!(out.contains("return i64 22"));
    }
    #[test]
    fn a_later_different_constant_overdefines_the_phi_and_all_its_users() {
        let (program, changed, _) = run(r"
function @f(%unknown : i64) -> i64 {
 block ^entry: branch %unknown, ^a, ^b
 block ^a: %x = const i64 1 jump ^join
 block ^b: %y = const i64 2 jump ^join
 block ^join: %p = phi i64 [^a: %x, ^b: %y] %r = binary add i64 %p, 10 return i64 %r
}");
        assert!(changed);
        let out = text(&program);
        assert!(out.contains("[^a: 1, ^b: 2]"));
        assert!(out.contains("binary add i64 %p, 10") && out.contains("return i64 %r"));
    }
    #[test]
    fn readonly_and_unwritten_scalars_fold_but_volatile_and_replaceable_storage_stays() {
        let (program, _, _) = run(r"
global @r : i8 [binding=internal, storage=readonly] = 255
global @u : i64 [binding=internal] = 42
global @w : i64 [binding=weak, storage=readonly, replaceable=yes] = 55
function @f(%p : ptr) -> i64 {
 block ^entry:
  %r = load i8 @r
  %u = load i64 @u
  %v = load volatile i64 @u
  %w = load i64 @w
  store i8 %r, %p
  store i64 %v, %p
  store i64 %w, %p
  return i64 %u
}");
        let out = text(&program);
        assert!(out.contains("store i8 -1, %p"), "{out}");
        assert!(out.contains("return i64 42"));
        assert!(out.contains("store i64 %v, %p") && out.contains("store i64 %w, %p"));
    }
    #[test]
    fn handler_entries_execute_without_an_ordinary_edge_and_noreturn_stops_the_block() {
        let (program, changed, _) = run(r"
declare function @stop() -> void [return=noreturn]
function @f() -> i64 {
 block ^entry: eh_try ^handler call void @stop() %after = binary add i64 3, 4 eh_end return i64 %after
 block ^handler: eh_catch_all %v = binary add i64 5, 6 return i64 %v
}");
        assert!(changed);
        let out = text(&program);
        assert!(out.contains("return i64 %after"));
        assert!(out.contains("return i64 11"));
    }
    #[test]
    fn constants_substitute_call_arguments_but_never_a_call_callee() {
        let (program, changed, count) = run(r"
function @f() -> void {
 block ^entry: %callee = copy ptr 0 %v = const i64 23
 call void %callee(%v) as (%arg : i64) -> void [unwind=no]
 return void
}");
        assert!(changed);
        assert_eq!(count, 1);
        let out = text(&program);
        assert!(out.contains("call void %callee(23)"));
    }
    #[test]
    fn unused_constants_grow_the_literal_pool_once_and_keep_a_settled_false_result() {
        let mut program = crate::lowir::test_program(
            b"function @f() -> void {block ^entry: %x = const i64 5 return void}",
        )
        .unwrap();
        let symbol: SymbolId = program.publications()[0];
        let before = program.statistics().literals;
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let mut context = Context::new(&program, symbol, &mut body, 1, &[], &[]);
        assert!(!context.run(Pass::Constants).unwrap());
        assert!(!context.run(Pass::Constants).unwrap());
        assert_eq!(context.propagated, 0);
        program.restore_body(symbol, body).unwrap();
        assert_eq!(program.statistics().literals, before + 1);
    }
}

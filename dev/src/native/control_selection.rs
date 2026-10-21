//! O0 scalar terminators, ordered MIR blocks and phi edge transfers. See NOTICE.
use super::{
    condition::Condition,
    condition_mir::FlagOp,
    mir::{self, BlockName, Instruction, Op},
    parallel_transfer::{self, Transfer},
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    value_facts::{Alias, normalize},
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{BlockId, Body, InstId, InstructionKind as I, Value},
    vocabulary::Type,
};
pub(super) struct Control {
    pub(super) targets: Vec<Option<mir::BlockId>>,
    pub(super) edges: Vec<(BlockId, mir::BlockId)>,
    pub(super) edge_ranges: Vec<std::ops::Range<usize>>,
    cached: Vec<Option<(BlockId, mir::BlockId)>>,
    pub(super) entry_loop: bool,
}
impl Control {
    pub(super) fn new(body: &Body, facts: &super::value_facts::Facts) -> Result<Self, Error> {
        let entry_loop = !facts.flow.predecessors(body.layout[0]).is_empty();
        let mut next = u32::from(entry_loop);
        let mut out = Self {
            targets: vec![None; body.blocks().len()],
            edges: Vec::new(),
            edge_ranges: vec![0..0; body.blocks().len()],
            cached: vec![None; body.blocks().len()],
            entry_loop,
        };
        for &block in &body.layout {
            if !facts.flow.reachable[block.index()] {
                continue;
            }
            out.edge_ranges[block.index()].start = out.edges.len();
            out.targets[block.index()] = Some(mir::BlockId(next));
            next = next.checked_add(1).ok_or(Error::Limit)?;
            if body.block(block).instructions.last().is_some_and(|&id| {
                matches!(body.instruction(id).kind, I::Branch(..) | I::Switch(..))
            }) {
                for &target in facts.flow.successors(block) {
                    if body
                        .block(target)
                        .instructions
                        .first()
                        .is_some_and(|&id| matches!(body.instruction(id).kind, I::Phi(..)))
                    {
                        out.edges.push((target, mir::BlockId(next)));
                        next = next.checked_add(1).ok_or(Error::Limit)?;
                    }
                }
            }
            out.edge_ranges[block.index()].end = out.edges.len();
        }
        Ok(out)
    }
    pub(super) fn prepare(&mut self, from: BlockId) {
        for &(to, id) in &self.edges[self.edge_ranges[from.index()].clone()] {
            self.cached[to.index()] = Some((from, id));
        }
    }
    fn target(&self, from: BlockId, to: BlockId) -> Result<mir::BlockId, Error> {
        self.cached[to.index()]
            .and_then(|(owner, id)| (owner == from).then_some(id))
            .or(self.targets[to.index()])
            .ok_or(Error::Limit)
    }
}
impl Selection<'_> {
    pub(super) fn jump(&mut self, target: mir::BlockId) {
        self.mir.instructions.push(Instruction {
            op: Op::Jump(target),
            location: self.location,
        });
    }
    pub(super) fn branch(&mut self, condition: Condition, target: mir::BlockId) {
        self.mir.instructions.push(Instruction {
            op: Op::Branch(condition, target),
            location: self.location,
        });
    }
    pub(super) fn terminator(
        &mut self,
        control: &Control,
        from: BlockId,
        kind: &I,
    ) -> Result<(), Error> {
        match kind {
            I::Jump(to) => {
                self.phi_transfers(from, *to)?;
                self.jump(control.targets[to.index()].ok_or(Error::Limit)?);
            }
            I::Branch(value, yes, no) => {
                let yes = control.target(from, *yes)?;
                let no = control.target(from, *no)?;
                match self.facts.resolve(self.body, *value) {
                    Alias::Temporary(temp) if self.facts.fused[temp.index()] => {
                        if !self.float_branch(temp, yes, no)? {
                            let cc = self.fused_flags(temp)?;
                            self.branch(cc, yes);
                            self.jump(no);
                        }
                    }
                    Alias::Constant(Constant::Integer(bits)) => {
                        self.jump(if bits != 0 { yes } else { no });
                    }
                    Alias::Symbol(_) => self.jump(yes),
                    resolved => {
                        let ty = if let Alias::Temporary(temp) = resolved {
                            self.body.temp(temp).ty.ok_or(Error::Limit)?
                        } else {
                            Type::Ptr
                        };
                        super::memory::scalar(ty)?;
                        let source = self.operand(*value, ty, super::x86::Register::R10)?;
                        if let Operand::Immediate(bits) = source {
                            self.jump(if bits != 0 { yes } else { no });
                        } else {
                            self.emit(ScalarOp::Condition(FlagOp::Compare(
                                ty,
                                source,
                                Operand::Immediate(0),
                            )));
                            self.branch(Condition::NotEqual, yes);
                            self.jump(no);
                        }
                    }
                }
            }
            I::Switch(value, fallback, cases) => {
                let ty = if let Value::Temp(temp) = *value {
                    self.body.temp(temp).ty.ok_or(Error::Limit)?
                } else {
                    Type::I64
                };
                super::memory::scalar(ty)?;
                let mut source = self.operand(*value, ty, super::x86::Register::R10)?;
                if source.register().is_none() {
                    self.load(super::x86::Register::R10, source, ty);
                    source = Operand::Register(super::x86::Register::R10);
                }
                for &(value, to) in cases {
                    let mut case = self.operand(value, ty, super::x86::Register::R11)?;
                    if matches!(case, Operand::Symbol(_)) {
                        self.load(super::x86::Register::R11, case, Type::Ptr);
                        case = Operand::Register(super::x86::Register::R11);
                    }
                    self.emit(ScalarOp::Condition(FlagOp::Compare(ty, source, case)));
                    self.branch(Condition::Equal, control.target(from, to)?);
                }
                self.jump(control.target(from, *fallback)?);
            }
            _ => return Err(super::value_selection::unsupported()),
        }
        Ok(())
    }
    pub(super) fn edge_blocks(&mut self, control: &Control, from: BlockId) -> Result<(), Error> {
        for &(to, id) in &control.edges[control.edge_ranges[from.index()].clone()] {
            let created = self.mir.begin_block(BlockName::Edge(
                self.body.block(from).name,
                self.body.block(to).name,
            ))?;
            if created != id {
                return Err(Error::Construction("phi edge block out of order".into()));
            }
            self.phi_transfers(from, to)?;
            self.jump(control.targets[to.index()].ok_or(Error::Limit)?);
            self.mir.end_block();
        }
        Ok(())
    }
    fn phi_transfers(&mut self, from: BlockId, to: BlockId) -> Result<(), Error> {
        let mut moves = std::mem::take(&mut self.edge_moves);
        moves.clear();
        for &id in &self.body.block(to).instructions {
            let inst = self.body.instruction(id);
            let I::Phi(ty, _) = &inst.kind else {
                break;
            };
            let temp = inst.result.ok_or(Error::Limit)?;
            let pos = self.facts.flow.positions[id.index()];
            if !self.facts.active[pos] {
                continue;
            }
            if !matches!(ty, Type::F32 | Type::F64) {
                super::memory::scalar(*ty)?;
            }
            let Some(value) = self.phis.lookup(self.body, id, from)? else {
                continue;
            };
            let destination = if let Some(reg) = self.placement.globals.registers[temp.index()] {
                self.placement.note_clobbers(1 << reg.code());
                Operand::Register(reg)
            } else {
                Operand::Frame(self.placement.home(self.body, temp, &mut self.mir)?)
            };
            let (source, source_type, address) = match self.facts.resolve(self.body, value) {
                Alias::Temporary(source) => {
                    let loc = self.temp_operand(source)?;
                    let own = self.body.temp(source).ty.ok_or(Error::Limit)?;
                    (
                        loc,
                        (loc.is_memory()
                            && super::abi::value_bytes(own)? < super::abi::value_bytes(*ty)?)
                        .then_some(own),
                        false,
                    )
                }
                Alias::Slot(slot) => (Operand::Frame(self.placement.slot(slot)?), None, true),
                Alias::Symbol(symbol) => (Operand::Symbol(symbol), None, false),
                Alias::Constant(_) if ty.float_rank() != 0 => {
                    (self.float_operand(value, *ty)?, None, false)
                }
                Alias::Constant(Constant::Integer(bits)) => (
                    Operand::Immediate(
                        i128::from(super::mir::low_word(normalize(bits, *ty))?).cast_unsigned(),
                    ),
                    None,
                    false,
                ),
                Alias::Constant(Constant::Float(_, bits)) => {
                    (Operand::Immediate(bits), None, false)
                }
            };
            moves.push(Transfer {
                destination,
                source,
                source_type,
                ty: *ty,
                address,
            });
        }
        let result = parallel_transfer::emit(self, &mut moves);
        self.edge_moves = moves;
        result
    }
}

#[derive(Default)]
pub(super) struct PhiInputs {
    first: Vec<Option<(usize, usize)>>,
    values: Vec<(BlockId, usize, Value)>,
}
impl PhiInputs {
    fn lookup(&mut self, body: &Body, id: InstId, from: BlockId) -> Result<Option<Value>, Error> {
        let I::Phi(_, pairs) = &body.instruction(id).kind else {
            return Err(Error::Limit);
        };
        if pairs.len() <= 8 {
            return Ok(pairs
                .iter()
                .rev()
                .find_map(|&(pred, value)| (pred == from).then_some(value)));
        }
        if self.first.is_empty() {
            self.first.resize(body.instructions().len(), None);
        }
        let (begin, past) = if let Some(range) = self.first[id.index()] {
            range
        } else {
            let begin = self.values.len();
            let past = begin.checked_add(pairs.len()).ok_or(Error::Limit)?;
            self.values.extend(
                pairs
                    .iter()
                    .enumerate()
                    .map(|(index, &(pred, value))| (pred, index, value)),
            );
            self.values[begin..past]
                .sort_unstable_by_key(|&(pred, index, _)| (pred.index(), index));
            self.first[id.index()] = Some((begin, past));
            (begin, past)
        };
        let values = &self.values[begin..past];
        let at = values.partition_point(|&(pred, _, _)| pred.index() <= from.index());
        Ok(at
            .checked_sub(1)
            .and_then(|index| values.get(index))
            .and_then(|&(pred, _, value)| (pred == from).then_some(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Call, Facts, Program},
        vocabulary::{Binary, Compare},
    };
    #[test]
    fn sinking_a_compare_does_not_extend_a_global_parameters_local_call_interval()
    -> Result<(), Error> {
        let parameters = [
            ("a", Type::I64),
            ("b", Type::I64),
            ("c", Type::I64),
            ("d", Type::I64),
            ("e", Type::I64),
            ("f", Type::I64),
        ];
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "target", &parameters, Type::I64)?;
        builder.block("entry")?;
        let zero = builder.integer(0)?;
        builder.effect(I::Return(Type::I64, Some(zero)))?;
        builder.finish()?;
        let callee = program
            .find_symbol(program.functions()[0].name)
            .ok_or(Error::Limit)?;
        let mut builder = FunctionBuilder::new(&mut program, "f", &parameters, Type::I64)?;
        builder.block("entry")?;
        let values = (0..6)
            .map(|index| builder.parameter(index))
            .collect::<Result<Vec<_>, _>>()?;
        let zero = builder.integer(0)?;
        let condition = builder.value(
            I::Compare(Compare::Eq, Type::I64, values[0], zero),
            "condition",
        )?;
        let arguments = builder.arguments(&values)?;
        let _unused = builder.value(
            I::Call(Call {
                result: Type::I64,
                callee: Value::Symbol(callee),
                arguments,
                copy_elision: false,
                signature: None,
            }),
            "unused",
        )?;
        let yes = builder.block_id("yes")?;
        let no = builder.block_id("no")?;
        builder.effect(I::Branch(condition, yes, no))?;
        for (block, prefix) in [("yes", "left"), ("no", "right")] {
            builder.block(block)?;
            let mut sum = values[0];
            for (index, &value) in values.iter().enumerate().skip(1) {
                sum = builder.value(
                    I::Binary(Binary::Add, Facts::default(), Type::I64, sum, value),
                    &format!("{prefix}{index}"),
                )?;
            }
            builder.effect(I::Return(Type::I64, Some(sum)))?;
        }
        builder.finish()?;
        let function = &program.functions()[1];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let selected = super::super::value_selection::select(&program, symbol, function, 0)?;
        assert_eq!(selected.global_registers, 0);
        assert_eq!(selected.fused_compares, 1);
        assert_eq!(
            selected
                .bindings
                .iter()
                .filter(|binding| binding.kind == mir::BindingKind::Parameter)
                .count(),
            6
        );
        let call = selected
            .instructions
            .iter()
            .position(|inst| matches!(inst.op, Op::Call(_)))
            .ok_or(Error::Limit)?;
        assert!(
            !selected.instructions[..call]
                .iter()
                .any(|inst| matches!(inst.op, Op::Scalar(ScalarOp::Load(_, _, Operand::Frame(_)))))
        );
        Ok(())
    }
}

//! Host integer entry normalization from `select/emit_calls.cpp`. See NOTICE.
use super::{
    mir::{Function, Instruction, Op},
    scalar_mir::{Operand, ScalarOp},
    value_facts::{Alias, Facts},
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, InstructionKind as I, TempId, Value},
    vocabulary::{Binary, Convert, Type, Unary},
};
fn narrow(ty: Type) -> bool {
    matches!(
        ty,
        Type::I1 | Type::I8 | Type::U8 | Type::I16 | Type::U16 | Type::I32 | Type::U32
    )
}
struct Observation<'a> {
    body: &'a Body,
    facts: &'a Facts,
    from: Vec<u64>,
    observed: Vec<bool>,
}
impl Observation<'_> {
    fn source(&self, value: Value) -> Option<TempId> {
        if let Alias::Temporary(temp) = self.facts.resolve(self.body, value) {
            Some(temp)
        } else {
            None
        }
    }
    fn same(&self, value: Value, resolved: TempId) -> bool {
        matches!(value, Value::Temp(original) if self.body.temp(original).ty == self.body.temp(resolved).ty)
    }
    fn mark(&mut self, mut bits: u64) {
        while bits != 0 {
            self.observed[bits.trailing_zeros() as usize] = true;
            bits &= bits - 1;
        }
    }
    fn check(&mut self, value: Value, safe: bool) {
        if let Some(resolved) = self.source(value)
            && (!safe || !self.same(value, resolved))
        {
            self.mark(self.from[resolved.index()]);
        }
    }
    fn small_mask(&self, ty: Type, a: Value, b: Value) -> bool {
        if !narrow(ty) || ty == Type::I1 {
            return false;
        }
        for v in [a, b] {
            if let Alias::Constant(Constant::Integer(mask)) = self.facts.resolve(self.body, v) {
                // Literal.low is the frozen target signed 64-bit mask.
                let low = mask & u128::from(u64::MAX);
                return low < 1_u128 << (ty.bits() - 1);
            }
        }
        false
    }
    fn propagated(&mut self, value: Value) -> u64 {
        let Some(resolved) = self.source(value) else {
            return 0;
        };
        let bits = self.from[resolved.index()];
        if self.same(value, resolved) {
            bits
        } else {
            self.mark(bits);
            0
        }
    }
    fn visit(&mut self, position: usize) -> bool {
        let inst = self
            .body
            .instruction(self.facts.flow.instructions[position]);
        let safe = match &inst.kind {
            I::Compare(_, ty, a, b) => {
                let bytes = u32::from(super::scalar_mir::bytes(*ty));
                for value in [*a, *b] {
                    if let Some(resolved) = self.source(value)
                        && let Value::Temp(original) = value
                        && (!self.same(value, resolved)
                            || self
                                .body
                                .temp(original)
                                .ty
                                .is_some_and(|ty| u32::from(super::scalar_mir::bytes(ty)) < bytes))
                    {
                        self.mark(self.from[resolved.index()]);
                    }
                }
                return false;
            }
            I::Binary(op, _, ty, a, b) => {
                let safe = matches!(op, Binary::Add | Binary::Sub | Binary::Mul)
                    || *op == Binary::And && self.small_mask(*ty, *a, *b);
                if !safe
                    && matches!(op, Binary::And | Binary::Or | Binary::Xor)
                    && narrow(*ty)
                    && let Some(result) = inst.result
                {
                    let mut bits = self.from[result.index()];
                    bits |= self.propagated(*a) | self.propagated(*b);
                    let grown = bits != self.from[result.index()];
                    self.from[result.index()] = bits;
                    return grown;
                }
                safe
            }
            I::Unary(op, ..) => matches!(op, Unary::Ctz | Unary::Clz),
            I::Convert(op, ..) => {
                if let Some(result) = inst.result
                    && !matches!(self.facts.resolve(self.body, Value::Temp(result)), Alias::Temporary(t) if t == result)
                {
                    return false;
                }
                *op == Convert::Truncate
            }
            I::Store(_, _, value, address) => {
                self.check(*value, true);
                self.check(*address, false);
                return false;
            }
            I::Call(call) => {
                self.check(call.callee, false);
                for &value in self.body.arguments(call.arguments) {
                    self.check(value, true);
                }
                return false;
            }
            _ => false,
        };
        self.body.visit_values(&inst.kind, |v| self.check(v, safe));
        false
    }
}
pub(super) fn observe(body: &Body, facts: &Facts) -> Vec<bool> {
    let mut query = Observation {
        body,
        facts,
        from: vec![0; body.temps().len()],
        observed: vec![false; body.parameter_count],
    };
    for (p, param) in body.temps().iter().take(body.parameter_count).enumerate() {
        if param.ty.is_some_and(narrow) {
            if p < 64 {
                query.from[p] = 1 << p;
            } else {
                query.observed[p] = true;
            }
        }
    }
    // Monotone per-function dataflow: a round adds at least one of the finite
    // parameter-origin bits to a bitwise result. No semantic queries are rerun.
    let mut grown = true;
    while grown {
        grown = false;
        for position in 0..facts.flow.instructions.len() {
            if facts.active[position] {
                grown |= query.visit(position);
            }
        }
    }
    query.observed
}
pub(super) fn normalize(
    body: &Body,
    facts: &Facts,
    mir: &mut Function,
    observed: &[bool],
) -> Result<(), Error> {
    for (p, &observed) in observed.iter().enumerate() {
        if !observed || facts.intervals[p].uses == 0 {
            continue;
        }
        let ty = body.temps()[p].ty.ok_or(Error::Limit)?;
        if let [part] = mir.layout.parts(&mir.layout.arguments[p])
            && let Some(reg) = part.register
            && reg.code() < 16
        {
            mir.instructions.push(Instruction {
                volatile: false,
                op: Op::Scalar(if ty == Type::I1 {
                    ScalarOp::Binary(Binary::And, Type::I64, reg, Operand::Immediate(1))
                } else {
                    ScalarOp::Extend(ty, reg)
                }),
                location: None,
            });
        }
    }
    Ok(())
}

// emit_calls.cpp::fold_entry_extensions: only the entry sequence is visible,
// before ordinary instructions are selected. Preserve the frozen explicit
// operand reads and writes, including reads through an address's base/index.
pub(super) fn fold_entry_extensions(instructions: &mut [Instruction]) {
    use super::x86::Register as R;
    for at in 0..instructions.len() {
        let Op::Scalar(ScalarOp::Extend(ty, reg)) = instructions[at].op else {
            continue;
        };
        let mut reader = None;
        let mut readers = 0;
        for (next, inst) in instructions.iter().enumerate().skip(at + 1) {
            let mut reads = false;
            let mut writes = false;
            let mut op = inst.op;
            super::frame_layout::operands(&mut op, |operand, _| {
                reads |= operand.mask() & (1 << reg.code()) != 0;
            });
            let destination = match inst.op {
                Op::Move(r, _)
                | Op::SymbolMove(r, _)
                | Op::Scalar(
                    ScalarOp::Move(r, _)
                    | ScalarOp::Load(r, _, _)
                    | ScalarOp::Lea(r, _)
                    | ScalarOp::Lea32(r, _),
                ) => Some((r, false)),
                Op::Scalar(ScalarOp::Extend(_, r) | ScalarOp::Binary(_, _, r, _)) => {
                    Some((r, true))
                }
                Op::Scalar(ScalarOp::ExtendMove(_, r, source)) => {
                    reads |= source == reg;
                    Some((r, true))
                }
                _ => None,
            };
            if let Some((destination, read)) = destination
                && destination == reg
            {
                writes = true;
                reads |= read;
            }
            if reads {
                readers += 1;
                reader = Some(next);
            }
            if writes || readers > 1 {
                break;
            }
        }
        if readers != 1 {
            continue;
        }
        if let Some(reader) = reader
            && let Op::Scalar(ScalarOp::Move(destination, Operand::Register(source))) =
                instructions[reader].op
            && destination.code() < R::XMM0.code()
            && source == reg
        {
            instructions[reader].op = Op::Scalar(ScalarOp::ExtendMove(ty, destination, source));
            instructions[at].op = Op::Nop;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        memory::{Base, Memory},
        x86::Register as R,
    };
    use super::*;

    fn instruction(op: ScalarOp) -> Instruction {
        Instruction {
            op: Op::Scalar(op),
            volatile: false,
            location: None,
        }
    }
    #[test]
    fn entry_extensions_follow_a_single_copy_until_the_incoming_register_is_written() {
        for ty in [
            Type::I8,
            Type::I16,
            Type::I32,
            Type::U8,
            Type::U16,
            Type::U32,
        ] {
            let mut insts = [
                instruction(ScalarOp::Extend(ty, R::RDI)),
                instruction(ScalarOp::Move(R::RBX, Operand::Register(R::RDI))),
                instruction(ScalarOp::Move(R::RDI, Operand::Register(R::RAX))),
                instruction(ScalarOp::Move(R::R12, Operand::Register(R::RDI))),
            ];
            fold_entry_extensions(&mut insts);
            assert!(matches!(insts[0].op, Op::Nop));
            assert!(
                matches!(insts[1].op, Op::Scalar(ScalarOp::ExtendMove(t, R::RBX, R::RDI)) if t == ty)
            );
        }
    }
    #[test]
    fn multiple_reads_address_reads_and_earlier_writes_keep_entry_extensions() {
        let copy = ScalarOp::Move(R::RBX, Operand::Register(R::RDI));
        for middle in [
            copy,
            ScalarOp::Move(R::RDI, Operand::Register(R::RAX)),
            ScalarOp::Load(
                R::R12,
                Type::I64,
                Operand::Memory(Memory::new(Base::Register(R::RDI), 0)),
            ),
            ScalarOp::Load(
                R::RDI,
                Type::I64,
                Operand::Memory(Memory::new(Base::Register(R::RDI), 0)),
            ),
        ] {
            let mut insts = [
                instruction(ScalarOp::Extend(Type::I32, R::RDI)),
                instruction(middle),
                instruction(copy),
            ];
            fold_entry_extensions(&mut insts);
            assert!(matches!(
                insts[0].op,
                Op::Scalar(ScalarOp::Extend(Type::I32, R::RDI))
            ));
        }
    }
}

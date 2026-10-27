//! Frozen single-use complementary-shift rotation selection. See NOTICE.
use super::{
    address_facts::Addresses,
    scalar_mir::{Rotation, ScalarOp},
    scalar_selection::Selection,
    value_facts::{Alias, Facts},
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, InstructionKind as I, TempId, Value},
    vocabulary::{Binary, Type},
};
#[derive(Clone, Copy)]
pub(super) struct Plan {
    source: Value,
    left: u8,
}
#[derive(Clone, Copy)]
struct Shift {
    temp: TempId,
    pos: usize,
    source: Value,
    count: u8,
}
fn shift(
    body: &Body,
    facts: &Facts,
    addresses: &Addresses,
    value: Value,
    consumer: usize,
    ty: Type,
    op: Binary,
) -> Option<Shift> {
    let Alias::Temporary(temp) = facts.resolve(body, value) else {
        return None;
    };
    let pos = facts.definitions[temp.index()]?;
    let interval = &facts.intervals[temp.index()];
    if temp.index() < body.parameter_count
        || !facts.active[pos]
        || facts.global[temp.index()]
        || facts.fused[temp.index()]
        || facts.tested[temp.index()]
        || facts.rotated[temp.index()]
        || facts.updates[temp.index()]
        || addresses.lazy[temp.index()]
        || addresses.folded[temp.index()].is_some()
        || facts.flow.blocks[pos] != facts.flow.blocks[consumer]
        || interval.uses != 1
        || interval.consumer != Some(consumer)
    {
        return None;
    }
    let I::Binary(binary, _, kind, source, count) =
        body.instruction(facts.flow.instructions[pos]).kind
    else {
        return None;
    };
    if binary != op || kind != ty {
        return None;
    }
    let Alias::Constant(Constant::Integer(bits)) = facts.resolve(body, count) else {
        return None;
    };
    if bits == 0 || bits >= u128::from(super::scalar_mir::bytes(ty)) * 8 {
        return None;
    }
    Some(Shift {
        temp,
        pos,
        source,
        count: u8::try_from(bits).ok()?,
    })
}
pub(super) fn analyze(body: &Body, facts: &mut Facts, addresses: &Addresses) {
    if facts.level < 1 {
        return;
    }
    for pos in 0..facts.flow.instructions.len() {
        if !facts.active[pos] {
            continue;
        }
        let inst = body.instruction(facts.flow.instructions[pos]);
        let (Some(result), I::Binary(Binary::Or | Binary::Xor | Binary::Add, _, ty, a, b)) =
            (inst.result, &inst.kind)
        else {
            continue;
        };
        let (ty, a, b) = (*ty, *a, *b);
        if !matches!(ty, Type::I64 | Type::U32) {
            continue;
        }
        for (a, b) in [(a, b), (b, a)] {
            let Some(left) = shift(body, facts, addresses, a, pos, ty, Binary::Shl) else {
                continue;
            };
            let Some(right) = shift(body, facts, addresses, b, pos, ty, Binary::UShr) else {
                continue;
            };
            if left.count + right.count != super::scalar_mir::bytes(ty) * 8
                || !facts.resolved_value_eq(body, left.source, right.source)
            {
                continue;
            }
            facts.rotations[result.index()] = Some(Plan {
                source: left.source,
                left: left.count,
            });
            for shift in [left, right] {
                facts.rotated[shift.temp.index()] = true;
                facts.active[shift.pos] = false;
                facts.reads[shift.pos] = pos;
            }
            if let Alias::Temporary(temp) = facts.resolve(body, left.source)
                && !facts.global[temp.index()]
                && facts.definitions[temp.index()].map_or(body.layout[0], |p| facts.flow.blocks[p])
                    == facts.flow.blocks[pos]
            {
                let last = &mut facts.intervals[temp.index()].last;
                *last = Some(last.unwrap_or(pos).max(pos));
            }
            break;
        }
    }
}
impl Selection<'_> {
    pub(super) fn rotate(&mut self, temp: TempId, ty: Type) -> Result<bool, Error> {
        let Some(plan) = self.facts.rotations.get(temp.index()).copied().flatten() else {
            return Ok(false);
        };
        let source = self.operand(plan.source, ty, R::R10)?;
        let r = self.result(temp, [self.dying(source, false), None], 0);
        self.load(r, source, if source.is_memory() { ty } else { Type::I64 });
        let width = super::scalar_mir::bytes(ty) * 8;
        let (direction, count) = if plan.left * 2 <= width {
            (Rotation::Left, plan.left)
        } else {
            (Rotation::Right, width - plan.left)
        };
        self.emit(ScalarOp::Rotate(direction, ty, r, count));
        self.finish(temp, r)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{condition::Condition, mir::Op};
    use super::*;
    #[test]
    fn rotations_preserve_zero_flags_and_participate_in_register_visits() -> Result<(), Error> {
        let program = crate::lowir::test_program(
            b"function @f(%x : i64) -> i64 { block ^entry: %l = binary shl i64 %x, 13 %r = binary ushr i64 %x, 51 %v = binary or i64 %l, %r return i64 %v }",
        )?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let mut selected = super::super::value_selection::select(&program, symbol, function, 1)?;
        let at = selected
            .instructions
            .iter()
            .position(|inst| {
                matches!(
                    inst.op,
                    Op::Scalar(ScalarOp::Rotate(Rotation::Left, Type::I64, _, 13))
                )
            })
            .ok_or(Error::Limit)?;
        let op = &mut selected.instructions[at].op;
        super::super::mir_registers::map(op, |_| R::R9);
        assert_eq!(super::super::mir_registers::written(*op), Some(R::R9));
        assert!(super::super::mir_cleanup::touches(*op, R::R9));
        assert!(!super::super::mir_cleanup::touches(*op, R::RCX));
        let mut prior = selected.instructions[at];
        prior.op = Op::Scalar(ScalarOp::Condition(
            super::super::condition_mir::FlagOp::Test(Type::I64, R::R9),
        ));
        selected.instructions.insert(0, prior);
        let at = at + 1;
        // A rotate must not allow earlier ZF/SF/PF producers to be removed:
        // only carry/overflow change, even for a nonzero immediate count.
        let mut branch = selected.instructions[at];
        branch.op = Op::Branch(Condition::Equal, super::super::mir::BlockId(0));
        selected.instructions.truncate(at + 1);
        selected.instructions.push(branch);
        assert!(!super::super::mir_cleanup::flags_dead_after(
            &selected,
            at.checked_sub(1).ok_or(Error::Limit)?,
            at + 2
        ));
        Ok(())
    }
}

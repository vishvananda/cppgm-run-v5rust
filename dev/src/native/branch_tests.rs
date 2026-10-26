//! Typed single-use mask tests and byte-demand propagation. See NOTICE.
use super::{
    address_facts::Addresses,
    condition::Condition as C,
    condition_mir::FlagOp,
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    value_facts::{Alias, Facts},
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, InstructionKind as I, Program, TempId, Value},
    vocabulary::{Binary, Binding, Compare, Type},
};
fn temporary(body: &Body, facts: &Facts, v: Value) -> Option<TempId> {
    if let Alias::Temporary(t) = facts.resolve(body, v) {
        Some(t)
    } else {
        None
    }
}
fn single(
    body: &Body,
    facts: &Facts,
    addresses: &Addresses,
    v: Value,
    op: Binary,
    ty: Type,
    branch: usize,
) -> Option<(TempId, usize, Value, Value)> {
    let t = temporary(body, facts, v)?;
    let p = facts.definitions[t.index()]?;
    if t.index() < body.parameter_count
        || facts.global[t.index()]
        || addresses.folded[t.index()].is_some()
        || addresses.lazy[t.index()]
        || facts.intervals[t.index()].uses != 1
        || !facts.active[p]
        || facts.flow.blocks[p] != facts.flow.blocks[branch]
    {
        return None;
    }
    if let I::Binary(binary, _, kind, a, b) = body.instruction(facts.flow.instructions[p]).kind
        && binary == op
        && kind == ty
    {
        Some((t, p, a, b))
    } else {
        None
    }
}
pub(super) fn fold(body: &Body, facts: &mut Facts, addresses: &Addresses) {
    if facts.level < 1 {
        return;
    }
    for branch in 0..facts.flow.instructions.len() {
        if !facts.active[branch] {
            continue;
        }
        let I::Branch(operand, ..) = body.instruction(facts.flow.instructions[branch]).kind else {
            continue;
        };
        let Some(temp) = temporary(body, facts, operand) else {
            continue;
        };
        let mut value = operand;
        if facts.fused[temp.index()] {
            let Some(definition) = facts.definitions[temp.index()] else {
                continue;
            };
            if definition + 1 != branch {
                continue;
            }
            let I::Compare(Compare::Eq | Compare::Ne, _, a, b) =
                body.instruction(facts.flow.instructions[definition]).kind
            else {
                continue;
            };
            if !matches!(
                facts.resolve(body, b),
                Alias::Constant(Constant::Integer(0))
            ) {
                continue;
            }
            value = a;
        } else if facts.definitions[temp.index()].is_none_or(|definition| definition + 1 != branch)
        {
            continue;
        }
        let Some(temp) = temporary(body, facts, value) else {
            continue;
        };
        let Some(ty) = body.temp(temp).ty else {
            continue;
        };
        if ty == Type::I1
            || ty == Type::I128
            || ty.float_rank() != 0
            || ty.bits() == 0 && ty != Type::Ptr
        {
            continue;
        }
        let Some((temp, definition, a, b)) =
            single(body, facts, addresses, value, Binary::And, ty, branch)
        else {
            continue;
        };
        if [a, b].into_iter().any(|operand| {
            temporary(body, facts, operand)
                .is_some_and(|temp| addresses.folded[temp.index()].is_some())
        }) {
            continue;
        }
        facts.tested[temp.index()] = true;
        facts.active[definition] = false;
        facts.reads[definition] = branch;
        let mut operands = [a, b];
        for (operand, other) in [(a, b), (b, a)] {
            if ty.bits() >= 32
                && let Some((temp, definition, one, bit)) =
                    single(body, facts, addresses, operand, Binary::Shl, ty, branch)
                && matches!(
                    facts.resolve(body, one),
                    Alias::Constant(Constant::Integer(1))
                )
            {
                facts.tested[temp.index()] = true;
                facts.active[definition] = false;
                facts.reads[definition] = branch;
                operands = [other, bit];
                break;
            }
        }
        for operand in operands {
            if let Some(temp) = temporary(body, facts, operand)
                && !facts.global[temp.index()]
                && facts.definitions[temp.index()]
                    .map_or(body.layout[0], |definition| facts.flow.blocks[definition])
                    == facts.flow.blocks[branch]
            {
                facts.intervals[temp.index()].last = Some(
                    facts.intervals[temp.index()]
                        .last
                        .unwrap_or(branch)
                        .max(branch),
                );
            }
        }
    }
}
pub(super) fn bytes_only(program: &Program, body: &Body, facts: &mut Facts, host: bool) {
    if facts.level < 1 {
        return;
    }
    let mut wide = vec![false; body.temps().len()];
    let mut again = Vec::new();
    for (p, &id) in facts.flow.instructions.iter().enumerate() {
        if !facts.active[p]
            && facts.reads[p] == p
            && body
                .instruction(id)
                .result
                .is_none_or(|t| !facts.tested[t.index()])
        {
            continue;
        }
        let inst = body.instruction(id);
        if inst.result.is_some_and(
            |t| !matches!(facts.resolve(body,Value::Temp(t)),Alias::Temporary(v) if v==t),
        ) {
            continue;
        }
        if matches!(
            inst.kind,
            I::Phi(..)
                | I::Binary(Binary::And | Binary::Or | Binary::Xor, _, _, _, _)
                | I::Select(..)
        ) {
            again.push(id);
        }
        visit_bytes(
            program,
            body,
            facts,
            &inst.kind,
            inst.result,
            host,
            &mut wide,
        );
    }
    loop {
        let before = wide.iter().filter(|&&v| v).count();
        for &id in &again {
            let i = body.instruction(id);
            visit_bytes(program, body, facts, &i.kind, i.result, host, &mut wide);
        }
        if before == wide.iter().filter(|&&v| v).count() {
            break;
        }
    }
    for (p, &id) in facts.flow.instructions.iter().enumerate() {
        let inst = body.instruction(id);
        if let Some(t) = inst.result
            && facts.active[p]
            && matches!(inst.kind, I::Compare(..) | I::Call(_))
        {
            facts.byte_only[t.index()] = !wide[t.index()]
                && (matches!(inst.kind, I::Compare(..))
                    || inst.kind.result_type().bits() != 0
                        && super::scalar_mir::bytes(inst.kind.result_type()) == 1);
        }
    }
}
fn visit_bytes(
    program: &Program,
    body: &Body,
    facts: &Facts,
    kind: &I,
    result: Option<TempId>,
    host: bool,
    wide: &mut [bool],
) {
    let boolean = |v| temporary(body, facts, v).is_some_and(|t| Facts::known_boolean(body, t, 0));
    let literal = |v| {
        matches!(
            facts.resolve(body, v),
            Alias::Constant(Constant::Integer(0 | 1))
        )
    };
    let byte = |ty: Type| ty.bits() != 0 && ty != Type::I128 && super::scalar_mir::bytes(ty) == 1;
    let safe = result.is_some_and(|t| !wide[t.index()]);
    let mut read = |v, safe: bool| {
        if !safe && let Some(t) = temporary(body, facts, v) {
            wide[t.index()] = true;
        }
    };
    match kind {
        I::Phi(_, pairs) => {
            for &(_, v) in pairs {
                read(v, safe);
            }
        }
        I::Binary(Binary::And | Binary::Or | Binary::Xor, _, ty, a, b)
            if ty.bits() != 0 && *ty != Type::I128 =>
        {
            read(*a, safe);
            read(*b, safe);
        }
        I::Branch(v, ..) | I::Select(_, v, ..) => {
            read(
                *v,
                boolean(*v)
                    || temporary(body, facts, *v)
                        .is_some_and(|t| body.temp(t).ty.is_some_and(byte)),
            );
            if let I::Select(_, _, a, b) = kind {
                read(*a, safe);
                read(*b, safe);
            }
        }
        I::Store(_, ty, v, addr) => {
            read(*v, super::scalar_mir::bytes(*ty) == 1);
            read(*addr, false);
        }
        I::Compare(pred, ty, a, b) => {
            let eq = matches!(pred, Compare::Eq | Compare::Ne);
            read(*a, byte(*ty) || eq && boolean(*a) && literal(*b));
            read(*b, byte(*ty) || eq && boolean(*b) && literal(*a));
        }
        I::Call(call) => {
            let own = host
                && program.callee_function(call.callee).is_some_and(|f| {
                    f.body.is_some()
                        && f.metadata.binding != Some(Binding::Weak)
                        && f.metadata.flags & 64 == 0
                });
            let sig = own.then(|| program.call_boundary(call)).flatten();
            read(call.callee, false);
            for (i, &v) in body.arguments(call.arguments).iter().enumerate() {
                read(
                    v,
                    sig.and_then(|s| s.parameters.get(i))
                        .is_some_and(|p| byte(p.ty)),
                );
            }
        }
        _ => body.visit_values(kind, |v| read(v, false)),
    }
}
impl Selection<'_> {
    pub(super) fn mask_flags(&mut self, temp: TempId, set: bool) -> Result<C, Error> {
        let definition = self.facts.definitions[temp.index()].ok_or(Error::Limit)?;
        let I::Binary(_, _, ty, mut left, mut right) = self
            .body
            .instruction(self.facts.flow.instructions[definition])
            .kind
        else {
            return Err(Error::Limit);
        };
        let tested = |v| {
            temporary(self.body, &self.facts, v).is_some_and(|temp| self.facts.tested[temp.index()])
        };
        if tested(left) {
            std::mem::swap(&mut left, &mut right);
        }
        let bit = tested(right);
        if bit {
            let temp = temporary(self.body, &self.facts, right).ok_or(Error::Limit)?;
            let definition = self.facts.definitions[temp.index()].ok_or(Error::Limit)?;
            let I::Binary(_, _, _, _, value) = self
                .body
                .instruction(self.facts.flow.instructions[definition])
                .kind
            else {
                return Err(Error::Limit);
            };
            let mut b = self.operand(value, ty, R::R11)?;
            let mut a = self.operand(left, ty, R::R10)?;
            if a.register().is_none() {
                self.load(R::R10, a, ty);
                a = Operand::Register(R::R10);
            }
            if b.is_memory() {
                self.load(R::R11, b, ty);
                b = Operand::Register(R::R11);
            }
            self.emit(ScalarOp::Condition(FlagOp::BitTest(
                ty,
                a.register().ok_or(Error::Limit)?,
                b,
            )));
            return Ok(if set { C::Below } else { C::AboveEqual });
        }
        let mut a = self.operand(left, ty, R::R10)?;
        let mut b = self.operand(right, ty, R::R11)?;
        if matches!(a, Operand::Immediate(_)) || a.register().is_none() && b.register().is_some() {
            std::mem::swap(&mut a, &mut b);
        }
        if matches!(a, Operand::Immediate(_)) || a.is_symbol() {
            self.load(R::R10, a, ty);
            a = Operand::Register(R::R10);
        }
        if b.is_memory() || b.is_symbol() {
            self.load(R::R11, b, ty);
            b = Operand::Register(R::R11);
        }
        self.emit(ScalarOp::Condition(FlagOp::MaskTest(ty, a, b)));
        Ok(if set { C::NotEqual } else { C::Equal })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{mir::Op, value_selection};
    #[test]
    fn boolean_phi_demands_propagate_wide_reads_back_to_comparisons() -> Result<(), Error> {
        for (use_value, extended) in [
            ("branch %merged, ^yes, ^no", false),
            ("return i64 %merged", true),
        ] {
            let text = format!(
                "function @f(%input : i64) -> i64 {{ block ^entry: %condition = cmp gt i64 %input, 0 branch %input, ^left, ^right block ^left: jump ^join block ^right: jump ^join block ^join: %merged = phi i64 [^left: %condition, ^right: 0] {use_value} block ^yes: return i64 1 block ^no: return i64 0 }}"
            );
            let program = crate::lowir::test_program(text.as_bytes())?;
            let function = &program.functions()[0];
            let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
            let selected = value_selection::select(&program, symbol, function, 1)?;
            assert_eq!(
                selected
                    .instructions
                    .iter()
                    .any(|i| matches!(i.op, Op::Scalar(ScalarOp::Movzx(_)))),
                extended
            );
        }
        Ok(())
    }
}

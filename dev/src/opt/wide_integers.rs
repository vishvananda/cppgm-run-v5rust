//! Late i128 halves and durable bridges from `opt/wide_integers.cpp`. See NOTICE.
use super::pass_context::Context;
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, Facts, InstId, Instruction, InstructionKind as I, LinkEditor, OptimizerSlot,
        Value,
    },
    vocabulary::{Binary as B, Compare as C, Convert as V, Projection, Type, Unary},
};
#[derive(Clone, Copy)]
struct Pair {
    lo: Value,
    hi: Value,
}
struct Wide<'a> {
    body: &'a mut Body,
    links: LinkEditor,
    halves: Vec<Option<Pair>>,
    kept: Vec<bool>,
    slots: Vec<Option<Pair>>,
    block: BlockId,
    after: Option<InstId>,
    location: Option<InstId>,
}
fn bits(body: &Body, v: Value) -> Option<u64> {
    let Value::Integer(id) = v else { return None };
    Some(u64::from_le_bytes(
        body.literal(id).to_le_bytes()[..8].try_into().ok()?,
    ))
}
fn wide(body: &Body, v: Value) -> bool {
    matches!(v, Value::Temp(t) if body.temp(t).ty == Some(Type::I128))
}
fn expandable(kind: &I) -> bool {
    match *kind {
        I::Phi(Type::I128, _)
        | I::Copy(Type::I128, _)
        | I::Compare(_, Type::I128, ..)
        | I::Select(Type::I128, ..) => true,
        I::Binary(op, _, Type::I128, ..) => matches!(
            op,
            B::And | B::Or | B::Xor | B::Add | B::Sub | B::Shl | B::Shr | B::UShr
        ),
        I::Unary(op, Type::I128, _) => matches!(op, Unary::BitNot | Unary::Neg),
        I::Convert(op, Type::I128, from, _) => {
            matches!(op, V::ZeroExtend | V::SignExtend) && from.bits() != 0 && from != Type::I128
        }
        I::Convert(V::Truncate, to, Type::I128, _) => to.bits() != 0,
        I::Load(flags, Type::I128, _) | I::Store(flags, Type::I128, ..) => !flags.volatile(),
        _ => false,
    }
}
impl Wide<'_> {
    fn literal(&mut self, bits: u64) -> Result<Value, Error> {
        self.body.integer(u128::from_le_bytes(
            i128::from(bits.cast_signed()).to_le_bytes(),
        ))
    }
    fn pair(&mut self, value: Value) -> Result<Pair, Error> {
        match value {
            Value::Integer(id) => {
                let bits = self.body.literal(id).to_le_bytes();
                let lo = self.literal(u64::from_le_bytes(
                    bits[..8].try_into().map_err(|_| Error::Limit)?,
                ))?;
                let hi = self.literal(u64::from_le_bytes(
                    bits[8..].try_into().map_err(|_| Error::Limit)?,
                ))?;
                Ok(Pair { lo, hi })
            }
            Value::Temp(t) => self.halves[t.index()].ok_or(Error::Limit),
            _ => Err(Error::Limit),
        }
    }
    fn append(&mut self, kind: I, result: bool) -> Result<Option<Value>, Error> {
        let temp = if result {
            Some(self.body.generated_temp(kind.result_type())?)
        } else {
            None
        };
        let id = self
            .body
            .append(self.block, Instruction::new(temp, kind, 0))?;
        if let Some(location) = self.location {
            self.body.inherit_location(id, location);
        }
        self.links.insert_after(self.block, self.after, id)?;
        self.after = Some(id);
        Ok(temp.map(Value::Temp))
    }
    fn emit(&mut self, kind: I) -> Result<Value, Error> {
        self.append(kind, true)?.ok_or(Error::Limit)
    }
    fn effect(&mut self, kind: I) -> Result<(), Error> {
        self.append(kind, false).map(|_| ())
    }
    fn binary(&mut self, op: B, a: Value, b: Value) -> Result<Value, Error> {
        let (x, y) = (bits(self.body, a), bits(self.body, b));
        if let (Some(x), Some(y)) = (x, y) {
            let folded = match op {
                B::And => Some(x & y),
                B::Or => Some(x | y),
                B::Xor => Some(x ^ y),
                B::Add => Some(x.wrapping_add(y)),
                B::Sub => Some(x.wrapping_sub(y)),
                B::Shl => Some(if y < 64 { x << y } else { 0 }),
                B::UShr => Some(if y < 64 { x >> y } else { 0 }),
                B::Shr => Some((x.cast_signed() >> y.min(63)).cast_unsigned()),
                _ => None,
            };
            if let Some(value) = folded {
                return self.literal(value);
            }
        }
        if op == B::And && (x == Some(0) || y == Some(0)) {
            return self.literal(0);
        }
        if op == B::And && y == Some(u64::MAX) {
            return Ok(a);
        }
        if op == B::And && x == Some(u64::MAX) {
            return Ok(b);
        }
        if matches!(op, B::Or | B::Xor | B::Add) && x == Some(0) {
            return Ok(b);
        }
        if matches!(
            op,
            B::Or | B::Xor | B::Add | B::Sub | B::Shl | B::Shr | B::UShr
        ) && y == Some(0)
        {
            return Ok(a);
        }
        if matches!(op, B::Shl | B::UShr | B::Shr) && x == Some(0) {
            return self.literal(0);
        }
        self.emit(I::Binary(op, Facts::default(), Type::I64, a, b))
    }
    fn compare(&mut self, pred: C, a: Value, b: Value) -> Result<Value, Error> {
        self.emit(I::Compare(pred, Type::I64, a, b))
    }
    fn select(&mut self, cond: Value, a: Value, b: Value) -> Result<Value, Error> {
        if a == b {
            return Ok(a);
        }
        self.emit(I::Select(Type::I64, cond, a, b))
    }
    fn convert(&mut self, op: V, to: Type, from: Type, v: Value) -> Result<Value, Error> {
        self.emit(I::Convert(op, to, from, v))
    }
    fn address_plus(&mut self, mut base: Value) -> Result<Value, Error> {
        if matches!(base, Value::Slot(_)) {
            base = self.emit(I::Addr(base))?;
        }
        let offset = self.literal(8)?;
        self.emit(I::Index(Type::I8, Some(Projection::Field), base, offset))
    }
    fn constant_shift(&mut self, op: B, a: Pair, k: u64) -> Result<Pair, Error> {
        let k = k & 127;
        if k == 0 {
            return Ok(a);
        }
        let zero = self.literal(0)?;
        let by = self.literal(k)?;
        let back = self.literal(64_u64.wrapping_sub(k))?;
        let over = self.literal(k.saturating_sub(64))?;
        if op == B::Shl {
            if k >= 64 {
                return Ok(Pair {
                    lo: zero,
                    hi: self.binary(B::Shl, a.lo, over)?,
                });
            }
            let lo = self.binary(B::Shl, a.lo, by)?;
            let up = self.binary(B::Shl, a.hi, by)?;
            let crossed = self.binary(B::UShr, a.lo, back)?;
            return Ok(Pair {
                lo,
                hi: self.binary(B::Or, up, crossed)?,
            });
        }
        let fill = if op == B::Shr {
            let count = self.literal(63)?;
            self.binary(B::Shr, a.hi, count)?
        } else {
            zero
        };
        if k >= 64 {
            return Ok(Pair {
                lo: self.binary(op, a.hi, over)?,
                hi: fill,
            });
        }
        let down = self.binary(B::UShr, a.lo, by)?;
        let crossed = self.binary(B::Shl, a.hi, back)?;
        let lo = self.binary(B::Or, down, crossed)?;
        Ok(Pair {
            lo,
            hi: self.binary(op, a.hi, by)?,
        })
    }
    fn shift(&mut self, op: B, a: Pair, count: Value) -> Result<Pair, Error> {
        if let Some(k) = bits(self.body, count) {
            return self.constant_shift(op, a, k);
        }
        let six = self.literal(63)?;
        let sixty_four = self.literal(64)?;
        let zero = self.literal(0)?;
        let one = self.literal(1)?;
        let low = self.binary(B::And, count, six)?;
        let rest = self.binary(B::Xor, low, six)?;
        let half = self.binary(B::And, count, sixty_four)?;
        let big = self.compare(C::Ne, half, zero)?;
        if op == B::Shl {
            let lo = self.binary(B::Shl, a.lo, low)?;
            let halved = self.binary(B::UShr, a.lo, one)?;
            let carried = self.binary(B::UShr, halved, rest)?;
            let up = self.binary(B::Shl, a.hi, low)?;
            let hi = self.binary(B::Or, up, carried)?;
            let new_lo = self.select(big, zero, lo)?;
            return Ok(Pair {
                lo: new_lo,
                hi: self.select(big, lo, hi)?,
            });
        }
        let hi = self.binary(op, a.hi, low)?;
        let doubled = self.binary(B::Shl, a.hi, one)?;
        let carried = self.binary(B::Shl, doubled, rest)?;
        let down = self.binary(B::UShr, a.lo, low)?;
        let lo = self.binary(B::Or, down, carried)?;
        let fill = if op == B::Shr {
            self.binary(B::Shr, a.hi, six)?
        } else {
            zero
        };
        let new_lo = self.select(big, hi, lo)?;
        Ok(Pair {
            lo: new_lo,
            hi: self.select(big, fill, hi)?,
        })
    }
    fn add_sub(&mut self, op: B, a: Pair, b: Pair) -> Result<Pair, Error> {
        let lo = self.binary(op, a.lo, b.lo)?;
        let carry = if bits(self.body, b.lo) == Some(0) {
            self.literal(0)?
        } else if op == B::Add {
            self.compare(C::Ult, lo, a.lo)?
        } else {
            self.compare(C::Ult, a.lo, b.lo)?
        };
        let high = self.binary(op, a.hi, b.hi)?;
        Ok(Pair {
            lo,
            hi: self.binary(op, high, carry)?,
        })
    }
    fn wide_compare(&mut self, pred: C, a: Pair, b: Pair) -> Result<Value, Error> {
        if matches!(pred, C::Eq | C::Ne) {
            let low = self.binary(B::Xor, a.lo, b.lo)?;
            let high = self.binary(B::Xor, a.hi, b.hi)?;
            let diff = self.binary(B::Or, low, high)?;
            let zero = self.literal(0)?;
            return self.compare(pred, diff, zero);
        }
        let (strict, low) = match pred {
            C::Le => (C::Lt, C::Ule),
            C::Lt => (C::Lt, C::Ult),
            C::Ge => (C::Gt, C::Uge),
            C::Gt => (C::Gt, C::Ugt),
            C::Ule => (C::Ult, C::Ule),
            C::Uge => (C::Ugt, C::Uge),
            _ => (pred, pred),
        };
        let same = self.compare(C::Eq, a.hi, b.hi)?;
        let high = self.compare(strict, a.hi, b.hi)?;
        let lower = self.compare(low, a.lo, b.lo)?;
        self.select(same, lower, high)
    }
    fn extend(&mut self, op: V, from: Type, v: Value) -> Result<Pair, Error> {
        if let Some(mut bits) = bits(self.body, v) {
            let width = from.bits();
            if width < 64 {
                let mask = (1_u64 << width) - 1;
                bits &= mask;
                if op == V::SignExtend && bits & (1 << (width - 1)) != 0 {
                    bits |= !mask;
                }
            }
            let lo = self.literal(bits)?;
            let hi = self.literal(if op == V::SignExtend && bits.cast_signed() < 0 {
                u64::MAX
            } else {
                0
            })?;
            return Ok(Pair { lo, hi });
        }
        let lo = if from == Type::I64 {
            v
        } else {
            self.convert(op, Type::I64, from, v)?
        };
        let hi = if op == V::SignExtend {
            let count = self.literal(63)?;
            self.binary(B::Shr, lo, count)?
        } else {
            self.literal(0)?
        };
        Ok(Pair { lo, hi })
    }
    fn load(&mut self, at: Value) -> Result<Pair, Error> {
        let split = if let Value::Slot(s) = at {
            self.slots[s.index()]
        } else {
            None
        };
        let lo = self.emit(I::Load(
            Facts::default(),
            Type::I64,
            split.map_or(at, |s| s.lo),
        ))?;
        let high_address = if let Some(s) = split {
            s.hi
        } else {
            self.address_plus(at)?
        };
        let hi = self.emit(I::Load(Facts::default(), Type::I64, high_address))?;
        Ok(Pair { lo, hi })
    }
    fn store(&mut self, flags: Facts, value: Value, at: Value) -> Result<(), Error> {
        let pair = self.pair(value)?;
        let split = if let Value::Slot(s) = at {
            self.slots[s.index()]
        } else {
            None
        };
        self.effect(I::Store(
            flags,
            Type::I64,
            pair.lo,
            split.map_or(at, |s| s.lo),
        ))?;
        let high_address = if let Some(s) = split {
            s.hi
        } else {
            self.address_plus(at)?
        };
        self.effect(I::Store(flags, Type::I64, pair.hi, high_address))
    }
    fn expand(
        &mut self,
        id: InstId,
        inst: &Instruction,
        phis: &mut Vec<(InstId, InstId, InstId)>,
    ) -> Result<(), Error> {
        let pair = match inst.kind.clone() {
            I::Phi(Type::I128, _) => {
                let lo = self.emit(I::Phi(Type::I64, Vec::new()))?;
                let lo_id = self.after.ok_or(Error::Limit)?;
                let hi = self.emit(I::Phi(Type::I64, Vec::new()))?;
                phis.push((id, lo_id, self.after.ok_or(Error::Limit)?));
                Pair { lo, hi }
            }
            I::Copy(Type::I128, v) => self.pair(v)?,
            I::Binary(op, _, _, left, right) => {
                let a = self.pair(left)?;
                let b = self.pair(right)?;
                match op {
                    B::Shl | B::Shr | B::UShr => self.shift(op, a, b.lo)?,
                    B::Add | B::Sub => self.add_sub(op, a, b)?,
                    _ => {
                        let lo = self.binary(op, a.lo, b.lo)?;
                        let hi = self.binary(op, a.hi, b.hi)?;
                        Pair { lo, hi }
                    }
                }
            }
            I::Unary(op, _, v) => {
                let a = self.pair(v)?;
                if op == Unary::BitNot {
                    let ones = self.literal(u64::MAX)?;
                    let lo = self.binary(B::Xor, a.lo, ones)?;
                    let hi = self.binary(B::Xor, a.hi, ones)?;
                    Pair { lo, hi }
                } else {
                    let zero = self.literal(0)?;
                    let lo = self.binary(B::Sub, zero, a.lo)?;
                    let negated = self.binary(B::Sub, zero, a.hi)?;
                    let borrow = self.compare(C::Ne, a.lo, zero)?;
                    Pair {
                        lo,
                        hi: self.binary(B::Sub, negated, borrow)?,
                    }
                }
            }
            I::Convert(V::Truncate, to, Type::I128, v) => {
                let lo = self.pair(v)?.lo;
                self.body.replace_kind(
                    id,
                    if to == Type::I64 {
                        I::Copy(to, lo)
                    } else {
                        I::Convert(V::Truncate, to, Type::I64, lo)
                    },
                );
                return Ok(());
            }
            I::Convert(op, _, from, v) => self.extend(op, from, v)?,
            I::Compare(pred, _, left, right) => {
                let a = self.pair(left)?;
                let b = self.pair(right)?;
                let result = self.wide_compare(pred, a, b)?;
                let ty = self
                    .body
                    .temp(inst.result.ok_or(Error::Limit)?)
                    .ty
                    .ok_or(Error::Limit)?;
                self.body.replace_kind(id, I::Copy(ty, result));
                return Ok(());
            }
            I::Select(_, cond, left, right) => {
                let a = self.pair(left)?;
                let b = self.pair(right)?;
                let lo = self.select(cond, a.lo, b.lo)?;
                let hi = self.select(cond, a.hi, b.hi)?;
                Pair { lo, hi }
            }
            I::Load(_, _, at) => self.load(at)?,
            I::Store(flags, _, v, at) => return self.store(flags, v, at),
            _ => return Err(Error::Limit),
        };
        self.halves[inst.result.ok_or(Error::Limit)?.index()] = Some(pair);
        Ok(())
    }
    fn decompose(&mut self, v: Value) -> Result<Pair, Error> {
        let lo = self.convert(V::Truncate, Type::I64, Type::I128, v)?;
        let count = self.literal(64)?;
        let down = self.emit(I::Binary(B::UShr, Facts::default(), Type::I128, v, count))?;
        let hi = self.convert(V::Truncate, Type::I64, Type::I128, down)?;
        Ok(Pair { lo, hi })
    }
    fn compose(&mut self, v: Value) -> Result<Value, Error> {
        let Value::Temp(t) = v else { return Ok(v) };
        if !wide(self.body, v) || self.kept[t.index()] {
            return Ok(v);
        }
        let pair = self.halves[t.index()].ok_or(Error::Limit)?;
        let lo = self.convert(V::ZeroExtend, Type::I128, Type::I64, pair.lo)?;
        let hi = self.convert(V::ZeroExtend, Type::I128, Type::I64, pair.hi)?;
        let count = self.literal(64)?;
        let up = self.emit(I::Binary(B::Shl, Facts::default(), Type::I128, hi, count))?;
        self.emit(I::Binary(B::Or, Facts::default(), Type::I128, up, lo))
    }
    fn bridge(&mut self, id: InstId, values: &mut Vec<Value>) -> Result<bool, Error> {
        values.clear();
        self.body
            .visit_values(&self.body.instruction(id).kind, |v| values.push(v));
        for v in values.iter_mut() {
            *v = self.compose(*v)?;
        }
        let mut rewritten = values.iter();
        self.body
            .rewrite_values(id, |_| rewritten.next().copied().ok_or(Error::Limit))?;
        let Some(t) = self
            .body
            .instruction(id)
            .result
            .filter(|&t| wide(self.body, Value::Temp(t)))
        else {
            return Ok(false);
        };
        self.kept[t.index()] = true;
        self.after = Some(id);
        self.halves[t.index()] = Some(self.decompose(Value::Temp(t))?);
        Ok(true)
    }
    fn split_slots(&mut self) -> Result<(), Error> {
        let mut whole = self
            .body
            .slots()
            .iter()
            .map(|s| s.ty == Type::I128)
            .collect::<Vec<_>>();
        for &b in &self.body.layout {
            for &id in &self.body.block(b).instructions {
                let inst = self.body.instruction(id);
                let mut position = 0;
                self.body.visit_values(&inst.kind, |v| {
                    let at = position;
                    position += 1;
                    if let Value::Slot(s) = v {
                        let access = match inst.kind {
                            I::Load(flags, Type::I128, _) if at == 0 => !flags.volatile(),
                            I::Store(flags, Type::I128, ..) if at == 1 => !flags.volatile(),
                            _ => false,
                        };
                        whole[s.index()] &= access;
                    }
                });
            }
        }
        self.slots.resize(whole.len(), None);
        for (s, whole) in whole.into_iter().enumerate() {
            if !whole {
                continue;
            }
            let lo = self.body.optimizer_slot(Type::I64, OptimizerSlot::Wide)?;
            let hi = self.body.optimizer_slot(Type::I64, OptimizerSlot::Wide)?;
            self.slots[s] = Some(Pair { lo, hi });
        }
        Ok(())
    }
    fn run(&mut self, order: &[BlockId]) -> Result<(), Error> {
        self.split_slots()?;
        let original = self.body.instructions().len();
        let temps = self.body.temps().len();
        self.halves.resize(temps, None);
        self.kept.resize(temps, false);
        for index in 0..temps {
            let t = self.body.temp_id(index).ok_or(Error::Limit)?;
            if self.body.temp(t).ty == Some(Type::I128)
                && (index < self.body.parameter_count || self.body.temp(t).definition.is_none())
            {
                self.kept[index] = true;
                self.halves[index] = Some(self.decompose(Value::Temp(t))?);
            }
        }
        let mut phis = Vec::new();
        let mut values = Vec::new();
        for &b in order {
            self.block = b;
            let mut prev = None;
            let mut next = self.links.first(b);
            while let Some(id) = next {
                let inst = self.body.instruction(id).clone();
                next = self.links.next(id);
                let mut touches = inst.kind.result_type() == Type::I128
                    || inst.result.is_some_and(|t| wide(self.body, Value::Temp(t)));
                self.body
                    .visit_values(&inst.kind, |v| touches |= wide(self.body, v));
                touches |= matches!(
                    inst.kind,
                    I::Store(_, Type::I128, ..) | I::Compare(_, Type::I128, ..)
                );
                if id.index() >= original || !touches {
                    prev = Some(id);
                    continue;
                }
                self.location = Some(id);
                self.after = prev;
                let expand = expandable(&inst.kind)
                    && !matches!(inst.kind, I::Select(_, cond, ..) if wide(self.body, cond));
                if expand {
                    self.expand(id, &inst, &mut phis)?;
                    if matches!(inst.kind, I::Convert(V::Truncate, ..) | I::Compare(..)) {
                        prev = Some(id);
                    } else {
                        self.links.unlink(b, id)?;
                        prev = self.after;
                    }
                } else {
                    prev = if self.bridge(id, &mut values)? {
                        self.after
                    } else {
                        Some(id)
                    };
                }
            }
        }
        for (phi, lo, hi) in phis {
            let I::Phi(_, pairs) = self.body.instruction(phi).kind.clone() else {
                return Err(Error::Limit);
            };
            let mut lows = Vec::with_capacity(pairs.len());
            let mut highs = Vec::with_capacity(pairs.len());
            for (pred, v) in pairs {
                let pair = self.pair(v)?;
                lows.push((pred, pair.lo));
                highs.push((pred, pair.hi));
            }
            self.body.replace_kind(lo, I::Phi(Type::I64, lows));
            self.body.replace_kind(hi, I::Phi(Type::I64, highs));
        }
        self.links.write_links(self.body)
    }
}
pub(super) fn expand(context: &mut Context<'_>) -> Result<bool, Error> {
    if context.level < 2
        || !context
            .body
            .temps()
            .iter()
            .any(|t| t.ty == Some(Type::I128))
    {
        return Ok(false);
    }
    context.ensure_cfg()?;
    let mut order = context.cfg()?.reverse_postorder().to_vec();
    let mut listed = vec![false; context.body.blocks().len()];
    for &b in &order {
        listed[b.index()] = true;
    }
    order.extend(
        context
            .body
            .layout
            .iter()
            .copied()
            .filter(|b| !listed[b.index()]),
    );
    let mut pass = Wide {
        block: *context.body.layout.first().ok_or(Error::Limit)?,
        links: LinkEditor::new(context.body)?,
        body: context.body,
        halves: Vec::new(),
        kept: Vec::new(),
        slots: Vec::new(),
        after: None,
        location: None,
    };
    pass.run(&order)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::{pass_context::Pass, pass_view};
    #[test]
    fn variable_shift_halves_preserve_a_call_bridge_and_its_location() {
        let source = "declare function @sink(%v:i128)->void function @f(%v:i128,%n:i128)->void {block ^entry:%shifted=binary shl i128 %v,%n !dbg(wide.cpp,3,4) call void @sink(%shifted) !dbg(wide.cpp,4,2) return void}";
        let (out, changed) = pass_view(source, 2, Pass::ExpandWide);
        assert!(changed.0);
        assert_eq!(out.matches("select i64").count(), 2);
        assert!(out.contains("binary and i64") && out.contains(", 63"));
        assert!(out.contains("binary shl i128") && out.contains(", 64"));
        assert!(out.contains("binary or i128"));
        assert!(out.contains("!dbg(wide.cpp, 4, 2)"));
    }
    #[test]
    fn split_phi_inputs_resolve_later_definitions_and_memory_flags_keep_the_frozen_policy() {
        let source = "function @f(%again:i1,%p:ptr)->i128 {block ^entry:%init=load typed i128 %p jump ^head block ^head:%v=phi i128 [^entry:%init,^step:%next] branch %again,^step,^exit block ^step:%next=binary add i128 %v,1 store typed i128 %next,%p jump ^head block ^exit:return i128 %v}";
        // Wide phis are a constructed-IR input; the text dialect rejects them.
        let mut program =
            crate::lowir::test_program(source.replace("i128", "i64").as_bytes()).unwrap();
        let symbol = program.publications()[0];
        let signature = program.function(program.symbol(symbol)).unwrap().signature;
        program.signature_mut(signature).result = super::Type::I128;
        let mut body = program.take_body(symbol).unwrap().unwrap();
        for index in 0..body.temps().len() {
            let temp = body.temp_id(index).unwrap();
            if body.temp(temp).ty == Some(super::Type::I64) {
                body.temp_mut(temp).ty = Some(super::Type::I128);
            }
        }
        for block in body.layout.clone() {
            for id in body.block(block).instructions.clone() {
                let kind = match body.instruction(id).kind.clone() {
                    super::I::Load(flags, _, at) => super::I::Load(flags, super::Type::I128, at),
                    super::I::Store(flags, _, v, at) => {
                        super::I::Store(flags, super::Type::I128, v, at)
                    }
                    super::I::Phi(_, pairs) => super::I::Phi(super::Type::I128, pairs),
                    super::I::Binary(op, flags, _, a, b) => {
                        super::I::Binary(op, flags, super::Type::I128, a, b)
                    }
                    super::I::Return(_, v) => super::I::Return(super::Type::I128, v),
                    kind => kind,
                };
                body.replace_kind(id, kind);
            }
        }
        let mut context = super::Context::new(&mut program, symbol, &mut body, 2, &[], &[]);
        assert!(context.run(Pass::ExpandWide).unwrap());
        body.compact(&mut program.names).unwrap();
        program.restore_body(symbol, body).unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        let mut out = Vec::new();
        crate::lowir::writer::write(&program, &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert_eq!(out.matches("phi i64").count(), 2);
        assert!(!out.contains("load typed"));
        assert_eq!(out.matches("store typed i64").count(), 2);
        assert!(out.contains("binary add i64") && out.contains("cmp ult i64"));
        assert!(out.contains("return i128"));
    }
}

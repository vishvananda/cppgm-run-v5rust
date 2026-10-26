//! Typed single-use memory update folding from frozen `find_rmw`. See NOTICE.
use super::{
    address_facts::Addresses,
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    value_facts::{Alias, Facts},
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, InstructionKind as I, TempId, Value},
    vocabulary::{Binary, Convert, Type},
};
#[derive(Clone, Copy)]
struct Update {
    binary: Binary,
    other: Source,
}
#[derive(Clone, Copy)]
enum Source {
    Value(Value),
    Offset(u128),
}
#[derive(Default)]
pub(super) struct Updates {
    plans: Vec<Option<Update>>,
}
struct Match {
    update: Update,
    definitions: [Option<usize>; 4],
    load: TempId,
}
struct Analysis<'a> {
    body: &'a Body,
    facts: &'a Facts,
    addresses: &'a Addresses,
    store: usize,
    ty: Type,
    address: Value,
}
impl Analysis<'_> {
    fn single(&self, v: Value, folded: bool) -> Option<(TempId, usize)> {
        let Alias::Temporary(temp) = self.facts.resolve(self.body, v) else {
            return None;
        };
        let pos = self.facts.definitions[temp.index()]?;
        if temp.index() < self.body.parameter_count
            || self.facts.global[temp.index()]
            || self.facts.fused[temp.index()]
            || self.facts.tested[temp.index()]
            || self.facts.updates[temp.index()]
            || self.addresses.lazy[temp.index()]
            || !folded && self.addresses.folded[temp.index()].is_some()
            || self.facts.intervals[temp.index()].uses != 1
            || self.facts.flow.blocks[pos] != self.facts.flow.blocks[self.store]
        {
            return None;
        }
        let ty = self.body.temp(temp).ty?;
        if ty.float_rank() != 0
            || matches!(ty, Type::I128 | Type::Object { .. })
            || !self.facts.active[pos] && self.addresses.folded[temp.index()].is_none()
        {
            return None;
        }
        Some((temp, pos))
    }
    fn kind(&self, pos: usize) -> &I {
        &self
            .body
            .instruction(self.facts.flow.instructions[pos])
            .kind
    }
    fn integer_convert(&self, pos: usize, ops: &[Convert]) -> bool {
        matches!(self.kind(pos),I::Convert(op,to,from,_) if ops.contains(op)&&to.bits()!=0&&from.bits()!=0&&*to!=Type::I128&&*from!=Type::I128)
    }
    fn load(&self, value: Value, narrow: bool) -> Option<(TempId, usize, Option<usize>)> {
        let (mut temp, mut pos) = self.single(value, true)?;
        let mut extension = None;
        if narrow && self.integer_convert(pos, &[Convert::ZeroExtend, Convert::SignExtend]) {
            let I::Convert(_, _, from, source) = *self.kind(pos) else {
                return None;
            };
            if super::scalar_mir::bytes(from) != super::scalar_mir::bytes(self.ty) {
                return None;
            }
            extension = Some(pos);
            (temp, pos) = self.single(source, true)?;
        }
        if let I::Load(flags, ty, address) = *self.kind(pos)
            && !flags.volatile()
            && ty == self.ty
            && self
                .facts
                .resolved_value_eq(self.body, address, self.address)
        {
            Some((temp, pos, extension))
        } else {
            None
        }
    }
    fn find(&self, value: Value) -> Option<Match> {
        let (_, mut operation) = self.single(value, false)?;
        let mut truncation = None;
        if self.integer_convert(operation, &[Convert::Truncate])
            && super::scalar_mir::bytes(self.ty) < 8
            && self.kind(operation).result_type() == self.ty
        {
            let I::Convert(_, _, _, source) = *self.kind(operation) else {
                return None;
            };
            truncation = Some(operation);
            operation = self.single(source, false)?.1;
        }
        let op = self.kind(operation);
        let bytes = super::scalar_mir::bytes(self.ty);
        let narrow = self.ty.bits() != 0
            && matches!(op,I::Binary(_,_,ty,_,_) if ty.bits()!=0&&super::scalar_mir::bytes(*ty)>bytes);
        if !narrow && (truncation.is_some() || op.result_type() != self.ty) {
            return None;
        }
        let (binary, other, load, load_pos, extension) = match *op {
            I::Binary(binary, _, ty, left, right)
                if ty.float_rank() == 0
                    && ty != Type::I128
                    && super::scalar_mir::bytes(ty) >= bytes
                    && matches!(
                        binary,
                        Binary::Add | Binary::Sub | Binary::And | Binary::Or | Binary::Xor
                    ) =>
            {
                if self.facts.resolved_value_eq(self.body, left, right) {
                    return None;
                }
                let (load, other) = if let Some(load) = self.load(left, narrow) {
                    (load, right)
                } else if binary != Binary::Sub {
                    (self.load(right, narrow)?, left)
                } else {
                    return None;
                };
                match self.facts.resolve(self.body, other) {
                    Alias::Constant(Constant::Integer(bits))
                        if bytes < 8
                            || super::mir::low_word(super::value_facts::normalize(
                                bits, self.ty,
                            ))
                            .is_ok_and(|v| i32::try_from(v).is_ok()) => {}
                    Alias::Temporary(temp)
                        if self.addresses.folded[temp.index()].is_none()
                            && !self.facts.updates[temp.index()]
                            && self.body.temp(temp).ty.is_some_and(|t| {
                                t.float_rank() == 0
                                    && !matches!(t, Type::I128 | Type::Object { .. })
                            }) => {}
                    _ => return None,
                }
                (binary, Source::Value(other), load.0, load.1, load.2)
            }
            I::Index(ty, _, base, offset) if self.ty == Type::Ptr && truncation.is_none() => {
                let Alias::Constant(Constant::Integer(bits)) =
                    self.facts.resolve(self.body, offset)
                else {
                    return None;
                };
                let amount = bits.checked_mul(u128::from(super::abi::value_bytes(ty).ok()?))?;
                if bits > 0x7fff_ffff || amount > 0x7fff_ffff {
                    return None;
                }
                let load = self.load(base, false)?;
                (Binary::Add, Source::Offset(amount), load.0, load.1, load.2)
            }
            _ => return None,
        };
        let past = if load_pos < self.store {
            self.store
        } else {
            self.facts.flow.ranges[self.facts.flow.blocks[load_pos].index()].end
        };
        for p in load_pos + 1..past {
            if self.facts.active[p]
                && !matches!(
                    self.kind(p),
                    I::Binary(..)
                        | I::Unary(..)
                        | I::Convert(..)
                        | I::Compare(..)
                        | I::Select(..)
                        | I::Index(..)
                        | I::Addr(..)
                        | I::Copy(..)
                        | I::Const(..)
                        | I::Load(..)
                )
            {
                return None;
            }
        }
        Some(Match {
            update: Update { binary, other },
            definitions: [Some(load_pos), Some(operation), extension, truncation],
            load,
        })
    }
}
impl Updates {
    pub(super) fn analyze(body: &Body, facts: &mut Facts, addresses: &mut Addresses) -> Self {
        let mut out = Self::default();
        if facts.level < 1 {
            return out;
        }
        for pos in 0..facts.flow.instructions.len() {
            if !facts.active[pos] {
                continue;
            }
            let I::Store(flags, ty, value, address) =
                body.instruction(facts.flow.instructions[pos]).kind
            else {
                continue;
            };
            if flags.volatile()
                || ty == Type::I1
                || ty == Type::I128
                || ty.bits() == 0 && ty != Type::Ptr
            {
                continue;
            }
            let Some(found) = (Analysis {
                body,
                facts,
                addresses,
                store: pos,
                ty,
                address,
            })
            .find(value) else {
                continue;
            };
            if out.plans.is_empty() {
                out.plans.resize(facts.flow.instructions.len(), None);
            }
            out.plans[pos] = Some(found.update);
            addresses.folded_count += 1;
            addresses.folded[found.load.index()] = Some(address);
            for def in found.definitions.into_iter().flatten() {
                let Some(result) = body.instruction(facts.flow.instructions[def]).result else {
                    continue;
                };
                facts.updates[result.index()] = true;
                facts.active[def] = false;
                facts.reads[def] = pos;
            }
            if let Source::Value(value) = found.update.other {
                for temp in super::global_placement::parts(body, facts, addresses, value)
                    .into_iter()
                    .flatten()
                {
                    if facts.global[temp.index()]
                        || facts.definitions[temp.index()]
                            .map_or(body.layout[0], |p| facts.flow.blocks[p])
                            != facts.flow.blocks[pos]
                    {
                        continue;
                    }
                    facts.intervals[temp.index()].last =
                        Some(facts.intervals[temp.index()].last.unwrap_or(pos).max(pos));
                }
            }
        }
        out
    }
}
impl Selection<'_> {
    pub(super) fn memory_update(&mut self, ty: Type, address: Value) -> Result<bool, Error> {
        let Some(plan) = self.updates.plans.get(self.pos).copied().flatten() else {
            return Ok(false);
        };
        let memory = self.address(address, R::R10)?;
        let mut source = match plan.other {
            Source::Value(v) => self.operand(v, ty, R::R11)?,
            Source::Offset(bits) => Operand::Immediate(bits),
        };
        if source.is_memory() || source.is_symbol() {
            self.load(R::R11, source, ty);
            source = Operand::Register(R::R11);
        }
        self.emit(ScalarOp::MemoryBinary(plan.binary, ty, memory, source));
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn updates_keep_the_store_width_and_pointer_element_size() -> Result<(), Error> {
        for (text, binary, ty) in [
            (
                "global @counter : i8 = 0 function @f() -> void { block ^entry: %old = load i8 @counter %wide = convert sext i64 i8 %old %sum = binary add i64 %wide, 3 %narrow = convert trunc i8 i64 %sum store i8 %narrow, @counter return void }",
                Binary::Add,
                Type::I8,
            ),
            (
                "global @pointer : ptr = 0 function @f() -> void { block ^entry: %old = load ptr @pointer %next = index i32 %old, 2 store ptr %next, @pointer return void }",
                Binary::Add,
                Type::Ptr,
            ),
        ] {
            let program = crate::lowir::test_program(text.as_bytes())?;
            let function = &program.functions()[0];
            let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
            let selected = super::super::value_selection::select(&program, symbol, function, 1)?;
            assert!(selected.instructions.iter().any(|i|matches!(i.op,super::super::mir::Op::Scalar(ScalarOp::MemoryBinary(op,kind,_,_)) if op==binary&&kind==ty)));
        }
        Ok(())
    }
    #[test]
    fn update_folding_stops_at_volatile_accesses_and_intervening_calls() -> Result<(), Error> {
        for (load, store, between, folded) in [
            ("load", "store", "", true),
            ("load volatile", "store", "", false),
            ("load", "store volatile", "", false),
            ("load", "store", "call void @touch()", false),
        ] {
            let text = format!(
                "global @counter : i32 = 0 declare function @touch() -> void function @f() -> void {{ block ^entry: %old = {load} i32 @counter {between} %sum = binary add i32 %old, 1 {store} i32 %sum, @counter return void }}"
            );
            let program = crate::lowir::test_program(text.as_bytes())?;
            let function = program
                .functions()
                .iter()
                .find(|f| f.body.is_some())
                .ok_or(Error::Limit)?;
            let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
            let selected = super::super::value_selection::select(&program, symbol, function, 1)?;
            assert_eq!(
                selected.instructions.iter().any(|i| matches!(
                    i.op,
                    super::super::mir::Op::Scalar(ScalarOp::MemoryBinary(
                        Binary::Add,
                        Type::I32,
                        _,
                        _
                    ))
                )),
                folded
            );
        }
        Ok(())
    }
}

//! Function-owned aliases, dead definitions and scalar use intervals. See NOTICE.
use super::{slot_analysis::Slot, x86::Register as R};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, InstructionKind as I, RuntimeInstruction as Runtime, TempId, Value},
    vocabulary::{Binary, Convert, Type},
};
#[derive(Clone, Copy)]
pub(super) enum Alias {
    Constant(Constant),
    Temporary(TempId),
    Symbol(crate::lowir::model::SymbolId),
    Slot(crate::lowir::model::SlotId),
}
#[derive(Clone, Copy, Default)]
pub(super) struct Interval {
    pub(super) last: Option<usize>,
    pub(super) hint: Option<R>,
    pub(super) forbid: u32,
    pub(super) uses: u32,
    pub(super) consumer: Option<usize>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Identity {
    Original(Value),
    Normalized(TempId),
}
pub(super) struct Facts {
    pub(super) flow: super::control_flow::Flow,
    pub(super) global: Vec<bool>,
    pub(super) definitions: Vec<Option<usize>>,
    pub(super) clobbers: Vec<(u32, u32)>,
    pub(super) reads: Vec<usize>,
    aliases: Vec<Option<Alias>>,
    identities: Vec<Option<Identity>>,
    pub(super) fused: Vec<bool>,
    pub(super) active: Vec<bool>,
    pub(super) intervals: Vec<Interval>,
}
pub(super) fn alias_compatible(to: Type, from: Type) -> bool {
    if to == from {
        return true;
    }
    if !matches!(to, Type::Ptr) && to.bits() == 0
        || !matches!(from, Type::Ptr) && from.bits() == 0
        || matches!(to, Type::I128)
        || matches!(from, Type::I128)
    {
        return false;
    }
    let tb = if to == Type::Ptr { 64 } else { to.bits() };
    let fb = if from == Type::Ptr { 64 } else { from.bits() };
    from == Type::I1
        || tb == 64
        || tb > fb
            && (!matches!(from, Type::I8 | Type::I16 | Type::I32)
                || matches!(to, Type::I8 | Type::I16 | Type::I32))
}
pub(super) fn normalize(bits: u128, ty: Type) -> u128 {
    if ty == Type::I128 {
        return bits;
    }
    let width = if ty == Type::Ptr { 64 } else { ty.bits() };
    if width == 0 {
        return bits;
    }
    let mask = (1_u128 << width) - 1;
    let low = bits & mask;
    if matches!(ty, Type::I8 | Type::I16 | Type::I32 | Type::I64) && low & (1 << (width - 1)) != 0 {
        low | !mask
    } else {
        low
    }
}

impl Facts {
    pub(super) fn resolve(&self, body: &Body, value: Value) -> Alias {
        match value {
            Value::Temp(id) => {
                let mut out = Alias::Temporary(id);
                while let Alias::Temporary(temp) = out
                    && let Some(next) = self.aliases[temp.index()]
                {
                    out = next;
                }
                out
            }
            Value::Integer(id) => Alias::Constant(Constant::Integer(body.literal(id))),
            Value::Float(id, kind) => Alias::Constant(Constant::Float(kind, body.literal(id))),
            Value::Symbol(id) => Alias::Symbol(id),
            Value::Slot(id) => Alias::Slot(id),
        }
    }
    pub(super) fn analyze(
        body: &Body,
        slots: &[Slot],
        flow: super::control_flow::Flow,
    ) -> Result<Self, Error> {
        let ids = flow.instructions.clone();
        let mut definitions = vec![None; body.temps().len()];
        for (pos, &id) in ids.iter().enumerate() {
            if let Some(result) = body.instruction(id).result {
                definitions[result.index()] = Some(pos);
            }
        }
        let mut facts = Self {
            global: vec![false; body.temps().len()],
            definitions,
            clobbers: vec![(0, 0); ids.len()],
            reads: (0..ids.len()).collect(),
            active: flow
                .blocks
                .iter()
                .map(|b| flow.reachable[b.index()])
                .collect(),
            flow,
            aliases: vec![None; body.temps().len()],
            identities: vec![None; body.temps().len()],
            fused: vec![false; body.temps().len()],
            intervals: vec![Interval::default(); body.temps().len()],
        };
        for (pos, &id) in ids.iter().enumerate() {
            let inst = body.instruction(id);
            if !facts.active[pos] {
                continue;
            }
            let alias = match inst.kind {
                I::Const(ty, value) => Some(match facts.resolve(body, value) {
                    Alias::Constant(Constant::Integer(bits))
                        if ty.bits() != 0 && ty != Type::I128 =>
                    {
                        Alias::Constant(Constant::Integer(alias_bits(bits, ty)))
                    }
                    other => other,
                }),
                I::Copy(ty, value) => {
                    let source = facts.resolve(body, value);
                    match source {
                        Alias::Constant(Constant::Integer(bits))
                            if (ty.bits() != 0 || ty == Type::Ptr) && ty != Type::I128 =>
                        {
                            Some(Alias::Constant(Constant::Integer(alias_bits(bits, ty))))
                        }
                        Alias::Constant(Constant::Float(..)) if ty.float_rank() != 0 => {
                            Some(source)
                        }
                        Alias::Temporary(temp)
                            if body.temp(temp).ty.is_some_and(|from| {
                                alias_compatible(ty, from)
                                    || (ty.bits() != 0
                                        && ty != Type::I128
                                        && Self::known_boolean(body, temp, 0))
                            }) =>
                        {
                            Some(source)
                        }
                        _ => None,
                    }
                }
                I::Load(_, ty, Value::Slot(slot))
                    if matches!(slots[slot.index()], Slot::Promoted(_)) =>
                {
                    let Slot::Promoted(value) = slots[slot.index()] else {
                        return Err(Error::Limit);
                    };
                    Some(match facts.resolve(body, value) {
                        Alias::Constant(Constant::Integer(bits))
                            if ty.bits() != 0 && ty != Type::I128 =>
                        {
                            Alias::Constant(Constant::Integer(alias_bits(bits, ty)))
                        }
                        other => other,
                    })
                }
                I::Store(_, _, _, Value::Slot(slot))
                    if matches!(slots[slot.index()], Slot::Dead | Slot::Promoted(_)) =>
                {
                    facts.active[pos] = false;
                    None
                }
                I::Convert(op, to, from, value) => {
                    let source = facts.resolve(body, value);
                    if let Alias::Temporary(temp) = source {
                        let ints = (to.bits() != 0 || to == Type::Ptr)
                            && (from.bits() != 0 || from == Type::Ptr)
                            && to != Type::I128
                            && from != Type::I128;
                        let boolean = Self::known_boolean(body, temp, 0)
                            && !(op == Convert::SignExtend && from == Type::I1);
                        let extension = (op == Convert::ZeroExtend
                            && matches!(from, Type::U8 | Type::U16 | Type::U32 | Type::I1)
                            || op == Convert::SignExtend
                                && matches!(from, Type::I8 | Type::I16 | Type::I32))
                            && alias_compatible(to, from);
                        if ints && (boolean || extension) {
                            Some(source)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                I::Index(_, _, base, offset) => {
                    let off = facts.resolve(body, offset);
                    let base = facts.resolve(body, base);
                    if matches!(off,Alias::Constant(Constant::Integer(bits)) if bits & u128::from(u64::MAX)==0)
                        && matches!(base,Alias::Temporary(t) if body.temp(t).ty==Some(Type::Ptr))
                    {
                        Some(base)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(result) = inst.result {
                if let Some(alias) = alias {
                    facts.identities[result.index()] =
                        facts.alias_identity(body, slots, &inst.kind, result, alias);
                }
                facts.aliases[result.index()] = alias;
                if alias.is_some() {
                    facts.active[pos] = false;
                }
            }
        }
        let mut uses = vec![0_u32; body.temps().len()];
        for (pos, &id) in ids.iter().enumerate() {
            if facts.active[pos] {
                body.visit_values(&body.instruction(id).kind, |v| {
                    if let Alias::Temporary(temp) = facts.resolve(body, v) {
                        uses[temp.index()] += 1;
                    }
                });
            }
        }
        for (pos, &id) in ids.iter().enumerate().rev() {
            if !facts.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            let pure = matches!(
                inst.kind,
                I::Copy(..)
                    | I::Phi(..)
                    | I::Unary(..)
                    | I::Binary(..)
                    | I::Addr(..)
                    | I::Index(..)
                    | I::Compare(..)
                    | I::Convert(..)
                    | I::Select(..)
                    | I::Runtime(Runtime::ExceptionSelector(_) | Runtime::StackAlloc(_))
            ) || matches!(inst.kind,I::Load(flags,_,_) if !flags.volatile());
            if pure && inst.result.is_some_and(|r| uses[r.index()] == 0) {
                facts.active[pos] = false;
                body.visit_values(&inst.kind, |v| {
                    if let Alias::Temporary(temp) = facts.resolve(body, v) {
                        uses[temp.index()] = uses[temp.index()].saturating_sub(1);
                    }
                });
            }
        }
        let entry = body.layout[0];
        for index in 0..body.parameter_count {
            facts.global[index] = !facts.flow.predecessors(entry).is_empty();
        }
        for (pos, &id) in ids.iter().enumerate() {
            if !facts.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            if matches!(inst.kind, I::Phi(..))
                && let Some(result) = inst.result
            {
                facts.global[result.index()] = true;
            }
            if let I::Phi(_, pairs) = &inst.kind {
                for &(pred, value) in pairs {
                    if facts.flow.reachable[pred.index()] {
                        let at = facts.flow.ranges[pred.index()].end - 1;
                        facts.note_use(body, value, at, pos);
                    }
                }
            } else {
                body.visit_values(&inst.kind, |value| facts.note_use(body, value, pos, pos));
            }
        }
        facts.refresh_clobbers(body)?;
        Ok(facts)
    }
    fn note_use(&mut self, body: &Body, value: Value, at: usize, consumer: usize) {
        if let Alias::Temporary(temp) = self.resolve(body, value) {
            let def_block =
                self.definitions[temp.index()].map_or(body.layout[0], |p| self.flow.blocks[p]);
            let used_block = self.flow.blocks[at];
            let interval = &mut self.intervals[temp.index()];
            interval.uses += 1;
            interval.consumer = Some(consumer);
            if def_block == used_block
                && !self.global[temp.index()]
                && self.definitions[temp.index()].is_none_or(|p| p <= at)
            {
                interval.last = Some(interval.last.map_or(at, |last| last.max(at)));
            } else {
                self.global[temp.index()] = true;
            }
        }
    }
    fn alias_identity(
        &self,
        body: &Body,
        slots: &[Slot],
        kind: &I,
        result: TempId,
        alias: Alias,
    ) -> Option<Identity> {
        let source = match *kind {
            I::Const(_, v) | I::Copy(_, v) | I::Convert(_, _, _, v) | I::Index(_, _, v, _) => v,
            I::Load(_, _, Value::Slot(slot)) => {
                let Slot::Promoted(v) = slots[slot.index()] else {
                    return None;
                };
                v
            }
            _ => return None,
        };
        Some(
            if matches!((self.resolve(body,source),alias),(Alias::Constant(Constant::Integer(before)),Alias::Constant(Constant::Integer(after))) if before!=after)
            {
                Identity::Normalized(result)
            } else {
                self.identity(source)
            },
        )
    }
    fn identity(&self, value: Value) -> Identity {
        if let Value::Temp(t) = value {
            self.identities[t.index()].unwrap_or(Identity::Original(value))
        } else {
            Identity::Original(value)
        }
    }
    pub(super) fn same_value(&self, left: Value, right: Value) -> bool {
        self.identity(left) == self.identity(right)
    }
    pub(super) fn known_boolean(body: &Body, temp: TempId, depth: u8) -> bool {
        if temp.index() < body.parameter_count {
            return false;
        }
        let Some(def) = body.temp(temp).definition else {
            return false;
        };
        let inst = body.instruction(def);
        if let I::Select(_, _, left, right) = inst.kind
            && depth < 4
        {
            return [left, right].into_iter().all(|v| match v {
                Value::Integer(id) => body.literal(id) <= 1,
                Value::Temp(t) => Self::known_boolean(body, t, depth + 1),
                _ => false,
            });
        }
        matches!(
            inst.kind,
            I::Compare(..) | I::Runtime(Runtime::AtomicCompareExchange { .. })
        ) || body.temp(temp).ty == Some(Type::I1)
            && !matches!(
                inst.kind,
                I::Call(..) | I::Load(..) | I::Runtime(Runtime::AtomicLoad(..))
            )
    }
    pub(super) fn sunk_compare(&self, body: &Body, pos: usize) -> bool {
        let inst = body.instruction(self.flow.instructions[pos]);
        let Some(result) = inst.result else {
            return false;
        };
        let (I::Compare(_, ty, ..) | I::Unary(crate::lowir::vocabulary::Unary::Not, ty, _)) =
            inst.kind
        else {
            return false;
        };
        let Some(reader) = self.intervals[result.index()].consumer else {
            return false;
        };
        let local = !self.global[result.index()] && self.intervals[result.index()].uses == 1;
        let later = reader > pos + 1 && self.flow.blocks[reader] == self.flow.blocks[pos];
        let branch = body.instruction(self.flow.instructions[reader]);
        ty.float_rank() == 0
            && ty != Type::I128
            && local
            && later
            && matches!(branch.kind, I::Branch(value, ..)
                if matches!(self.resolve(body, value), Alias::Temporary(t) if t == result))
    }
    pub(super) fn compare_scratch(&self, body: &Body, pos: usize) -> bool {
        let inst = body.instruction(self.flow.instructions[pos]);
        let I::Compare(_, ty, a, b) = inst.kind else {
            return false;
        };
        let Some(result) = inst.result else {
            return false;
        };
        if ty.float_rank() != 0
            || ty == Type::I128
            || !self.active[pos]
            || self.fused[result.index()]
        {
            return false;
        }
        if !matches!(self.resolve(body, a), Alias::Constant(Constant::Integer(_)))
            && !matches!(self.resolve(body, b), Alias::Constant(Constant::Integer(_)))
        {
            return false;
        }
        let interval = &self.intervals[result.index()];
        if interval.uses == 1
            && matches!(ty, Type::I32 | Type::U32 | Type::I64 | Type::Ptr)
            && let Some(consumer) = interval.consumer
            && matches!(body.instruction(self.flow.instructions[consumer]).kind,I::Return(_,Some(Value::Temp(t))) if t==result)
        {
            return false;
        }
        true
    }
    pub(super) fn refresh_clobbers(&mut self, body: &Body) -> Result<(), Error> {
        self.rebuild_clobbers(body, None)
    }
    pub(super) fn apply_calls(
        &mut self,
        body: &Body,
        calls: &super::call_facts::Calls,
    ) -> Result<(), Error> {
        self.rebuild_clobbers(body, Some(calls))
    }
    fn call_hints(
        &mut self,
        body: &Body,
        call: &crate::lowir::model::Call,
        plan: &super::call_facts::CallPlan,
        pos: usize,
    ) {
        for (arg, layout) in body
            .arguments(call.arguments)
            .iter()
            .zip(&plan.layout.arguments)
        {
            let Alias::Temporary(temp) = self.resolve(body, *arg) else {
                continue;
            };
            if temp.index() < body.parameter_count {
                continue;
            }
            let interval = &mut self.intervals[temp.index()];
            let address = body
                .temp(temp)
                .definition
                .is_some_and(|def| matches!(body.instruction(def).kind, I::Addr(_)));
            if (interval.uses == 1 || address)
                && interval.last == Some(pos)
                && interval.hint.is_none()
                && let [part] = plan.layout.parts(layout)
            {
                interval.hint = part.register;
            }
        }
    }
    fn rebuild_clobbers(
        &mut self,
        body: &Body,
        calls: Option<&super::call_facts::Calls>,
    ) -> Result<(), Error> {
        let ids = self.flow.instructions.clone();
        for interval in &mut self.intervals {
            interval.forbid = 0;
        }
        let mut late = [None; 32];
        let mut early = [None; 32];
        let mask = |interval: &Interval,
                    late: &[Option<usize>; 32],
                    early: &[Option<usize>; 32]| {
            let mut out = 0_u32;
            for reg in 0..32 {
                if interval.last.is_some_and(|last| {
                    late[reg].is_some_and(|at| at < last) || early[reg].is_some_and(|at| at <= last)
                }) {
                    out |= 1 << reg;
                }
            }
            out
        };
        for (pos, &id) in ids.iter().enumerate().rev() {
            if pos + 1 == ids.len() || self.flow.blocks[pos] != self.flow.blocks[pos + 1] {
                late = [None; 32];
                early = [None; 32];
            }
            if !self.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            if let Some(result) = inst.result {
                self.intervals[result.index()].forbid =
                    mask(&self.intervals[result.index()], &late, &early);
            }
            let fixed = if let I::Binary(
                op @ (Binary::Div | Binary::Mod | Binary::UDiv | Binary::UMod),
                _,
                ty,
                _,
                right,
            ) = inst.kind
                && ty.float_rank() == 0
                && ty != Type::I128
            {
                super::division::Form::at(self, body, op, right).clobbers()
            } else if let I::CopyObject(bytes, align, ..) = inst.kind {
                if super::bulk::direct_copy(bytes, super::bulk::alignment(align)) {
                    0
                } else {
                    (1 << R::RDI.code()) | (1 << R::RSI.code()) | (1 << R::RCX.code())
                }
            } else if let I::ZeroInit(bytes, ..) = inst.kind {
                if super::bulk::direct_zero(bytes) {
                    0
                } else {
                    (1 << R::RDI.code()) | (1 << R::RAX.code()) | (1 << R::RCX.code())
                }
            } else if let I::Load(_, ty @ Type::Object { .. }, _)
            | I::Store(_, ty @ Type::Object { .. }, _, _)
            | I::Copy(ty @ Type::Object { .. }, _) = inst.kind
            {
                super::bulk::object_clobbers(ty)?
            } else if self.compare_scratch(body, pos) {
                1 << R::RDX.code()
            } else if matches!(inst.kind,I::Binary(Binary::Shl|Binary::Shr|Binary::UShr,_,_,_,count) if !matches!(self.resolve(body,count),Alias::Constant(Constant::Integer(_))))
            {
                1 << R::RCX.code()
            } else {
                0
            };
            let wide_multiply = matches!(inst.kind, I::Binary(Binary::Mul, _, Type::I128, ..));
            let fixed = if wide_multiply {
                (1 << R::RAX.code()) | (1 << R::RDX.code())
            } else {
                fixed
            };
            let call = calls.and_then(|c| c.at(pos));
            let runtime_call = matches!(
                inst.kind,
                I::Runtime(
                    Runtime::Throw(..)
                        | Runtime::Resume
                        | Runtime::EhCatch(..)
                        | Runtime::EhCatchAll(_)
                )
            );
            let runtime_call = runtime_call
                || matches!(
                    inst.kind,
                    I::Binary(
                        Binary::Div | Binary::Mod | Binary::UDiv | Binary::UMod,
                        _,
                        Type::I128,
                        ..
                    )
                );
            let runtime_call = runtime_call
                || matches!(inst.kind, I::Convert(_, Type::I128, from, _) if from.float_rank()!=0)
                || matches!(inst.kind, I::Convert(_, to, Type::I128, _) if to.float_rank()!=0);
            let clobber = fixed
                | if call.is_some() || runtime_call {
                    super::abi::CALLER_SAVED | 0xffff_0000
                } else {
                    0
                };
            let before = (if wide_multiply { 0 } else { fixed }) | call.map_or(0, |c| c.early);
            self.clobbers[pos] = (clobber, before);
            for reg in 0..32 {
                if clobber & (1 << reg) != 0 {
                    late[reg] = Some(pos);
                }
                if before & (1 << reg) != 0 {
                    early[reg] = Some(pos);
                }
            }
            if let I::Return(_, Some(value)) = inst.kind
                && let Alias::Temporary(temp) = self.resolve(body, value)
            {
                let two = body
                    .temp(temp)
                    .definition
                    .is_some_and(|def| matches!(body.instruction(def).kind, I::Binary(..)));
                if (!two || body.temp(temp).ty.is_some_and(|ty| ty.float_rank() != 0))
                    && self.intervals[temp.index()].last == Some(pos)
                {
                    self.intervals[temp.index()].hint = Some(
                        if matches!(body.temp(temp).ty, Some(Type::F32 | Type::F64)) {
                            R::XMM0
                        } else {
                            R::RAX
                        },
                    );
                }
            }
            if let Some(plan) = call
                && let I::Call(call) = &inst.kind
            {
                self.call_hints(body, call, plan, pos);
            }
        }
        for interval in self.intervals.iter_mut().take(body.parameter_count) {
            interval.forbid = mask(interval, &late, &early);
        }
        Ok(())
    }
}
fn alias_bits(bits: u128, ty: Type) -> u128 {
    if matches!(
        ty,
        Type::I1 | Type::I8 | Type::U8 | Type::I16 | Type::U16 | Type::I32 | Type::U32
    ) {
        normalize(bits, ty)
    } else {
        bits
    }
}

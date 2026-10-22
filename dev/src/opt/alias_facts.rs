//! Function-owned alias and content facts from `opt/memory_forwarding.cpp`. See NOTICE.
use super::memory_contents::bytes;
use crate::lowir::{
    Error,
    model::{Body, InstructionKind as I, Program, RuntimeInstruction as R, Value},
    vocabulary::{Effects, Type},
};
use std::collections::HashMap;

pub(super) fn signed_word(bits: u128) -> i64 {
    let b = bits.to_le_bytes();
    i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}
pub(super) fn signed_size(bytes: u64) -> i64 {
    i64::from_le_bytes(bytes.to_le_bytes())
}
pub(super) fn literal_integer(value: i64) -> u128 {
    u128::from_le_bytes(i128::from(value).to_le_bytes())
}
pub(super) fn writes(program: &Program, kind: &I) -> bool {
    match kind {
        I::Call(call) => program.call_boundary(call).is_none_or(|s| {
            !matches!(
                s.boundary.effects,
                Some(Effects::ReadNone | Effects::ReadOnly)
            )
        }),
        I::Runtime(R::StackAlloc(_)) => false,
        I::Store(..) | I::CopyObject(..) | I::ZeroInit(..) | I::Runtime(_) => true,
        _ => false,
    }
}
pub(super) fn marker(kind: &I) -> bool {
    matches!(kind, I::Runtime(R::EhTry(_) | R::EhCleanup(_) | R::EhEnd))
}
pub(super) fn eh(kind: &I) -> bool {
    matches!(
        kind,
        I::Runtime(
            R::EhTry(_)
                | R::EhCleanup(_)
                | R::EhEnd
                | R::EhCatch(..)
                | R::EhCatchAll(_)
                | R::EhFilter(..)
                | R::Throw(..)
                | R::Resume
                | R::Exception(_)
                | R::ExceptionSelector(_)
        )
    )
}
pub(super) fn twin(ty: Type) -> Type {
    match ty {
        Type::I8 => Type::U8,
        Type::U8 => Type::I8,
        Type::I16 => Type::U16,
        Type::U16 => Type::I16,
        Type::I32 => Type::U32,
        Type::U32 => Type::I32,
        Type::I64 => Type::Ptr,
        Type::Ptr => Type::I64,
        _ => ty,
    }
}
pub(super) fn integral(ty: Type) -> bool {
    ty == Type::Ptr || ty.bits() != 0 && ty != Type::I128
}
pub(super) fn exact(body: &Body, value: Value, ty: Type) -> bool {
    match value {
        Value::Integer(_) => ty.float_rank() == 0,
        Value::Float(..) => true,
        Value::Temp(t) => body.temp(t).ty == Some(ty),
        _ => false,
    }
}
pub(super) fn typed_apart(typed_a: bool, a: Type, typed_b: bool, b: Type) -> bool {
    let wide = |t| {
        !matches!(
            t,
            Type::Void | Type::I1 | Type::I8 | Type::U8 | Type::Object { .. }
        )
    };
    typed_a && typed_b && (a == Type::Ptr) != (b == Type::Ptr) && wide(a) && wide(b)
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Address {
    pub(super) base: Option<Value>,
    pub(super) offset: i64,
}
impl Address {
    pub(super) fn same_base(self, other: Self) -> bool {
        self.base == other.base
    }
    pub(super) fn shifted(mut self, offset: i64) -> Self {
        self.offset = self.offset.wrapping_add(offset);
        self
    }
}
pub(super) struct Addresses {
    pub(super) values: Vec<Address>,
    pub(super) escaped: Vec<bool>,
}
impl Addresses {
    pub(super) fn analyze(body: &Body) -> Self {
        let mut out = Self {
            values: vec![Address::default(); body.temps().len()],
            escaped: vec![false; body.slots().len()],
        };
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                out.analyze_instruction(body, body.instruction(id));
            }
        }
        for &block in &body.layout {
            for &id in body
                .block(block)
                .instructions
                .iter()
                .take_while(|&&id| matches!(body.instruction(id).kind, I::Phi(..)))
            {
                body.visit_values(&body.instruction(id).kind, |v| {
                    if let Some(Value::Slot(slot)) = out.facts(body, Some(v)).base {
                        out.escaped[slot.index()] = true;
                    }
                });
            }
        }
        out
    }
    fn analyze_instruction(&mut self, body: &Body, inst: &crate::lowir::model::Instruction) {
        match inst.kind {
            I::Addr(value) | I::Copy(Type::Ptr, value) if inst.result.is_some() => {
                let at = self.facts(body, Some(value));
                if let Some(result) = inst.result {
                    self.values[result.index()] = at;
                }
                return;
            }
            I::Index(ty, _, base, offset) if inst.result.is_some() => {
                let at = self.facts(body, Some(base));
                if at.base.is_some()
                    && let Value::Integer(offset) = offset
                {
                    if let Some(result) = inst.result {
                        self.values[result.index()] = at.shifted(
                            signed_word(body.literal(offset)).wrapping_mul(signed_size(bytes(ty))),
                        );
                    }
                    return;
                }
                if let Some(Value::Slot(slot)) = at.base {
                    self.escaped[slot.index()] = true;
                }
                return;
            }
            I::Phi(..) => return,
            _ => (),
        }
        let mut position = 0;
        body.visit_values(&inst.kind, |v| {
            let at = position;
            position += 1;
            if matches!(inst.kind, I::Load(..)) && at == 0
                || matches!(inst.kind, I::Store(..)) && at == 1
                || matches!(inst.kind, I::CopyObject(..) | I::ZeroInit(..))
            {
                return;
            }
            let f = self.facts(body, Some(v));
            if let Some(Value::Slot(slot)) = f.base
                && (matches!(v, Value::Temp(_)) || !matches!(inst.kind, I::Load(..)))
            {
                self.escaped[slot.index()] = true;
            }
        });
    }
    pub(super) fn facts(&self, body: &Body, value: Option<Value>) -> Address {
        match value {
            Some(v @ (Value::Slot(_) | Value::Symbol(_))) => Address {
                base: Some(v),
                offset: 0,
            },
            Some(v @ Value::Temp(temp)) if temp.index() < self.values.len() => {
                let f = self.values[temp.index()];
                if f.base.is_some() || body.temp(temp).ty != Some(Type::Ptr) {
                    f
                } else {
                    Address {
                        base: Some(v),
                        offset: 0,
                    }
                }
            }
            _ => Address::default(),
        }
    }
    pub(super) fn private(&self, at: Address) -> bool {
        matches!(at.base,Some(Value::Slot(slot)) if !self.escaped[slot.index()])
    }
    pub(super) fn may_alias(&self, a: Address, a_bytes: u64, b: Address, b_bytes: u64) -> bool {
        if let (Some(base_a), Some(base_b)) = (a.base, b.base) {
            if base_a == base_b {
                return a.offset < b.offset.wrapping_add(signed_size(b_bytes))
                    && b.offset < a.offset.wrapping_add(signed_size(a_bytes));
            }
            if !matches!(base_a, Value::Temp(_)) && !matches!(base_b, Value::Temp(_)) {
                return false;
            }
            return !self.private(a) && !self.private(b);
        }
        !self.private(a) && !self.private(b)
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Key {
    pub(super) base: Option<Value>,
    pub(super) offset: i64,
    pub(super) ty: Type,
}
impl Key {
    pub(super) fn new(at: Address, value: Option<Value>, ty: Type) -> Self {
        Self {
            base: at.base.or(value),
            offset: if at.base.is_some() { at.offset } else { 0 },
            ty,
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Known {
    pub(super) value: Option<Value>,
    pub(super) generation: u32,
    pub(super) typed: bool,
    pub(super) ty: Type,
    pub(super) at: Address,
    pub(super) bytes: u64,
}
#[derive(Clone, Copy)]
pub(super) struct ZeroRange {
    pub(super) base: Address,
    pub(super) start: i64,
    pub(super) end: i64,
}
#[derive(Clone, Copy)]
struct FactId(u32);
impl FactId {
    fn index(self) -> usize {
        self.0 as usize
    }
}
pub(super) struct Contents {
    index: HashMap<Key, FactId>,
    facts: Vec<Known>,
    undo: Vec<(FactId, Known)>,
    live: Vec<FactId>,
    zeros: Vec<ZeroRange>,
    generation: u32,
    clock: u32,
    saved_live: Vec<FactId>,
    saved_zeros: Vec<ZeroRange>,
}
#[derive(Clone, Copy)]
pub(super) struct Saved {
    undo: usize,
    generation: u32,
    live: usize,
    zeros: usize,
}
impl Contents {
    pub(super) fn new() -> Self {
        Self {
            index: HashMap::new(),
            facts: Vec::new(),
            undo: Vec::new(),
            live: Vec::new(),
            zeros: Vec::new(),
            generation: 1,
            clock: 1,
            saved_live: Vec::new(),
            saved_zeros: Vec::new(),
        }
    }
    pub(super) fn save(&mut self) -> Saved {
        let saved = Saved {
            undo: self.undo.len(),
            generation: self.generation,
            live: self.saved_live.len(),
            zeros: self.saved_zeros.len(),
        };
        self.saved_live.extend_from_slice(&self.live);
        self.saved_zeros.extend_from_slice(&self.zeros);
        saved
    }
    pub(super) fn restore(&mut self, saved: Saved) {
        for (id, fact) in self.undo.drain(saved.undo..).rev() {
            self.facts[id.index()] = fact;
        }
        self.generation = saved.generation;
        self.live.clear();
        self.live.extend_from_slice(&self.saved_live[saved.live..]);
        self.saved_live.truncate(saved.live);
        self.zeros.clear();
        self.zeros
            .extend_from_slice(&self.saved_zeros[saved.zeros..]);
        self.saved_zeros.truncate(saved.zeros);
    }
    pub(super) fn clear(&mut self) {
        self.clock = self.clock.wrapping_add(1);
        self.generation = self.clock;
        self.live.clear();
        self.zeros.clear();
    }
    pub(super) fn is_empty(&self) -> bool {
        self.live.is_empty() && self.zeros.is_empty()
    }
    pub(super) fn find(&self, key: Key) -> Option<Known> {
        self.index
            .get(&key)
            .map(|id| self.facts[id.index()])
            .filter(|k| k.generation == self.generation)
    }
    pub(super) fn live(&self) -> impl Iterator<Item = Known> + '_ {
        self.live
            .iter()
            .map(|id| self.facts[id.index()])
            .filter(|k| k.generation == self.generation)
    }
    pub(super) fn zeros(&self) -> &[ZeroRange] {
        &self.zeros
    }
    pub(super) fn remember(
        &mut self,
        at: Address,
        address: Value,
        ty: Type,
        value: Value,
        typed: bool,
    ) -> Result<(), Error> {
        let key = Key::new(at, Some(address), ty);
        let entry = self.index.get(&key).copied();
        let live = entry.is_some_and(|id| self.facts[id.index()].generation == self.generation);
        if !live && self.live.len() >= 64 {
            self.clear();
        }
        let fact = Known {
            value: Some(value),
            generation: self.generation,
            typed,
            ty,
            at,
            bytes: bytes(ty),
        };
        let id = if let Some(id) = entry {
            self.undo.push((id, self.facts[id.index()]));
            self.facts[id.index()] = fact;
            id
        } else {
            let id = FactId(u32::try_from(self.facts.len()).map_err(|_| Error::Limit)?);
            self.undo.push((
                id,
                Known {
                    value: None,
                    generation: 0,
                    typed: false,
                    ty,
                    at: Address::default(),
                    bytes: 0,
                },
            ));
            self.facts.push(fact);
            self.index.insert(key, id);
            id
        };
        if !live {
            self.live.push(id);
        }
        Ok(())
    }
    pub(super) fn known_zero(&self, at: Address, size: u64) -> bool {
        at.base.is_some()
            && self.zeros.iter().any(|r| {
                r.base.same_base(at)
                    && r.start <= at.offset
                    && at.offset.wrapping_add(signed_size(size)) <= r.end
            })
    }
    pub(super) fn note_zero(&mut self, mut at: Address, size: u64) {
        if at.base.is_none() || size == 0 || size >= 1_u64 << 32 {
            return;
        }
        if self.zeros.len() >= 16 {
            self.zeros.remove(0);
        }
        let start = at.offset;
        at.offset = 0;
        self.zeros.push(ZeroRange {
            base: at,
            start,
            end: start.wrapping_add(signed_size(size)),
        });
    }
    pub(super) fn invalidate(
        &mut self,
        addresses: &Addresses,
        at: Address,
        size: u64,
        writer: Option<(bool, Type)>,
    ) {
        let mut kept = 0;
        for index in 0..self.live.len() {
            let id = self.live[index];
            let found = self.facts[id.index()];
            if found.generation != self.generation {
                continue;
            }
            let apart = writer.is_some_and(|(typed, ty)| {
                typed_apart(typed, ty, found.typed, found.ty)
                    && !(at.base.is_some() && at.same_base(found.at))
            });
            if !apart && addresses.may_alias(at, size, found.at, found.bytes) {
                self.undo.push((id, found));
                self.facts[id.index()].generation = 0;
                continue;
            }
            self.live[kept] = id;
            kept += 1;
        }
        self.live.truncate(kept);
        let mut zeros = Vec::with_capacity(self.zeros.len());
        for &range in &self.zeros {
            let mut place = range.base;
            place.offset = range.start;
            if !addresses.may_alias(
                at,
                size,
                place,
                u64::from_le_bytes(range.end.wrapping_sub(range.start).to_le_bytes()),
            ) {
                zeros.push(range);
            } else if at.same_base(range.base) && size < 1_u64 << 32 {
                let end = at.offset.wrapping_add(signed_size(size));
                if range.start < at.offset {
                    zeros.push(ZeroRange {
                        end: at.offset,
                        ..range
                    });
                }
                if end < range.end {
                    zeros.push(ZeroRange {
                        start: end,
                        ..range
                    });
                }
            }
        }
        self.zeros = zeros;
    }
}

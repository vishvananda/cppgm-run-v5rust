//! Scalar identities and address/call folding from `opt/value_simplify.cpp`.
//! See NOTICE.
use super::{
    memory_contents::{self as memory, LibraryCall, bytes},
    numeric_fold as fold,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{
        Body, DataItem, Facts, GlobalInit, InstId, Instruction, InstructionKind as I, Program,
        Value,
    },
    vocabulary::{Binary as B, Compare as C, Convert as X, Type, Unary},
};

fn low_word(bits: u128) -> u64 {
    let bytes = bits.to_le_bytes();
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

pub(super) fn producer(body: &Body, value: Value) -> Option<(InstId, &Instruction)> {
    let Value::Temp(temp) = value else {
        return None;
    };
    if body.temp_is_parameter(temp) {
        return None;
    }
    let id = body.temp(temp).definition?;
    Some((id, body.instruction(id)))
}
pub(super) fn commutative(kind: &I) -> bool {
    matches!(kind,I::Binary(B::Add|B::Mul|B::And|B::Or|B::Xor,_,ty,..) if ty.bits()!=0)
        || matches!(kind, I::Compare(C::Eq | C::Ne, ..))
}
pub(super) fn canonicalize(kind: &mut I) {
    let commutes = commutative(kind);
    match kind {
        I::Binary(_, _, _, left, right) | I::Compare(_, _, left, right) => {
            let literal_first = matches!(left, Value::Integer(_) | Value::Float(..))
                && !matches!(right, Value::Integer(_) | Value::Float(..));
            let reverse = matches!((*left,*right),(Value::Temp(left),Value::Temp(right)) if left.index()>right.index())
                && commutes;
            if literal_first || reverse {
                if commutes {
                    std::mem::swap(left, right);
                } else if let I::Compare(pred, _, left, right) = kind {
                    std::mem::swap(left, right);
                    *pred = pred.swapped();
                }
            }
        }
        _ => (),
    }
}
pub(super) struct Algebra<'left> {
    pub(super) program: &'left Program,
    pub(super) body: &'left mut Body,
    pub(super) unwritten: &'left [bool],
    pub(super) uses: &'left mut [u32],
    pub(super) nonzero: &'left [bool],
    pub(super) changed: bool,
}
impl Algebra<'_> {
    fn constant(&self, v: Value) -> Option<Constant> {
        fold::literal(self.body, v)
    }
    fn known_nonzero(&self, v: Value) -> bool {
        matches!(v,Value::Temp(defined_type) if self.nonzero.get(defined_type.index()).copied().unwrap_or(false))
    }
    fn literal(&mut self, c: Constant) -> Result<Value, Error> {
        self.body.constant(c)
    }
    pub(super) fn boolean(&self, v: Value, depth: u8) -> bool {
        if let Some(c) = self.constant(v) {
            return matches!(c,Constant::Integer(bits) if bits<=1);
        }
        let Some((_, def)) = producer(self.body, v) else {
            return false;
        };
        if depth > 3 {
            return false;
        }
        match &def.kind {
            I::Compare(..) | I::Unary(Unary::Not, ..) => true,
            I::Convert(X::ZeroExtend | X::Truncate, _, _, v) => self.boolean(*v, depth + 1),
            I::Phi(_, pairs) => pairs.iter().all(|&(_, v)| {
                Some(v) == def.result.map(Value::Temp) || self.boolean(v, depth + 1)
            }),
            _ => false,
        }
    }
    pub(super) fn fold(&mut self, kind: &I) -> Result<Option<Value>, Error> {
        let value = match *kind {
            I::Const(ty, v) | I::Copy(ty, v) => self.constant(v).and_then(|c| fold::copy(ty, c)),
            I::Unary(op, ty, v) => self.constant(v).and_then(|c| fold::unary(op, ty, c)),
            I::Binary(op, _, ty, left, right) => self
                .constant(left)
                .zip(self.constant(right))
                .and_then(|(left, right)| fold::binary(op, ty, left, right)),
            I::Compare(op, ty, left, right) => self
                .constant(left)
                .zip(self.constant(right))
                .and_then(|(left, right)| fold::compare(op, ty, left, right)),
            I::Convert(op, to, from, v) => self
                .constant(v)
                .and_then(|c| fold::convert(op, to, from, c)),
            _ => None,
        };
        value.map(|c| self.literal(c)).transpose()
    }
    pub(super) fn identity(&mut self, kind: &mut I) -> Result<Option<Value>, Error> {
        match *kind {
            I::Select(_, cond, yes, no) => {
                if let Some(Constant::Integer(bits)) = self.constant(cond) {
                    return Ok(Some(if bits != 0 { yes } else { no }));
                }
                Ok((yes == no).then_some(yes))
            }
            I::Copy(ty, value) => self.copy_identity(ty, value),
            I::Index(_, _, base, offset) => Ok(
                (matches!(base,Value::Temp(defined_type) if self.body.temp(defined_type).ty==Some(Type::Ptr))
                    && self
                        .constant(offset)
                        .is_some_and(|c| fold::is_integer(Type::I64, c, 0)))
                .then_some(base),
            ),
            I::Load(..) => self.readonly_load(kind),
            I::Call(_) => self.known_strlen(kind),
            I::Binary(..) => self.binary_identity(kind),
            I::Compare(..) => self.compare_identity(kind),
            I::Convert(..) => Ok(self.convert_identity(kind)),
            _ => Ok(None),
        }
    }
    fn copy_identity(&mut self, ty: Type, v: Value) -> Result<Option<Value>, Error> {
        let Value::Temp(temp) = v else {
            return Ok(None);
        };
        if self.body.temp(temp).ty == Some(ty) {
            return Ok(Some(v));
        }
        if self.uses.get(temp.index()) != Some(&1) {
            return Ok(None);
        }
        let Some((id, def)) = producer(self.body, v) else {
            return Ok(None);
        };
        let I::Load(facts, from, _) = def.kind else {
            return Ok(None);
        };
        if facts.volatile()
            || ty.bits() == 0
            || from.bits() == 0
            || bytes(from) != bytes(ty)
            || ty == Type::I128
            || from == Type::I128
        {
            return Ok(None);
        }
        self.body.retype_load(id, ty)?;
        self.uses[temp.index()] = u32::MAX;
        Ok(Some(v))
    }
    pub(super) fn readonly_base(&self, address: Value) -> Option<(Value, i64)> {
        let mut base = address;
        let mut offset = 0_i64;
        for _ in 0..8 {
            if !matches!(base, Value::Temp(_)) {
                break;
            }
            let (_, def) = producer(self.body, base)?;
            match def.kind {
                I::Addr(v) | I::Copy(Type::Ptr, v) => base = v,
                I::Index(ty, _, v, Value::Integer(id)) => {
                    let bits = u64::try_from(self.body.literal(id) & u128::from(u64::MAX)).ok()?;
                    offset = offset.wrapping_add(
                        i64::from_le_bytes(bits.to_le_bytes())
                            .wrapping_mul(i64::from_le_bytes(bytes(ty).to_le_bytes())),
                    );
                    base = v;
                }
                _ => return None,
            }
        }
        matches!(base, Value::Symbol(_)).then_some((base, offset))
    }
    fn readonly_load(&mut self, kind: &I) -> Result<Option<Value>, Error> {
        let I::Load(facts, ty, address) = *kind else {
            return Ok(None);
        };
        if facts.volatile() || !(ty.bits() != 0 || ty == Type::Ptr) {
            return Ok(None);
        }
        let Some((Value::Symbol(base), offset)) = self.readonly_base(address) else {
            return Ok(None);
        };
        if ty == Type::Ptr {
            return Ok(
                memory::address(self.program, base, offset, self.unwritten, false)
                    .map(|(s, _)| Value::Symbol(s)),
            );
        }
        memory::integer(self.program, base, offset, ty, self.unwritten)
            .map(|bits| self.literal(fold::integer(ty, bits)))
            .transpose()
    }
    pub(super) fn readonly_table_address(&mut self, kind: &mut I) -> Result<bool, Error> {
        let I::Load(facts, Type::Ptr, address) = *kind else {
            return Ok(false);
        };
        if facts.volatile() {
            return Ok(false);
        }
        let Some((Value::Symbol(base), offset)) = self.readonly_base(address) else {
            return Ok(false);
        };
        let Some((target, addend)) =
            memory::address(self.program, base, offset, self.unwritten, true)
        else {
            return Ok(false);
        };
        if addend == 0 {
            return Ok(false);
        }
        let offset = self
            .body
            .integer(u128::from_le_bytes(i128::from(addend).to_le_bytes()))?;
        *kind = I::Index(Type::I8, None, Value::Symbol(target), offset);
        Ok(true)
    }
    fn known_strlen(&mut self, kind: &I) -> Result<Option<Value>, Error> {
        let I::Call(call) = kind else {
            return Ok(None);
        };
        let Value::Symbol(symbol) = call.callee else {
            return Ok(None);
        };
        let args = self.body.arguments(call.arguments);
        if args.len() != 1
            || call.result.bits() == 0
            || !memory::library(self.program, symbol, LibraryCall::Strlen)
        {
            return Ok(None);
        }
        let mut value = args[0];
        for _ in 0..4 {
            if !matches!(value, Value::Temp(_)) {
                break;
            }
            let Some((_, def)) = producer(self.body, value) else {
                return Ok(None);
            };
            match def.kind {
                I::Addr(v) | I::Copy(Type::Ptr, v) => value = v,
                _ => return Ok(None),
            }
        }
        let Value::Symbol(symbol) = value else {
            return Ok(None);
        };
        let Some(global) = memory::contents(self.program, symbol, self.unwritten) else {
            return Ok(None);
        };
        let GlobalInit::Data(items) = &global.init else {
            return Ok(None);
        };
        if items.len() > 4096 {
            return Ok(None);
        }
        for (index, &item) in items.iter().enumerate() {
            match item {
                DataItem::Constant(Type::I8 | Type::U8, Constant::Integer(bits))
                    if bits & 255 != 0 => {}
                DataItem::Constant(Type::I8 | Type::U8, Constant::Integer(_))
                | DataItem::Zero(1..) => {
                    return self
                        .literal(fold::integer(
                            call.result,
                            u128::try_from(index).map_err(|_| Error::Limit)?,
                        ))
                        .map(Some);
                }
                _ => return Ok(None),
            }
        }
        Ok(None)
    }
    pub(super) fn merge_index(&mut self, kind: &mut I) -> Result<bool, Error> {
        let I::Index(ty, projection, base, Value::Integer(outer)) = *kind else {
            return Ok(false);
        };
        let Some((_, def)) = producer(self.body, base) else {
            return Ok(false);
        };
        let I::Index(inner_ty, _, inner_base, Value::Integer(inner)) = def.kind else {
            return Ok(false);
        };
        let low = |bits: u128| {
            let right = bits.to_le_bytes();
            i64::from_le_bytes([
                right[0], right[1], right[2], right[3], right[4], right[5], right[6], right[7],
            ])
        };
        let left = low(self.body.literal(inner))
            .wrapping_mul(i64::from_le_bytes(bytes(inner_ty).to_le_bytes()));
        let right =
            low(self.body.literal(outer)).wrapping_mul(i64::from_le_bytes(bytes(ty).to_le_bytes()));
        let value = self.body.integer(u128::from_le_bytes(
            i128::from(left.wrapping_add(right)).to_le_bytes(),
        ))?;
        *kind = I::Index(Type::I8, projection, inner_base, value);
        Ok(true)
    }
    fn binary_identity(&mut self, kind: &I) -> Result<Option<Value>, Error> {
        let I::Binary(op, _, ty, x, y) = *kind else {
            return Ok(None);
        };
        if ty.bits() == 0 {
            return Ok(None);
        }
        if let Some(cy) = self.constant(y) {
            let integral = fold::integer_bits(ty, cy);
            let zero = integral == Some(0);
            let one = integral == Some(1);
            if zero
                && matches!(
                    op,
                    B::Add | B::Sub | B::Or | B::Xor | B::Shl | B::Shr | B::UShr
                )
                || one && matches!(op, B::Mul | B::Div | B::UDiv)
                || op == B::And && fold::all_ones(ty, cy)
            {
                return Ok(Some(x));
            }
            if zero && matches!(op, B::Mul | B::And) {
                return self
                    .literal(fold::integer(
                        ty,
                        match cy {
                            Constant::Integer(bits) => bits,
                            Constant::Float(..) => 0,
                        },
                    ))
                    .map(Some);
            }
            if op == B::Or && fold::all_ones(ty, cy) {
                return Ok(Some(y));
            }
            if op == B::And
                && ty != Type::I128
                && let Some(bits) = integral
            {
                let possible = u128::from(self.possible_bits(x, ty, 0));
                if possible & !bits & u128::from(u64::MAX) == 0 {
                    return Ok(Some(x));
                }
                if possible & bits == 0 {
                    return self.literal(fold::integer(ty, 0)).map(Some);
                }
            }
            if let Some((_, def)) = producer(self.body, x)
                && let I::Convert(X::ZeroExtend, _, from, _) = def.kind
                && op == B::UShr
                && from.bits() != 0
                && matches!(cy,Constant::Integer(bits) if bits>>64==0&&bits>=u128::from(bytes(from)*8)&&bits<u128::from(bytes(ty)*8))
            {
                return self.literal(fold::integer(ty, 0)).map(Some);
            }
        }
        if x == y && matches!(x, Value::Temp(_)) {
            if matches!(op, B::And | B::Or) {
                return Ok(Some(x));
            }
            if matches!(op, B::Sub | B::Xor) {
                return self.literal(fold::integer(ty, 0)).map(Some);
            }
        }
        Ok(None)
    }
    fn compare_identity(&mut self, kind: &mut I) -> Result<Option<Value>, Error> {
        let I::Compare(pred, ty, x, y) = *kind else {
            return Ok(None);
        };
        if ty.bits() != 0 {
            if self.known_nonzero(x)
                && self.constant(y).is_some_and(|c| fold::is_integer(ty, c, 0))
                && matches!(pred, C::Ne | C::Eq)
            {
                return self
                    .literal(fold::integer(Type::I64, u128::from(pred == C::Ne)))
                    .map(Some);
            }
            for (d, base, p) in [(x, y, pred), (y, x, pred.swapped())] {
                if let Some((_, def)) = producer(self.body, d)
                    && let I::Binary(B::Sub, _, defined_type, left, right) = def.kind
                    && defined_type == ty
                    && left == base
                    && self.known_nonzero(base)
                    && self
                        .constant(right)
                        .is_some_and(|c| fold::is_integer(ty, c, 1))
                    && matches!(p, C::Uge | C::Ugt | C::Ult | C::Ule)
                {
                    return self
                        .literal(fold::integer(
                            Type::I64,
                            u128::from(matches!(p, C::Ult | C::Ule)),
                        ))
                        .map(Some);
                }
            }
        }
        if ty.float_rank() == 0 && x == y && matches!(x, Value::Temp(_) | Value::Symbol(_)) {
            return self
                .literal(fold::integer(
                    Type::I64,
                    u128::from(matches!(pred, C::Eq | C::Le | C::Ge | C::Ule | C::Uge)),
                ))
                .map(Some);
        }
        if ty.float_rank() != 0 {
            return Ok(None);
        }
        let left_zero = self.constant(x).is_some_and(|c| fold::is_integer(ty, c, 0));
        let right = self.constant(y);
        let right_zero = right.is_some_and(|c| fold::is_integer(ty, c, 0));
        if right_zero && matches!(pred, C::Ult | C::Uge)
            || left_zero && matches!(pred, C::Ugt | C::Ule)
        {
            return self
                .literal(fold::integer(
                    Type::I64,
                    u128::from(matches!(pred, C::Uge | C::Ule)),
                ))
                .map(Some);
        }
        let Some(cy) = right else {
            return Ok(None);
        };
        if !matches!(pred, C::Eq | C::Ne) || !self.boolean(x, 0) {
            return Ok(None);
        }
        let one = fold::is_integer(ty, cy, 1);
        if !one && !right_zero {
            return Ok(None);
        }
        let mut v = x;
        while let Some((_, def)) = producer(self.body, v) {
            if let I::Convert(_, _, _, from) = def.kind {
                v = from;
            } else {
                break;
            }
        }
        if one == (pred == C::Eq) {
            return Ok(
                (matches!(v,Value::Temp(defined_type) if self.body.temp(defined_type).ty==Some(Type::I64))
                    && self.boolean(v, 0))
                .then_some(v),
            );
        }
        if let Some((_, def)) = producer(self.body, v)
            && let I::Compare(p, defined_type, left, right) = def.kind
            && defined_type.float_rank() == 0
        {
            *kind = I::Compare(p.inverted(), defined_type, left, right);
            self.changed = true;
        }
        Ok(None)
    }
    fn convert_identity(&self, kind: &I) -> Option<Value> {
        let I::Convert(op, to, from, v) = *kind else {
            return None;
        };
        if matches!(
            op,
            X::SignExtend | X::ZeroExtend | X::Truncate | X::FloatExtend | X::FloatTruncate
        ) && to == from
            && matches!(v,Value::Temp(defined_type) if self.body.temp(defined_type).ty==Some(to))
        {
            return Some(v);
        }
        if op == X::Truncate
            && let Some((_, def)) = producer(self.body, v)
            && let I::Convert(X::SignExtend | X::ZeroExtend, _, _, left) = def.kind
            && matches!(left,Value::Temp(defined_type) if self.body.temp(defined_type).ty==Some(to))
        {
            return Some(left);
        }
        None
    }
    pub(super) fn possible_bits(&self, v: Value, ty: Type, depth: u8) -> u64 {
        let width = bytes(ty) * 8;
        let all = if width >= 64 {
            u64::MAX
        } else {
            (1_u64 << width) - 1
        };
        if let Some(c) = self.constant(v) {
            return fold::integer_bits(ty, c).map_or(all, |bits| low_word(bits) & all);
        }
        let Some((_, def)) = producer(self.body, v) else {
            return all;
        };
        if depth >= 4 || ty.bits() == 0 || ty == Type::I128 || def.kind.result_type() != ty {
            return all;
        }
        if let I::Convert(X::ZeroExtend, _, from, left) = def.kind {
            return if from.bits() != 0 && from != Type::I128 && bytes(from) < bytes(ty) {
                self.possible_bits(left, from, depth + 1)
            } else {
                all
            };
        }
        if let I::Binary(op, _, _, left, right) = def.kind {
            match op {
                B::And => {
                    return self.possible_bits(left, ty, depth + 1)
                        & self.possible_bits(right, ty, depth + 1);
                }
                B::Or | B::Xor => {
                    return self.possible_bits(left, ty, depth + 1)
                        | self.possible_bits(right, ty, depth + 1);
                }
                B::Shl | B::UShr => {
                    if let Some(bits) = self.constant(right).and_then(|c| fold::integer_bits(ty, c))
                        && bits >> 64 == 0
                        && bits < u128::from(width)
                    {
                        let from = self.possible_bits(left, ty, depth + 1);
                        let shift = u32::try_from(bits).unwrap_or(0);
                        return (if op == B::Shl {
                            from << shift
                        } else {
                            from >> shift
                        }) & all;
                    }
                }
                _ => (),
            }
        }
        all
    }
    pub(super) fn mask_or(&mut self, kind: &mut I) -> bool {
        let I::Binary(B::And, facts, ty, x, y) = *kind else {
            return false;
        };
        if ty.bits() == 0 || ty == Type::I128 {
            return false;
        }
        let Some(bits) = self.constant(y).and_then(|c| fold::integer_bits(ty, c)) else {
            return false;
        };
        let Some((_, def)) = producer(self.body, x) else {
            return false;
        };
        let I::Binary(B::Or, _, defined_type, left, right) = def.kind else {
            return false;
        };
        if defined_type != ty {
            return false;
        }
        for (left, right) in [(left, right), (right, left)] {
            if u128::from(self.possible_bits(left, ty, 0)) & bits & u128::from(u64::MAX) == 0 {
                *kind = I::Binary(B::And, facts, ty, right, y);
                return true;
            }
        }
        false
    }
    pub(super) fn reassociate(&mut self, kind: &mut I) -> Result<bool, Error> {
        let I::Binary(mut op, _, ty, x, y) = *kind else {
            return Ok(false);
        };
        if ty.bits() == 0 {
            return Ok(false);
        }
        let Some(mut cy) = self.constant(y) else {
            return Ok(false);
        };
        let Some((_, def)) = producer(self.body, x) else {
            return Ok(false);
        };
        let I::Binary(mut inner, _, defined_type, left, right) = def.kind else {
            return Ok(false);
        };
        if defined_type != ty {
            return Ok(false);
        }
        let Some(mut cx) = self.constant(right) else {
            return Ok(false);
        };
        let zero = fold::integer(ty, 0);
        if op == B::Sub {
            let Some(c) = fold::binary(B::Sub, ty, zero, cy) else {
                return Ok(false);
            };
            cy = c;
            op = B::Add;
        }
        if inner == B::Sub {
            let Some(c) = fold::binary(B::Sub, ty, zero, cx) else {
                return Ok(false);
            };
            cx = c;
            inner = B::Add;
        }
        if op != inner || !matches!(op, B::Add | B::Mul | B::And | B::Or | B::Xor) {
            return Ok(false);
        }
        let Some(c) = fold::binary(op, ty, cx, cy) else {
            return Ok(false);
        };
        *kind = I::Binary(
            op,
            Facts::arithmetic(false, false),
            ty,
            left,
            self.literal(c)?,
        );
        Ok(true)
    }
    pub(super) fn narrow_compare(&mut self, kind: &mut I) -> Result<bool, Error> {
        let I::Compare(mut pred, ty, x, y) = *kind else {
            return Ok(false);
        };
        if ty.bits() == 0 || ty == Type::I128 {
            return Ok(false);
        }
        if !matches!(y, Value::Integer(_)) {
            let Some((_, left)) = producer(self.body, x) else {
                return Ok(false);
            };
            let Some((_, right)) = producer(self.body, y) else {
                return Ok(false);
            };
            let I::Convert(op, to, from, xa) = left.kind else {
                return Ok(false);
            };
            let I::Convert(other, to_b, from_b, yb) = right.kind else {
                return Ok(false);
            };
            if op != other
                || !matches!(xa, Value::Temp(_))
                || !matches!(yb, Value::Temp(_))
                || to != ty
                || to_b != ty
                || !matches!(op, X::ZeroExtend | X::SignExtend)
                || from != from_b
                || from.bits() == 0
                || from == Type::I128
                || bytes(from) >= bytes(ty)
            {
                return Ok(false);
            }
            if op == X::ZeroExtend {
                pred = unsigned_predicate(pred);
            } else if !matches!(pred, C::Eq | C::Ne | C::Lt | C::Le | C::Gt | C::Ge) {
                return Ok(false);
            }
            *kind = I::Compare(pred, from, xa, yb);
            return Ok(true);
        }
        let Some((_, ext)) = producer(self.body, x) else {
            return Ok(false);
        };
        let I::Convert(op, to, narrow, left) = ext.kind else {
            return Ok(false);
        };
        if !matches!(left, Value::Temp(_))
            || !matches!(op, X::ZeroExtend | X::SignExtend)
            || narrow.bits() == 0
            || narrow == Type::I128
            || to != ty
            || bytes(narrow) >= bytes(ty)
        {
            return Ok(false);
        }
        let bits = if narrow == Type::I1 {
            1
        } else {
            8 * bytes(narrow)
        };
        let Some(Constant::Integer(c)) = self.constant(y) else {
            return Ok(false);
        };
        let c = i64::from_le_bytes(low_word(fold::normalize(ty, c)).to_le_bytes());
        if op == X::ZeroExtend {
            if c < 0
                || u64::from_le_bytes(c.to_le_bytes())
                    > (if bits == 64 {
                        u64::MAX
                    } else {
                        (1_u64 << bits) - 1
                    })
            {
                return Ok(false);
            }
            pred = unsigned_predicate(pred);
        } else {
            let low = -(1_i64 << (bits - 1));
            let high = (1_i64 << (bits - 1)) - 1;
            if c < low || c > high || !matches!(pred, C::Eq | C::Ne | C::Lt | C::Le | C::Gt | C::Ge)
            {
                return Ok(false);
            }
        }
        let value = self.literal(fold::integer(
            narrow,
            u128::from_le_bytes(i128::from(c).to_le_bytes()),
        ))?;
        *kind = I::Compare(pred, narrow, left, value);
        Ok(true)
    }
    fn quotient(&self, v: Value, ty: Type) -> Option<(B, Value, Value, Type)> {
        let (_, q) = producer(self.body, v)?;
        let I::Binary(op, facts, defined_type, left, right @ Value::Integer(id)) = q.kind else {
            return None;
        };
        if !facts.exact()
            || defined_type != ty
            || !matches!(left, Value::Temp(_))
            || !matches!(op, B::Div | B::UDiv | B::Shr | B::UShr)
            || self.body.literal(id) == 0
        {
            return None;
        }
        let Value::Temp(temp) = left else {
            return None;
        };
        let from = self.body.temp(temp).ty?;
        (from == ty || from == Type::Ptr && bytes(ty) == 8).then_some((op, left, right, from))
    }
    pub(super) fn exact_compare(&mut self, kind: &mut I) -> Result<bool, Error> {
        let I::Compare(pred, ty, x, y) = *kind else {
            return Ok(false);
        };
        if ty.bits() == 0 || ty == Type::I128 || !matches!(pred, C::Eq | C::Ne) {
            return Ok(false);
        }
        let Some((op, left, divisor, from)) = self.quotient(x, ty) else {
            return Ok(false);
        };
        if let Value::Integer(id) = y {
            if fold::normalize(ty, self.body.literal(id)) != 0 {
                return Ok(false);
            }
            *kind = I::Compare(pred, from, left, self.body.integer(0)?);
            return Ok(true);
        }
        let Some((other, right, d, from_b)) = self.quotient(y, ty) else {
            return Ok(false);
        };
        if op != other
            || from != from_b
            || !(divisor == d || self.constant(divisor) == self.constant(d))
        {
            return Ok(false);
        }
        *kind = I::Compare(pred, from, left, right);
        Ok(true)
    }
    pub(super) fn simplify_branch(&mut self, kind: &mut I) {
        let I::Branch(mut v, yes, no) = *kind else {
            return;
        };
        for _ in 0..4 {
            let Some((_, def)) = producer(self.body, v) else {
                break;
            };
            let next = match def.kind {
                I::Convert(op, _, _, left @ Value::Temp(_))
                    if op == X::ZeroExtend || op == X::Truncate && self.boolean(left, 0) =>
                {
                    Some(left)
                }
                I::Compare(C::Ne, ty, left @ Value::Temp(defined_type), right)
                    if ty.bits() != 0
                        && self
                            .constant(right)
                            .is_some_and(|c| fold::is_integer(ty, c, 0))
                        && self.body.temp(defined_type).ty == Some(ty) =>
                {
                    Some(left)
                }
                I::Binary(
                    B::Div | B::UDiv | B::Shr | B::UShr,
                    facts,
                    _,
                    left @ Value::Temp(defined_type),
                    Value::Integer(id),
                ) if facts.exact() && self.body.literal(id) != 0 => {
                    let ty = self.body.temp(defined_type).ty.unwrap_or(Type::Void);
                    if ty == Type::Ptr || ty.bits() != 0 && ty != Type::I128 {
                        Some(left)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let Some(next) = next else {
                break;
            };
            v = next;
            self.changed = true;
        }
        *kind = I::Branch(v, yes, no);
    }
}
fn unsigned_predicate(pred: C) -> C {
    match pred {
        C::Lt => C::Ult,
        C::Le => C::Ule,
        C::Gt => C::Ugt,
        C::Ge => C::Uge,
        _ => pred,
    }
}

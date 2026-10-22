//! Bounded library-call rewrites from `opt/value_simplify.cpp`. See NOTICE.
use super::{
    memory_contents::{self as memory, LibraryCall},
    numeric_fold as fold,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{BlockId, Body, Facts, InstId, Instruction, InstructionKind as I, Program, Value},
    vocabulary::{Binary, Compare, Convert, Type},
};

pub(super) fn bulk(
    program: &Program,
    body: &Body,
    instruction: &mut Instruction,
    replacements: &mut [Option<Value>],
) -> bool {
    let I::Call(call) = &instruction.kind else {
        return false;
    };
    let Value::Symbol(symbol) = call.callee else {
        return false;
    };
    let [to, from, count] = body.arguments(call.arguments) else {
        return false;
    };
    let zero = memory::library(program, symbol, LibraryCall::Memset);
    if !zero && !memory::library(program, symbol, LibraryCall::Memcpy) {
        return false;
    }
    let Some(Constant::Integer(count @ 1..=128)) = fold::literal(body, *count) else {
        return false;
    };
    let Ok(count) = u64::try_from(count) else {
        return false;
    };
    if zero && !fold::literal(body, *from).is_some_and(|v| fold::is_integer(Type::I32, v, 0)) {
        return false;
    }
    if let Some(result) = instruction.result {
        replacements[result.index()] = Some(*to);
    }
    instruction.kind = if zero {
        I::ZeroInit(count, 0, *to)
    } else {
        I::CopyObject(count, 0, *from, *to)
    };
    instruction.result = None;
    true
}
struct Emission<'left_base> {
    body: &'left_base mut Body,
    block: BlockId,
    source: InstId,
    ids: Vec<InstId>,
}
impl Emission<'_> {
    fn value(&mut self, kind: I, ty: Type) -> Result<Value, Error> {
        let temp = self.body.generated_temp(ty)?;
        let id = self
            .body
            .append(self.block, Instruction::new(Some(temp), kind, 0))?;
        self.body.inherit_location(id, self.source);
        self.ids.push(id);
        Ok(Value::Temp(temp))
    }
    fn load(&mut self, mut base: Value, offset: u64, ty: Type) -> Result<Value, Error> {
        if offset != 0 && matches!(base, Value::Symbol(_)) {
            base = self.value(I::Addr(base), Type::Ptr)?;
        }
        if offset != 0 {
            let at = self.body.integer(u128::from(offset))?;
            base = self.value(I::Index(Type::I8, None, base, at), Type::Ptr)?;
        }
        self.value(I::Load(Facts::default(), ty, base), ty)
    }
}
pub(super) fn equality_memcmp(
    program: &Program,
    body: &mut Body,
    level: u8,
    block: BlockId,
    id: InstId,
    uses: &[u32],
    alive: &[bool],
) -> Result<Option<Vec<InstId>>, Error> {
    if level < 2 {
        return Ok(None);
    }
    let call = body.instruction(id);
    let Some(result) = call.result else {
        return Ok(None);
    };
    let I::Call(call) = &call.kind else {
        return Ok(None);
    };
    let Value::Symbol(symbol) = call.callee else {
        return Ok(None);
    };
    if !memory::library(program, symbol, LibraryCall::Memcmp) {
        return Ok(None);
    }
    let [left_base, right_base, count_value] = body.arguments(call.arguments) else {
        return Ok(None);
    };
    if !matches!(left_base, Value::Temp(_) | Value::Symbol(_))
        || !matches!(right_base, Value::Temp(_) | Value::Symbol(_))
    {
        return Ok(None);
    }
    let Some(Constant::Integer(count @ 1..=16)) = fold::literal(body, *count_value) else {
        return Ok(None);
    };
    let (left_base, right_base, count) = (
        *left_base,
        *right_base,
        u64::try_from(count).map_err(|_| Error::Limit)?,
    );
    let Some(&use_count) = uses.get(result.index()) else {
        return Ok(None);
    };
    if use_count == 0 {
        return Ok(None);
    }
    let mut compares = 0_u32;
    for &at in &body.layout {
        for &user in &body.block(at).instructions {
            if !alive.get(user.index()).copied().unwrap_or(true) {
                continue;
            }
            let kind = &body.instruction(user).kind;
            let mut reads = false;
            body.visit_values(kind, |v| reads |= v == Value::Temp(result));
            if !reads {
                continue;
            }
            let I::Compare(Compare::Eq | Compare::Ne, _, left_base, right_base) = *kind else {
                return Ok(None);
            };
            if left_base != Value::Temp(result)
                || right_base == Value::Temp(result)
                || !fold::literal(body, right_base)
                    .is_some_and(|v| fold::is_integer(Type::I32, v, 0))
            {
                return Ok(None);
            }
            compares = compares.checked_add(1).ok_or(Error::Limit)?;
        }
    }
    if compares != use_count {
        return Ok(None);
    }
    let chunk = if count >= 8 {
        8
    } else if count >= 4 {
        4
    } else if count >= 2 {
        2
    } else {
        1
    };
    let ty = match chunk {
        8 => Type::I64,
        4 => Type::U32,
        2 => Type::U16,
        _ => Type::U8,
    };
    let mut emit = Emission {
        body,
        block,
        source: id,
        ids: Vec::new(),
    };
    let mut diff = None;
    for offset in [0, count - chunk]
        .into_iter()
        .take(if count == chunk { 1 } else { 2 })
    {
        let x = emit.load(left_base, offset, ty)?;
        let y = emit.load(right_base, offset, ty)?;
        let d = emit.value(I::Binary(Binary::Xor, Facts::default(), ty, x, y), ty)?;
        diff = Some(if let Some(previous) = diff {
            emit.value(I::Binary(Binary::Or, Facts::default(), ty, previous, d), ty)?
        } else {
            d
        });
    }
    let zero = emit.body.integer(0)?;
    let differs = emit.value(
        I::Compare(Compare::Ne, ty, diff.ok_or(Error::Limit)?, zero),
        Type::I64,
    )?;
    emit.body.replace_kind(
        id,
        I::Convert(Convert::Truncate, Type::I32, Type::I64, differs),
    );
    Ok(Some(emit.ids))
}

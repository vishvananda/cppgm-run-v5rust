//! Counted indexed store ranges from `opt/loop_passes.cpp::indexed_stores`. See NOTICE.
use super::{
    alias_facts::{literal_integer, signed_word},
    induction::{Exit, Induction},
    loop_memory::Emitter,
    loop_motion::invariant,
    memory_contents::bytes,
    natural_loops::{Forest, LoopId},
    numeric_fold::normalize,
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{Body, Facts as Flags, InstId, InstructionKind as I, LinkEditor, Value},
    vocabulary::{Convert, Type},
};
struct Fill {
    store: InstId,
    base: Value,
    source: Option<Value>,
    bytes: u64,
}
fn is_iv(body: &Body, mut v: Value, iv: Induction) -> bool {
    for _ in 0..3 {
        let Value::Temp(t) = v else { break };
        if t == iv.value {
            return true;
        }
        let Some(def) = body.temp(t).definition else {
            return false;
        };
        v = match body.instruction(def).kind {
            I::Copy(_, v) | I::Convert(Convert::ZeroExtend | Convert::SignExtend, _, _, v) => v,
            _ => return false,
        };
    }
    false
}
fn object_of(body: &Body, value: Value) -> Option<Value> {
    if let Value::Temp(t) = value
        && let Some(def) = body.temp(t).definition
        && let I::Addr(v) = body.instruction(def).kind
    {
        return Some(v);
    }
    matches!(value, Value::Slot(_) | Value::Symbol(_)).then_some(value)
}
fn element_base(
    context: &Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    iv: Induction,
    address: Value,
    size: u64,
) -> Option<Value> {
    let Value::Temp(t) = address else { return None };
    let def = context.body.temp(t).definition?;
    let I::Index(ty, _, base, index) = context.body.instruction(def).kind else {
        return None;
    };
    (bytes(ty) == size
        && is_iv(context.body, index, iv)
        && (invariant(context, forest, loop_, base) || object_of(context.body, base).is_some()))
    .then_some(base)
}
fn collect(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    iv: Induction,
    test: Exit,
) -> Result<Option<Vec<Fill>>, Error> {
    context.body.recount_uses()?;
    context.invalidate_uses();
    let l = forest.get(loop_);
    let mut fills = Vec::new();
    let mut loads = Vec::new();
    for &b in &l.blocks {
        for &id in &context.body.block(b).instructions {
            match context.body.instruction(id).kind {
                I::Store(flags, ty, value, address) => {
                    let size = bytes(ty);
                    let Some(base) = element_base(context, forest, loop_, iv, address, size) else {
                        return Ok(None);
                    };
                    if flags.volatile()
                        || ty.bits() == 0
                        || size == 0
                        || size > 8
                        || b == l.header
                        || !context.cfg()?.dominates(b, l.latches[0])
                    {
                        return Ok(None);
                    }
                    let source = if matches!(value, Value::Integer(_)) {
                        None
                    } else {
                        let Value::Temp(t) = value else {
                            return Ok(None);
                        };
                        let Some(def) = context.body.temp(t).definition else {
                            return Ok(None);
                        };
                        let I::Load(flags, load_ty, address) = context.body.instruction(def).kind
                        else {
                            return Ok(None);
                        };
                        let Some(src) = element_base(context, forest, loop_, iv, address, size)
                        else {
                            return Ok(None);
                        };
                        if flags.volatile()
                            || load_ty != ty
                            || context.body.temp(t).uses != 1
                            || object_of(context.body, src).is_none()
                        {
                            return Ok(None);
                        }
                        Some(src)
                    };
                    fills.push(Fill {
                        store: id,
                        base,
                        source,
                        bytes: size,
                    });
                }
                I::Load(..) => loads.push(id),
                I::Jump(_) | I::Branch(..) => (),
                ref kind if super::pure(kind) => (),
                _ => return Ok(None),
            }
        }
    }
    if fills.is_empty() {
        return Ok(None);
    }
    for id in loads {
        if !fills.iter().any(|f|matches!(context.body.instruction(f.store).kind,I::Store(_,_,Value::Temp(t),_) if context.body.temp(t).definition==Some(id))) {return Ok(None);}
    }
    for (i, f) in fills.iter().enumerate() {
        if u64::try_from(test.trips.ok_or(Error::Limit)?)
            .map_err(|_| Error::Limit)?
            .checked_mul(f.bytes)
            .ok_or(Error::Limit)?
            > 256
        {
            return Ok(None);
        }
        for (j, g) in fills.iter().enumerate() {
            if i != j && f.base == g.base {
                return Ok(None);
            }
            if let Some(src) = g.source
                && (object_of(context.body, f.base).is_none()
                    || object_of(context.body, f.base) == object_of(context.body, src))
            {
                return Ok(None);
            }
        }
    }
    Ok(Some(fills))
}
fn start(
    e: &mut Emitter<'_>,
    pre: crate::lowir::model::BlockId,
    at: &mut Option<InstId>,
    mut base: Value,
    object: Option<Value>,
    first: i64,
    size: u64,
) -> Result<Value, Error> {
    if let Some(object) = object {
        base = e.value(pre, at, I::Addr(object), Type::Ptr)?;
    }
    if first == 0 {
        return Ok(base);
    }
    let offset = e.literal(first.wrapping_mul(super::alias_facts::signed_size(size)))?;
    e.value(pre, at, I::Index(Type::I8, None, base, offset), Type::Ptr)
}
pub(super) fn rewrite(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    test: Exit,
    ivs: &[Induction],
    apply: bool,
) -> Result<bool, Error> {
    let l = forest.get(loop_);
    if !matches!(test.trips, Some(1..=64))
        || test.exiting != l.header
        || apply && l.preheader.is_none()
    {
        return Ok(false);
    }
    let Some(iv) = ivs.iter().copied().rev().find(|iv| {
        test.compared == Value::Temp(iv.value)
            && !iv.pointer
            && iv.step == 1
            && matches!(iv.init, Some(Value::Integer(_)))
    }) else {
        return Ok(false);
    };
    let Some(Value::Integer(k)) = iv.init else {
        return Err(Error::Limit);
    };
    let first = signed_word(context.body.literal(k));
    if !(0..=64).contains(&first) {
        return Ok(false);
    }
    let Some(fills) = collect(context, forest, loop_, iv, test)? else {
        return Ok(false);
    };
    if !apply {
        return Ok(true);
    }
    let pre = l.preheader.ok_or(Error::Limit)?;
    let objects = fills
        .iter()
        .map(|f| {
            (
                if invariant(context, forest, loop_, f.base) {
                    None
                } else {
                    object_of(context.body, f.base)
                },
                f.source.and_then(|s| {
                    if invariant(context, forest, loop_, s) {
                        None
                    } else {
                        object_of(context.body, s)
                    }
                }),
            )
        })
        .collect::<Vec<_>>();
    let links = LinkEditor::new(context.body)?;
    let mut at = links.previous(
        *context
            .body
            .block(pre)
            .instructions
            .last()
            .ok_or(Error::Limit)?,
    );
    let mut e = Emitter {
        body: context.body,
        links,
        location: Some(fills[0].store),
    };
    for (f, (object, source_object)) in fills.into_iter().zip(objects) {
        let I::Store(_, ty, value, _) = e.body.instruction(f.store).kind else {
            return Err(Error::Limit);
        };
        let total = u64::try_from(test.trips.ok_or(Error::Limit)?)
            .map_err(|_| Error::Limit)?
            .checked_mul(f.bytes)
            .ok_or(Error::Limit)?;
        let dst = start(&mut e, pre, &mut at, f.base, object, first, f.bytes)?;
        if let Some(src) = f.source {
            let src = start(&mut e, pre, &mut at, src, source_object, first, f.bytes)?;
            e.emit(pre, &mut at, I::CopyObject(total, 0, src, dst), Type::Void)?;
            continue;
        }
        let Value::Integer(k) = value else {
            return Err(Error::Limit);
        };
        let bits = normalize(ty, e.body.literal(k));
        if bits == 0 {
            e.emit(pre, &mut at, I::ZeroInit(total, 0, dst), Type::Void)?;
            continue;
        }
        let low = super::alias_facts::signed_word(bits);
        let low = u64::from_le_bytes(low.to_le_bytes());
        let mask = if f.bytes == 8 {
            u64::MAX
        } else {
            (1_u64 << (8 * f.bytes)) - 1
        };
        let mut pattern = 0_u64;
        for k in (0..8).step_by(usize::try_from(f.bytes).map_err(|_| Error::Limit)?) {
            pattern |= (low & mask) << (8 * k);
        }
        let mut offset = 0;
        while offset < total {
            let wide = offset + 8 <= total;
            let address = if offset == 0 {
                dst
            } else {
                let offset = e
                    .body
                    .integer(literal_integer(super::alias_facts::signed_size(offset)))?;
                e.value(
                    pre,
                    &mut at,
                    I::Index(Type::I8, None, dst, offset),
                    Type::Ptr,
                )?
            };
            let (ty, value) = if wide {
                (Type::I64, e.body.integer(u128::from(pattern))?)
            } else {
                (ty, value)
            };
            e.emit(
                pre,
                &mut at,
                I::Store(Flags::default(), ty, value, address),
                Type::Void,
            )?;
            offset += if wide { 8 } else { f.bytes };
        }
    }
    e.links.commit(e.body)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::{pass_context::Pass, pass_view};
    #[test]
    fn indexed_ranges_respect_the_byte_budget_and_volatile_barrier() {
        let source = "global @g : obj<512x8> = zero function @f()->void {block ^entry:jump ^head block ^head:%i = phi i64 [^entry:0,^step:%next] %c = cmp lt i64 %i,32 branch %c,^step,^exit block ^step:%p = index i64 @g,%i store i64 0,%p %next = binary add i64 %i,1 jump ^head block ^exit:return void}";
        let (out, facts) = pass_view(source, 2, Pass::RemoveLoops);
        assert!(facts.0);
        assert!(out.contains("zeroinit 256"));
        assert!(!out.contains("phi"));
        assert!(
            !pass_view(&source.replace("%i,32", "%i,33"), 2, Pass::RemoveLoops)
                .1
                .0
        );
        assert!(
            !pass_view(
                &source.replace("store i64", "store volatile i64"),
                2,
                Pass::RemoveLoops
            )
            .1
            .0
        );
    }
}

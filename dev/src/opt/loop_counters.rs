//! Guarded narrow counter facts from `opt/loop_passes.cpp::mark_no_wrap`. See NOTICE.
use super::{
    alias_facts::signed_word, induction::Facts, memory_contents::bytes, natural_loops::Forest,
    numeric_fold::normalize, pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{Facts as Flags, InstructionKind as I, Value},
    vocabulary::{Binary, Compare, Type},
};

pub(super) fn mark(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg()?;
    let mut forest = Forest::default();
    forest.find(context.body.blocks().len(), context.cfg()?)?;
    context.prepare_block_map(super::pass_context::Pass::MarkCounters, true);
    let mut any = false;
    for loop_ in forest.ids() {
        let l = forest.get(loop_);
        if l.latches.len() != 1 {
            continue;
        }
        for id in context.body.block(l.header).instructions.clone() {
            let I::Phi(ty, _) = context.body.instruction(id).kind else {
                break;
            };
            if !matches!(ty, Type::I8 | Type::U8 | Type::I16 | Type::U16 | Type::I32) {
                continue;
            }
            let facts = Facts {
                body: context.body,
                cfg: context.cfg()?,
                forest: &forest,
                block_of: context.block_map(),
            };
            let Some(iv) = facts.induction(loop_, id) else {
                continue;
            };
            if iv.pointer || iv.step != 1 {
                continue;
            }
            let step = context.body.temp(iv.next).definition.ok_or(Error::Limit)?;
            let I::Binary(Binary::Add, flags, _, _, _) = context.body.instruction(step).kind else {
                continue;
            };
            let signed = matches!(ty, Type::I8 | Type::I16 | Type::I32);
            let below = if let Some(Value::Integer(k)) = iv.init {
                signed_word(normalize(ty, context.body.literal(k)))
                    < (1_i64 << (8 * bytes(ty) - u64::from(signed))) - 1
            } else {
                false
            };
            let in_header = context.body.block(l.header).instructions.contains(&step);
            let mut guarded = false;
            for which in 0..2 {
                if which == 0 && in_header || which == 1 && !below {
                    continue;
                }
                let b = if which == 0 { l.header } else { l.latches[0] };
                let Some(&last) = context.body.block(b).instructions.last() else {
                    continue;
                };
                let I::Branch(Value::Temp(t), yes, no) = context.body.instruction(last).kind else {
                    continue;
                };
                let Some(def) = context.body.temp(t).definition else {
                    continue;
                };
                let I::Compare(pred, cmp_ty, a, b) = context.body.instruction(def).kind else {
                    continue;
                };
                if cmp_ty != ty || forest.contains(loop_, yes) == forest.contains(loop_, no) {
                    continue;
                }
                let tested = Value::Temp(if which == 0 { iv.value } else { iv.next });
                for (value, other, pred) in [(a, b, pred), (b, a, pred.swapped())] {
                    if value != tested || !facts.invariant(loop_, other) {
                        continue;
                    }
                    let pred = if forest.contains(loop_, yes) {
                        pred
                    } else {
                        pred.inverted()
                    };
                    guarded |= pred == if signed { Compare::Lt } else { Compare::Ult };
                }
            }
            if guarded {
                let mut inst = context.body.instruction(step).clone();
                if let I::Binary(_, at, ..) = &mut inst.kind {
                    *at = Flags::arithmetic(true, flags.exact());
                }
                context.body.update_instruction(step, inst);
                any = true;
            }
        }
    }
    Ok(any)
}

#[cfg(test)]
mod tests {
    use super::super::{pass_context::Pass, pass_view};
    #[test]
    fn a_header_guard_cannot_prove_a_step_that_runs_before_it() {
        let guarded = "function @f(%n:i32)->i32 {block ^entry:jump ^head block ^head:%i = phi i32 [^entry:0,^step:%next] %c = cmp lt i32 %i, %n branch %c,^step,^exit block ^step:%next = binary add i32 %i,1 jump ^head block ^exit:return i32 %i}";
        assert!(
            pass_view(guarded, 2, Pass::MarkCounters)
                .0
                .contains("binary add nowrap")
        );
        let before = guarded
            .replace("%c = cmp", "%next = binary add i32 %i,1 %c = cmp")
            .replace("block ^step:%next = binary add i32 %i,1", "block ^step:");
        assert!(!pass_view(&before, 2, Pass::MarkCounters).1.0);
    }
    #[test]
    fn a_latch_guard_requires_a_start_below_the_target_maximum() {
        let source = "function @f(%n:u8)->u8 {block ^entry:jump ^head block ^head:%i = phi u8 [^entry:254,^head:%next] %next = binary add u8 %i,1 %c = cmp ult u8 %next,%n branch %c,^head,^exit block ^exit:return u8 %next}";
        assert!(pass_view(source, 2, Pass::MarkCounters).1.0);
        assert!(
            !pass_view(&source.replace(":254", ":255"), 2, Pass::MarkCounters)
                .1
                .0
        );
    }
}

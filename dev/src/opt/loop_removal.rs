//! Bounded loop-removal scheduling and effect-free bypass from `opt/loop_passes.cpp`.
//! See NOTICE.
use super::{
    induction::{Exit, Facts},
    iteration_values, loop_fills,
    natural_loops::{Forest, LoopId},
    numeric_fold::normalize,
    pass_context::{Context, Pass},
};
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstId, Instruction, InstructionKind as I, OptimizerBlock, Value},
    vocabulary::Binary,
};

#[derive(Default, Clone, Copy)]
struct Effects {
    stores: u32,
    loads: u32,
    others: u32,
    traps: u32,
}
impl Effects {
    fn add(&mut self, other: Self) -> Result<(), Error> {
        self.stores = self.stores.checked_add(other.stores).ok_or(Error::Limit)?;
        self.loads = self.loads.checked_add(other.loads).ok_or(Error::Limit)?;
        self.others = self.others.checked_add(other.others).ok_or(Error::Limit)?;
        self.traps = self.traps.checked_add(other.traps).ok_or(Error::Limit)?;
        Ok(())
    }
    fn bypassable(self) -> bool {
        self.stores == 0 && self.loads == 0 && self.others == 0 && self.traps == 0
    }
    fn candidate(self, level: u8) -> bool {
        self.bypassable()
            || self.stores == 1 && self.loads <= 1 && self.others == 0
            || level >= 2
                && self.stores > 1
                && self.stores <= 4
                && self.loads <= self.stores
                && self.others == 0
                && self.traps == 0
    }
}
fn effects(body: &Body, forest: &Forest) -> Result<Vec<Effects>, Error> {
    let mut effects = vec![Effects::default(); forest.len()];
    for &block in &body.layout {
        let Some(loop_) = forest.innermost(block) else {
            continue;
        };
        for &id in &body.block(block).instructions {
            let kind = &body.instruction(id).kind;
            let item = match kind {
                I::Store(..) => Effects {
                    stores: 1,
                    ..Effects::default()
                },
                I::Load(..) => Effects {
                    loads: 1,
                    ..Effects::default()
                },
                I::Binary(Binary::Div | Binary::Mod | Binary::UDiv | Binary::UMod, ..) => Effects {
                    traps: 1,
                    ..Effects::default()
                },
                I::Jump(_) | I::Branch(..) => Effects::default(),
                kind if super::pure(kind) => Effects::default(),
                _ => Effects {
                    others: 1,
                    ..Effects::default()
                },
            };
            effects[loop_.index()].add(item)?;
        }
    }
    for loop_ in forest.ids().rev() {
        if let Some(parent) = forest.get(loop_).parent {
            let child = effects[loop_.index()];
            effects[parent.index()].add(child)?;
        }
    }
    Ok(effects)
}
fn phis(body: &Body, block: BlockId) -> Vec<InstId> {
    body.block(block)
        .instructions
        .iter()
        .copied()
        .take_while(|&id| matches!(body.instruction(id).kind, I::Phi(..)))
        .collect()
}
fn incoming(body: &Body, id: InstId, pred: BlockId) -> Option<Value> {
    let I::Phi(_, pairs) = &body.instruction(id).kind else {
        return None;
    };
    pairs.iter().find(|&&(b, _)| b == pred).map(|&(_, v)| v)
}
fn redirected(value: Value, test: Exit) -> Value {
    if value == test.compared {
        test.value
    } else {
        value
    }
}
fn bypass(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    test: Exit,
) -> Result<bool, Error> {
    let header = forest.get(loop_).header;
    let entries = context
        .cfg()?
        .predecessors(header)
        .iter()
        .copied()
        .filter(|&b| !forest.contains(loop_, b))
        .collect::<Vec<_>>();
    for pred in entries {
        let phis = phis(context.body, test.target);
        let conflict = phis.iter().any(|&id| {
            incoming(context.body, id, pred).is_some_and(|old| {
                Some(old) != incoming(context.body, id, test.exiting).map(|v| redirected(v, test))
            })
        });
        let terminator = *context
            .body
            .block(pred)
            .instructions
            .last()
            .ok_or(Error::Limit)?;
        let from = if conflict {
            let from = context.body.optimizer_block(OptimizerBlock::LoopExit)?;
            context
                .body
                .append(from, Instruction::new(None, I::Jump(test.target), 0))?;
            let at = context
                .body
                .layout
                .iter()
                .position(|&b| b == pred)
                .ok_or(Error::Limit)?;
            context.body.layout.insert(at + 1, from);
            context.body.redirect_successors(terminator, header, from);
            from
        } else {
            context
                .body
                .redirect_successors(terminator, header, test.target);
            pred
        };
        for id in phis {
            if incoming(context.body, id, from).is_some() {
                continue;
            }
            let value = incoming(context.body, id, test.exiting).ok_or(Error::Limit)?;
            context
                .body
                .append_phi_predecessor(id, from, redirected(value, test))?;
        }
    }
    context
        .body
        .remove_phi_predecessor(test.target, test.exiting);
    context.body.layout.retain(|&b| !forest.contains(loop_, b));
    let invariant = Facts {
        body: context.body,
        cfg: context.cfg()?,
        forest,
        block_of: context.block_map(),
    }
    .invariant(loop_, test.compared);
    if !invariant {
        for block in context.body.layout.clone() {
            for id in context.body.block(block).instructions.clone() {
                context
                    .body
                    .rewrite_values(id, |v| Ok(redirected(v, test)))?;
            }
        }
    }
    Ok(true)
}
fn try_loop(context: &mut Context<'_>, forest: &Forest, loop_: LoopId) -> Result<bool, Error> {
    if forest.get(loop_).latches.len() != 1 {
        return Ok(false);
    }
    let facts = Facts {
        body: context.body,
        cfg: context.cfg()?,
        forest,
        block_of: context.block_map(),
    };
    let Some(mut analysis) = facts.exit(loop_) else {
        return Ok(false);
    };
    if let Some((ty, last)) = analysis.counted {
        analysis.test.value = context
            .body
            .integer(normalize(ty, u128::from_le_bytes(last.to_le_bytes())))?;
    }
    let test = analysis.test;
    let facts = Facts {
        body: context.body,
        cfg: context.cfg()?,
        forest,
        block_of: context.block_map(),
    };
    let private = facts.private_values(loop_, test);
    let free = facts.effect_free(loop_);
    if !private {
        if free && iteration_values::compute(context, forest, loop_, test, &analysis.inductions)? {
            // The exit values are now available to the bypass below.
        } else if super::loop_copies::emit(
            context,
            forest,
            loop_,
            test,
            &analysis.inductions,
            false,
        )? {
            return Ok(true);
        } else {
            return super::wide_fills::widen(context, forest, loop_, test, &analysis.inductions);
        }
    } else if !free {
        if context.level >= 2
            && super::indexed_stores::rewrite(
                context,
                forest,
                loop_,
                test,
                &analysis.inductions,
                false,
            )?
        {
            if forest.get(loop_).preheader.is_none() {
                return super::loop_motion::make_preheader(context, forest, loop_);
            }
            super::indexed_stores::rewrite(
                context,
                forest,
                loop_,
                test,
                &analysis.inductions,
                true,
            )?;
        } else if !loop_fills::emit(context, forest, loop_, test, &analysis.inductions)? {
            if super::loop_copies::emit(context, forest, loop_, test, &analysis.inductions, true)? {
                return Ok(true);
            }
            return super::wide_fills::widen(context, forest, loop_, test, &analysis.inductions);
        }
    }
    if !bypass(context, forest, loop_, test)? {
        return Ok(false);
    }
    context.loops_removed = context.loops_removed.checked_add(1).ok_or(Error::Limit)?;
    Ok(true)
}
fn round(context: &mut Context<'_>, forest: &Forest) -> Result<bool, Error> {
    let effects = effects(context.body, forest)?;
    for loop_ in forest.ids().rev() {
        if effects[loop_.index()].others == 0
            && effects[loop_.index()].stores != 0
            && super::loop_memory::count_to_pointer(context, forest, loop_)?
        {
            return Ok(true);
        }
    }
    for loop_ in forest.ids().rev() {
        let e = effects[loop_.index()];
        if e.others == 0
            && e.stores == 1
            && e.loads == 1
            && super::loop_memory::hoist_stored(context, forest, loop_)?
        {
            return Ok(true);
        }
    }
    let mut tries = 0;
    for loop_ in forest.ids().rev() {
        if tries == 16 {
            break;
        }
        if !effects[loop_.index()].candidate(context.level) {
            continue;
        }
        tries += 1;
        if try_loop(context, forest, loop_)? {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn remove(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut any = false;
    let mut forest = Forest::default();
    for iteration in 0..8 {
        context.ensure_cfg()?;
        forest.find(context.body.blocks().len(), context.cfg()?)?;
        if forest.len() == 0 {
            break;
        }
        context.prepare_block_map(Pass::RemoveLoops, iteration == 0);
        if !round(context, &forest)? {
            break;
        }
        context.invalidate_cfg();
        any = true;
    }
    Ok(any)
}

#[cfg(test)]
mod tests {
    use super::super::pass_context::Pass;
    #[test]
    fn pure_counted_loop_replaces_its_tested_value_at_the_exit() {
        let source = "function @f()->i32 {block ^entry:jump ^head block ^head:%i = phi i32 [^entry: 0,^step: %next] %test = cmp lt i32 %i, 5 branch %test, ^step, ^exit block ^step:%next = binary add i32 %i, 1 jump ^head block ^exit:return i32 %i}";
        let (output, facts) = super::super::pass_view(source, 1, Pass::RemoveLoops);
        assert!(facts.0);
        assert!(output.contains("return i32 5"));
        assert!(!output.contains("phi"));
        let traps = source.replace(
            "%next = binary add",
            "%trap = binary div i32 1, %i %next = binary add",
        );
        assert!(!super::super::pass_view(&traps, 1, Pass::RemoveLoops).1.0);
    }
    #[test]
    fn live_secondary_induction_uses_its_own_target_arithmetic() {
        let source = "function @f()->i16 {block ^entry:jump ^head block ^head:%i = phi i32 [^entry: 0,^step: %next] %v = phi i16 [^entry: 32760,^step: %vnext] %test = cmp lt i32 %i, 5 branch %test, ^step, ^exit block ^step:%next = binary add i32 %i, 1 %vnext = binary add i16 %v, 3 jump ^head block ^exit:return i16 %v}";
        let (output, facts) = super::super::pass_view(source, 1, Pass::RemoveLoops);
        assert!(facts.0);
        assert!(output.contains("convert trunc i16 i64 5"));
        assert!(output.contains("binary mul i16"));
        assert!(output.contains("binary add i16 32760"));
    }
    #[test]
    fn conflicting_exit_phi_gets_its_own_named_entry_edge() {
        let source = "function @f(%c : i64)->i64 {block ^entry:branch %c, ^head, ^exit block ^head:%i = phi i64 [^entry: 0,^step: %next] %test = cmp ne i64 %i, 5 branch %test, ^step, ^exit block ^step:%next = binary add i64 %i, 1 jump ^head block ^exit:%r = phi i64 [^entry: 17,^head: %i] return i64 %r}";
        let (output, facts) = super::super::pass_view(source, 1, Pass::RemoveLoops);
        assert!(facts.0);
        assert!(output.contains("block ^loop_exit_"));
        assert!(output.contains(": 17"));
        assert!(output.contains(": 5"));
    }
}

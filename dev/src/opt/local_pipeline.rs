//! Frozen function-pass order and round bounds from `opt/optimizer.cpp`. See NOTICE.
use super::{
    exception_regions,
    pass_context::{Context, Pass as P},
};
use crate::lowir::{Error, vocabulary::Type};

pub(super) fn finish(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut changed = false;
    if context.run(P::ExpandWide)? {
        context.run(P::PromoteSlots)?;
        scalar_rounds(context)?;
        changed = true;
    }
    for _ in 0..3 {
        if !context.run(P::FormSelects)? {
            break;
        }
        context.run(P::CleanupCfg)?;
        changed = true;
    }
    context.ensure_cfg()?;
    let mut forest = super::natural_loops::Forest::default();
    forest.find(context.body.blocks().len(), context.cfg()?)?;
    if forest.len() != 0 {
        return Err(Error::NotImplemented("LowIR loop finishing"));
    }
    if changed {
        context.run(P::SplitBranchCompares)?;
    }
    Ok(changed)
}

pub(super) fn simplify(context: &mut Context<'_>) -> Result<bool, Error> {
    context.body.record_switch_types();
    let object_slots = context
        .body
        .slots()
        .iter()
        .any(|slot| matches!(slot.ty, Type::Object { .. }));
    if object_slots {
        scalar_rounds(context)?;
    }
    for round in usize::from(object_slots)..4 {
        let mut changed = context.run(P::SplitSlots)?;
        if changed {
            context.run(P::DeadCode)?;
        }
        changed = context.run(P::PromoteSlots)? || changed;
        if !changed && round != 0 {
            break;
        }
        scalar_rounds(context)?;
        if context.body.slots().is_empty() {
            break;
        }
    }
    let mut again = context.run(P::StripRegions)?;
    again = context.run(P::CoalesceCopies)? || again;
    if again {
        if !context.body.slots().is_empty() && context.run(P::SplitSlots)? {
            context.run(P::DeadCode)?;
        }
        if !context.body.slots().is_empty() {
            context.run(P::PromoteSlots)?;
        }
        scalar_rounds(context)?;
    }
    again = context.run(P::RemoveLoops)?;
    if context.level >= 3 && context.run(P::UnrollLoops)? {
        again = true;
    }
    if context.level >= 2 && context.run(P::FormSwitches)? {
        again = true;
    }
    if again {
        scalar_rounds(context)?;
    }
    let nounwind = exception_regions::proves_nounwind(context);
    if context.level >= 2 && context.run(P::NameReturnedSlot)? {
        scalar_rounds(context)?;
    }
    if context.level >= 2 {
        context.run(P::SplitOrBranches)?;
        context.run(P::SpecializePhiCompares)?;
        context.run(P::SplitBranchCompares)?;
    }
    Ok(nounwind)
}
fn scalar_rounds(context: &mut Context<'_>) -> Result<(), Error> {
    if context.body.layout.len() == 1 {
        for _ in 0..4 {
            let mut changed = context.run(P::SimplifyValues)?;
            changed = context.run(P::ForwardMemory)? || changed;
            changed = context.run(P::DeadCode)? || changed;
            if !changed {
                break;
            }
        }
        if context.run(P::CleanupCfg)? && context.body.layout.len() == 1 {
            context.run(P::DeadCode)?;
        }
        if context.body.layout.len() == 1 {
            return Ok(());
        }
    }
    for _ in 0..4 {
        let mut changed = context.run(P::Constants)?;
        changed = context.run(P::CleanupCfg)? || changed;
        changed = context.run(P::SimplifyValues)? || changed;
        changed = context.run(P::ForwardMemory)? || changed;
        changed = context.run(P::DeadCode)? || changed;
        changed = context.run(P::ThreadJumps)? || changed;
        changed = context.run(P::CleanupCfg)? || changed;
        if !changed {
            break;
        }
    }
    context.restore_order()
}

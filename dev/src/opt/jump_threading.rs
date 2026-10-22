//! Function-local jump threading from `opt/jump_threading.cpp`. See NOTICE.
use super::{flow_cleanup, pass_context::Context, slot_promotion};
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstId, InstructionKind as I, TempId, Value},
};

pub(super) struct Threader<'a, 'p> {
    pub(super) context: &'a mut Context<'p>,
    pub(super) demoted: u32,
    pub(super) touched: bool,
}
pub(super) fn last(body: &Body, block: BlockId) -> Option<InstId> {
    body.block(block).instructions.last().copied()
}
pub(super) fn phis(body: &Body, block: BlockId) -> impl Iterator<Item = InstId> + '_ {
    body.block(block)
        .instructions
        .iter()
        .copied()
        .take_while(|&id| matches!(body.instruction(id).kind, I::Phi(..)))
}
pub(super) fn incoming(body: &Body, id: InstId, from: BlockId) -> Option<Value> {
    let I::Phi(_, pairs) = &body.instruction(id).kind else {
        return None;
    };
    pairs
        .iter()
        .find_map(|&(block, value)| (block == from).then_some(value))
}
pub(super) fn definition(body: &Body, value: Value) -> Option<InstId> {
    let Value::Temp(temp) = value else {
        return None;
    };
    (!body.temp_is_parameter(temp))
        .then(|| body.temp(temp).definition)
        .flatten()
}
pub(super) fn mapped(value: Value, values: &[Option<Value>]) -> Value {
    if let Value::Temp(temp) = value {
        values.get(temp.index()).copied().flatten().unwrap_or(value)
    } else {
        value
    }
}
pub(super) fn successors(kind: &I) -> Vec<BlockId> {
    match kind {
        I::Jump(to) => vec![*to],
        I::Branch(_, a, b) => vec![*a, *b],
        I::Switch(_, default, pairs) => std::iter::once(*default)
            .chain(pairs.iter().map(|&(_, b)| b))
            .collect(),
        _ => Vec::new(),
    }
}
pub(super) fn append_layout(body: &mut Body, after: BlockId, new: BlockId) {
    let mut layout = Vec::with_capacity(body.layout.len() + 1);
    for &block in &body.layout {
        layout.push(block);
        if block == after {
            layout.push(new);
        }
    }
    body.layout = layout;
}
impl Threader<'_, '_> {
    pub(super) fn rebuild(&mut self) -> Result<(), Error> {
        self.context.invalidate_cfg();
        self.context.ensure_cfg()
    }
    pub(super) fn folded_branch(&mut self) -> Result<(), Error> {
        self.context.branches_folded = self
            .context
            .branches_folded
            .checked_add(1)
            .ok_or(Error::Limit)?;
        Ok(())
    }
    fn known_edges(&mut self) -> Result<bool, Error> {
        let mut changed = false;
        for block in self.context.body.layout.clone() {
            let cfg = self.context.cfg()?;
            if !cfg.reachable(block) || cfg.handler(block) || cfg.predecessors(block).len() != 1 {
                continue;
            }
            let pred = cfg.predecessors(block)[0];
            let (Some(pt), Some(bt)) = (
                last(self.context.body, pred),
                last(self.context.body, block),
            ) else {
                continue;
            };
            if pred == block {
                continue;
            }
            let I::Branch(condition @ Value::Temp(_), yes, no) =
                self.context.body.instruction(pt).kind
            else {
                continue;
            };
            let I::Branch(other, keep, drop) = self.context.body.instruction(bt).kind else {
                continue;
            };
            if yes == no || condition != other {
                continue;
            }
            let (keep, drop) = if yes == block {
                (keep, drop)
            } else {
                (drop, keep)
            };
            if keep != drop {
                self.context.body.remove_phi_predecessor(drop, block);
            }
            self.context.body.replace_kind(bt, I::Jump(keep));
            self.folded_branch()?;
            changed = true;
        }
        Ok(changed)
    }
    fn run(&mut self) -> Result<bool, Error> {
        self.context.ensure_cfg()?;
        let mut changed =
            self.context.level >= 2 && super::dominating_facts::rewrite(self.context)?;
        changed = self.known_edges()? || changed;
        if changed {
            self.rebuild()?;
        }
        self.touched = changed;
        let mut counted = false;
        if self.context.level >= 2 {
            if self.known_compares()? {
                changed = true;
                self.rebuild()?;
            } else {
                counted = true;
            }
        }
        self.touched = changed;
        Ok(self.constant_phis(counted)? || changed)
    }
}
pub(super) fn thread(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut threading = Threader {
        context,
        demoted: 0,
        touched: false,
    };
    let changed = threading.run()?;
    if changed {
        threading.context.invalidate_cfg();
        flow_cleanup::cleanup(threading.context)?;
        if threading.demoted != 0 {
            slot_promotion::promote(threading.context)?;
        }
    }
    Ok(changed)
}

/// A semantic operand location: phi inputs are paired with their incoming edge.
/// Inline value positions count values, rather than C++'s unused operand slots.
#[derive(Clone, Copy)]
pub(super) struct Use {
    pub(super) instruction: InstId,
    pub(super) block: BlockId,
    pub(super) position: usize,
    pub(super) phi: bool,
    pub(super) definition: InstId,
}
pub(super) fn collect_uses(
    context: &Context<'_>,
    block: BlockId,
    defined: &[InstId],
    reject_from: bool,
) -> Result<Option<Vec<Use>>, Error> {
    let mut uses = Vec::new();
    for &at in &context.body.layout {
        for &id in &context.body.block(at).instructions {
            let kind = &context.body.instruction(id).kind;
            let mut operands = Vec::new();
            if let I::Phi(_, pairs) = kind {
                for (position, &(from, value)) in pairs.iter().enumerate() {
                    operands.push((position, from, value));
                }
            } else {
                context
                    .body
                    .visit_values(kind, |value| operands.push((operands.len(), at, value)));
            }
            let phi = matches!(kind, I::Phi(..));
            for (position, from, value) in operands {
                if let Some(definition) = definition(context.body, value)
                    && defined.contains(&definition)
                {
                    if phi && (at == block || reject_from && from == block)
                        || context.cfg()?.handler(from)
                    {
                        return Ok(None);
                    }
                    uses.push(Use {
                        instruction: id,
                        block: from,
                        position,
                        phi,
                        definition,
                    });
                }
            }
        }
    }
    Ok(Some(uses))
}
pub(super) fn replace_use(body: &mut Body, site: Use, value: Value) -> Result<(), Error> {
    let mut position = 0;
    body.rewrite_values(site.instruction, |old| {
        let result = if position == site.position {
            value
        } else {
            old
        };
        position += 1;
        Ok(result)
    })
}
pub(super) fn use_value(body: &Body, site: Use) -> Option<Value> {
    let mut found = None;
    let mut position = 0;
    body.visit_values(&body.instruction(site.instruction).kind, |value| {
        if position == site.position {
            found = Some(value);
        }
        position += 1;
    });
    found
}
pub(super) fn temp_type(
    body: &Body,
    temp: TempId,
) -> Result<crate::lowir::vocabulary::Type, Error> {
    body.temp(temp).ty.ok_or(Error::Limit)
}

#[cfg(test)]
mod tests {
    use super::super::pass_context::Pass;
    #[test]
    fn decided_phi_edges_are_routed_and_empty_merges_are_cleaned() {
        let (out, facts) = super::super::pass_view(
            "function @f(%c : i64)->i64 {block ^entry:branch %c, ^a, ^b block ^a:jump ^join block ^b:jump ^join block ^join:%x = phi i64 [^a: 0, ^b: 1] branch %x, ^yes, ^no block ^yes:return i64 1 block ^no:return i64 0}",
            1,
            Pass::ThreadJumps,
        );
        assert!(facts.0);
        assert_eq!(facts.1, 2);
        assert!(!out.contains("phi "));
        assert!(!out.contains("^join"));
    }
    #[test]
    fn signed_range_facts_thread_the_decided_path() {
        let (out, facts) = super::super::pass_view(
            "function @f(%v : i8)->i64 {block ^entry:%a = cmp lt i8 %v, -1 branch %a, ^path, ^miss block ^path:jump ^test block ^test:%b = cmp lt i8 %v, 0 branch %b, ^yes, ^no block ^miss:return i64 3 block ^yes:return i64 1 block ^no:return i64 0}",
            2,
            Pass::ThreadJumps,
        );
        assert!(facts.0);
        assert_eq!(facts.1, 1);
        assert!(!out.contains("%b = cmp"));
    }
    #[test]
    fn new_block_names_do_not_collide_with_explicit_thread_names() {
        let (out, facts) = super::super::pass_view(
            "function @f(%c : i64,%v : i64)->i64 {block ^entry:branch %c, ^a, ^b block ^a:jump ^join block ^b:jump ^join block ^join:%x = phi i64 [^a: 1, ^b: 2] %cmp = cmp eq i64 %x, %v branch %cmp, ^thread_5, ^no block ^thread_5:return i64 1 block ^no:return i64 0}",
            2,
            Pass::SpecializePhiCompares,
        );
        assert!(facts.0);
        assert!(out.contains("block ^thread_5:"));
        assert!(out.contains("thread_"));
    }
}

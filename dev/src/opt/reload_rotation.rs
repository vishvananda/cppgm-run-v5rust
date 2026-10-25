//! Late O3 reload-driven loop rotation from `opt/loop_rotation.cpp`. See NOTICE.
use super::{
    alias_facts,
    full_unrolling::{incoming, mapped, phis},
    induction, loop_motion,
    natural_loops::{Forest, LoopId},
    pass_context::{Context, Pass},
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, Facts as Flags, InstId, Instruction, InstructionKind as I, LinkEditor,
        OptimizerBlock, OptimizerSlot, Value,
    },
    vocabulary::{Binary, Type},
};

fn copyable(kind: &I) -> bool {
    if matches!(kind.result_type(), Type::Object { .. }) {
        return false;
    }
    match kind {
        I::Compare(..)
        | I::Unary(..)
        | I::Convert(..)
        | I::Copy(..)
        | I::Index(..)
        | I::Select(..) => true,
        I::Binary(op, ..) => !matches!(op, Binary::Div | Binary::Mod | Binary::UDiv | Binary::UMod),
        I::Load(flags, ty, _) => !flags.volatile() && *ty != Type::I128,
        _ => false,
    }
}
fn header_definition(body: &Body, defined: &[InstId], v: Value) -> Option<InstId> {
    let Value::Temp(t) = v else { return None };
    if body.temp_is_parameter(t) {
        return None;
    }
    body.temp(t).definition.filter(|id| defined.contains(id))
}
fn reload(
    context: &Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    inside: BlockId,
    defined: &[InstId],
) -> bool {
    let l = forest.get(loop_);
    let writes = l.blocks.iter().any(|&b| {
        context
            .body
            .block(b)
            .instructions
            .iter()
            .any(|&id| alias_facts::writes(context.program, &context.body.instruction(id).kind))
    });
    for &id in &context.body.block(inside).instructions {
        let kind = &context.body.instruction(id).kind;
        if alias_facts::writes(context.program, kind) {
            break;
        }
        let I::Load(flags, ty, address) = *kind else {
            continue;
        };
        if flags.volatile() || matches!(ty, Type::Object { .. }) {
            continue;
        }
        let at = loop_motion::locate(context.body, address);
        if !at.known {
            continue;
        }
        let outside =
            induction::invariant(context.body, forest, context.block_map(), loop_, at.root);
        if outside && !writes {
            return true;
        }
        let phi = header_definition(context.body, defined, at.root)
            .filter(|&id| matches!(context.body.instruction(id).kind, I::Phi(..)));
        if !outside && phi.is_none() {
            continue;
        }
        let every = l.latches.iter().all(|&lt| {
            let input = if outside {
                Some(at.root)
            } else {
                phi.and_then(|id| incoming(context.body, id, lt))
            };
            context.body.block(lt).instructions.iter().any(|&j| {
                let I::Load(_, prior_ty, address) = context.body.instruction(j).kind else {
                    return false;
                };
                if prior_ty != ty {
                    return false;
                }
                let p = loop_motion::locate(context.body, address);
                p.known && Some(p.root) == input && p.offset == at.offset
            })
        });
        if every {
            return true;
        }
    }
    false
}
struct Use {
    inst: InstId,
    block: BlockId,
    position: usize,
    def: InstId,
    phi: bool,
}
fn uses(context: &Context<'_>, header: BlockId, defined: &[InstId]) -> Option<Vec<Use>> {
    let mut uses = Vec::new();
    for &b in &context.body.layout {
        for &id in &context.body.block(b).instructions {
            let kind = &context.body.instruction(id).kind;
            let mut position = 0;
            let mut ok = true;
            context.body.visit_values(kind, |v| {
                let at = position;
                position += 1;
                let Some(def) = header_definition(context.body, defined, v) else {
                    return;
                };
                let phi = matches!(kind, I::Phi(..));
                let from = if let I::Phi(_, pairs) = kind {
                    pairs[at].0
                } else {
                    b
                };
                if context.cfg().is_ok_and(|cfg| cfg.handler(from)) {
                    ok = false;
                }
                if b == header && !phi && !matches!(context.body.instruction(def).kind, I::Phi(..))
                {
                    return;
                }
                uses.push(Use {
                    inst: id,
                    block: from,
                    position: at,
                    def,
                    phi,
                });
            });
            if !ok {
                return None;
            }
        }
    }
    Some(uses)
}
fn append_before(
    body: &mut Body,
    links: &mut LinkEditor,
    block: BlockId,
    before: InstId,
    inst: Instruction,
) -> Result<InstId, Error> {
    let id = body.append(block, inst)?;
    links.insert_after(block, links.previous(before), id)?;
    Ok(id)
}
fn demote(
    context: &mut Context<'_>,
    header: BlockId,
    defined: &[InstId],
    uses: &[Use],
) -> Result<(), Error> {
    let mut links = LinkEditor::new(context.body)?;
    for &d in defined {
        let inst = context.body.instruction(d).clone();
        let ty = context
            .body
            .temp(inst.result.ok_or(Error::Limit)?)
            .ty
            .ok_or(Error::Limit)?;
        let phi = matches!(inst.kind, I::Phi(..));
        if !phi && !uses.iter().any(|u| u.def == d) {
            continue;
        }
        let slot = context.body.optimizer_slot(ty, OptimizerSlot::Demote)?;
        if let I::Phi(_, pairs) = &inst.kind {
            for &(from, v) in pairs {
                let term = links.last(from).ok_or(Error::Limit)?;
                let id = append_before(
                    context.body,
                    &mut links,
                    from,
                    term,
                    Instruction::new(None, I::Store(Flags::default(), ty, v, slot), 0),
                )?;
                context.body.inherit_location(id, d);
            }
        } else {
            let id = context.body.append(
                header,
                Instruction::new(
                    None,
                    I::Store(
                        Flags::default(),
                        ty,
                        Value::Temp(inst.result.ok_or(Error::Limit)?),
                        slot,
                    ),
                    0,
                ),
            )?;
            context.body.inherit_location(id, d);
            links.insert_after(header, Some(d), id)?;
        }
        for u in uses.iter().filter(|u| u.def == d) {
            let before = if u.phi {
                links.last(u.block).ok_or(Error::Limit)?
            } else {
                u.inst
            };
            let result = context.body.generated_temp(ty)?;
            let id = append_before(
                context.body,
                &mut links,
                u.block,
                before,
                Instruction::new(Some(result), I::Load(Flags::default(), ty, slot), 0),
            )?;
            context.body.inherit_location(id, d);
            let mut at = 0;
            context.body.rewrite_values(u.inst, |v| {
                let next = if at == u.position {
                    Value::Temp(result)
                } else {
                    v
                };
                at += 1;
                Ok(next)
            })?;
        }
        if phi {
            links.unlink(header, d)?;
        }
    }
    links.commit(context.body)
}
fn rotate(context: &mut Context<'_>, forest: &Forest, loop_: LoopId) -> Result<bool, Error> {
    let l = forest.get(loop_);
    let h = l.header;
    let cfg = context.cfg()?;
    if cfg.handler(h) || l.latches.is_empty() || cfg.reverse_postorder().first() == Some(&h) {
        return Ok(false);
    }
    let Some(&term) = context.body.block(h).instructions.last() else {
        return Ok(false);
    };
    let I::Branch(_, yes, no) = context.body.instruction(term).kind else {
        return Ok(false);
    };
    if yes == no || forest.contains(loop_, yes) == forest.contains(loop_, no) {
        return Ok(false);
    }
    let (inside, outside) = if forest.contains(loop_, yes) {
        (yes, no)
    } else {
        (no, yes)
    };
    if inside == h || cfg.handler(inside) || cfg.handler(outside) {
        return Ok(false);
    }
    if l.latches.iter().any(|&lt| {
        lt == h
            || context
                .body
                .block(lt)
                .instructions
                .last()
                .is_none_or(|&id| !matches!(context.body.instruction(id).kind, I::Jump(_)))
    }) {
        return Ok(false);
    }
    if cfg
        .predecessors(h)
        .iter()
        .any(|&p| cfg.handler(p) || context.body.block(p).instructions.is_empty())
    {
        return Ok(false);
    }
    let mut defined = Vec::new();
    let mut plain = 0;
    for &id in &context.body.block(h).instructions {
        if id == term {
            break;
        }
        let inst = context.body.instruction(id);
        if matches!(inst.kind, I::Phi(..)) {
            if plain != 0 || matches!(inst.kind.result_type(), Type::Object { .. }) {
                return Ok(false);
            }
        } else {
            plain += 1;
            if plain > 4 || inst.result.is_none() || !copyable(&inst.kind) {
                return Ok(false);
            }
        }
        defined.push(id);
    }
    if !reload(context, forest, loop_, inside, &defined) {
        return Ok(false);
    }
    for &id in &defined {
        let I::Phi(_, pairs) = &context.body.instruction(id).kind else {
            break;
        };
        if pairs
            .iter()
            .any(|&(_, v)| header_definition(context.body, &defined, v).is_some())
        {
            return Ok(false);
        }
    }
    let Some(uses) = uses(context, h, &defined) else {
        return Ok(false);
    };
    demote(context, h, &defined, &uses)?;
    let pre = context
        .body
        .optimizer_block(OptimizerBlock::LoopPreheader)?;
    context
        .body
        .append(pre, Instruction::new(None, I::Jump(inside), 0))?;
    let at = context
        .body
        .layout
        .iter()
        .position(|&b| b == h)
        .ok_or(Error::Limit)?;
    context.body.layout.insert(at + 1, pre);
    context.body.redirect_successors(term, inside, pre);
    context.body.rename_phi_predecessor(inside, h, pre);
    let header_ids = context.body.block(h).instructions.clone();
    let mut links = LinkEditor::new(context.body)?;
    for &lt in &l.latches {
        let jump = links.last(lt).ok_or(Error::Limit)?;
        let mut at = links.previous(jump);
        let mut map = vec![None; context.body.temps().len()];
        for &id in &header_ids {
            if id == term {
                break;
            }
            let old = context.body.instruction(id).result;
            let copied = context
                .body
                .copy_instruction(lt, id, |v| mapped(&map, v), |b| b)?;
            if let Some(old) = old {
                map[old.index()] = context.body.instruction(copied).result.map(Value::Temp);
            }
            links.insert_after(lt, at, copied)?;
            at = Some(copied);
        }
        let I::Branch(v, yes, no) = context.body.instruction(term).kind else {
            return Err(Error::Limit);
        };
        context.body.replace_kind(
            jump,
            I::Branch(
                mapped(&map, v),
                if yes == pre { inside } else { outside },
                if no == pre { inside } else { outside },
            ),
        );
        for (target, from) in [(inside, pre), (outside, h)] {
            for id in phis(context.body, target) {
                let value = incoming(context.body, id, from).ok_or(Error::Limit)?;
                let mut inst = context.body.instruction(id).clone();
                let I::Phi(_, pairs) = &mut inst.kind else {
                    return Err(Error::Limit);
                };
                pairs.push((lt, mapped(&map, value)));
                context.body.update_instruction(id, inst);
            }
        }
    }
    links.commit(context.body)?;
    Ok(true)
}
pub(super) fn run(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut forest = Forest::default();
    let mut any = false;
    for _ in 0..16 {
        context.ensure_cfg()?;
        forest.find(context.body.blocks().len(), context.cfg()?)?;
        context.prepare_block_map(Pass::RotateLoops, false);
        let mut one = false;
        for loop_ in forest.ids().rev() {
            if rotate(context, &forest, loop_)? {
                one = true;
                break;
            }
        }
        if !one {
            break;
        }
        any = true;
        context.invalidate_cfg();
    }
    Ok(any)
}

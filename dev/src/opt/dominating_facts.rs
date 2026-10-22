//! Scoped edge equalities from `opt/jump_threading.cpp`. See NOTICE.
use super::{
    jump_threading::{definition, last},
    numeric_fold as fold,
    pass_context::Context,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{BlockId, InstructionKind as I, TempId, Value},
    vocabulary::{Compare as C, Type},
};
struct Facts {
    known: Vec<Option<Value>>,
    unequal: Vec<(TempId, Value)>,
    undo: Vec<TempId>,
    changed: bool,
}
impl Facts {
    fn set(&mut self, temp: TempId, value: Value) {
        if let Some(at) = self.known.get_mut(temp.index())
            && at.is_none()
        {
            *at = Some(value);
            self.undo.push(temp);
        }
    }
    fn enter(&mut self, c: &mut Context<'_>, block: BlockId) -> Result<(), Error> {
        let cfg = c.cfg()?;
        if cfg.reverse_postorder().first() == Some(&block)
            || cfg.predecessors(block).len() != 1
            || cfg.handler(block)
        {
            return Ok(());
        }
        let Some(tail) = last(c.body, cfg.predecessors(block)[0]) else {
            return Ok(());
        };
        let I::Branch(Value::Temp(condition), yes, no) = c.body.instruction(tail).kind else {
            return Ok(());
        };
        if yes == no || condition.index() >= self.known.len() {
            return Ok(());
        }
        let ty = c.body.temp(condition).ty.ok_or(Error::Limit)?;
        if (ty.bits() == 0 && ty != Type::Ptr) || ty == Type::I128 {
            return Ok(());
        }
        let taken = yes == block;
        let compare = definition(c.body, Value::Temp(condition)).and_then(|id| {
            if let I::Compare(pred, ty, x, k) = c.body.instruction(id).kind {
                Some((pred, ty, x, k))
            } else {
                None
            }
        });
        let Some((pred, ty, mut x, mut k)) = compare else {
            let zero = c.body.constant(Constant::Integer(0))?;
            if taken {
                self.unequal.push((condition, zero));
            } else {
                self.set(condition, zero);
            }
            return Ok(());
        };
        let value = c.body.constant(Constant::Integer(u128::from(taken)))?;
        self.set(condition, value);
        if !matches!(pred, C::Eq | C::Ne) || ty.float_rank() != 0 || ty == Type::I128 {
            return Ok(());
        }
        if !matches!(x, Value::Temp(_)) {
            std::mem::swap(&mut x, &mut k);
        }
        if let Value::Temp(temp) = x
            && temp.index() < self.known.len()
            && matches!(fold::literal(c.body, k), Some(Constant::Integer(_)))
            && c.body.temp(temp).ty == Some(ty)
        {
            if (pred == C::Eq) == taken {
                self.set(temp, k);
            } else {
                self.unequal.push((temp, k));
            }
        }
        Ok(())
    }
    fn rewrite(&mut self, c: &mut Context<'_>, block: BlockId) -> Result<(), Error> {
        for id in c.body.block(block).instructions.clone() {
            if matches!(c.body.instruction(id).kind, I::Phi(..)) {
                continue;
            }
            c.body.rewrite_values(id, |value| {
                if let Value::Temp(temp) = value
                    && let Some(replacement) = self.known.get(temp.index()).copied().flatten()
                {
                    self.changed = true;
                    Ok(replacement)
                } else {
                    Ok(value)
                }
            })?;
            let instruction = c.body.instruction(id);
            let I::Compare(pred, ty, mut x, mut k) = instruction.kind else {
                continue;
            };
            let Some(result) = instruction.result.filter(|t| t.index() < self.known.len()) else {
                continue;
            };
            if !matches!(pred, C::Eq | C::Ne) || self.unequal.is_empty() {
                continue;
            }
            if !matches!(x, Value::Temp(_)) {
                std::mem::swap(&mut x, &mut k);
            }
            let Value::Temp(temp) = x else {
                continue;
            };
            let Some(constant @ Constant::Integer(_)) = fold::literal(c.body, k) else {
                continue;
            };
            if self.unequal.iter().any(|&(other, value)| {
                temp == other
                    && fold::literal(c.body, value).is_some_and(|other| {
                        matches!(other, Constant::Integer(_)) && fold::same(ty, constant, other)
                    })
            }) {
                let value = c
                    .body
                    .constant(Constant::Integer(u128::from(pred == C::Ne)))?;
                self.set(result, value);
            }
        }
        Ok(())
    }
}
pub(super) fn rewrite(context: &mut Context<'_>) -> Result<bool, Error> {
    let Some(&entry) = context.cfg()?.reverse_postorder().first() else {
        return Ok(false);
    };
    let mut facts = Facts {
        known: vec![None; context.body.temps().len()],
        unequal: Vec::new(),
        undo: Vec::new(),
        changed: false,
    };
    let mut stack = vec![(entry, 0_usize, 0_usize, 0_usize)];
    facts.enter(context, entry)?;
    if !facts.undo.is_empty() || !facts.unequal.is_empty() {
        facts.rewrite(context, entry)?;
    }
    while let Some(&(block, child, undo, unequal)) = stack.last() {
        if let Some(&next) = context.cfg()?.children(block).get(child) {
            stack.last_mut().ok_or(Error::Limit)?.1 += 1;
            stack.push((next, 0, facts.undo.len(), facts.unequal.len()));
            facts.enter(context, next)?;
            if !facts.undo.is_empty() || !facts.unequal.is_empty() {
                facts.rewrite(context, next)?;
            }
        } else {
            for temp in facts.undo.drain(undo..) {
                facts.known[temp.index()] = None;
            }
            facts.unequal.truncate(unequal);
            stack.pop();
        }
    }
    Ok(facts.changed)
}

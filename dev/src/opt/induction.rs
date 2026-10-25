//! Loop induction, exit and external-use facts from `opt/loop_passes.cpp`.
//! See NOTICE.
use super::{
    alias_facts::{signed_size, signed_word},
    control_flow::Cfg,
    memory_contents::bytes,
    natural_loops::{Forest, LoopId},
    numeric_fold::normalize,
    trip_counts,
};
use crate::lowir::model::{BlockId, Body, InstId, InstructionKind as I, TempId, Value};
use crate::lowir::vocabulary::{Binary, Compare, Type};

#[derive(Clone, Copy)]
pub(super) struct Induction {
    pub(super) value: TempId,
    pub(super) init: Option<Value>,
    pub(super) next: TempId,
    pub(super) step: i64,
    pub(super) pointer: bool,
}
#[derive(Clone, Copy)]
pub(super) struct Exit {
    pub(super) exiting: BlockId,
    pub(super) target: BlockId,
    pub(super) compared: Value,
    pub(super) bound: Value,
    pub(super) value: Value,
    pub(super) trips: Option<i64>,
}
pub(super) struct ExitAnalysis {
    pub(super) test: Exit,
    pub(super) inductions: Vec<Induction>,
    pub(super) counted: Option<(Type, i128)>,
}
pub(super) struct Facts<'a> {
    pub(super) body: &'a Body,
    pub(super) cfg: &'a Cfg,
    pub(super) forest: &'a Forest,
    pub(super) block_of: &'a [Option<BlockId>],
}
pub(super) fn invariant(
    body: &Body,
    forest: &Forest,
    block_of: &[Option<BlockId>],
    loop_: LoopId,
    value: Value,
) -> bool {
    let Value::Temp(temp) = value else {
        return true;
    };
    if body.temp_is_parameter(temp) {
        return true;
    }
    let Some(id) = body.temp(temp).definition else {
        return false;
    };
    if id.index() >= block_of.len() {
        return true;
    }
    block_of[id.index()].is_some_and(|b| !forest.contains(loop_, b))
}
impl Facts<'_> {
    pub(super) fn invariant(&self, loop_: LoopId, value: Value) -> bool {
        invariant(self.body, self.forest, self.block_of, loop_, value)
    }
    pub(super) fn induction(&self, loop_: LoopId, id: InstId) -> Option<Induction> {
        let loop_id = loop_;
        let loop_ = self.forest.get(loop_);
        let phi = self.body.instruction(id);
        let I::Phi(ty, pairs) = &phi.kind else {
            return None;
        };
        if pairs.len() != 2 || loop_.latches.len() != 1 {
            return None;
        }
        let latch = loop_.latches[0];
        let next = pairs.iter().find(|&&(b, _)| b == latch)?.1;
        let mut init = loop_
            .preheader
            .and_then(|p| pairs.iter().find(|&&(b, _)| b == p).map(|&(_, v)| v));
        for &(block, value) in pairs {
            if block != latch {
                init = Some(value);
            }
        }
        if init.is_some_and(|v| !self.invariant(loop_id, v)) {
            return None;
        }
        let Value::Temp(next) = next else {
            return None;
        };
        let def = self.body.temp(next).definition?;
        let value = phi.result?;
        let (step, pointer) = match self.body.instruction(def).kind {
            I::Index(element, _, Value::Temp(base), Value::Integer(k))
                if base == value && *ty == Type::Ptr =>
            {
                (
                    signed_word(self.body.literal(k)).wrapping_mul(signed_size(bytes(element))),
                    true,
                )
            }
            I::Binary(op, _, step_ty, Value::Temp(base), Value::Integer(k))
                if base == value
                    && step_ty == *ty
                    && ty.bits() != 0
                    && matches!(op, Binary::Add | Binary::Sub) =>
            {
                let k = signed_word(normalize(step_ty, self.body.literal(k)));
                (
                    if op == Binary::Add {
                        k
                    } else {
                        k.wrapping_neg()
                    },
                    false,
                )
            }
            _ => return None,
        };
        (step != 0).then_some(Induction {
            value,
            init,
            next,
            step,
            pointer,
        })
    }
    fn inductions(&self, loop_: LoopId) -> Vec<Induction> {
        self.body
            .block(self.forest.get(loop_).header)
            .instructions
            .iter()
            .copied()
            .take_while(|&id| matches!(self.body.instruction(id).kind, I::Phi(..)))
            .filter_map(|id| self.induction(loop_, id))
            .collect()
    }
    fn choose(
        &self,
        loop_: LoopId,
        mut test: Exit,
        ivs: &[Induction],
        comparison: InstId,
        leave_on_true: bool,
    ) -> Option<(Exit, Option<(Type, i128)>)> {
        let I::Compare(pred, ty, left, right) = self.body.instruction(comparison).kind else {
            return None;
        };
        for iv in ivs {
            for side in 0..2 {
                let (value, other) = if side == 0 {
                    (left, right)
                } else {
                    (right, left)
                };
                let Value::Temp(t) = value else {
                    continue;
                };
                if (t != iv.value && t != iv.next) || !self.invariant(loop_, other) {
                    continue;
                }
                let mut pred = if side == 0 { pred } else { pred.swapped() };
                if matches!(other,Value::Integer(k) if self.body.literal(k)==0) {
                    pred = match pred {
                        Compare::Ugt => Compare::Ne,
                        Compare::Ule => Compare::Eq,
                        _ => pred,
                    };
                }
                test.compared = value;
                test.bound = other;
                test.value = other;
                test.trips = None;
                if !iv.pointer
                    && ty.bits() != 0
                    && let (Some(Value::Integer(a)), Value::Integer(b)) = (iv.init, other)
                {
                    let init = i128::from(signed_word(normalize(ty, self.body.literal(a))));
                    let bound = i128::from(signed_word(normalize(ty, self.body.literal(b))));
                    if let Some((trips, last)) = trip_counts::count(
                        ty,
                        pred,
                        !leave_on_true,
                        init,
                        iv.step,
                        bound,
                        t == iv.next,
                    ) {
                        test.trips = Some(trips);
                        return Some((test, Some((ty, last))));
                    }
                }
                let equality =
                    pred == Compare::Ne && !leave_on_true || pred == Compare::Eq && leave_on_true;
                if equality && (iv.pointer || iv.step == 1 || iv.step == -1) {
                    return Some((test, None));
                }
            }
        }
        None
    }
    pub(super) fn exit(&self, loop_: LoopId) -> Option<ExitAnalysis> {
        let l = self.forest.get(loop_);
        let mut edge = None;
        for &block in &l.blocks {
            for &target in self.cfg.successors(block) {
                if self.forest.contains(loop_, target) {
                    continue;
                }
                if edge.is_some() {
                    return None;
                }
                edge = Some((block, target));
            }
        }
        let (exiting, target) = edge?;
        if self.cfg.handler(target) {
            return None;
        }
        let ivs = self.inductions(loop_);
        let I::Branch(Value::Temp(condition), yes, no) = self
            .body
            .instruction(*self.body.block(exiting).instructions.last()?)
            .kind
        else {
            return None;
        };
        if yes == no {
            return None;
        }
        let comparison = self.body.temp(condition).definition?;
        let I::Compare(_, _, left, _) = self.body.instruction(comparison).kind else {
            return None;
        };
        let placeholder = left;
        let test = Exit {
            exiting,
            target,
            compared: placeholder,
            bound: placeholder,
            value: placeholder,
            trips: None,
        };
        let (test, counted) = self.choose(loop_, test, &ivs, comparison, yes == target)?;
        Some(ExitAnalysis {
            test,
            inductions: ivs,
            counted,
        })
    }
    pub(super) fn effect_free(&self, loop_: LoopId) -> bool {
        self.forest.get(loop_).blocks.iter().all(|&b| {
            self.body.block(b).instructions.iter().all(|&id| {
                let kind = &self.body.instruction(id).kind;
                matches!(kind, I::Jump(_) | I::Branch(..))
                    || super::pure(kind)
                        && !matches!(
                            kind,
                            I::Binary(Binary::Div | Binary::Mod | Binary::UDiv | Binary::UMod, ..)
                        )
            })
        })
    }
    pub(super) fn private_values(&self, loop_: LoopId, test: Exit) -> bool {
        for &block in &self.body.layout {
            if self.forest.contains(loop_, block) {
                continue;
            }
            for &id in &self.body.block(block).instructions {
                let mut ok = true;
                self.body
                    .visit_values(&self.body.instruction(id).kind, |v| {
                        ok &= !matches!(v, Value::Temp(_))
                            || self.invariant(loop_, v)
                            || v == test.compared;
                    });
                if !ok {
                    return false;
                }
            }
        }
        true
    }
}

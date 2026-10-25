//! Frozen whole-copy admission and wave budgets from `opt/inliner.cpp`. See NOTICE.
use super::{
    inline_shape::Shape,
    unit_graph::{CallGraph, FunctionIndex, discardable},
};
use crate::lowir::model::{Metadata, Program};
use crate::lowir::vocabulary::Binding;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Wave {
    Ordinary,
    Once,
    Late,
}
/// Frozen inliner budget overrides; zero fields select the default policy.
#[derive(Default, Clone, Copy)]
#[non_exhaustive]
pub struct Limits {
    pub(crate) caller_budget: u32,
    pub(crate) once_cap: u32,
    pub(crate) once_caller_budget: u32,
    pub(crate) hint_late_cap: u32,
}
impl Limits {
    /// Apply one decimal `name=value` override, rejecting invalid or zero limits.
    pub fn parse_override(&mut self, bytes: &[u8]) -> bool {
        let Some(equals) = bytes.iter().position(|&byte| byte == b'=') else {
            return false;
        };
        let value = &bytes[equals + 1..];
        if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
            return false;
        }
        let Some(value) = value
            .iter()
            .try_fold(0_u32, |n, byte| {
                n.checked_mul(10)?.checked_add(u32::from(byte - b'0'))
            })
            .filter(|&n| n != 0)
        else {
            return false;
        };
        let destination = match &bytes[..equals] {
            b"caller-budget" => &mut self.caller_budget,
            b"once-cap" => &mut self.once_cap,
            b"once-caller-budget" => &mut self.once_caller_budget,
            b"hint-late-cap" => &mut self.hint_late_cap,
            _ => return false,
        };
        *destination = value;
        true
    }
}

#[derive(Clone, Copy)]
pub(super) struct Options {
    pub(super) level: u8,
    pub(super) limits: Limits,
    pub(super) inline_thunk_targets: bool,
}
impl Options {
    pub(super) fn new(level: u8) -> Self {
        Self {
            level,
            limits: Limits::default(),
            inline_thunk_targets: false,
        }
    }
}
pub(super) struct Budgets {
    pub(super) once: u64,
    pub(super) growth: u64,
    pub(super) force: u64,
}
impl Budgets {
    pub(super) fn new(input: u64) -> Self {
        let budget = input.max(10_240);
        Self {
            once: budget,
            growth: budget,
            force: budget,
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Admission {
    pub(super) charge: u32,
    pub(super) once: bool,
}
pub(super) struct Caller {
    pub(super) index: FunctionIndex,
    pub(super) wrapper: bool,
    pub(super) leaf_wrapper: bool,
    pub(super) settled: bool,
    pub(super) budget: u64,
    pub(super) once: u64,
    pub(super) folded: u64,
}
impl Caller {
    pub(super) fn new(
        index: FunctionIndex,
        shape: Shape,
        graph: &CallGraph,
        metadata: &Metadata,
        options: Options,
    ) -> Self {
        let settled = options.level >= 2
            && (graph.calls(index) == 0
                || shape.size > 320 && !(discardable(metadata) && graph.calls(index) == 1));
        Self {
            index,
            wrapper: shape.fast_return && shape.hot <= 32,
            leaf_wrapper: false,
            settled,
            budget: u64::from(if options.limits.caller_budget == 0 {
                1536
            } else {
                options.limits.caller_budget
            }),
            once: u64::from(if options.limits.once_caller_budget == 0 {
                1024 * if options.level >= 2 { 2 } else { 1 }
            } else {
                options.limits.once_caller_budget
            }),
            folded: 2048,
        }
    }
}
fn fast(shape: Shape, hint: bool, scale: u32) -> bool {
    shape.blocks > 1
        && shape.hot_call
        && !shape.looping
        && shape.fast_return
        && !shape.allocates
        && shape.fast <= (if hint { 48 } else { 24 }) * scale
        && shape.hot <= (if hint { 160 } else { 96 }) * scale
}
pub(super) struct Policy<'a> {
    pub(super) program: &'a Program,
    pub(super) graph: &'a CallGraph,
    pub(super) shapes: &'a [Shape],
    pub(super) input: &'a [Shape],
    pub(super) original: &'a [u32],
    pub(super) caller: &'a Caller,
    pub(super) budgets: &'a Budgets,
    pub(super) options: Options,
    pub(super) wave: Wave,
}
impl Policy<'_> {
    fn fit(&self, s: Shape, current: Shape, g: FunctionIndex, hint: bool, loop_site: bool) -> u8 {
        if s.hot <= 4 && !s.call {
            return 1;
        }
        let call_free = self.options.level >= 2 && !s.hot_call;
        let shared =
            current.looping && self.graph.calls(g) > 1 && !hint && (!call_free || current.hot > 48);
        let settled = self.caller.settled && self.graph.calls(g) <= 3;
        let (num, den) = if settled { (3, 2) } else { (1, 1) };
        let straight = s.blocks == 1 && s.returns == 1 && !s.hot_call;
        let cap = (if hint || call_free {
            48 * num / den
        } else if straight {
            40
        } else if s.hot_call {
            6
        } else {
            24
        }) * if loop_site { 2 } else { 1 };
        u8::from(!shared && (s.hot <= cap || fast(s, hint, if loop_site { 2 } else { 1 }))) * 2
    }
    pub(super) fn admit(
        &self,
        g: FunctionIndex,
        handler: bool,
        loop_site: bool,
        dispatch: bool,
        grown: &mut [u32],
    ) -> Option<Admission> {
        let callee = self.graph.symbol(g);
        let m = self
            .program
            .callee_metadata(crate::lowir::model::Value::Symbol(callee))?;
        let shape = self.shapes[g.index()];
        if self.graph.recursive(g)
            || g == self.caller.index
            || m.flags & (32 | 64) != 0
            || shape.unusable
            || shape.eh
            || handler && shape.unwinds
        {
            return None;
        }
        let hint = m.flags & (16 | 8) != 0;
        let size = shape.hot;
        let single = discardable(m) && self.graph.calls(g) == 1 && !self.graph.observed(g);
        let admit = |charge, once| Some(Admission { charge, once });
        if m.flags & 8 != 0 {
            return admit(0, false);
        }
        if dispatch
            && shape.loop_dispatch
            && self.options.level >= 2
            && size <= 96
            && u64::from(size) <= self.caller.budget
        {
            return admit(size, false);
        }
        if self.caller.wrapper && size > 16 || self.caller.leaf_wrapper && size > 32 {
            return None;
        }
        if self.options.level >= 2
            && shape.looping
            && shape.calls == 1
            && size <= 12
            && u64::from(size) <= self.caller.budget
        {
            return admit(size, false);
        }
        let scale = if loop_site { 2 } else { 1 };
        let settled = self.caller.settled && self.graph.calls(g) <= 3;
        let (num, den) = if settled { (3, 2) } else { (1, 1) };
        if self.wave == Wave::Late {
            let leaf = shape.blocks == 1 && shape.returns == 1 && !shape.call;
            let call_free = self.options.level >= 2 && !shape.hot_call;
            let hint_cap = if self.options.limits.hint_late_cap == 0 {
                48
            } else {
                self.options.limits.hint_late_cap
            };
            let cap = if leaf && !call_free {
                40
            } else if hint || call_free {
                (if hint { hint_cap } else { 48 }).wrapping_mul(num) / den
            } else {
                6
            };
            return ((size <= cap.wrapping_mul(scale) || fast(shape, hint, scale))
                && u64::from(size) <= self.caller.budget)
                .then_some(Admission {
                    charge: size,
                    once: false,
                });
        }
        if self.wave == Wave::Ordinary {
            let mut fit = self.fit(shape, shape, g, hint, loop_site);
            if fit == 0
                && loop_site
                && !shape.allocates
                && self.input.get(g.index()).is_some_and(|s| s.hot < shape.hot)
                && grown[g.index()] < 2
            {
                let old = self.input[g.index()];
                let judged = Shape {
                    size: old.size,
                    hot: old.hot,
                    hot_call: old.hot_call,
                    blocks: old.blocks,
                    returns: old.returns,
                    call: old.call,
                    calls: old.calls,
                    fast: old.fast,
                    fast_return: old.fast_return,
                    ..shape
                };
                fit = self.fit(judged, shape, g, hint, loop_site);
                if fit != 0 {
                    grown[g.index()] += 1;
                }
            }
            if fit == 1 {
                return admit(0, false);
            }
            if fit == 2 {
                let charge = self
                    .original
                    .get(callee.index())
                    .copied()
                    .unwrap_or(size)
                    .max(size);
                if u64::from(charge) <= self.caller.budget {
                    return admit(charge, false);
                }
            }
        }
        let cap = if self.options.limits.once_cap == 0 {
            512 * if self.options.level >= 2 { 2 } else { 1 }
        } else {
            self.options.limits.once_cap
        };
        let sole = self.options.level >= 2
            && m.binding == Some(Binding::Internal)
            && self.graph.callers(g) == 1
            && !self.graph.observed(g)
            && discardable(m)
            && size.wrapping_mul(self.graph.calls(g)) <= cap;
        if !(single || sole)
            || size > cap
            || u64::from(size) > self.caller.once
            || u64::from(size) > self.budgets.once
        {
            if self.caller.settled
                && self.wave != Wave::Once
                && self.graph.calls(g) <= 2
                && size
                    <= (if self.shapes[self.caller.index.index()].size <= 128 {
                        256
                    } else {
                        128
                    })
                && u64::from(size) <= self.caller.budget
                && shape.calls.wrapping_mul(8) <= shape.size
            {
                return admit(size, false);
            }
            return None;
        }
        admit(size, true)
    }
}

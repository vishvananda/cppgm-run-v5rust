//! Callee sizing, cold regions and copy safety from `opt/inliner.cpp`. See NOTICE.
use super::{alias_facts::eh, control_flow::Cfg, natural_loops::Forest};
use crate::lowir::{
    Error,
    model::{
        Arity, Body, InstructionKind as I, Program, RuntimeInstruction as R, SymbolId, Unwind,
        Value,
    },
    vocabulary::Role,
};
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent inferred body facts used by frozen admission rules"
)]
#[derive(Clone, Copy, Default)]
pub(super) struct Shape {
    pub(super) size: u32,
    pub(super) hot: u32,
    pub(super) hot_call: bool,
    pub(super) blocks: u32,
    pub(super) returns: u32,
    pub(super) eh: bool,
    pub(super) call: bool,
    pub(super) calls: u32,
    pub(super) looping: bool,
    pub(super) loop_dispatch: bool,
    pub(super) unusable: bool,
    pub(super) unwinds: bool,
    pub(super) fast: u32,
    pub(super) fast_return: bool,
    pub(super) allocates: bool,
}
pub(super) fn phi_type(ty: crate::lowir::vocabulary::Type) -> bool {
    use crate::lowir::vocabulary::Type as T;
    matches!(ty, T::Ptr | T::F32 | T::F64) || ty.bits() != 0 && ty != T::I128
}
fn count(n: usize) -> Result<u32, Error> {
    u32::try_from(n).map_err(|_| Error::Limit)
}
fn calls(body: &Body, block: crate::lowir::model::BlockId) -> bool {
    body.block(block)
        .instructions
        .iter()
        .any(|&id| matches!(body.instruction(id).kind, I::Call(_)))
}
pub(super) fn cold(program: &Program, body: &Body, cfg: &Cfg, unreachable: bool) -> Vec<bool> {
    let mut cold = vec![false; body.blocks().len()];
    let mut rank = vec![None; body.blocks().len()];
    for (r, &b) in cfg.reverse_postorder().iter().enumerate() {
        rank[b.index()] = Some(r);
    }
    for (r, &b) in cfg.reverse_postorder().iter().enumerate().rev() {
        let ends = body.block(b).instructions.iter().any(|&id| {
            let k = &body.instruction(id).kind;
            matches!(k, I::Call(_)) && program.may_end_block(k)
                || unreachable && matches!(k, I::Unreachable)
        });
        let all = !cfg.successors(b).is_empty()
            && cfg
                .successors(b)
                .iter()
                .all(|s| cold[s.index()] && rank[s.index()].is_some_and(|i| i > r));
        cold[b.index()] = (ends || all) && body.layout.first() != Some(&b);
    }
    cold
}
fn unwinds(program: &Program, kind: &I, nounwind: &[bool]) -> bool {
    match kind {
        I::Call(call) => {
            program
                .call_boundary(call)
                .is_none_or(|s| s.boundary.unwind != Unwind::No)
                && !matches!(call.callee,Value::Symbol(s)if nounwind.get(s.index()).copied().unwrap_or(false))
        }
        I::Runtime(R::Throw(..) | R::Resume) => true,
        _ => false,
    }
}
pub(super) fn of(program: &Program, symbol: SymbolId, nounwind: &[bool]) -> Result<Shape, Error> {
    let function = program
        .function(program.symbol(symbol))
        .ok_or(Error::Limit)?;
    let body = function.body.as_ref().ok_or(Error::Limit)?;
    let sig = program.signature(function.signature);
    let mut out = Shape {
        unusable: sig.boundary.arity == Arity::Variadic || body.layout.is_empty(),
        ..Shape::default()
    };
    let mut unwind = false;
    for &b in &body.layout {
        out.blocks = out.blocks.checked_add(1).ok_or(Error::Limit)?;
        out.size = out
            .size
            .checked_add(count(body.block(b).instructions.len())?)
            .ok_or(Error::Limit)?;
        for &id in &body.block(b).instructions {
            let kind = &body.instruction(id).kind;
            out.eh |= eh(kind);
            if matches!(kind, I::Call(_)) {
                out.call = true;
                out.calls = out.calls.checked_add(1).ok_or(Error::Limit)?;
            }
            if matches!(kind, I::Return(..)) {
                out.returns = out.returns.checked_add(1).ok_or(Error::Limit)?;
            }
            unwind |= unwinds(program, kind, nounwind);
            out.unusable |= matches!(
                kind,
                I::Runtime(R::VaStart(_) | R::VaArg(..) | R::StackAlloc(_))
            ) || body.layout.first() == Some(&b) && matches!(kind, I::Phi(..));
        }
    }
    out.unwinds = unwind
        && sig.boundary.unwind != Unwind::No
        && !nounwind.get(symbol.index()).copied().unwrap_or(false);
    out.hot = out.size;
    out.hot_call = out.call;
    if out.blocks <= 1 {
        return Ok(out);
    }
    let mut cfg = Cfg::light(body)?;
    let mut rank = vec![None; body.blocks().len()];
    for (r, &b) in cfg.reverse_postorder().iter().enumerate() {
        rank[b.index()] = Some(r);
    }
    out.looping = cfg.reverse_postorder().iter().any(|&b| {
        cfg.successors(b)
            .iter()
            .any(|&s| cfg.reachable(s) && rank[s.index()] <= rank[b.index()])
    });
    let cold = cold(program, body, &cfg, false);
    out.hot_call = false;
    for &b in cfg.reverse_postorder() {
        if cold[b.index()] {
            out.hot = out
                .hot
                .checked_sub(count(body.block(b).instructions.len())?)
                .ok_or(Error::Limit)?;
            continue;
        }
        for &id in &body.block(b).instructions {
            if let I::Call(call) = &body.instruction(id).kind {
                out.hot_call = true;
                out.allocates |= matches!(call.callee,Value::Symbol(s)if program.callee_metadata(Value::Symbol(s)).is_some_and(|m|m.role==Some(Role::AllocateMemory)));
            }
        }
    }
    let entry = *body.layout.first().ok_or(Error::Limit)?;
    let mut region = vec![false; body.blocks().len()];
    let mut work = Vec::new();
    if !calls(body, entry) {
        region[entry.index()] = true;
        work.push(entry);
    }
    while let Some(b) = work.pop() {
        out.fast = out
            .fast
            .checked_add(count(body.block(b).instructions.len())?)
            .ok_or(Error::Limit)?;
        out.fast_return |= body
            .block(b)
            .instructions
            .iter()
            .any(|&id| matches!(body.instruction(id).kind, I::Return(..)));
        for &s in cfg.successors(b) {
            if !region[s.index()] && cfg.reachable(s) && !calls(body, s) {
                region[s.index()] = true;
                work.push(s);
            }
        }
    }
    if !out.looping && region[entry.index()] {
        let mut longest = vec![0_u32; body.blocks().len()];
        let mut returns = vec![false; body.blocks().len()];
        for (r, &b) in cfg.reverse_postorder().iter().enumerate().rev() {
            if !region[b.index()] {
                continue;
            }
            let own = count(body.block(b).instructions.len())?;
            let mut tail = 0;
            returns[b.index()] = body
                .block(b)
                .instructions
                .iter()
                .any(|&id| matches!(body.instruction(id).kind, I::Return(..)));
            for &s in cfg.successors(b) {
                if region[s.index()] && returns[s.index()] && rank[s.index()].is_some_and(|i| i > r)
                {
                    tail = tail.max(longest[s.index()]);
                    returns[b.index()] = true;
                }
            }
            longest[b.index()] = own.checked_add(tail).ok_or(Error::Limit)?;
        }
        if returns[entry.index()] {
            out.fast = longest[entry.index()];
        }
    }
    if out.looping && out.hot_call {
        cfg.complete()?;
        let mut forest = Forest::default();
        forest.find(body.blocks().len(), &cfg)?;
        for &b in cfg.reverse_postorder() {
            if cold[b.index()] || forest.innermost(b).is_none() {
                continue;
            }
            out.loop_dispatch|=body.block(b).instructions.iter().any(|&id|matches!(&body.instruction(id).kind,I::Call(c)if !matches!(c.callee,Value::Symbol(_))));
        }
    }
    Ok(out)
}

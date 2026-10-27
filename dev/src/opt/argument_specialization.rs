//! Call-site generations and parameter removal from `opt/interprocedural.cpp`.
//! See NOTICE. Agreements belong to one immutable call-graph snapshot.
use super::{numeric_fold as fold, unit_graph::CallGraph};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Arity, Body, InstId, InstructionKind as I, Program, SymbolId, Value},
    vocabulary::Binding,
};

#[derive(Clone, Copy, Default)]
struct Agreement {
    value: Option<Value>,
    constant: Option<Constant>,
    varies: bool,
}
impl Agreement {
    fn materialize(self, body: &mut Body) -> Result<Value, Error> {
        self.constant
            .map_or_else(|| self.value.ok_or(Error::Limit), |c| body.constant(c))
    }
}
#[derive(Clone, Copy)]
struct Site {
    caller: usize,
    generation: u32,
    instruction: InstId,
}
struct Specializer<'a> {
    graph: &'a CallGraph,
    ok: Vec<bool>,
    agree: Vec<Vec<Agreement>>,
    sites: Vec<Vec<Site>>,
    generation: Vec<u32>,
}
fn body(program: &Program, symbol: SymbolId) -> Result<&Body, Error> {
    program
        .function(program.symbol(symbol))
        .and_then(|f| f.body.as_ref())
        .ok_or(Error::Limit)
}
impl Specializer<'_> {
    fn record(&mut self, program: &Program, caller: usize, agreeing: bool) -> Result<(), Error> {
        let body = body(program, self.graph.functions[caller])?;
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                let I::Call(call) = &body.instruction(id).kind else {
                    continue;
                };
                let Value::Symbol(symbol) = call.callee else {
                    continue;
                };
                let Some(index) = self.graph.position(symbol) else {
                    continue;
                };
                let f = index.index();
                if !self.ok[f] {
                    continue;
                }
                self.sites[f].push(Site {
                    caller,
                    generation: self.generation[caller],
                    instruction: id,
                });
                if !agreeing {
                    continue;
                }
                let function = program
                    .function(program.symbol(symbol))
                    .ok_or(Error::Limit)?;
                let sig = program.signature(function.signature);
                let args = body.arguments(call.arguments);
                if call.signature.is_some() || args.len() != sig.parameters.len() {
                    self.ok[f] = false;
                    continue;
                }
                for ((a, &v), parameter) in self.agree[f].iter_mut().zip(args).zip(&sig.parameters)
                {
                    let literal =
                        if matches!(parameter.ty, crate::lowir::vocabulary::Type::Object { .. }) {
                            None
                        } else {
                            fold::literal(body, v)
                        };
                    if literal.is_none() && !matches!(v, Value::Symbol(_)) {
                        a.varies = true;
                        continue;
                    }
                    if a.value.is_none() {
                        a.value = Some(v);
                        a.constant = literal;
                        continue;
                    }
                    let same = match (literal, a.constant) {
                        (Some(c), Some(old)) => fold::same(parameter.ty, old, c),
                        (Some(_), None) => false,
                        (None, _) => a.value == Some(v),
                    };
                    a.varies |= !same;
                }
            }
        }
        Ok(())
    }
    fn substitute(&self, body: &mut Body, f: usize) -> Result<bool, Error> {
        let agreements = &self.agree[f];
        let mut made = vec![None; agreements.len()];
        let mut changed = false;
        let mut values = Vec::new();
        for block in body.layout.clone() {
            for id in body.block(block).instructions.clone() {
                // Literal creation is lazy in first-use order, including float bits.
                values.clear();
                let mut first = matches!(body.instruction(id).kind, I::Call(_));
                body.visit_values(&body.instruction(id).kind, |v| {
                    let callee = std::mem::take(&mut first);
                    if let Value::Temp(t) = v
                        && let Some(a) = agreements.get(t.index())
                        && a.value.is_some()
                        && !a.varies
                        && !(callee && a.constant.is_some())
                    {
                        values.push(t);
                    }
                });
                for &t in &values {
                    if made[t.index()].is_none() {
                        made[t.index()] = Some(agreements[t.index()].materialize(body)?);
                    }
                }
                let mut first = matches!(body.instruction(id).kind, I::Call(_));
                body.rewrite_values(id, |v| {
                    let callee = std::mem::take(&mut first);
                    if let Value::Temp(t) = v
                        && let Some(a) = agreements.get(t.index())
                        && !a.varies
                        && !(callee && a.constant.is_some())
                        && let Some(value) = made[t.index()]
                    {
                        changed = true;
                        return Ok(value);
                    }
                    Ok(v)
                })?;
            }
        }
        Ok(changed)
    }
    fn remove(&self, program: &mut Program, f: usize, removed: &[bool]) -> Result<(), Error> {
        let symbol = self.graph.functions[f];
        program.remove_function_parameters(symbol, removed)?;
        let mut body = program.take_body(symbol)?.ok_or(Error::Limit)?;
        let result = body
            .retain_parameters(removed)
            .and_then(|()| body.compact());
        program.restore_body(symbol, body)?;
        result?;
        for site in &self.sites[f] {
            if site.generation != self.generation[site.caller] {
                continue;
            }
            let caller = self.graph.functions[site.caller];
            let mut body = program.take_body(caller)?.ok_or(Error::Limit)?;
            let result = (|| {
                let I::Call(call) = &body.instruction(site.instruction).kind else {
                    return Ok(());
                };
                if call.callee != Value::Symbol(symbol) {
                    return Ok(());
                }
                let mut call = call.clone();
                let args = body.arguments(call.arguments);
                if args.len() != removed.len() {
                    return Err(Error::Limit);
                }
                let kept = args
                    .iter()
                    .zip(removed)
                    .filter_map(|(&v, &r)| (!r).then_some(v))
                    .collect::<Vec<_>>();
                call.arguments = body.add_arguments(&kept)?;
                body.replace_kind(site.instruction, I::Call(call));
                Ok(())
            })();
            program.restore_body(caller, body)?;
            result?;
        }
        Ok(())
    }
    fn renew(&mut self, program: &Program, f: usize) -> Result<(), Error> {
        self.generation[f] = self.generation[f].checked_add(1).ok_or(Error::Limit)?;
        self.record(program, f, false)
    }
}
pub(super) fn specialize(
    program: &mut Program,
    mut simplify: impl FnMut(&mut Program, SymbolId) -> Result<(), Error>,
) -> Result<u64, Error> {
    let graph = CallGraph::build(program)?;
    let n = graph.functions.len();
    let mut sp = Specializer {
        graph: &graph,
        ok: vec![false; n],
        agree: vec![Vec::new(); n],
        sites: vec![Vec::new(); n],
        generation: vec![0; n],
    };
    for (f, &symbol) in graph.functions.iter().enumerate() {
        let function = program
            .function(program.symbol(symbol))
            .ok_or(Error::Limit)?;
        let sig = program.signature(function.signature);
        let index = graph.position(symbol).ok_or(Error::Limit)?;
        if function.metadata.binding == Some(Binding::Internal)
            && !graph.observed(index)
            && graph.calls(index) > 0
            && !graph.recursive(index)
            && sig.boundary.arity == Arity::Fixed
            && !sig.parameters.is_empty()
        {
            sp.ok[f] = true;
            sp.agree[f].resize(sig.parameters.len(), Agreement::default());
        }
    }
    for g in 0..n {
        sp.record(program, g, true)?;
    }
    let callers = graph.reverse_calls();
    let mut changed_callers = vec![false; n];
    let mut count = 0;
    for (f, callers) in callers.iter().enumerate() {
        if !sp.ok[f] {
            continue;
        }
        let s = graph.functions[f];
        let mut body = program.take_body(s)?.ok_or(Error::Limit)?;
        let result = sp.substitute(&mut body, f);
        program.restore_body(s, body)?;
        let substituted = result?;
        if substituted {
            simplify(program, s)?;
            sp.renew(program, f)?;
        }
        let mut body = program.take_body(s)?.ok_or(Error::Limit)?;
        let result = body.recount_uses();
        let removed = body
            .temps()
            .iter()
            .take(sp.agree[f].len())
            .map(|t| t.uses == 0)
            .collect::<Vec<_>>();
        program.restore_body(s, body)?;
        result?;
        let remove = removed.contains(&true);
        if !remove && !substituted {
            continue;
        }
        if remove {
            sp.remove(program, f, &removed)?;
            sp.renew(program, f)?;
            for &g in callers {
                changed_callers[g.index()] = true;
            }
        }
        count += 1;
    }
    for (g, changed) in changed_callers.into_iter().enumerate() {
        if changed {
            simplify(program, graph.functions[g])?;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parameter_removal_repairs_compacted_call_sites_of_later_candidates() {
        let mut program = crate::lowir::test_program(b"function @root(%x:i64)->i64 {block ^entry:%a=call i64 @first(%x,7) return i64 %a} function @first(%x:i64,%bias:i64)->i64 [binding=internal,no_inline=yes] {block ^entry:%b=call i64 @second(%bias,%x) return i64 %b} function @second(%bias:i64,%x:i64)->i64 [binding=internal,no_inline=yes] {block ^entry:%r=binary add i64 %x,7 return i64 %r}").unwrap();
        let mut nounwind = Vec::new();
        assert_eq!(
            specialize(&mut program, |program, symbol| {
                super::super::simplify_function(
                    program,
                    symbol,
                    2,
                    &mut nounwind,
                    &[],
                    &mut super::super::Statistics::default(),
                    super::super::Verification::default(),
                )
            })
            .unwrap(),
            2
        );
        crate::lowir::validate_test_program(&program).unwrap();
        let mut out = Vec::new();
        crate::lowir::writer::write(&program, &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("function @first(%x : i64)"));
        assert!(out.contains("function @second(%x : i64)"));
        assert!(out.contains("call i64 @second(%x)"));
        assert!(out.contains("binary add i64 %x, 7"));
    }
    #[test]
    fn signed_zero_disagreement_keeps_a_float_parameter() {
        let mut program = crate::lowir::test_program(b"function @root()->f64 {block ^entry:%a=call f64 @f(0.0) %b=call f64 @f(-0.0) %c=binary add f64 %a,%b return f64 %c} function @f(%v:f64)->f64 [binding=internal,no_inline=yes] {block ^entry:return f64 %v}").unwrap();
        assert_eq!(specialize(&mut program, |_, _| Ok(())).unwrap(), 0);
        crate::lowir::validate_test_program(&program).unwrap();
    }
}

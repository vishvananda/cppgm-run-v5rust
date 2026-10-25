//! Callee-first bounded inline waves from `opt/inliner.cpp`. See NOTICE.
use super::{
    body_expansion::Expansion,
    control_flow::Cfg,
    inline_arguments as args,
    inline_policy::{Admission, Budgets, Caller, Options, Policy, Wave},
    inline_shape::{self, Shape},
    natural_loops::Forest,
    partial_inline::Cut,
    unit_graph::{CallGraph, FunctionIndex, discardable},
};
use crate::lowir::{
    Error,
    model::{Arity, BlockId, Body, InstId, InstructionKind as I, Program, SymbolId, Value},
};
#[derive(Default)]
pub(super) struct Stats {
    pub(super) calls: u64,
    pub(super) instructions: u64,
}
pub(super) type Simplifier<'a> =
    dyn FnMut(&mut Program, SymbolId, &mut Vec<bool>) -> Result<(), Error> + 'a;
pub(super) struct Shared<'a> {
    pub(super) budgets: &'a mut Budgets,
    pub(super) original: &'a [u32],
    pub(super) nounwind: &'a mut Vec<bool>,
    pub(super) stats: &'a mut Stats,
}
pub(super) struct Runner<'a, 'b> {
    pub(super) program: &'a mut Program,
    pub(super) options: Options,
    pub(super) wave: Wave,
    pub(super) budgets: &'a mut Budgets,
    pub(super) original: &'a [u32],
    pub(super) nounwind: &'a mut Vec<bool>,
    pub(super) simplify: &'a mut Simplifier<'b>,
    pub(super) stats: &'a mut Stats,
    graph: CallGraph,
    shapes: Vec<Shape>,
    input: Vec<Shape>,
    failures: Vec<u32>,
    folded_names: u32,
}
impl<'a, 'b> Runner<'a, 'b> {
    pub(super) fn new(
        program: &'a mut Program,
        options: Options,
        wave: Wave,
        shared: Shared<'a>,
        simplify: &'a mut Simplifier<'b>,
    ) -> Result<Self, Error> {
        let Shared {
            budgets,
            original,
            nounwind,
            stats,
        } = shared;
        let graph = CallGraph::build(program)?;
        let shapes = graph
            .functions
            .iter()
            .map(|&s| inline_shape::of(program, s, nounwind))
            .collect::<Result<Vec<_>, _>>()?;
        let input = if wave == Wave::Ordinary {
            shapes.clone()
        } else {
            Vec::new()
        };
        let failures = vec![0; shapes.len()];
        Ok(Self {
            program,
            options,
            wave,
            budgets,
            original,
            nounwind,
            simplify,
            stats,
            graph,
            shapes,
            input,
            failures,
            folded_names: 0,
        })
    }
    fn folded(
        &mut self,
        body: &Body,
        call_id: InstId,
        g: FunctionIndex,
        handler: bool,
        caller: &mut Caller,
    ) -> Result<Option<SymbolId>, Error> {
        let target = self.graph.symbol(g);
        let shape = self.shapes[g.index()];
        let metadata = *self
            .program
            .callee_metadata(Value::Symbol(target))
            .ok_or(Error::Limit)?;
        let hint = metadata.flags & (8 | 16) != 0;
        if !shape.eh
            || shape.size > 256
            || self.graph.recursive(g)
            || shape.unusable
            || metadata.flags & (32 | 64) != 0
            || shape.looping && !hint
            || self.failures[g.index()] >= 16
            || caller.folded == 0
        {
            return Ok(None);
        }
        let I::Call(call) = &body.instruction(call_id).kind else {
            return Err(Error::Limit);
        };
        let constants = body
            .arguments(call.arguments)
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| args::constant(self.program, body, v).map(|v| (i, v)))
            .collect::<Vec<_>>();
        if constants.is_empty() {
            return Ok(None);
        }
        let name = loop {
            let text = format!("__folded_copy{}", self.folded_names);
            self.folded_names = self.folded_names.checked_add(1).ok_or(Error::Limit)?;
            let name = self.program.names.intern(&text)?;
            if self.program.find_symbol(name).is_none() {
                break name;
            }
        };
        let copy = self.program.inline_copy(target, name)?;
        let mut copied = self.program.take_body(copy)?.ok_or(Error::Limit)?;
        let result = (|| {
            substitute_constants(&mut copied, body, constants)?;
            Ok(())
        })();
        self.program.restore_body(copy, copied)?;
        result?;
        (self.simplify)(self.program, copy, self.nounwind)?;
        let folded = inline_shape::of(self.program, copy, self.nounwind)?;
        if folded.eh
            || folded.unusable
            || folded.size > if hint { 80 } else { 40 }
            || u64::from(folded.size) > caller.folded
            || handler && folded.unwinds
        {
            self.program.discard_function_definition(copy)?;
            self.failures[g.index()] += 1;
            return Ok(None);
        }
        caller.folded -= u64::from(folded.size);
        Ok(Some(copy))
    }
    fn leaf_path(&self, body: &Body, cfg: &Cfg) -> bool {
        let leaf = |b| {
            body.block(b).instructions.iter().all(|&id| {
                let I::Call(call) = &body.instruction(id).kind else {
                    return true;
                };
                let Value::Symbol(s) = call.callee else {
                    return false;
                };
                let Some(g) = self.graph.position(s) else {
                    return false;
                };
                self.program
                    .callee_metadata(call.callee)
                    .is_some_and(|m| m.flags & 8 != 0)
                    || self.shapes[g.index()].size <= 4 && !self.shapes[g.index()].call
            })
        };
        let Some(&entry) = body.layout.first().filter(|&&b| leaf(b)) else {
            return false;
        };
        let mut seen = vec![false; body.blocks().len()];
        let mut work = vec![entry];
        seen[entry.index()] = true;
        while let Some(b) = work.pop() {
            if body
                .block(b)
                .instructions
                .last()
                .is_some_and(|&id| matches!(body.instruction(id).kind, I::Return(..)))
            {
                return true;
            }
            for &s in cfg.successors(b) {
                if !seen[s.index()] && cfg.reachable(s) && leaf(s) {
                    seen[s.index()] = true;
                    work.push(s);
                }
            }
        }
        false
    }
    fn partial(
        &self,
        g: FunctionIndex,
        body: &Body,
        call_id: InstId,
        site: Site,
        caller: &Caller,
        source: Option<&Body>,
    ) -> Result<Option<Cut>, Error> {
        let target = self.graph.symbol(g);
        let shape = self.shapes[g.index()];
        let metadata = self
            .program
            .callee_metadata(Value::Symbol(target))
            .ok_or(Error::Limit)?;
        if self.options.level < 2
            || metadata.flags & (32 | 64) != 0
            || shape.unusable
            || shape.eh
            || site.handler && shape.unwinds
            || body
                .instruction(call_id)
                .result
                .is_some_and(|t| body.temp(t).ty.is_none_or(|ty| !inline_shape::phi_type(ty)))
            || !args::partial_parameters(self.program, target)
        {
            return Ok(None);
        }
        let cycle = self.graph.recursive(g) && g != caller.index;
        let hint = metadata.flags & (8 | 16) != 0;
        let cap = if cycle {
            8
        } else {
            (if self.graph.calls(g) <= 2 {
                48
            } else if hint {
                40
            } else {
                24
            }) * if site.looping { 2 } else { 1 }
        };
        let source = if let Some(source) = source {
            source
        } else if g == caller.index {
            body
        } else {
            self.program
                .function(self.program.symbol(target))
                .and_then(|f| f.body.as_ref())
                .ok_or(Error::Limit)?
        };
        let Some(cut) = Cut::find(source, cap)? else {
            return Ok(None);
        };
        Ok((cut.size <= cap
            && (!shape.allocates || cut.size <= 8)
            && u64::from(cut.size) <= caller.budget)
            .then_some(cut))
    }
    fn process(&mut self, symbol: SymbolId) -> Result<bool, Error> {
        let index = self.graph.position(symbol).ok_or(Error::Limit)?;
        let shape = self.shapes[index.index()];
        let metadata = *self
            .program
            .callee_metadata(Value::Symbol(symbol))
            .ok_or(Error::Limit)?;
        let mut caller = Caller::new(index, shape, &self.graph, &metadata, self.options);
        let mut body = self.program.take_body(symbol)?.ok_or(Error::Limit)?;
        let result = self.process_body(symbol, &mut body, &mut caller);
        self.program.restore_body(symbol, body)?;
        result
    }
    fn process_body(
        &mut self,
        symbol: SymbolId,
        body: &mut Body,
        caller: &mut Caller,
    ) -> Result<bool, Error> {
        let shape = self.shapes[caller.index.index()];
        let mut cfg = Cfg::light(body)?;
        let handlers = body.layout.iter().any(|&b| cfg.handler(b));
        if handlers || shape.looping {
            cfg.complete()?;
        }
        caller.leaf_wrapper = !caller.wrapper
            && self.options.level >= 2
            && shape.hot <= 48
            && self
                .program
                .callee_metadata(Value::Symbol(symbol))
                .is_some_and(discardable)
            && self.leaf_path(body, &cfg);
        let mut sites = vec![Site::default(); body.blocks().len()];
        if handlers {
            for &b in cfg.reverse_postorder() {
                sites[b.index()].handler = cfg.handler(b)
                    || cfg
                        .dominator(b)
                        .filter(|&p| p != b)
                        .is_some_and(|p| sites[p.index()].handler);
            }
        }
        if self.options.level >= 2 {
            for (s, cold) in
                sites
                    .iter_mut()
                    .zip(inline_shape::cold(self.program, body, &cfg, true))
            {
                s.cold = cold;
            }
        }
        if shape.looping {
            let mut forest = Forest::default();
            forest.find(body.blocks().len(), &cfg)?;
            for &b in cfg.reverse_postorder() {
                sites[b.index()].looping = forest.innermost(b).is_some();
            }
        }
        let (original, recursive_depth) = self.recursive_source(symbol, body, caller.index)?;
        let mut traversal =
            Traversal::new(body, sites, self.shapes.len(), original, recursive_depth);
        while let Some(block) = traversal.work.pop() {
            for id in body.block(block).instructions.clone() {
                let Some(choice) = self.choose(body, id, block, caller, &mut traversal)? else {
                    continue;
                };
                if self.expand(body, id, block, caller, &mut traversal, choice)? {
                    break;
                }
            }
        }
        if traversal.changed {
            settle(body, &traversal.next, &traversal.replacements)?;
        }
        Ok(traversal.changed)
    }
    fn choose(
        &mut self,
        body: &mut Body,
        id: InstId,
        block: BlockId,
        caller: &mut Caller,
        traversal: &mut Traversal,
    ) -> Result<Option<Choice>, Error> {
        if !matches!(body.instruction(id).kind,I::Call(ref c)if matches!(c.callee,Value::Symbol(_)))
        {
            return Ok(None);
        }
        if !traversal.replacements.is_empty() {
            body.rewrite_values(id, |v| resolve(v, &traversal.replacements))?;
        }
        let I::Call(call) = body.instruction(id).kind.clone() else {
            return Err(Error::Limit);
        };
        let Value::Symbol(target) = call.callee else {
            return Ok(None);
        };
        let Some(g) = self.graph.position(target) else {
            return Ok(None);
        };
        if !args::compatible(self.program, body, &call, target) {
            return Ok(None);
        }
        let site = traversal.sites[block.index()];
        let dispatch =
            self.shapes[g.index()].loop_dispatch && args::known_target(self.program, body, &call);
        let forced = self
            .program
            .callee_metadata(call.callee)
            .is_some_and(|m| m.flags & 8 != 0);
        let cold = site.cold && self.shapes[g.index()].size > 4 && !forced;
        let policy = Policy {
            program: self.program,
            graph: &self.graph,
            shapes: &self.shapes,
            input: &self.input,
            original: self.original,
            caller,
            budgets: self.budgets,
            options: self.options,
            wave: self.wave,
        };
        let admission = if cold {
            None
        } else {
            policy.admit(
                g,
                site.handler,
                site.looping,
                dispatch,
                &mut traversal.grown,
            )
        };
        let mut choice = Choice {
            target: g,
            forced,
            admission,
            full: false,
            partial: None,
            folded: None,
        };
        if choice.admission.is_none() {
            self.alternative(body, id, block, caller, traversal, &mut choice)?;
        }
        Ok(choice.admission.map(|_| choice))
    }
    fn alternative(
        &mut self,
        body: &mut Body,
        id: InstId,
        block: BlockId,
        caller: &mut Caller,
        traversal: &mut Traversal,
        choice: &mut Choice,
    ) -> Result<(), Error> {
        let g = choice.target;
        let site = traversal.sites[block.index()];
        let original_count = traversal
            .original
            .as_ref()
            .map_or(0, |_| self.shapes[caller.index.index()].size);
        if self.wave == Wave::Late
            && g == caller.index
            && traversal.original.is_some()
            && site.depth < traversal.recursive_depth
            && original_count <= traversal.recursive_budget
            && u64::from(original_count) <= caller.budget
            && !(site.handler && self.shapes[g.index()].unwinds)
        {
            choice.full = true;
            choice.admission = Some(Admission {
                charge: original_count,
                once: false,
            });
        } else if self.wave == Wave::Late {
            if g == caller.index && traversal.original.is_none() && traversal.changed {
                settle(body, &traversal.next, &traversal.replacements)?;
            }
            let source = if g == caller.index {
                traversal.original.as_ref()
            } else {
                None
            };
            choice.partial = self.partial(g, body, id, site, caller, source)?;
            choice.admission = choice.partial.as_ref().map(|p| Admission {
                charge: p.size,
                once: false,
            });
        }
        if choice.admission.is_none() && self.wave == Wave::Ordinary {
            choice.folded = self.folded(body, id, g, site.handler, caller)?;
            if choice.folded.is_some() {
                choice.admission = Some(Admission {
                    charge: 0,
                    once: false,
                });
            }
        }
        Ok(())
    }
    fn expand(
        &mut self,
        body: &mut Body,
        id: InstId,
        block: BlockId,
        caller: &mut Caller,
        traversal: &mut Traversal,
        choice: Choice,
    ) -> Result<bool, Error> {
        let Choice {
            target: g,
            forced,
            admission,
            full,
            partial,
            folded,
        } = choice;
        let admission = admission.ok_or(Error::Limit)?;
        let target = self.graph.symbol(g);
        let site = traversal.sites[block.index()];
        let snapshot = if partial.is_some() && g == caller.index && traversal.original.is_none() {
            Some(body.inline_snapshot())
        } else {
            None
        };
        let source = if g == caller.index && traversal.original.is_some() {
            traversal.original.as_ref().ok_or(Error::Limit)?
        } else if let Some(snapshot) = &snapshot {
            snapshot
        } else {
            self.program
                .function(self.program.symbol(folded.unwrap_or(target)))
                .and_then(|f| f.body.as_ref())
                .ok_or(Error::Limit)?
        };
        let cost = if folded.is_some() {
            instruction_count(source)?
        } else if let Some(partial) = &partial {
            partial.size
        } else {
            self.shapes[g.index()].size
        };
        let unit = if folded.is_none() && forced {
            &mut self.budgets.force
        } else {
            &mut self.budgets.growth
        };
        if !admission.once && u64::from(cost) > *unit {
            if let Some(copy) = folded {
                self.program.discard_function_definition(copy)?;
            }
            return Ok(false);
        }
        if !admission.once {
            *unit -= u64::from(cost);
        }
        let before = body.blocks().len();
        Expansion {
            body,
            next: &mut traversal.next,
            replacements: &mut traversal.replacements,
        }
        .copy(block, id, source, partial.as_ref())?;
        if let Some(copy) = folded {
            self.program.discard_function_definition(copy)?;
        }
        traversal.sites.resize(
            body.blocks().len(),
            Site {
                depth: site.depth.wrapping_add(1),
                ..site
            },
        );
        traversal.sites[before].depth = site.depth;
        if admission.once {
            caller.once -= u64::from(admission.charge);
            self.budgets.once -= u64::from(admission.charge);
        } else {
            caller.budget = caller.budget.saturating_sub(u64::from(admission.charge));
        }
        if full {
            traversal.recursive_budget -= self.shapes[caller.index.index()].size;
        } else if partial.is_none() {
            self.graph.consume_call(g);
        }
        self.stats.calls = self.stats.calls.checked_add(1).ok_or(Error::Limit)?;
        self.stats.instructions = self
            .stats
            .instructions
            .checked_add(u64::from(cost))
            .ok_or(Error::Limit)?;
        traversal.changed = true;
        traversal
            .work
            .push(body.block_id(before).ok_or(Error::Limit)?);
        if full {
            for k in (before + 1..body.blocks().len()).rev() {
                traversal.work.push(body.block_id(k).ok_or(Error::Limit)?);
            }
        }
        Ok(true)
    }
    fn recursive_source(
        &self,
        symbol: SymbolId,
        body: &Body,
        index: FunctionIndex,
    ) -> Result<(Option<Body>, u8), Error> {
        let shape = self.shapes[index.index()];
        if self.wave != Wave::Late
            || self.options.level < 2
            || !self.graph.recursive(index)
            || shape.eh
            || shape.unusable
            || shape.looping
            || shape.size > 64
        {
            return Ok((None, 0));
        }
        let calls=body.layout.iter().flat_map(|&b|body.block(b).instructions.iter()).filter(|&&id|matches!(&body.instruction(id).kind,I::Call(c)if c.callee==Value::Symbol(symbol))).count();
        let calls = u64::try_from(calls).map_err(|_| Error::Limit)?;
        let mut copies = 0;
        let mut level = 1;
        let mut depth = 0;
        if calls >= 2 {
            for d in 1..=2 {
                level *= calls;
                copies += level;
                if copies * u64::from(shape.size) > 320 {
                    break;
                }
                depth = d;
            }
        }
        Ok(if depth == 0 {
            (None, 0)
        } else {
            (Some(body.inline_snapshot()), depth)
        })
    }
    pub(super) fn run(&mut self) -> Result<(), Error> {
        for index in self.graph.bottom_up.clone() {
            let symbol = self.graph.symbol(index);
            let f = self
                .program
                .function(self.program.symbol(symbol))
                .ok_or(Error::Limit)?;
            let thunk = f.metadata.object.is_some_and(|n| {
                let name = self.program.names.bytes(n);
                name.starts_with(b"_ZTh") || name.starts_with(b"_ZTv")
            });
            if thunk
                && (!self.options.inline_thunk_targets
                    || self.program.signature(f.signature).boundary.arity == Arity::Variadic)
            {
                continue;
            }
            let changed = self.process(symbol)?;
            if changed || self.wave == Wave::Ordinary {
                (self.simplify)(self.program, symbol, self.nounwind)?;
                self.shapes[index.index()] = inline_shape::of(self.program, symbol, self.nounwind)?;
            }
        }
        Ok(())
    }
}
struct Choice {
    target: FunctionIndex,
    forced: bool,
    admission: Option<Admission>,
    full: bool,
    partial: Option<Cut>,
    folded: Option<SymbolId>,
}
struct Traversal {
    sites: Vec<Site>,
    next: Vec<Option<BlockId>>,
    replacements: Vec<Option<Value>>,
    work: Vec<BlockId>,
    changed: bool,
    grown: Vec<u32>,
    original: Option<Body>,
    recursive_depth: u8,
    recursive_budget: u32,
}
impl Traversal {
    fn new(
        body: &Body,
        sites: Vec<Site>,
        functions: usize,
        original: Option<Body>,
        recursive_depth: u8,
    ) -> Self {
        let mut next = vec![None; body.blocks().len()];
        for pair in body.layout.windows(2) {
            next[pair[0].index()] = Some(pair[1]);
        }
        let recursive_budget = if original.is_some() { 320 } else { 0 };
        Self {
            sites,
            next,
            replacements: Vec::new(),
            work: body.layout.iter().rev().copied().collect(),
            changed: false,
            grown: vec![0; functions],
            original,
            recursive_depth,
            recursive_budget,
        }
    }
}
fn substitute_constants(
    copied: &mut Body,
    source: &Body,
    constants: Vec<(usize, Value)>,
) -> Result<(), Error> {
    for (index, value) in constants {
        let value = if let Value::Integer(id) = value {
            copied.integer(source.literal(id))?
        } else {
            value
        };
        for block in copied.layout.clone() {
            for id in copied.block(block).instructions.clone() {
                let call = matches!(copied.instruction(id).kind, I::Call(_));
                let mut first = true;
                copied.rewrite_values(id, |v| {
                    let callee_slot = call && std::mem::take(&mut first);
                    Ok(
                        if matches!(v, Value::Temp(t) if t.index() == index)
                            && !(callee_slot
                                && matches!(value, Value::Integer(_) | Value::Float(..)))
                        {
                            value
                        } else {
                            v
                        },
                    )
                })?;
            }
        }
    }
    Ok(())
}
#[derive(Default, Clone, Copy)]
struct Site {
    handler: bool,
    cold: bool,
    looping: bool,
    depth: u8,
}
pub(super) fn instruction_count(body: &Body) -> Result<u32, Error> {
    body.layout.iter().try_fold(0_u32, |n, &b| {
        n.checked_add(u32::try_from(body.block(b).instructions.len()).map_err(|_| Error::Limit)?)
            .ok_or(Error::Limit)
    })
}
fn resolve(mut value: Value, replacements: &[Option<Value>]) -> Result<Value, Error> {
    for _ in 0..=replacements.len() {
        let Value::Temp(t) = value else {
            return Ok(value);
        };
        let Some(Some(next)) = replacements.get(t.index()) else {
            return Ok(value);
        };
        value = *next;
    }
    Err(Error::Construction(
        "cyclic inline result replacements".into(),
    ))
}
fn settle(
    body: &mut Body,
    next: &[Option<BlockId>],
    replacements: &[Option<Value>],
) -> Result<(), Error> {
    let mut layout = Vec::with_capacity(body.blocks().len());
    let mut cursor = body.layout.first().copied();
    while let Some(b) = cursor {
        if layout.len() >= body.blocks().len() {
            return Err(Error::Limit);
        }
        layout.push(b);
        cursor = next[b.index()];
    }
    body.layout = layout;
    if !replacements.is_empty() {
        for b in body.layout.clone() {
            for id in body.block(b).instructions.clone() {
                body.rewrite_values(id, |v| resolve(v, replacements))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moved_phi_preserves_the_calling_edge() {
        let source = b"function @g(%c:i64)->i64 {block ^e:branch %c,^a,^b block ^a:return i64 11 block ^b:return i64 22}\nfunction @f(%p:i64,%c:i64)->i64 {block ^e:branch %p,^call,^other block ^call:%v=call i64 @g(%c) jump ^join block ^other:jump ^join block ^join:%r=phi i64 [^call:%v,^other:33] return i64 %r}";
        let mut program = crate::lowir::test_program(source).unwrap();
        crate::opt::optimize(&mut program, 1).unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        let output = view(&program);
        assert!(output.contains("%v = phi i64"));
        assert!(output.contains("%r = phi i64"));
        assert!(!output.contains("return i64 33"));
    }

    fn wave(source: &str, kind: Wave, level: u8) -> (Program, Stats) {
        let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
        let mut original = vec![0; program.symbols().len()];
        let mut input = 0;
        for (index, symbol) in program.symbols().iter().enumerate() {
            if let Some(body) = program.function(symbol).and_then(|f| f.body.as_ref()) {
                let count = instruction_count(body).unwrap();
                original[index] = count;
                input += u64::from(count);
            }
        }
        let mut budgets = Budgets::new(input);
        let mut nounwind = vec![false; original.len()];
        let mut stats = Stats::default();
        let mut simplify = |_: &mut Program, _, _: &mut Vec<bool>| Ok(());
        Runner::new(
            &mut program,
            Options::new(level),
            kind,
            Shared {
                budgets: &mut budgets,
                original: &original,
                nounwind: &mut nounwind,
                stats: &mut stats,
            },
            &mut simplify,
        )
        .unwrap()
        .run()
        .unwrap();
        for symbol in program.publications().to_vec() {
            if let Some(mut body) = program.take_body(symbol).unwrap() {
                body.compact(&mut program.names).unwrap();
                program.restore_body(symbol, body).unwrap();
            }
        }
        crate::lowir::validate_test_program(&program).unwrap();
        (program, stats)
    }
    fn view(program: &Program) -> String {
        let mut text = Vec::new();
        crate::lowir::writer::write(program, &mut text).unwrap();
        String::from_utf8(text).unwrap()
    }
    #[test]
    fn copying_retypes_arguments_returns_and_continuation_phi_edges() {
        let (program, stats) = wave(
            r"
function @g(%p : i64) -> i64 [binding=internal] { block ^entry: return i64 %p }
function @f(%p : ptr) -> i64 {
 block ^entry: %r = call i64 @g(%p) jump ^exit
 block ^exit: %v = phi i64 [^entry: %r] return i64 %v
}",
            Wave::Ordinary,
            1,
        );
        assert_eq!(stats.calls, 1);
        let text = view(&program);
        assert!(text.contains("copy i64 %p"));
        assert!(text.contains("phi i64 [^inline_cont_2: %r]"));
    }
    #[test]
    fn handler_sites_keep_unwinding_callees_but_copy_nonthrowing_ones() {
        for (boundary, copies) in [("", 0), (" [unwind=no]", 1)] {
            let source = format!(
                r"
declare function @external() -> void
function @g() -> void{boundary} {{ block ^entry: call void @external() return void }}
function @f() -> void {{
 block ^entry: eh_try ^handler call void @external() eh_end return void
 block ^handler: eh_catch_all call void @g() return void
}}"
            );
            let (_, stats) = wave(&source, Wave::Ordinary, 1);
            assert_eq!(stats.calls, copies);
        }
    }
    #[test]
    fn partial_self_copy_stops_at_the_slow_call_and_keeps_the_fast_return() {
        let (program, stats) = wave(
            r"
function @f(%p : i64) -> i64 {
 block ^entry: branch %p, ^rec, ^done
 block ^done: return i64 0
 block ^rec: %n = binary sub i64 %p, 1 %a = call i64 @f(%n) return i64 %a
}",
            Wave::Late,
            2,
        );
        assert_eq!(stats.calls, 1);
        let text = view(&program);
        assert_eq!(text.matches("call i64 @f").count(), 1);
        assert!(text.contains("^inline_entry_"));
        assert!(text.contains("phi i64"));
    }
    #[test]
    fn nested_copies_keep_generated_kinds_and_drop_copied_source_block_names() {
        let (program, stats) = wave(
            r"
function @g(%p : i64) -> i64 { block ^entry: branch %p, ^yes, ^no block ^yes: return i64 7 block ^no: return i64 9 }
function @h(%p : i64) -> i64 { block ^entry: %x = call i64 @g(%p) return i64 %x }
function @f(%p : i64) -> i64 { block ^entry: %x = call i64 @h(%p) return i64 %x }
",
            Wave::Ordinary,
            1,
        );
        assert_eq!(stats.calls, 2);
        let text = view(&program);
        assert!(text.contains("block ^yes_"));
        assert!(text.contains("block ^inline_copy_"));
    }
    #[test]
    fn pruning_hides_discarded_definitions_in_both_writer_paths() {
        let (mut program, stats) = wave(
            r"
function @g() -> i64 [binding=internal] { block ^entry: return i64 3 }
function @f() -> i64 { block ^entry: %r = call i64 @g() return i64 %r }
",
            Wave::Ordinary,
            1,
        );
        assert_eq!(stats.calls, 1);
        assert_eq!(
            super::super::unit_graph::prune_unreachable(&mut program).unwrap(),
            1
        );
        assert!(!view(&program).contains("@g"));
        assert_eq!(program.function_declarations().count(), 0);
    }
}

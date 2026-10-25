//! Typed call-site graph and iterative SCC order from `opt/call_graph.cpp`. See NOTICE.
use crate::lowir::{
    Error,
    model::{Body, DataItem, GlobalInit, InstructionKind as I, Metadata, Program, SymbolId, Value},
    vocabulary::Binding,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FunctionIndex(u32);
impl FunctionIndex {
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}
pub(super) struct CallGraph {
    pub(super) functions: Vec<SymbolId>,
    index_of: Vec<Option<FunctionIndex>>,
    callee_first: Vec<usize>,
    callees: Vec<FunctionIndex>,
    calls_in: Vec<u32>,
    callers_in: Vec<u32>,
    observed: Vec<bool>,
    recursive: Vec<bool>,
    component: Vec<u32>,
    pub(super) bottom_up: Vec<FunctionIndex>,
}
impl CallGraph {
    pub(super) fn reverse_calls(&self) -> Vec<Vec<FunctionIndex>> {
        let mut callers = vec![Vec::new(); self.functions.len()];
        for g in 0..self.functions.len() {
            let index = self.index_of[self.functions[g].index()];
            if let Some(index) = index {
                for &f in &self.callees[self.callee_first[g]..self.callee_first[g + 1]] {
                    callers[f.index()].push(index);
                }
            }
        }
        callers
    }
    pub(super) fn build(program: &Program) -> Result<Self, Error> {
        let mut functions = Vec::new();
        let mut index_of = vec![None; program.symbols().len()];
        for &symbol in program.publications() {
            if is_definition(program, symbol) && index_of[symbol.index()].is_none() {
                let index =
                    FunctionIndex(u32::try_from(functions.len()).map_err(|_| Error::Limit)?);
                index_of[symbol.index()] = Some(index);
                functions.push(symbol);
            }
        }
        let count = functions.len();
        let mut out = Self {
            functions,
            index_of,
            callee_first: vec![0; count + 1],
            callees: Vec::new(),
            calls_in: vec![0; count],
            callers_in: vec![0; count],
            observed: vec![false; count],
            recursive: vec![false; count],
            component: vec![0; count],
            bottom_up: Vec::with_capacity(count),
        };
        out.call_sites(program)?;
        out.components()?;
        Ok(out)
    }
    pub(super) fn symbol(&self, index: FunctionIndex) -> SymbolId {
        self.functions[index.index()]
    }
    pub(super) fn position(&self, symbol: SymbolId) -> Option<FunctionIndex> {
        self.index_of.get(symbol.index()).copied().flatten()
    }
    pub(super) fn calls(&self, f: FunctionIndex) -> u32 {
        self.calls_in[f.index()]
    }
    pub(super) fn callers(&self, f: FunctionIndex) -> u32 {
        self.callers_in[f.index()]
    }
    pub(super) fn observed(&self, f: FunctionIndex) -> bool {
        self.observed[f.index()]
    }
    pub(super) fn recursive(&self, f: FunctionIndex) -> bool {
        self.recursive[f.index()]
    }
    pub(super) fn consume_call(&mut self, f: FunctionIndex) {
        self.calls_in[f.index()] = self.calls_in[f.index()].wrapping_sub(1);
    }
    fn observe(&mut self, symbol: SymbolId) {
        if let Some(index) = self.index_of[symbol.index()] {
            self.observed[index.index()] = true;
        }
    }
    fn record_values(&mut self, body: &Body, instruction: &I) -> Result<(), Error> {
        let mut first = true;
        let mut result = Ok(());
        body.visit_values(instruction, |value| {
            let callee = first && matches!(instruction, I::Call(_));
            first = false;
            if result.is_ok() {
                result = self.record_symbol(value, callee);
            }
        });
        result
    }
    fn record_symbol(&mut self, value: Value, callee: bool) -> Result<(), Error> {
        let Value::Symbol(symbol) = value else {
            return Ok(());
        };
        let Some(target) = self.index_of[symbol.index()].filter(|_| callee) else {
            self.observe(symbol);
            return Ok(());
        };
        self.callees.push(target);
        self.calls_in[target.index()] = self.calls_in[target.index()]
            .checked_add(1)
            .ok_or(Error::Limit)?;
        Ok(())
    }
    fn call_sites(&mut self, program: &Program) -> Result<(), Error> {
        for index in 0..self.functions.len() {
            self.callee_first[index] = self.callees.len();
            let function = program
                .function(program.symbol(self.functions[index]))
                .ok_or(Error::Limit)?;
            self.observed[index] |=
                function.metadata.role.is_some() || function.metadata.flags & (4 | 64) != 0;
            let body = function.body.as_ref().ok_or(Error::Limit)?;
            for &block in &body.layout {
                for &id in &body.block(block).instructions {
                    self.record_values(body, &body.instruction(id).kind)?;
                }
            }
        }
        self.callee_first[self.functions.len()] = self.callees.len();
        let mut last = vec![None; self.functions.len()];
        for caller in 0..self.functions.len() {
            for &callee in &self.callees[self.callee_first[caller]..self.callee_first[caller + 1]] {
                if last[callee.index()] != Some(caller) {
                    last[callee.index()] = Some(caller);
                    self.callers_in[callee.index()] = self.callers_in[callee.index()]
                        .checked_add(1)
                        .ok_or(Error::Limit)?;
                }
            }
        }
        // Aliases alone do not observe their target. A jump thunk's body has
        // the ordinary forwarding call; no second, hidden graph edge is needed.
        data_references(program, |symbol| self.observe(symbol));
        Ok(())
    }
    fn components(&mut self) -> Result<(), Error> {
        let count = self.functions.len();
        let mut number = vec![None; count];
        let mut low = vec![0; count];
        let mut stack = Vec::new();
        let mut on_stack = vec![false; count];
        let mut frames = Vec::new();
        let mut counter = 0_u32;
        let mut component = 0_u32;
        for root in 0..count {
            if number[root].is_some() {
                continue;
            }
            frames.push((root, self.callee_first[root]));
            number[root] = Some(counter);
            low[root] = counter;
            counter = counter.checked_add(1).ok_or(Error::Limit)?;
            stack.push(root);
            on_stack[root] = true;
            while let Some(&(function, next)) = frames.last() {
                if next < self.callee_first[function + 1] {
                    let callee = self.callees[next].index();
                    frames.last_mut().ok_or(Error::Limit)?.1 += 1;
                    self.recursive[function] |= callee == function;
                    match number[callee] {
                        Some(ordinal) if on_stack[callee] => {
                            low[function] = low[function].min(ordinal);
                        }
                        Some(_) => (),
                        None => {
                            number[callee] = Some(counter);
                            low[callee] = counter;
                            counter = counter.checked_add(1).ok_or(Error::Limit)?;
                            stack.push(callee);
                            on_stack[callee] = true;
                            frames.push((callee, self.callee_first[callee]));
                        }
                    }
                    continue;
                }
                frames.pop();
                if let Some(&(parent, _)) = frames.last() {
                    low[parent] = low[parent].min(low[function]);
                }
                if Some(low[function]) != number[function] {
                    continue;
                }
                let first = stack
                    .iter()
                    .rposition(|&member| member == function)
                    .ok_or(Error::Limit)?;
                let cycle = stack.len() - first > 1;
                for &member in &stack[first..] {
                    on_stack[member] = false;
                    self.component[member] = component;
                    self.recursive[member] |= cycle;
                }
                // Definition order breaks ties within a component, regardless
                // of the traversal order that discovered its members.
                stack[first..].sort_unstable();
                for member in stack.drain(first..) {
                    self.bottom_up.push(FunctionIndex(
                        u32::try_from(member).map_err(|_| Error::Limit)?,
                    ));
                }
                component = component.checked_add(1).ok_or(Error::Limit)?;
            }
        }
        Ok(())
    }
}

fn is_definition(program: &Program, symbol: SymbolId) -> bool {
    program
        .function(program.symbol(symbol))
        .is_some_and(|function| function.body.is_some())
}
fn data_references(program: &Program, mut mark: impl FnMut(SymbolId)) {
    for symbol in program.symbols() {
        if let Some(global) = program.global(symbol) {
            match &global.init {
                GlobalInit::Address(symbol, _) => mark(*symbol),
                GlobalInit::Data(items) => {
                    for item in items {
                        if let DataItem::Address(_, symbol, _) = item {
                            mark(*symbol);
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
pub(super) fn discardable(metadata: &Metadata) -> bool {
    matches!(metadata.binding, Some(Binding::Weak | Binding::Internal))
        && metadata.role.is_none()
        && metadata.flags & (4 | 64) == 0
        && metadata.tls_for.is_none()
}
fn mark_definition(
    program: &Program,
    live: &mut [bool],
    work: &mut Vec<SymbolId>,
    symbol: SymbolId,
) {
    if live[symbol.index()] {
        return;
    }
    live[symbol.index()] = true;
    if is_definition(program, symbol) {
        work.push(symbol);
    }
}
pub(super) fn prune_unreachable(program: &mut Program) -> Result<u32, Error> {
    let mut live = vec![false; program.symbols().len()];
    let mut work = Vec::new();
    for &symbol in program.publications() {
        if is_definition(program, symbol)
            && !discardable(
                program
                    .symbol_metadata(program.symbol(symbol))
                    .ok_or(Error::Limit)?,
            )
        {
            mark_definition(program, &mut live, &mut work, symbol);
        }
    }
    data_references(program, |symbol| {
        mark_definition(program, &mut live, &mut work, symbol);
    });
    while let Some(symbol) = work.pop() {
        let body = program
            .function(program.symbol(symbol))
            .and_then(|function| function.body.as_ref())
            .ok_or(Error::Limit)?;
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                body.visit_values(&body.instruction(id).kind, |value| {
                    if let Value::Symbol(symbol) = value {
                        mark_definition(program, &mut live, &mut work, symbol);
                    }
                });
            }
        }
    }
    let mut order = Vec::with_capacity(program.publications().len());
    let mut removed = 0_u32;
    for index in 0..program.publications().len() {
        let symbol = program.publications()[index];
        if is_definition(program, symbol) && !live[symbol.index()] {
            program.discard_function_definition(symbol)?;
            removed = removed.checked_add(1).ok_or(Error::Limit)?;
        } else {
            order.push(symbol);
        }
    }
    program.replace_publications(order);
    if removed != 0 {
        let dropped = program
            .symbols()
            .iter()
            .enumerate()
            .map(|(index, symbol)| {
                !live[index]
                    && program.function(symbol).is_some_and(|function| {
                        function.body.is_none() && discardable(&function.metadata)
                    })
            })
            .collect::<Vec<_>>();
        program.drop_aliases_of(&dropped);
    }
    // Rust recognizes jump thunks from the surviving bodies; there is no
    // program-owned jump-thunk table retaining facts for discarded definitions.
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    fn index(program: &Program, graph: &CallGraph, name: &str) -> usize {
        graph
            .functions
            .iter()
            .position(|&symbol| program.names.get(program.symbol(symbol).name) == name)
            .unwrap()
    }
    #[test]
    fn call_sites_observations_and_distinct_callers_are_separate() {
        let mut program = crate::lowir::test_program(
            br"
global @table : obj<16x8> = { ptr addr @data ptr addr @callee }
function @root(%pointer : ptr) -> void {
 block ^entry:
  call void @callee()
  call void @callee()
  call void %pointer(@argument) as (%p : ptr) -> void
  return void
 block ^dead:
  call void @callee()
  return void
}
function @other() -> void { block ^entry: call void @callee() return void }
function @callee() -> void [binding=internal] { block ^entry: return void }
function @argument() -> void [binding=internal] { block ^entry: return void }
function @data() -> void [binding=internal] { block ^entry: return void }
function @aliased() -> void [binding=internal] { block ^entry: return void }
function @role() -> void [binding=internal, role=init] { block ^entry: return void }
function @rooted() -> void [binding=internal, object_root=yes] { block ^entry: return void }
function @replaceable() -> void [binding=weak, replaceable=yes] { block ^entry: return void }
alias object alias_only = @aliased
",
        )
        .unwrap();
        // Retained lowering can have more than one publication of a symbol;
        // its graph definition still appears once, at its first position.
        let mut publications = program.publications().to_vec();
        publications.extend_from_slice(program.publications());
        program.replace_publications(publications);
        let graph = CallGraph::build(&program).unwrap();
        assert_eq!(graph.functions.len(), 9);
        let callee = index(&program, &graph, "callee");
        assert_eq!(graph.calls_in[callee], 4);
        assert_eq!(graph.callers_in[callee], 2);
        assert_eq!(graph.callees.len(), 4);
        for name in [
            "callee",
            "argument",
            "data",
            "role",
            "rooted",
            "replaceable",
        ] {
            assert!(graph.observed[index(&program, &graph, name)], "{name}");
        }
        assert!(!graph.observed[index(&program, &graph, "aliased")]);
        assert!(!graph.observed[index(&program, &graph, "root")]);
    }
    #[test]
    fn sccs_finish_callees_first_with_definition_order_inside_cycles() {
        let program = crate::lowir::test_program(
            br"
function @a() -> void { block ^entry: call void @c() return void }
function @b() -> void { block ^entry: return void }
function @c() -> void { block ^entry: call void @d() return void }
function @d() -> void { block ^entry: call void @c() return void }
function @e() -> void { block ^entry: call void @e() return void }
function @f() -> void { block ^entry: call void @a() return void }
",
        )
        .unwrap();
        let graph = CallGraph::build(&program).unwrap();
        let order = graph
            .bottom_up
            .iter()
            .map(|&index| program.names.get(program.symbol(graph.symbol(index)).name))
            .collect::<Vec<_>>();
        assert_eq!(order, ["c", "d", "a", "b", "e", "f"]);
        assert_eq!(graph.recursive, [false, false, true, true, true, false]);
        assert_eq!(graph.component, [1, 2, 0, 0, 3, 4]);
    }
    #[test]
    fn pruning_keeps_roots_data_addresses_and_tls_but_not_unrooted_cycles_or_aliases() {
        let mut program = crate::lowir::test_program(
            br"
declare function @unused_external() -> void [binding=weak]
global @tls : i64 [storage=thread_local] = 0
global @data : ptr [binding=internal] = addr @from_data
function @root() -> void { block ^entry: call void @live() %p = addr @addressed return void }
function @live() -> void [binding=internal] { block ^entry: return void }
function @addressed() -> void [binding=internal] { block ^entry: return void }
function @from_data() -> void [binding=internal] { block ^entry: return void }
function @wrapper() -> ptr [binding=weak, tls_for=@tls] { block ^entry: return ptr @tls }
function @role() -> void [binding=internal, role=init] { block ^entry: return void }
function @rooted() -> void [binding=internal, object_root=yes] { block ^entry: return void }
function @replaceable() -> void [binding=weak, replaceable=yes] { block ^entry: return void }
function @dead() -> void [binding=internal] { block ^entry: call void @dead2() return void }
function @dead2() -> void [binding=weak] { block ^entry: call void @dead() return void }
alias object dead_alias = @dead
alias object live_alias = @live
alias object external_alias = @unused_external
",
        )
        .unwrap();
        let before = program.publications().to_vec();
        let dead = before
            .iter()
            .copied()
            .filter(|&symbol| {
                matches!(
                    program.names.get(program.symbol(symbol).name),
                    "dead" | "dead2"
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(prune_unreachable(&mut program).unwrap(), 2);
        crate::lowir::validate_test_program(&program).unwrap();
        let after = program.publications();
        assert_eq!(
            after,
            before
                .into_iter()
                .filter(|symbol| !dead.contains(symbol))
                .collect::<Vec<_>>()
        );
        for symbol in dead {
            assert!(!is_definition(&program, symbol));
        }
        assert_eq!(program.aliases().len(), 1);
        assert_eq!(program.names.get(program.aliases()[0].object), "live_alias");
        let live_alias = program.names.intern("live_alias").unwrap();
        let dead_alias = program.names.intern("dead_alias").unwrap();
        assert!(program.has_alias(live_alias));
        assert!(!program.has_alias(dead_alias));
        assert_eq!(prune_unreachable(&mut program).unwrap(), 0);
    }
    #[test]
    fn no_removed_definition_leaves_declaration_aliases_untouched() {
        let mut program = crate::lowir::test_program(
            b"declare function @external() -> void [binding=weak]\nalias object external_alias = @external\n",
        )
        .unwrap();
        assert_eq!(prune_unreachable(&mut program).unwrap(), 0);
        assert_eq!(program.aliases().len(), 1);
        let graph = CallGraph::build(&program).unwrap();
        assert!(graph.functions.is_empty() && graph.bottom_up.is_empty());
    }
    #[test]
    fn deep_call_chain_uses_an_explicit_traversal_stack() {
        let count = 4096;
        let mut text = String::new();
        for index in 0..count {
            write!(text, "function @f{index}() -> void {{block ^entry: ").unwrap();
            if index + 1 < count {
                write!(text, "call void @f{}() ", index + 1).unwrap();
            }
            text.push_str("return void}\n");
        }
        let program = crate::lowir::test_program(text.as_bytes()).unwrap();
        let graph = CallGraph::build(&program).unwrap();
        assert_eq!(graph.bottom_up.len(), count);
        assert_eq!(graph.bottom_up[0].index(), count - 1);
        assert_eq!(graph.bottom_up[count - 1].index(), 0);
        assert!(!graph.recursive.iter().any(|&value| value));
    }
}

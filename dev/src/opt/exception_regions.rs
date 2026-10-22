//! Persistent ordinary-path region stacks from `opt/eh_regions.cpp`. See NOTICE.
use super::{control_flow::is_eh, pass_context::Context};
use crate::lowir::{
    Error,
    model::{Body, InstId, InstructionKind as I, RuntimeInstruction as R, Unwind, Value},
};
use std::collections::HashMap;

fn can_unwind(context: &Context<'_>, kind: &I) -> bool {
    let I::Call(call) = kind else {
        return matches!(kind, I::Runtime(R::Throw(..) | R::Resume));
    };
    if context
        .program
        .call_boundary(call)
        .is_some_and(|s| s.boundary.unwind == Unwind::No)
    {
        return false;
    }
    !matches!(call.callee,Value::Symbol(symbol) if context.nounwind.get(symbol.index()).copied().unwrap_or(false))
}
pub(super) fn proves_nounwind(context: &Context<'_>) -> bool {
    if context
        .program
        .symbol_metadata(context.program.symbol(context.symbol))
        .is_some_and(|m| m.flags & 64 != 0)
    {
        return false;
    }
    for &block in &context.body.layout {
        for &id in &context.body.block(block).instructions {
            let kind = &context.body.instruction(id).kind;
            if matches!(kind,I::Runtime(runtime) if is_eh(runtime)) || can_unwind(context, kind) {
                return false;
            }
        }
    }
    true
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RegionId(u32);
impl RegionId {
    fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct StackId(u32);
impl StackId {
    fn index(self) -> usize {
        self.0 as usize
    }
}
struct Node {
    region: Option<RegionId>,
    below: StackId,
}
struct Regions {
    nodes: Vec<Node>,
    interned: HashMap<(RegionId, StackId), StackId>,
    marked: Vec<bool>,
    region_of: Vec<Option<RegionId>>,
    openers: Vec<InstId>,
    entry: Vec<Option<StackId>>,
    normal: Vec<bool>,
    unwinds: Vec<bool>,
    popped: Vec<Option<RegionId>>,
}
impl Regions {
    fn new(body: &Body) -> Result<Self, Error> {
        let mut out = Self {
            nodes: vec![Node {
                region: None,
                below: StackId(0),
            }],
            interned: HashMap::new(),
            marked: vec![true],
            region_of: vec![None; body.instructions().len()],
            openers: Vec::new(),
            entry: vec![None; body.blocks().len()],
            normal: vec![false; body.blocks().len()],
            unwinds: Vec::new(),
            popped: vec![None; body.instructions().len()],
        };
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                if matches!(&body.instruction(id).kind,I::Runtime(runtime) if runtime.handler_target().is_some())
                {
                    out.region_of[id.index()] = Some(RegionId(
                        u32::try_from(out.openers.len()).map_err(|_| Error::Limit)?,
                    ));
                    out.openers.push(id);
                }
            }
        }
        out.unwinds = vec![false; out.openers.len()];
        Ok(out)
    }
    fn push(&mut self, below: StackId, region: RegionId) -> Result<StackId, Error> {
        let key = (region, below);
        if let Some(&known) = self.interned.get(&key) {
            return Ok(known);
        }
        let id = StackId(u32::try_from(self.nodes.len()).map_err(|_| Error::Limit)?);
        self.nodes.push(Node {
            region: Some(region),
            below,
        });
        self.marked.push(false);
        self.interned.insert(key, id);
        Ok(id)
    }
    fn enter(
        &mut self,
        context: &Context<'_>,
        stack: StackId,
        id: InstId,
    ) -> Result<Option<StackId>, Error> {
        let kind = &context.body.instruction(id).kind;
        if let Some(region) = self.region_of[id.index()] {
            return self.push(stack, region).map(Some);
        }
        if matches!(kind, I::Runtime(R::EhEnd)) {
            if stack == StackId(0) {
                return Ok(None);
            }
            self.popped[id.index()] = self.nodes[stack.index()].region;
            return Ok(Some(self.nodes[stack.index()].below));
        }
        if can_unwind(context, kind) {
            let mut walk = stack;
            while walk != StackId(0) && !self.marked[walk.index()] {
                let node = &self.nodes[walk.index()];
                self.unwinds[node.region.ok_or(Error::Limit)?.index()] = true;
                self.marked[walk.index()] = true;
                walk = node.below;
            }
        } else if matches!(kind,I::Runtime(runtime) if is_eh(runtime)) {
            return Ok(None);
        }
        Ok(Some(stack))
    }
    fn propagate(&mut self, context: &Context<'_>) -> Result<bool, Error> {
        let Some(&entry) = context.body.layout.first() else {
            return Ok(false);
        };
        let mut work = vec![entry];
        self.entry[entry.index()] = Some(StackId(0));
        self.normal[entry.index()] = true;
        while let Some(block) = work.pop() {
            let mut stack = self.entry[block.index()].ok_or(Error::Limit)?;
            for &id in &context.body.block(block).instructions {
                let Some(next) = self.enter(context, stack, id)? else {
                    return Ok(false);
                };
                stack = next;
            }
            for &target in context.cfg()?.successors(block) {
                if let Some(existing) = self.entry[target.index()] {
                    if existing != stack {
                        return Ok(false);
                    }
                } else {
                    self.entry[target.index()] = Some(stack);
                    self.normal[target.index()] = true;
                    work.push(target);
                }
            }
        }
        Ok(true)
    }
    fn empty_cleanup(body: &Body, id: InstId) -> bool {
        let I::Runtime(R::EhCleanup(Some(target))) = body.instruction(id).kind else {
            return false;
        };
        let [first, second] = body.block(target).instructions.as_slice() else {
            return false;
        };
        matches!(body.instruction(*first).kind, I::Runtime(R::EhEnd))
            && matches!(body.instruction(*second).kind, I::Runtime(R::Resume))
    }
    fn strip(&self, context: &mut Context<'_>) -> Result<bool, Error> {
        let mut strip = vec![false; self.openers.len()];
        let mut any = false;
        for &block in &context.body.layout {
            if !self.normal[block.index()] {
                continue;
            }
            for &id in &context.body.block(block).instructions {
                if let Some(region) = self.region_of[id.index()]
                    && (!self.unwinds[region.index()]
                        || (context.level >= 2 && Self::empty_cleanup(context.body, id)))
                {
                    strip[region.index()] = true;
                    any = true;
                }
            }
        }
        if !any {
            return Ok(false);
        }
        let mut live = vec![true; context.body.instructions().len()];
        for &block in &context.body.layout {
            if !self.normal[block.index()] {
                continue;
            }
            for &id in &context.body.block(block).instructions {
                let opener = self.region_of[id.index()].is_some_and(|region| strip[region.index()]);
                let closer = self.popped[id.index()].is_some_and(|region| strip[region.index()]);
                live[id.index()] = !opener && !closer;
            }
        }
        context.body.retain_instructions(&live)
    }
}
pub(super) fn strip(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut regions = Regions::new(context.body)?;
    if regions.openers.is_empty() {
        return Ok(false);
    }
    context.ensure_cfg()?;
    if !regions.propagate(context)? {
        return Ok(false);
    }
    let changed = regions.strip(context)?;
    if changed {
        context.invalidate_cfg();
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        model::{Program, SymbolId},
        writer,
    };
    fn first(program: &Program) -> SymbolId {
        program
            .publications()
            .iter()
            .copied()
            .find(|&id| {
                program
                    .function(program.symbol(id))
                    .is_some_and(|f| f.body.is_some())
            })
            .unwrap()
    }
    fn text(program: &Program) -> String {
        let mut bytes = Vec::new();
        writer::write(program, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }
    fn run(source: &str, level: u8, known: bool) -> (String, String, bool) {
        let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
        let before = text(&program);
        let mut nounwind = vec![known; program.symbols().len()];
        let id = first(&program);
        nounwind[id.index()] = false;
        let mut body = program.take_body(id).unwrap().unwrap();
        let changed = {
            let mut context = Context::new(&program, id, &mut body, level, &nounwind, &[]);
            strip(&mut context).unwrap()
        };
        program.restore_body(id, body).unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        (before, text(&program), changed)
    }
    const NESTED: &str = r"
declare function @g() -> void
function @f() -> void {
 block ^entry:
  eh_try ^outer
  eh_cleanup ^inner
  call void @g()
  eh_end
  eh_end
  return void
 block ^inner: eh_end resume
 block ^outer: eh_catch_all resume
}
";
    #[test]
    fn direct_nounwind_facts_strip_normal_openers_but_leave_handler_code_for_cfg_cleanup() {
        let (before, after, changed) = run(NESTED, 1, true);
        assert!(changed);
        assert_ne!(before, after);
        let normal = after.split("block ^inner:").next().unwrap();
        assert!(
            !normal.contains("eh_try")
                && !normal.contains("eh_cleanup")
                && !normal.contains("eh_end")
        );
        assert!(after.contains("block ^inner:") && after.contains("eh_catch_all"));
    }
    #[test]
    fn unsafe_nested_stacks_stay_at_o1_and_only_empty_cleanup_strips_at_o2() {
        let (before, after, changed) = run(NESTED, 1, false);
        assert!(!changed);
        assert_eq!(before, after);
        let (_, after, changed) = run(NESTED, 2, false);
        assert!(changed);
        let normal = after.split("block ^inner:").next().unwrap();
        assert!(normal.contains("eh_try ^outer"));
        assert!(!normal.contains("eh_cleanup"));
        assert_eq!(normal.matches("eh_end").count(), 1);
    }
    #[test]
    fn signature_unwind_no_has_the_same_effect_as_a_proven_callee() {
        let source = NESTED.replace("@g() -> void", "@g() -> void [unwind=no]");
        let (_, after, changed) = run(&source, 1, false);
        assert!(changed);
        assert!(!after.contains("eh_try"));
    }
    #[test]
    fn conflicting_stacks_empty_pops_and_unmodeled_normal_eh_keep_every_region() {
        for source in [
            "function @f(%c : i64) -> void {block ^entry: branch %c, ^left, ^right block ^left: eh_try ^landing jump ^join block ^right: jump ^join block ^join: eh_end return void block ^landing: eh_end resume}",
            "function @f() -> void {block ^entry: eh_try ^landing eh_end eh_end return void block ^landing: eh_end resume}",
            "function @f() -> void {block ^entry: eh_try ^landing eh_cleanup eh_end return void block ^landing: eh_end resume}",
            "function @f() -> void {block ^entry: eh_try ^landing eh_catch_all eh_end return void block ^landing: eh_end resume}",
        ] {
            let (before, after, changed) = run(source, 2, true);
            assert!(!changed, "{source}");
            assert_eq!(before, after);
        }
    }
    #[test]
    fn openers_outside_ordinary_reachability_and_nonempty_unsafe_cleanups_stay() {
        let source = "function @f() -> void {block ^entry: return void block ^dead: eh_try ^landing eh_end return void block ^landing: eh_end resume}";
        let (before, after, changed) = run(source, 2, true);
        assert!(!changed);
        assert_eq!(before, after);
        let source = NESTED.replace(
            "block ^inner: eh_end resume",
            "block ^inner: eh_end call void @g() resume",
        );
        let (before, after, changed) = run(&source, 2, false);
        assert!(!changed);
        assert_eq!(before, after);
    }
    #[test]
    fn nounwind_proofs_ignore_the_callees_signature_only_when_the_fact_is_proven() {
        for (binding, known, expected) in [
            ("", false, false),
            ("", true, true),
            ("[binding=weak, replaceable=yes]", true, false),
        ] {
            let source = format!(
                "declare function @g() -> void\nfunction @f() -> void {binding} {{block ^entry: call void @g() return void}}\n"
            );
            let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
            let id = first(&program);
            let facts = vec![known; program.symbols().len()];
            let mut body = program.take_body(id).unwrap().unwrap();
            let context = Context::new(&program, id, &mut body, 1, &facts, &[]);
            assert_eq!(proves_nounwind(&context), expected);
        }
    }
}

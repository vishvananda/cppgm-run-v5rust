// Translated from sema/analyzer_lambdas.cpp's capture frames. See NOTICE.
//! Active bodies collect captures; completed closures own ordered member facts.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, DataMemberKind, EntityId, ScopeId, ScopeKind},
    lambdas::Capture,
    types::{RefKind, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

pub(super) struct Frame {
    class: EntityId,
    pub call: EntityId,
    object: Option<TypeId>,
    default: Option<ast::CaptureDefault>,
    captures: Vec<EntityId>,
    captured: std::collections::HashSet<EntityId>,
    this_captured: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FrameId(u32);

#[derive(Default)]
pub(super) struct Frames {
    records: Vec<Frame>,
    by_call: std::collections::HashMap<EntityId, FrameId>,
    crossed: Vec<FrameId>,
}
impl Frames {
    pub(super) fn push(&mut self, frame: Frame) -> Result<(), Error> {
        if self.by_call.contains_key(&frame.call) {
            return Err(Error::Invariant("duplicate active closure body"));
        }
        let id = FrameId(
            u32::try_from(self.records.len()).map_err(|_| Error::Limit("closure frame count"))?,
        );
        self.by_call.insert(frame.call, id);
        self.records.push(frame);
        Ok(())
    }
    pub(super) fn pop(&mut self) -> Result<Frame, Error> {
        let frame = self
            .records
            .pop()
            .ok_or(Error::Invariant("closure capture frame"))?;
        let id = self
            .by_call
            .remove(&frame.call)
            .ok_or(Error::Invariant("closure frame index"))?;
        if id.0 as usize != self.records.len() {
            return Err(Error::Invariant("closure frame order"));
        }
        Ok(frame)
    }
    pub(super) fn get(&self, call: EntityId) -> Option<&Frame> {
        self.by_call
            .get(&call)
            .map(|id| &self.records[id.0 as usize])
    }
    fn get_mut(&mut self, call: EntityId) -> Option<&mut Frame> {
        let id = self.by_call.get(&call)?;
        Some(&mut self.records[id.0 as usize])
    }
    fn frame_mut(&mut self, id: FrameId) -> &mut Frame {
        &mut self.records[id.0 as usize]
    }
}

impl Frame {
    pub(super) fn object_type(&self) -> Option<TypeId> {
        if self.default.is_some() || self.this_captured {
            self.object
        } else {
            None
        }
    }
    fn add(
        &mut self,
        entity: EntityId,
        written: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.captured.insert(entity) {
            self.captures.push(entity);
        } else if written {
            return Err(Error::at(location, "an entity captured twice (5.1.2p8)"));
        }
        Ok(())
    }
}
impl Analyzer<'_> {
    pub(super) fn explicit_captures(
        &mut self,
        class: EntityId,
        call: EntityId,
        lambda: ast::Lambda,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Frame, Error> {
        let mut frame = Frame {
            class,
            call,
            object: self.current_object_type()?,
            default: lambda.capture_default,
            captures: Vec::new(),
            captured: std::collections::HashSet::new(),
            this_captured: false,
        };
        for index in 0..self.tree.captures.get(lambda.captures).len() {
            let capture = self.tree.captures.get(lambda.captures)[index];
            let reference = match capture.kind {
                ast::CaptureKind::This => {
                    if frame.this_captured
                        || matches!(frame.default, Some(ast::CaptureDefault::Copy))
                    {
                        return Err(Error::at(
                            location,
                            "`this` captured twice or after a copy capture-default (5.1.2p8)",
                        ));
                    }
                    if frame.object.is_none() {
                        return Err(Error::at(
                            location,
                            "`this` captured outside a non-static member function (5.1.2p8)",
                        ));
                    }
                    self.capture_this(location)?;
                    frame.this_captured = true;
                    continue;
                }
                ast::CaptureKind::Reference(_) => true,
                ast::CaptureKind::Copy(_) => false,
            };
            if capture.pack {
                return Err(Error::at(
                    location,
                    "a capture pack expansion that names no function parameter pack (14.5.3p5)",
                ));
            }
            if reference && matches!(frame.default, Some(ast::CaptureDefault::Reference)) {
                return Err(Error::at(
                    location,
                    "a reference capture after a reference capture-default (5.1.2p8)",
                ));
            }
            if !reference && matches!(frame.default, Some(ast::CaptureDefault::Copy)) {
                return Err(Error::at(
                    location,
                    "a copy capture after a copy capture-default (5.1.2p8)",
                ));
            }
            let (ast::CaptureKind::Copy(name) | ast::CaptureKind::Reference(name)) = capture.kind
            else {
                return Err(Error::Invariant("identifier capture"));
            };
            let found = self.unqualified(scope, name, super::lookup::Filter::Ordinary)?;
            let binding = found.map(|binding| self.graph.binding(binding));
            let automatic = binding.is_some_and(|binding| {
                let entity = self.graph.entity(binding.entity);
                binding.kind == BindingKind::Parameter
                    || binding.kind == BindingKind::Variable
                        && entity.variable.is_some_and(|variable| variable.automatic)
                        && entity.data_member.is_none()
                        && entity.home.is_some_and(|home| {
                            matches!(
                                self.graph.scope(home).kind,
                                ScopeKind::Block | ScopeKind::Function
                            )
                        })
            });
            if !automatic || self.body_context.is_none() {
                return Err(Error::at(
                    location,
                    "a capture names no automatic variable of the reaching scope (5.1.2p10)",
                ));
            }
            let entity = binding.ok_or(Error::Invariant("capture binding"))?.entity;
            if !self.capture_variable(entity, location)? {
                return Err(Error::at(
                    location,
                    "a capture of a variable a local class cannot use (5.1.2p10)",
                ));
            }
            frame.add(entity, true, location)?;
            if !reference {
                return Err(Error::Missing("lambda copy capture"));
            }
        }
        Ok(frame)
    }

    pub(super) fn capture_variable(
        &mut self,
        entity: EntityId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        // This query only follows body/scope IDs and adds captures; it cannot
        // enter another capture query. Retain its capacity across name uses,
        // and restore empty scratch even when a crossed body rejects the use.
        let mut crossed = std::mem::take(&mut self.capture_frames.crossed);
        let result = self.capture_variable_in_context(entity, location, &mut crossed);
        crossed.clear();
        self.capture_frames.crossed = crossed;
        result
    }

    fn capture_variable_in_context(
        &mut self,
        entity: EntityId,
        location: SourceOffset,
        crossed: &mut Vec<FrameId>,
    ) -> Result<bool, Error> {
        let owner = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("captured entity owner"))?;
        let mut reached = false;
        for function in self
            .body_context
            .iter()
            .map(|body| body.entity)
            .chain(self.enclosing_bodies.iter().rev().copied())
        {
            if self
                .graph
                .entity(function)
                .scope
                .is_some_and(|scope| self.graph.encloses(scope, owner))
            {
                reached = true;
                break;
            }
            let Some(&index) = self.capture_frames.by_call.get(&function) else {
                return Ok(false);
            };
            crossed.push(index);
        }
        if !reached {
            return Ok(false);
        }
        for &index in crossed.iter().rev() {
            let frame = self.capture_frames.frame_mut(index);
            if frame.captured.contains(&entity) {
                continue;
            }
            match frame.default {
                None => {
                    return Err(Error::at(
                        location,
                        "a lambda odr-uses an automatic variable it does not capture (5.1.2p11)",
                    ));
                }
                Some(ast::CaptureDefault::Copy) => {
                    return Err(Error::Missing("lambda copy capture"));
                }
                Some(ast::CaptureDefault::Reference) => frame.add(entity, false, location)?,
            }
        }
        Ok(true)
    }

    pub(super) fn capture_this(&mut self, location: SourceOffset) -> Result<(), Error> {
        if self.unevaluated != 0 {
            return Ok(());
        }
        for function in self
            .body_context
            .iter()
            .map(|body| body.entity)
            .chain(self.enclosing_bodies.iter().rev().copied())
        {
            let Some(frame) = self.capture_frames.get_mut(function) else {
                break;
            };
            if frame.this_captured {
                break;
            }
            if frame.default.is_none() {
                return Err(Error::at(
                    location,
                    "a lambda uses `this` without capturing it (5.1.2p10)",
                ));
            }
            frame.this_captured = true;
        }
        Ok(())
    }

    pub(super) fn finish_captures(&mut self, frame: Frame) -> Result<(), Error> {
        let class = frame.class;
        if frame.this_captured {
            let object = frame.object.ok_or(Error::Invariant("captured this type"))?;
            let ty = self.graph.types.pointer(object)?;
            let member = self.capture_member(class, ty)?;
            self.graph
                .closures
                .records
                .get_mut(&class)
                .ok_or(Error::Invariant("closure facts"))?
                .this_member = Some(member);
        }
        let mut captures = Vec::with_capacity(frame.captures.len());
        for entity in frame.captures {
            let mut ty = self
                .graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("capture type"))?;
            if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            let ty = self.graph.types.reference(ty, RefKind::Lvalue)?;
            let member = self.capture_member(class, ty)?;
            captures.push(Capture { entity, member });
        }
        if frame.this_captured || !captures.is_empty() {
            self.graph.entity_mut(class).layout_category = super::layout::LayoutCategory::NonPod;
        }
        self.graph.closures.captures.insert(class, captures);
        Ok(())
    }
    fn capture_member(&mut self, class: EntityId, ty: TypeId) -> Result<EntityId, Error> {
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("capture class scope"))?;
        let member = self.graph.add_entity(Some(ty), None, Keyword::Int)?;
        let entity = self.graph.entity_mut(member);
        entity.home = Some(scope);
        entity.data_member = Some(DataMemberKind::Instance);
        self.graph.entity_mut(class).members.push(member);
        Ok(member)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn failed_nested_capture_restores_body_and_capture_contexts() {
        let source =
            SourceBuffer::new(b"int f(){int x=1;return [&]{return []{return x;}();}();}".to_vec())
                .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"capture-rollback").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let result = analyzer.declarations(declarations, analyzer.graph.global);
        assert!(matches!(
            result,
            Err(Error::Diagnostic {
                message: "a lambda odr-uses an automatic variable it does not capture (5.1.2p11)",
                ..
            })
        ));
        assert!(analyzer.capture_frames.records.is_empty());
        assert!(analyzer.capture_frames.by_call.is_empty());
        assert!(analyzer.capture_frames.crossed.is_empty());
        assert!(analyzer.capture_frames.crossed.capacity() >= 2);
        assert!(analyzer.body_context.is_none());
        assert!(analyzer.enclosing_bodies.is_empty());
        assert!(analyzer.callee_name.is_none());
        assert_eq!(analyzer.unevaluated, 0);
        assert_eq!(analyzer.template_depth, 0);
    }

    #[test]
    fn nested_captures_share_entity_identity_and_own_distinct_reference_members() {
        let source = SourceBuffer::new(
            b"int f(){int x=1;auto a=[&]{auto b=[&x]{++x;return x;};return b();};return a();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"nested-captures").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expression_count = tree.expressions.mark();
        let graph = super::super::template_bodies::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(tree.expressions.mark(), expression_count);
        assert_eq!(graph.closures.records.len(), 2);
        let mut members = Vec::new();
        let mut entities = Vec::new();
        for (&class, closure) in &graph.closures.records {
            let [capture] = graph.lower_closure_captures(class) else {
                panic!("one capture per closure");
            };
            members.push(capture.member);
            entities.push(capture.entity);
            assert!(matches!(
                graph
                    .types
                    .get(graph.entity(capture.member).ty.unwrap())
                    .kind,
                TypeKind::Reference {
                    kind: RefKind::Lvalue,
                    ..
                }
            ));
            assert_eq!(graph.entity(class).members, [capture.member]);
            assert!(closure.invoker.is_none());
        }
        assert_eq!(entities[0], entities[1]);
        assert_ne!(members[0], members[1]);
    }

    #[test]
    fn captured_this_precedes_written_entities_and_constants_remain_value_reads() {
        let source = SourceBuffer::new(b"struct A{int n;int f(){int x=2;const int c=3;auto a=[&x,this]{return n+x+c;};auto b=[&]{return sizeof(x)+c;};return a()+b();}};int main(){A a;return a.f();}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"capture-order").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::template_bodies::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let mut counts = Vec::new();
        for (&class, closure) in &graph.closures.records {
            let captures = graph.lower_closure_captures(class);
            counts.push(captures.len());
            if let Some(this) = closure.this_member {
                assert_eq!(graph.entity(class).members, [this, captures[0].member]);
                assert_eq!(graph.lower_member_offset(this).unwrap(), 0);
                assert_eq!(graph.lower_member_offset(captures[0].member).unwrap(), 8);
            } else {
                assert!(captures.is_empty());
            }
        }
        counts.sort_unstable();
        assert_eq!(counts, [0, 1]);
    }
}

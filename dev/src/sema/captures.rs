// Translated from sema/analyzer_lambdas.cpp's capture frames. See NOTICE.
//! Active bodies collect captures; completed closures own ordered member facts.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, DataMemberKind, EntityId, ScopeId, ScopeKind},
    lambdas::Capture,
    types::{Cv, RefKind, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

pub(super) struct Frame {
    class: EntityId,
    pub call: EntityId,
    object: Option<TypeId>,
    default: Option<ast::CaptureDefault>,
    captures: Vec<EntityId>,
    captured: std::collections::HashMap<EntityId, bool>,
    mutable: bool,
    location: SourceOffset,
    this_captured: bool,
    sealed: bool,
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
        copy: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.sealed && !self.captured.contains_key(&entity) {
            return Err(Error::at(
                location,
                "a templated lambda's body names an automatic variable its closure did not capture",
            ));
        }
        if let std::collections::hash_map::Entry::Vacant(entry) = self.captured.entry(entity) {
            entry.insert(copy);
            self.captures.push(entity);
        } else if written {
            return Err(Error::at(location, "an entity captured twice (5.1.2p8)"));
        }
        Ok(())
    }
}
impl Analyzer<'_> {
    /// Scan resolved uses while checking the member-template definition. This
    /// fixes capture order before any specialization can be demanded.
    pub(super) fn scan_capture(
        &mut self,
        binding: super::graph::BindingId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let Some(call) = self.capture_scan else {
            return Ok(());
        };
        let binding = self.graph.binding(binding);
        let entity = self.graph.entity(binding.entity);
        let Some(owner) = entity.home else {
            return Ok(());
        };
        let frame = self
            .capture_frames
            .get(call)
            .ok_or(Error::Invariant("capture scan frame"))?;
        if frame.default.is_none() {
            return Ok(());
        }
        if entity.data_member == Some(DataMemberKind::Instance)
            || self.graph.scope(owner).kind == ScopeKind::Class
                && entity.function.is_some_and(|function| !function.is_static)
        {
            if frame.object.is_some() {
                self.capture_frames
                    .get_mut(call)
                    .ok_or(Error::Invariant("capture scan frame"))?
                    .this_captured = true;
            }
            return Ok(());
        }
        let automatic = binding.kind == BindingKind::Parameter
            || binding.kind == BindingKind::Variable
                && entity.variable.is_some_and(|variable| variable.automatic)
                && matches!(
                    self.graph.scope(owner).kind,
                    ScopeKind::Block | ScopeKind::Function
                );
        let class_scope = self
            .graph
            .entity(frame.class)
            .scope
            .ok_or(Error::Invariant("closure scan scope"))?;
        if automatic
            && !self.graph.encloses(class_scope, owner)
            && !self
                .graph
                .entity(call)
                .scope
                .is_some_and(|scope| self.graph.encloses(scope, owner))
        {
            let copy = matches!(frame.default, Some(ast::CaptureDefault::Copy));
            // The definition walk fixes only this closure's potential captures.
            // Enclosing bodies acquire captures through their own evaluated uses.
            self.capture_frames
                .get_mut(call)
                .ok_or(Error::Invariant("capture scan frame"))?
                .add(binding.entity, false, copy, location)?;
        }
        Ok(())
    }

    pub(super) fn scan_capture_this(&mut self) -> Result<(), Error> {
        if let Some(call) = self.capture_scan {
            let frame = self
                .capture_frames
                .get_mut(call)
                .ok_or(Error::Invariant("this capture scan"))?;
            if frame.default.is_some() {
                frame.this_captured = true;
            }
        }
        Ok(())
    }

    pub(super) fn scan_nested_lambda(
        &mut self,
        lambda: ast::LambdaId,
        mut scope: ScopeId,
    ) -> Result<(), Error> {
        if self.capture_scan.is_none() {
            return Ok(());
        }
        let lambda = *self.tree.lambdas.get(lambda);
        for index in 0..self.tree.captures.get(lambda.captures).len() {
            let capture = self.tree.captures.get(lambda.captures)[index];
            match capture.kind {
                ast::CaptureKind::This => {
                    if let Some(call) = self.capture_scan {
                        let frame = self
                            .capture_frames
                            .get_mut(call)
                            .ok_or(Error::Invariant("nested capture scan"))?;
                        if frame.default.is_some() {
                            frame.this_captured = true;
                        }
                    }
                }
                ast::CaptureKind::Copy(name) | ast::CaptureKind::Reference(name) => {
                    if let Some(binding) =
                        self.unqualified(scope, name, super::lookup::Filter::Ordinary)?
                    {
                        self.scan_capture(binding, SourceOffset(0))?;
                    }
                }
            }
        }
        if let Some(parameters) = lambda.template_parameters {
            scope = self
                .graph
                .add_scope(ScopeKind::TemplateParameters, scope, None, None)?;
            self.template_parameters(parameters, scope, true)?;
        }
        let inner = self.graph.add_scope(ScopeKind::Block, scope, None, None)?;
        let ty = self.graph.types.scalar(super::types::Scalar::Basic(
            crate::types::FundamentalType::Int,
        ))?;
        if let Some(parameters) = lambda.parameters {
            for index in 0..self.tree.parameters.get(parameters.parameters).len() {
                let parameter = self.tree.parameters.get(parameters.parameters)[index];
                let name = self.declarator_name(parameter.declarator)?;
                if let Some(name) = name {
                    let entity = self.graph.add_entity(Some(ty), Some(name), Keyword::Int)?;
                    let _binding = self.graph.add_binding(
                        inner,
                        Some(inner),
                        super::graph::Binding {
                            kind: BindingKind::Parameter,
                            entity,
                            name: Some(name),
                            ty: Some(ty),
                            key: Keyword::Int,
                        },
                    )?;
                }
            }
        }
        if let ast::LambdaBody::Parsed(body) = lambda.body {
            self.statement(body, inner)?;
        }
        Ok(())
    }

    pub(super) fn sealed_closure_frame(&self, call: EntityId) -> Result<Option<Frame>, Error> {
        let Some(class) = self.graph.lower_call_closure(call) else {
            return Ok(None);
        };
        let closure = self
            .graph
            .lower_closure(class)
            .ok_or(Error::Invariant("closure frame facts"))?;
        let Some(lambda) = closure.template else {
            return Ok(None);
        };
        let lambda = self.tree.lambdas.get(lambda);
        let object = closure
            .this_member
            .map(|member| self.graph.lower_entity_type(member))
            .transpose()?
            .and_then(|ty| {
                if let TypeKind::Pointer(object) = self.graph.types.get(ty).kind {
                    Some(object)
                } else {
                    None
                }
            });
        let mut frame = Frame {
            class,
            call,
            object,
            default: lambda.capture_default,
            captures: Vec::new(),
            captured: std::collections::HashMap::new(),
            mutable: lambda.is_mutable,
            location: SourceOffset(0),
            this_captured: closure.this_member.is_some(),
            sealed: false,
        };
        for capture in self.graph.lower_closure_captures(class) {
            let copy = !matches!(
                self.graph
                    .types
                    .get(self.graph.lower_entity_type(capture.member)?)
                    .kind,
                TypeKind::Reference { .. }
            );
            frame.add(capture.entity, false, copy, SourceOffset(0))?;
        }
        frame.sealed = true;
        Ok(Some(frame))
    }

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
            captured: std::collections::HashMap::new(),
            mutable: lambda.is_mutable,
            location,
            this_captured: false,
            sealed: false,
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
            frame.add(entity, true, !reference, location)?;
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
            let frame = self.capture_frames.frame_mut(index);
            if frame.sealed {
                if !frame.captured.contains_key(&entity) {
                    return Err(Error::at(
                        location,
                        "a templated lambda's body names an automatic variable its closure did not capture",
                    ));
                }
                reached = true;
                break;
            }
            crossed.push(index);
        }
        if !reached {
            return Ok(false);
        }
        for &index in crossed.iter().rev() {
            let frame = self.capture_frames.frame_mut(index);
            if frame.captured.contains_key(&entity) {
                continue;
            }
            match frame.default {
                None => {
                    return Err(Error::at(
                        location,
                        "a lambda odr-uses an automatic variable it does not capture (5.1.2p11)",
                    ));
                }
                Some(default) => {
                    frame.add(
                        entity,
                        false,
                        matches!(default, ast::CaptureDefault::Copy),
                        location,
                    )?;
                }
            }
        }
        Ok(true)
    }

    pub(super) fn capture_this(&mut self, location: SourceOffset) -> Result<(), Error> {
        if self.deferring_type_query() {
            self.type_queries
                .uses
                .push(super::type_queries::DeferredUse::This(location));
        }
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
            let copy = frame.captured.get(&entity).copied().unwrap_or(false)
                && !matches!(self.graph.types.get(ty).kind, TypeKind::Function { .. });
            let constructor = if copy {
                self.capture_constructor(class, ty, frame.location)?
            } else {
                ty = self.graph.types.reference(ty, RefKind::Lvalue)?;
                None
            };
            let member = self.capture_member(class, ty)?;
            captures.push(Capture {
                entity,
                member,
                constructor,
            });
        }
        if frame.this_captured || !captures.is_empty() {
            self.graph.entity_mut(class).layout_category = super::layout::LayoutCategory::NonPod;
        }
        self.graph.closures.captures.insert(class, captures);
        Ok(())
    }
    fn capture_constructor(
        &mut self,
        closure: EntityId,
        mut ty: TypeId,
        location: SourceOffset,
    ) -> Result<Option<super::construction::ConstructorId>, Error> {
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let Some(class) = self.graph.class_of(ty) else {
            return Ok(None);
        };
        self.complete_class(class)?;
        if !self.graph.entity(class).complete {
            return Err(Error::at(
                location,
                "a capture by copy of an object of incomplete class type",
            ));
        }
        self.prepare_copy_properties(class)?;
        let source = super::expression_facts::ExprFacts::new(
            ty,
            super::expression_facts::ValueCategory::Lvalue,
        );
        let selected = self.select_subobject_copy(class, source, true);
        let selected = match selected {
            Ok(selected) if !self.selected_copy_properties(class, selected)?.deleted => selected,
            Ok(_) | Err(Error::Diagnostic { .. }) => {
                return Err(Error::at(
                    location,
                    "a capture by copy of a class object without an accessible copy constructor (5.1.2p21)",
                ));
            }
            Err(error) => return Err(error),
        };
        let scope = self
            .graph
            .entity(closure)
            .scope
            .ok_or(Error::Invariant("capture context"))?;
        if let super::copy_initialization::CopyOrigin::Declared(binding) = selected.origin {
            self.check_special_member_access(binding, class, scope, location).map_err(|error| {
                if matches!(error, Error::Diagnostic { .. }) {
                    Error::at(location, "a capture by copy of a class object without an accessible copy constructor (5.1.2p21)")
                } else { error }
            })?;
        }
        let constructor = self.copy_constructor_plan(class, selected, location)?;
        self.demand_constructor_use(constructor)?;
        Ok(Some(constructor))
    }

    /// A potentially captured name designates the nearest copied member.
    /// Unevaluated decltype queries may consult a default without capturing it.
    pub(super) fn captured_type(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        potential: bool,
    ) -> Result<TypeId, Error> {
        let record = self.graph.entity(entity);
        let Some(owner) = record.home else {
            return Ok(ty);
        };
        if !matches!(
            self.graph.scope(owner).kind,
            ScopeKind::Block | ScopeKind::Function
        ) || record.variable.is_some_and(|variable| !variable.automatic)
            || record.data_member.is_some()
        {
            return Ok(ty);
        }
        for function in self
            .body_context
            .iter()
            .map(|body| body.entity)
            .chain(self.enclosing_bodies.iter().rev().copied())
        {
            let Some(frame) = self.capture_frames.get(function) else {
                break;
            };
            if self
                .graph
                .entity(function)
                .scope
                .is_some_and(|scope| self.graph.encloses(scope, owner))
            {
                break;
            }
            let copy = frame.captured.get(&entity).copied();
            if potential && copy.is_none() && frame.default.is_none() {
                break;
            }
            if !copy
                .unwrap_or(potential && matches!(frame.default, Some(ast::CaptureDefault::Copy)))
            {
                continue;
            }
            let member = match self.graph.types.get(ty).kind {
                TypeKind::Reference { element, .. } => element,
                _ => ty,
            };
            if matches!(self.graph.types.get(member).kind, TypeKind::Function { .. }) {
                break;
            }
            return if frame.mutable {
                Ok(member)
            } else {
                self.graph
                    .types
                    .qualify(member, Cv(self.graph.types.get(member).cv.0 | Cv::CONST.0))
            };
        }
        Ok(ty)
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
    #[test]
    fn copy_capture_facts_keep_order_types_and_constructor_selection_without_new_expressions() {
        let source = SourceBuffer::new(b"struct A{A(){}A(const A&){}~A(){}};int f(){return 1;}int main(){int x=1;int ar[2]={2,3};int(&fn)()=f;A obj;auto l=[x,ar,fn,obj]{return x+ar[0]+fn();};return l();}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"copy-capture-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let graph = super::super::template_bodies::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(graph.closures.records.len(), 1);
        let (&class, closure) = graph.closures.records.iter().next().unwrap();
        assert!(!closure.convertible);
        let captures = graph.lower_closure_captures(class);
        assert_eq!(captures.len(), 4);
        assert_eq!(
            graph.entity(class).members,
            captures.iter().map(|c| c.member).collect::<Vec<_>>()
        );
        for (capture, name) in captures.iter().zip([b"x".as_slice(), b"ar", b"fn", b"obj"]) {
            assert_eq!(
                cursor.name(graph.entity(capture.entity).name.unwrap()),
                name
            );
        }
        assert!(matches!(
            graph
                .types
                .get(graph.entity(captures[1].member).ty.unwrap())
                .kind,
            TypeKind::Array { .. }
        ));
        assert!(
            matches!(graph.types.get(graph.entity(captures[2].member).ty.unwrap()).kind, TypeKind::Reference { element, .. }
            if matches!(graph.types.get(element).kind, TypeKind::Function { .. }))
        );
        assert!(
            captures[..3]
                .iter()
                .all(|capture| capture.constructor.is_none())
        );
        let constructor = captures[3].constructor.unwrap();
        assert_eq!(
            Some(graph.lower_constructor(constructor).class),
            graph.class_of(graph.entity(captures[3].member).ty.unwrap())
        );
        assert!(!graph.lower_constructor_trivial(constructor));
    }
}

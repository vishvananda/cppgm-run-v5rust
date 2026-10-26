// Translated from sema/analyzer_exceptions.cpp and analyzer_exception_sets.cpp. See NOTICE.
//! Exception specifications belong to declarations, never canonical function types.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    graph::{EntityId, ScopeId},
    types::{RefKind, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind, Qualifier, QualifierKind},
};

#[derive(Clone, Copy, Debug)]
pub(super) struct SpecificationId(u32);
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ExceptionSet {
    All,
    None,
    Dynamic(Vec<TypeId>),
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum DestructorSpecification {
    #[default]
    Explicit,
    Implicit,
}
#[derive(Clone, Debug)]
pub(super) enum ExceptionState {
    Absent,
    Pending(SpecificationId),
    Evaluating,
    Value(ExceptionSet),
    Template(super::template_exceptions::TemplateException),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OmittedSpecification {
    Compare,
    InheritSystemNothrow,
}
#[derive(Clone, Copy)]
enum SpecificationMode {
    Ordinary,
    ImplicitDestructor,
    TemplateExpression,
    TemplateCompatibility,
}
#[derive(Clone, Copy)]
pub(super) struct PendingSpecification {
    entity: EntityId,
    qualifier: Option<Qualifier>,
    scope: ScopeId,
    limit: usize,
    location: SourceOffset,
    first: bool,
    done: bool,
    mode: SpecificationMode,
}

impl Analyzer<'_> {
    pub(super) fn prepare_throw_specification_layouts(
        &mut self,
        entity: EntityId,
    ) -> Result<(), Error> {
        if !self.graph.lower_function_defined(entity) {
            return Ok(());
        }
        let count = self
            .graph
            .lower_throw_specification(entity)
            .map_or(0, <[TypeId]>::len);
        for index in 0..count {
            let ty = self
                .graph
                .lower_throw_specification(entity)
                .ok_or(Error::Invariant("exception specification disappeared"))?[index];
            let object = match self.graph.types.get(ty).kind {
                TypeKind::Reference { element, .. } => element,
                _ => ty,
            };
            self.prepare_rtti_layouts(object, SourceOffset(0))?;
        }
        Ok(())
    }

    pub(super) fn substitute_throw_specification(&mut self, entity: EntityId) -> Result<(), Error> {
        if self.graph.source_exceptions.specification(entity).is_some() {
            return Ok(());
        }
        let Some(primary) = self.graph.function_templates.primary(entity) else {
            return Ok(());
        };
        let primary = self.graph.binding(primary).entity;
        let Some(types) = self
            .graph
            .source_exceptions
            .specification(primary)
            .map(<[TypeId]>::to_vec)
        else {
            return Ok(());
        };
        let Some(head) = self.graph.entity(primary).template_head else {
            return Ok(());
        };
        let arguments = self
            .graph
            .function_templates
            .arguments
            .get(&entity)
            .ok_or(Error::Invariant(
                "specification without specialization arguments",
            ))?
            .iter()
            .copied()
            .map(Some)
            .collect::<Vec<_>>();
        let mut substitution = super::function_templates::Substitution::new(
            self.graph.templates.depth(head),
            &arguments,
        );
        let Some(types) = self.substitute_pack_list(&types, &mut substitution)? else {
            return Ok(());
        };
        let mut adjusted = Vec::with_capacity(types.len());
        for ty in types {
            if matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Pack(_) | TypeKind::Expansion(_)
            ) {
                return Ok(());
            }
            let ty = self.graph.types.parameter(ty)?;
            let object = match self.graph.types.get(ty).kind {
                TypeKind::Reference { element, .. } => element,
                _ => ty,
            };
            self.prepare_type_information(object)?;
            adjusted.push(ty);
        }
        self.graph
            .source_exceptions
            .record_specification(entity, adjusted);
        Ok(())
    }

    pub(super) fn pending_virtual_noexcept(&self, function: EntityId) -> bool {
        matches!(self.graph.entity(function).exceptions, ExceptionState::Pending(id)
            if matches!(self.specifications[id.0 as usize].qualifier, Some(Qualifier { kind: QualifierKind::Noexcept(Some(_)), .. })))
    }

    pub(super) fn virtual_nothrow(&mut self, function: EntityId) -> Result<bool, Error> {
        if let ExceptionState::Pending(id) = self.graph.entity(function).exceptions
            && matches!(self.specifications[id.0 as usize].qualifier, Some(Qualifier { kind: QualifierKind::Throw(types), .. }) if !types.is_empty())
        {
            // A nonempty dynamic set allows exceptions even before its names
            // are resolved in the complete-class context.
            return Ok(false);
        }
        self.function_may_throw(function)
            .map(|may_throw| !may_throw)
    }

    pub(super) fn function_may_throw(&mut self, entity: EntityId) -> Result<bool, Error> {
        if self.graph.entity(entity).builtin.is_some() {
            return Ok(false);
        }
        if self.graph.entity(entity).constructor_exceptions
            == super::construction::ConstructorExceptionSource::SubobjectPlan
        {
            let binding = self.graph.function_origin(entity)?;
            let plan = self.declared_constructor_plan(binding)?;
            return Ok(!self.graph.constructors.get(plan).nothrow);
        }
        if self.graph.function_templates.primary(entity).is_some()
            && matches!(
                self.graph.entity(entity).exceptions,
                ExceptionState::Template(_)
            )
        {
            self.resolve_specialization_exception(entity)?;
        }
        if let ExceptionState::Pending(id) = self.graph.entity(entity).exceptions {
            self.resolve_specification(id)?;
        }
        Ok(!matches!(
            self.graph.entity(entity).exceptions,
            ExceptionState::Value(ExceptionSet::None)
        ))
    }
    pub(super) fn declarator_specifies_exceptions(
        &self,
        mut id: Option<ast::DeclaratorId>,
    ) -> bool {
        while let Some(current) = id {
            let node = self.tree.declarators.get(current);
            for syntax_index in 0..self.tree.suffixes.get(node.suffixes).len() {
                let syntax_item = self.tree.suffixes.get(node.suffixes)[syntax_index];
                let suffix = &syntax_item;
                if let ast::Suffix::Function { qualifiers, .. } = suffix
                    && self.tree.qualifiers.get(*qualifiers).iter().any(|q| {
                        matches!(q.kind, QualifierKind::Throw(_) | QualifierKind::Noexcept(_))
                    })
                {
                    return true;
                }
            }
            id = node.nested;
        }
        false
    }
    fn nonthrowing_specification(&self, qualifier: Option<Qualifier>) -> bool {
        qualifier.is_some_and(|q| {
            matches!(q.kind, QualifierKind::Noexcept(None))
                || matches!(q.kind, QualifierKind::Throw(types) if types.is_empty())
                || matches!(q.kind, QualifierKind::Noexcept(Some(expr)) if self.true_operand(expr))
        })
    }
    pub(super) fn record_function_exceptions(
        &mut self,
        entity: EntityId,
        info: &DeclaratorInfo,
        mut scope: ScopeId,
        location: SourceOffset,
        limit: usize,
    ) -> Result<(), Error> {
        // Reserved builtins keep the registry's implicit exception facts;
        // explicit declarations need not repeat those facts (GCC's rule).
        if self.graph.entity(entity).builtin.is_some() {
            return Ok(());
        }
        let mut qualifier =
            info.qualifiers.and_then(|list| {
                self.tree.qualifiers.get(list).iter().copied().find(|q| {
                    matches!(q.kind, QualifierKind::Throw(_) | QualifierKind::Noexcept(_))
                })
            });
        let first = matches!(self.graph.entity(entity).exceptions, ExceptionState::Absent);
        if first {
            // Only a syntactically non-throwing first C declaration in a
            // system inclusion gives later omitted specifications this rule.
            let inherit = self.nonthrowing_specification(qualifier)
                && self
                    .graph
                    .entity(entity)
                    .function
                    .is_some_and(|props| props.language == super::linkage::Language::C)
                && self.positions.in_system_header(location);
            self.graph.entity_mut(entity).exception_omission = if inherit {
                OmittedSpecification::InheritSystemNothrow
            } else {
                OmittedSpecification::Compare
            };
        } else if qualifier.is_none()
            && self.graph.entity(entity).exception_omission
                == OmittedSpecification::InheritSystemNothrow
        {
            // The first specification remains in force for calls and emission.
            return Ok(());
        }
        let special = self
            .graph
            .entity(entity)
            .function
            .and_then(|props| props.special);
        if self.graph.entity(entity).function.is_some_and(|props| {
            props.allocation == Some(super::function_names::AllocationOrigin::Predeclared)
        }) {
            if matches!(
                qualifier,
                Some(Qualifier {
                    kind: QualifierKind::Throw(_),
                    ..
                })
            ) && self.template_depth == 0
            {
                self.exception_set(qualifier, info.lookup)?;
            }
            self.graph
                .entity_mut(entity)
                .function
                .as_mut()
                .ok_or(Error::Invariant("allocation facts lost"))?
                .allocation = Some(super::function_names::AllocationOrigin::Declared);
            return Ok(());
        }
        // 15.4p15 applies to every omitted specification, including an
        // out-of-class definition or another redeclaration. Compare that
        // implicit value with the first declaration's retained facts.
        if qualifier.is_none()
            && special.is_some_and(super::function_names::SpecialName::deallocation)
        {
            qualifier = Some(Qualifier {
                location,
                kind: QualifierKind::Noexcept(None),
            });
        }
        if !first
            && !self.graph.exceptions_enabled
            && special.is_some_and(super::function_names::SpecialName::allocation)
        {
            // Under -fno-exceptions the first allocation/deallocation
            // declaration retains its specification across redeclarations.
            return Ok(());
        }
        if !first
            && self.template_depth != 0
            && self.graph.entity(entity).template_head.is_none()
            && matches!(
                self.graph.entity(entity).exceptions,
                ExceptionState::Template(_)
            )
        {
            // pending_noexcept does not queue an ordinary member of a
            // class-template pattern. Its expression is retained for the
            // instance, but the pattern's first nothrow value stays false.
            // A redeclaration with another operand waits for instantiation;
            // one with no operand compares that retained pattern value.
            if self.nonthrowing_specification(qualifier) {
                return Err(Error::at(
                    location,
                    "redeclaration with an incompatible exception-specification",
                ));
            }
            return Ok(());
        }
        if self.template_depth != 0 && qualifier.is_some_and(|q| {
            !matches!(q.kind, QualifierKind::Noexcept(None))
                && !matches!(q.kind, QualifierKind::Throw(types) if types.is_empty())
                && !matches!(q.kind, QualifierKind::Noexcept(Some(expr)) if self.true_operand(expr))
        }) {
            if let Some(Qualifier {
                kind: QualifierKind::Noexcept(Some(operand)),
                ..
            }) = qualifier
            {
                return self
                    .record_template_exception(entity, info, scope, operand, location, limit);
            }
            if matches!(
                qualifier,
                Some(Qualifier {
                    kind: QualifierKind::Throw(_),
                    ..
                })
            ) {
                return self.record_template_throw(entity, qualifier, scope, location);
            }
            return Err(Error::Invariant("unexpected template specification"));
        }
        if !first
            && self.template_depth != 0
            && matches!(
                self.graph.entity(entity).exceptions,
                ExceptionState::Template(_)
            )
        {
            // With no new operand, the C++ template comparison waits for
            // instantiation when its first specification is pending.
            return Ok(());
        }
        let implicit_destructor = !first
            && qualifier.is_none()
            && self.template_depth == 0
            && (special == Some(super::function_names::SpecialName::Destructor)
                || self
                    .graph
                    .entity(entity)
                    .member_function
                    .is_some_and(|key| key.kind == super::graph::MemberKind::Destructor));
        if implicit_destructor && !self.semantic() {
            // Types mode never synthesizes the subobjects' destructor facts.
            // The C++ omitted redeclaration therefore has the implicit true
            // specification, while its first omitted declaration allowed all.
            qualifier = Some(Qualifier {
                location,
                kind: QualifierKind::Noexcept(None),
            });
        }
        let operand = qualifier.and_then(|q| {
            if let QualifierKind::Noexcept(expr) = q.kind {
                expr
            } else {
                None
            }
        });
        if !first
            && self.class_depth != 0
            && (operand.is_some()
                || matches!(
                    self.graph.entity(entity).exceptions,
                    ExceptionState::Pending(_)
                ))
        {
            // C++ defers comparison when a member's operand needs its class.
            return Ok(());
        }
        if operand.is_some()
            && let Some(clause) = info.parameters
        {
            let prototype =
                self.graph
                    .add_scope(super::graph::ScopeKind::Function, scope, None, None)?;
            let count = self.graph.types.function_parameter_count(info.ty)?;
            for (index, &parameter) in self
                .tree
                .parameters
                .get(clause.parameters)
                .iter()
                .take(count)
                .enumerate()
            {
                let ty = self.graph.types.function_parameter(info.ty, index)?;
                if let Some(name) = self.declarator_name(parameter.declarator)? {
                    let adjusted = self.graph.types.parameter_object(ty)?;
                    let entity = self
                        .graph
                        .add_entity(Some(adjusted), Some(name), Keyword::Int)?;
                    self.graph.add_binding(
                        prototype,
                        None,
                        super::graph::Binding {
                            kind: super::graph::BindingKind::Parameter,
                            name: Some(name),
                            entity,
                            ty: Some(ty),
                            key: Keyword::Int,
                        },
                    )?;
                }
            }
            scope = prototype;
        }
        let id = SpecificationId(
            u32::try_from(self.specifications.len())
                .map_err(|_| Error::Limit("exception specification limit"))?,
        );
        self.specifications.push(PendingSpecification {
            entity,
            qualifier,
            scope,
            limit,
            location,
            first,
            done: false,
            mode: if implicit_destructor && self.semantic() {
                SpecificationMode::ImplicitDestructor
            } else if self.template_depth == 0 && self.graph.entity(entity).template_head.is_none()
            {
                SpecificationMode::Ordinary
            } else {
                SpecificationMode::TemplateCompatibility
            },
        });
        if first {
            self.graph.entity_mut(entity).exceptions = ExceptionState::Pending(id);
        }
        let member_throw = self.class_depth != 0
            && qualifier.is_some_and(
                |q| matches!(q.kind, QualifierKind::Throw(types) if !types.is_empty()),
            );
        if member_throw {
            self.pending_throws.push(id);
        }
        let literal_true = operand.is_some_and(|expr| self.true_operand(expr));
        if !first || (!member_throw && (operand.is_none() || literal_true)) {
            self.resolve_specification(id)?;
        }
        Ok(())
    }
    fn true_operand(&self, mut expr: ast::ExprId) -> bool {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(expr).kind {
            expr = inner;
        }
        matches!(
            self.tree.expressions.get(expr).kind,
            ExprKind::Keyword(Keyword::True)
        )
    }
    fn resolve_specification(&mut self, id: SpecificationId) -> Result<(), Error> {
        let pending = self.specifications[id.0 as usize];
        if pending.done {
            return Ok(());
        }
        if matches!(pending.mode, SpecificationMode::TemplateExpression) {
            let ExceptionState::Template(pattern) = self.graph.entity(pending.entity).exceptions
            else {
                return Err(Error::Invariant("missing template exception pattern"));
            };
            self.check_template_exception(pattern, pending)?;
            self.specifications[id.0 as usize].done = true;
            return Ok(());
        }
        if pending.first {
            self.graph.entity_mut(pending.entity).exceptions = ExceptionState::Evaluating;
        } else if let ExceptionState::Pending(first) = self.graph.entity(pending.entity).exceptions
        {
            self.resolve_specification(first)?;
        }
        let saved_limit = self.graph.lookup_limit;
        self.graph.lookup_limit = Some(pending.limit);
        // A specification has its declaration's implicit object, independent
        // of any body whose analysis happened to demand the specification.
        let saved_body = self.body_context.take();
        let saved_default = self.default_argument_scope.take();
        let saved_member = self.member_initializer_class.take();
        let saved_allocation_context = self.global_allocation_context;
        self.global_allocation_context = if matches!(pending.mode, SpecificationMode::Ordinary) {
            super::allocation::GlobalAllocationContext::Declaration
        } else {
            super::allocation::GlobalAllocationContext::Instantiation
        };
        let result = (|| {
            self.body_context = self.specification_body(&pending)?;
            if matches!(pending.mode, SpecificationMode::ImplicitDestructor) {
                self.graph
                    .entity(pending.entity)
                    .member_function
                    .ok_or(Error::Invariant(
                        "implicit destructor specification without member",
                    ))
                    .and_then(|member| self.implicit_destructor_nothrow(member.class))
                    .map(|nothrow| {
                        if nothrow {
                            ExceptionSet::None
                        } else {
                            ExceptionSet::All
                        }
                    })
            } else {
                self.exception_set_context(
                    pending.qualifier,
                    pending.scope,
                    true,
                    Some(pending.entity),
                )
            }
        })();
        self.body_context = saved_body;
        self.default_argument_scope = saved_default;
        self.member_initializer_class = saved_member;
        self.graph.lookup_limit = saved_limit;
        self.global_allocation_context = saved_allocation_context;
        let result = result?;
        if pending.first {
            self.graph.entity_mut(pending.entity).exceptions = ExceptionState::Value(result);
        } else {
            let ExceptionState::Value(first) = &self.graph.entity(pending.entity).exceptions else {
                return Err(Error::Invariant("unresolved first exception specification"));
            };
            let incompatible = if matches!(pending.mode, SpecificationMode::Ordinary) {
                *first != result
            } else {
                matches!(first, ExceptionSet::None) != matches!(result, ExceptionSet::None)
            };
            if incompatible {
                return Err(Error::at(
                    pending.location,
                    "redeclaration with an incompatible exception-specification",
                ));
            }
        }
        self.specifications[id.0 as usize].done = true;
        Ok(())
    }
    pub(super) fn record_template_exception(
        &mut self,
        entity: EntityId,
        info: &DeclaratorInfo,
        scope: ScopeId,
        operand: ast::ExprId,
        location: SourceOffset,
        limit: usize,
    ) -> Result<(), Error> {
        if !matches!(self.graph.entity(entity).exceptions, ExceptionState::Absent) {
            if matches!(
                self.graph.entity(entity).exceptions,
                ExceptionState::Value(_)
            ) {
                // C++ has no expression-equivalence key when the first
                // declaration wrote no pending operand. Its facts stay.
                return Ok(());
            }
            let ExceptionState::Template(first) = self.graph.entity(entity).exceptions else {
                return Err(Error::Invariant("unexpected template exception state"));
            };
            let second = super::template_exceptions::TemplateException {
                operand,
                scope,
                parameters: info.parameters,
                // Both prototypes use the first entity's source parameters.
                ty: first.ty,
                limit,
            };
            let earlier = self.exception_prototype(first)?;
            let later = self.exception_prototype(second)?;
            if let Some(first) = self.expression_key(first.operand, earlier)?
                && let Some(second) = self.expression_key(operand, later)?
                && first != second
            {
                return Err(Error::at(
                    location,
                    "redeclaration with an incompatible exception-specification",
                ));
            }
            return Ok(());
        }
        let pattern = super::template_exceptions::TemplateException {
            operand,
            scope,
            parameters: info.parameters,
            ty: info.ty,
            limit,
        };
        self.graph.entity_mut(entity).exceptions = ExceptionState::Template(pattern);
        let id = SpecificationId(
            u32::try_from(self.specifications.len())
                .map_err(|_| Error::Limit("exception specification limit"))?,
        );
        self.specifications.push(PendingSpecification {
            entity,
            qualifier: None,
            scope,
            limit,
            location,
            first: true,
            done: false,
            mode: SpecificationMode::TemplateExpression,
        });
        // Keep the queue's source order shared with ordinary specifications.
        if self.graph.entity(entity).template_head.is_none() {
            // A class template's ordinary member waits for instantiation;
            // its specification's original typed syntax is retained only.
            self.specifications[id.0 as usize].done = true;
        }
        Ok(())
    }
    fn exception_set(
        &mut self,
        qualifier: Option<Qualifier>,
        scope: ScopeId,
    ) -> Result<ExceptionSet, Error> {
        self.exception_set_context(qualifier, scope, true, None)
    }
    fn exception_set_context(
        &mut self,
        qualifier: Option<Qualifier>,
        scope: ScopeId,
        complete: bool,
        owner: Option<EntityId>,
    ) -> Result<ExceptionSet, Error> {
        match qualifier.map(|q| q.kind) {
            None => Ok(ExceptionSet::All),
            Some(QualifierKind::Noexcept(operand)) => {
                let nothrow = if let Some(operand) = operand {
                    self.noexcept_value(operand, scope)?
                } else {
                    true
                };
                Ok(if nothrow {
                    ExceptionSet::None
                } else {
                    ExceptionSet::All
                })
            }
            Some(QualifierKind::Throw(types)) => {
                self.dynamic_exception_set(types, scope, complete, owner)
            }
            _ => Err(Error::Invariant("invalid exception qualifier")),
        }
    }
    pub(super) fn specification_body(
        &mut self,
        pending: &PendingSpecification,
    ) -> Result<Option<super::procedural_statements::BodyContext>, Error> {
        let operand = matches!(pending.mode, SpecificationMode::TemplateExpression)
            || matches!(
                pending.qualifier,
                Some(Qualifier {
                    kind: QualifierKind::Noexcept(Some(_)),
                    ..
                })
            );
        if !self.semantic() && !operand {
            return Ok(None);
        }
        let entity = self.graph.entity(pending.entity);
        let ty = entity.ty.ok_or(Error::Invariant(
            "exception specification without function type",
        ))?;
        let cv = self.graph.types.get(ty).cv;
        let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("exception specification on nonfunction"));
        };
        let home = entity
            .home
            .filter(|&home| self.graph.scope(home).kind == super::graph::ScopeKind::Class);
        let object = home
            .and_then(|home| self.graph.scope(home).owner)
            .filter(|_| !entity.function.is_some_and(|function| function.is_static));
        let mut body = super::procedural_statements::BodyContext::new(result, pending.entity);
        if operand && let Some(object) = object {
            let object = self.graph.entity(object).ty.ok_or(Error::Invariant(
                "exception specification class without type",
            ))?;
            // noexcept_value reads Type::cv, independently of function_cv.
            let object = self.graph.types.qualify(object, cv)?;
            body.object_pointer = Some(self.graph.types.pointer(object)?);
        }
        Ok(Some(body))
    }
    // A specification uses typed occurrences and converted constant evaluation
    // in every view. Typing retains the view's ordinary declaration/body demand.
    pub(super) fn noexcept_value(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let saved = self.evaluating_noexcept;
        self.evaluating_noexcept = true;
        let result = (|| {
            self.exception_operand_type(operand, scope)?;
            let saved = self.unevaluated;
            self.unevaluated = 1;
            let result = self.convert_bool(operand);
            self.unevaluated = saved;
            result?;
            self.with_semantics(|analyzer| analyzer.converted_constant_bool(operand, scope))
        })();
        self.evaluating_noexcept = saved;
        result
    }
    fn dynamic_exception_set(
        &mut self,
        types: crate::syntax::storage::List<ast::TypeId>,
        scope: ScopeId,
        complete: bool,
        owner: Option<EntityId>,
    ) -> Result<ExceptionSet, Error> {
        let mut adjusted = Vec::new();
        for syntax_index in 0..self.tree.type_lists.get(types).len() {
            let syntax = self.tree.type_lists.get(types)[syntax_index];
            let ty = self.type_syntax(syntax, scope)?;
            if let TypeKind::Pack(elements) = &self.graph.types.get(ty).kind {
                let count = elements.len();
                for index in 0..count {
                    let TypeKind::Pack(elements) = &self.graph.types.get(ty).kind else {
                        return Err(Error::Invariant("exception type pack changed"));
                    };
                    let element = self.graph.types.parameter(elements[index])?;
                    adjusted.push(self.graph.types.canonical_function(element)?);
                }
                continue;
            }
            let location = self.tree.types.get(syntax).location;
            if matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Reference {
                    kind: RefKind::Rvalue,
                    ..
                }
            ) {
                return Err(Error::at(
                    location,
                    "an rvalue reference type in an exception-specification",
                ));
            }
            let ty = self.graph.types.parameter(ty)?;
            if complete {
                self.check_complete_throw_type(ty, location)?;
            }
            adjusted.push(self.graph.types.canonical_function(ty)?);
        }
        if let Some(owner) = owner {
            for &ty in &adjusted {
                if !self.graph.dependent_type(ty) {
                    let object = match self.graph.types.get(ty).kind {
                        TypeKind::Reference { element, .. } => element,
                        _ => ty,
                    };
                    self.prepare_type_information(object)?;
                }
            }
            self.graph
                .source_exceptions
                .record_specification(owner, adjusted.clone());
        }
        adjusted.sort_unstable();
        adjusted.dedup();
        Ok(if adjusted.is_empty() {
            ExceptionSet::None
        } else {
            ExceptionSet::Dynamic(adjusted)
        })
    }
    fn record_template_throw(
        &mut self,
        entity: EntityId,
        qualifier: Option<Qualifier>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        match &self.graph.entity(entity).exceptions {
            ExceptionState::Value(ExceptionSet::None) => {
                return Err(Error::at(
                    location,
                    "redeclaration with an incompatible exception-specification",
                ));
            }
            ExceptionState::Template(_) | ExceptionState::Value(_) => return Ok(()),
            ExceptionState::Absent => (),
            ExceptionState::Pending(_) | ExceptionState::Evaluating => {
                return Err(Error::Invariant(
                    "pending ordinary specification under template head",
                ));
            }
        }
        // A class template's ordinary member waits for its instantiation;
        // a function template records its written adjusted types immediately.
        let set = if self.graph.entity(entity).template_head.is_some() {
            self.exception_set_context(qualifier, scope, false, Some(entity))?
        } else {
            ExceptionSet::All
        };
        self.graph.entity_mut(entity).exceptions = ExceptionState::Value(set);
        Ok(())
    }
    fn check_complete_throw_type(
        &mut self,
        mut ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let reference = matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. });
        if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let pointer = !reference && matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(_));
        if pointer && let TypeKind::Pointer(element) = self.graph.types.get(ty).kind {
            ty = element;
        }
        if let TypeKind::Class(entity) = self.graph.types.get(ty).kind
            && !self.complete_class(entity)?
        {
            return Err(Error::at(
                location,
                "an incomplete class type in an exception-specification (15.4p2)",
            ));
        }
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Scalar(super::types::Scalar::Void)
        ) && !reference
            && !pointer
        {
            return Err(Error::at(
                location,
                "void in an exception-specification (15.4p2)",
            ));
        }
        Ok(())
    }
    pub(super) fn finish_class_exceptions(&mut self) -> Result<(), Error> {
        let pending = std::mem::take(&mut self.pending_throws);
        for id in pending {
            self.resolve_specification(id)?;
        }
        self.check_exception_overrides(false)
    }
    pub(super) fn finish_unit_exceptions(&mut self) -> Result<(), Error> {
        for index in 0..self.specifications.len() {
            self.resolve_specification(SpecificationId(
                u32::try_from(index).map_err(|_| Error::Limit("exception specification limit"))?,
            ))?;
        }
        self.check_exception_overrides(true)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TargetUnwind {
    MayThrow,
    NoThrow,
}
impl Analyzer<'_> {
    // A declaration's exception specification is independent of canonical function type identity.
    pub(super) fn note_nothrow_target(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        mut declarator: Option<ast::DeclaratorId>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let ty = match self.graph.types.get(ty).kind {
            TypeKind::Pointer(element)
            | TypeKind::Reference { element, .. }
            | TypeKind::MemberPointer { element, .. } => element,
            _ => ty,
        };
        if !matches!(self.graph.types.get(ty).kind, TypeKind::Function { .. }) {
            return Ok(());
        }
        let mut function = None;
        let mut derivations = 0_usize;
        while let Some(id) = declarator {
            let node = self.tree.declarators.get(id);
            derivations = derivations
                .checked_add(self.tree.pointers.get(node.pointers).len())
                .ok_or(Error::Limit("declarator derivation limit"))?;
            for &suffix in self.tree.suffixes.get(node.suffixes).iter().rev() {
                function = if let ast::Suffix::Function { qualifiers, .. } = suffix {
                    Some(qualifiers)
                } else {
                    None
                };
                derivations = 0;
            }
            declarator = node.nested;
        }
        let Some(function) = function.filter(|_| derivations <= 1) else {
            return Ok(());
        };
        for syntax_index in 0..self.tree.qualifiers.get(function).len() {
            let qualifier = self.tree.qualifiers.get(function)[syntax_index];
            let nothrow = match qualifier.kind {
                QualifierKind::Throw(types) if types.is_empty() => true,
                QualifierKind::Noexcept(None) => true,
                QualifierKind::Noexcept(Some(_)) if self.template_depth == 0 => matches!(
                    self.exception_set(Some(qualifier), scope)?,
                    ExceptionSet::None
                ),
                _ => false,
            };
            if nothrow {
                self.graph.entity_mut(entity).target_unwind = TargetUnwind::NoThrow;
            }
        }
        Ok(())
    }
    fn designated_entity(&self, expr: ast::ExprId) -> Result<Option<EntityId>, Error> {
        let facts = self.graph.expression_facts.get(expr)?;
        let binding = facts.function.or(facts.member).or_else(|| {
            if let ExprKind::Id(name) = self.tree.expressions.get(expr).kind {
                self.graph.resolved_name(name)
            } else {
                None
            }
        });
        Ok(binding.map(|binding| self.graph.binding(binding).entity))
    }
    pub(super) fn callee_target_entity(
        &self,
        callee: ast::ExprId,
    ) -> Result<Option<EntityId>, Error> {
        use crate::lex::pp_lexer::Punctuator as P;
        let mut expr = self.graph.expression_facts.root(callee);
        if let ExprKind::Unary { op, operand } = self.tree.expressions.get(expr).kind
            && op.token.as_punctuator() == Some(P::Multiply)
        {
            expr = self.graph.expression_facts.root(operand);
        }
        if let ExprKind::Binary { op, right, .. } = self.tree.expressions.get(expr).kind
            && matches!(op.token.as_punctuator(), Some(P::DotStar | P::ArrowStar))
        {
            expr = self.graph.expression_facts.root(right);
        }
        self.designated_entity(expr)
    }
    pub(super) fn check_nothrow_initializer(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
    ) -> Result<(), Error> {
        use crate::lex::pp_lexer::Punctuator as P;
        if self.graph.entity(entity).target_unwind != TargetUnwind::NoThrow {
            return Ok(());
        }
        let Some(expr) = self.single_initializer(initializer) else {
            return Ok(());
        };
        let mut expr = self.graph.expression_facts.root(expr);
        if let ExprKind::Unary { op, operand } = self.tree.expressions.get(expr).kind
            && op.token.as_punctuator() == Some(P::BitAnd)
        {
            expr = self.graph.expression_facts.root(operand);
        }
        let Some(source) = self.designated_entity(expr)? else {
            return Ok(());
        };
        let allows = if self.graph.entity(source).function.is_some() {
            if let ExceptionState::Pending(id) = self.graph.entity(source).exceptions {
                self.resolve_specification(id)?;
            }
            !matches!(
                self.graph.entity(source).exceptions,
                ExceptionState::Value(ExceptionSet::None)
            )
        } else {
            let source_type = self.graph.entity(source).ty.ok_or(Error::Invariant(
                "function pointer initializer without type",
            ))?;
            let source_type = match self.graph.types.get(source_type).kind {
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::MemberPointer { element, .. } => element,
                _ => source_type,
            };
            matches!(
                self.graph.types.get(source_type).kind,
                TypeKind::Function { .. }
            ) && self.graph.entity(source).target_unwind != TargetUnwind::NoThrow
        };
        if allows {
            return Err(Error::at(
                initializer.location,
                "a function pointer or reference that allows no exceptions initialized with one that does",
            ));
        }
        Ok(())
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
    fn member_pattern_comparison_preserves_the_instance_specification() {
        for (specification, compatible) in [
            ("", false),
            ("noexcept", false),
            ("noexcept(sizeof(T)==8)", true),
        ] {
            let source = SourceBuffer::new(
                format!(
                    "template<class T>struct X{{static void operator delete(void*)noexcept(sizeof(T)==4);}};\
                     template<class T>void X<T>::operator delete(void*){specification}{{}}"
                )
                .into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"member-pattern").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let name = cursor.intern_name(b"X").unwrap();
            let expressions = tree.expressions.mark();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let global = analyzer.graph.global;
            analyzer
                .declaration(analyzer.tree.decl_lists.get(declarations)[0], global)
                .unwrap();
            let class = analyzer
                .graph
                .own(global, name, super::super::lookup::Filter::Ordinary)
                .unwrap();
            let class = analyzer.graph.binding(class).entity;
            let scope = analyzer.graph.entity(class).scope.unwrap();
            let key = super::super::function_names::SpecialName::Operator {
                token: crate::lex::vocabulary::SimpleToken::keyword(Keyword::Delete),
                array: false,
            };
            let binding = analyzer
                .graph
                .functions_at(scope, super::super::function_names::ValueKey::Special(key))
                .next()
                .unwrap();
            let function = analyzer.graph.binding(binding).entity;
            let ExceptionState::Template(first) = analyzer.graph.entity(function).exceptions else {
                panic!("class member lost its deferred specification");
            };
            let result =
                analyzer.declaration(analyzer.tree.decl_lists.get(declarations)[1], global);
            if compatible {
                result.unwrap();
            } else {
                assert!(matches!(result, Err(Error::Diagnostic { message, .. })
                    if message == "redeclaration with an incompatible exception-specification"));
            }
            assert!(matches!(analyzer.graph.entity(function).exceptions,
                ExceptionState::Template(saved) if saved.operand == first.operand));
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
            assert_eq!(analyzer.tree.expressions.mark(), expressions);
            assert_eq!(analyzer.template_depth, 0);
            assert!(analyzer.graph.lookup_limit.is_none());
        }
    }
    #[test]
    fn indirect_unwind_facts_are_independent_of_canonical_pointer_type() {
        let source = SourceBuffer::new(b"int f(int x)noexcept{return x;}int(*p)(int)noexcept=f;int(*q)(int)=f;extern int(*late)(int);int main(){return p(1)+q(2)+late(3);}extern int(*late)(int)noexcept;".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"unwind-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let mut entity = |name: &[u8]| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                        .unwrap(),
                )
                .entity
        };
        let p = graph.entity(entity(b"p"));
        let q = graph.entity(entity(b"q"));
        assert_eq!(p.ty, q.ty);
        assert!(p.target_unwind == TargetUnwind::NoThrow);
        assert!(q.target_unwind == TargetUnwind::MayThrow);
        let declaration = graph.entity(entity(b"main")).function_definition.unwrap();
        let ast::DeclKind::Function { body, .. } = tree.declarations.get(declaration).kind else {
            panic!("main is not a function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("unparsed main");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("main has no compound body");
        };
        let ast::StmtKind::Return(Some(expr)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("main has no return");
        };
        let ExprKind::Binary {
            left, right: late, ..
        } = tree.expressions.get(expr).kind
        else {
            panic!("return has no outer operands");
        };
        let ExprKind::Binary { left, right, .. } = tree.expressions.get(left).kind else {
            panic!("return has no inner operands");
        };
        let boundaries: Vec<_> = [left, right, late]
            .into_iter()
            .filter_map(|id| match graph.lower_call(id).unwrap() {
                super::super::lowering::CallTarget::Indirect(ty, nothrow) => Some((ty, nothrow)),
                super::super::lowering::CallTarget::Direct(_) => None,
            })
            .collect();
        assert_eq!(boundaries.len(), 3);
        assert_eq!(boundaries[0].0, boundaries[1].0);
        assert!(boundaries[0].1);
        assert!(!boundaries[1].1);
        assert_eq!(boundaries[0].0, boundaries[2].0);
        assert!(boundaries[2].1);
    }
}

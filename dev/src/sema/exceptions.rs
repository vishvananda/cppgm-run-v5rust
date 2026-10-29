// Translated from sema/analyzer_exceptions.cpp and analyzer_exception_sets.cpp. See NOTICE.
//! Exception specifications belong to declarations, never canonical function types.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    graph::{EntityId, ScopeId},
    types::{Cv, RefKind, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind, Qualifier, QualifierKind},
};

#[derive(Clone, Copy, Debug)]
pub(super) struct SpecificationId(u32);
impl SpecificationId {
    fn for_index(index: usize) -> Result<Self, Error> {
        u32::try_from(index)
            .map(Self)
            .map_err(|_| Error::Limit("exception specification limit"))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ExceptionSet {
    All,
    None,
    Dynamic(Vec<TypeId>),
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SpecificationOrigin {
    #[default]
    Omitted,
    Written,
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
#[derive(Clone, Copy, PartialEq, Eq)]
enum SpecificationSchedule {
    Unit,
    Demand,
}
#[derive(Clone, Copy)]
enum ThrowListUse {
    Declaration(Option<EntityId>),
    Comparison,
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
    schedule: SpecificationSchedule,
}
impl PendingSpecification {
    pub(super) fn owner(self) -> EntityId {
        self.entity
    }
}

#[derive(Default)]
pub(super) struct Specifications {
    records: Vec<PendingSpecification>,
    unit: UnitProgress,
}
enum UnitProgress {
    Open(usize),
    Closed,
}
impl Default for UnitProgress {
    fn default() -> Self {
        Self::Open(0)
    }
}
impl Specifications {
    pub(super) fn unit_closed(&self) -> bool {
        matches!(self.unit, UnitProgress::Closed)
    }
    fn close_unit(&mut self) {
        self.unit = UnitProgress::Closed;
    }
    #[cfg(feature = "telemetry")]
    pub(super) fn storage_bytes(&self) -> usize {
        self.records
            .capacity()
            .saturating_mul(std::mem::size_of::<PendingSpecification>())
    }
    fn push(&mut self, pending: PendingSpecification) -> Result<SpecificationId, Error> {
        let id = SpecificationId::for_index(self.records.len())?;
        self.records.push(pending);
        Ok(id)
    }
    fn get(&self, id: SpecificationId) -> PendingSpecification {
        self.records[id.0 as usize]
    }
    fn complete(&mut self, id: SpecificationId) {
        self.records[id.0 as usize].done = true;
    }
    fn unit_head(&self) -> Result<Option<(SpecificationId, PendingSpecification)>, Error> {
        let UnitProgress::Open(index) = self.unit else {
            return Ok(None);
        };
        let Some(pending) = self.records.get(index).copied() else {
            return Ok(None);
        };
        Ok(Some((SpecificationId::for_index(index)?, pending)))
    }
    fn advance_unit(&mut self) {
        if let UnitProgress::Open(index) = &mut self.unit {
            *index += 1;
        }
    }
}

impl super::graph::Graph {
    pub(crate) fn lower_demand_function_nothrow(
        &mut self,
        tree: &mut ast::Tree,
        cursor: &mut crate::syntax::cursor::Cursor<'_>,
        budget: crate::support::stack::StackBudget,
        stack_base: usize,
        function: EntityId,
    ) -> Result<bool, Error> {
        let function = self.lower_function_entity(function);
        let entity = self.entity(function);
        if entity.constructor_exceptions
            == super::construction::ConstructorExceptionSource::SubobjectPlan
        {
            if let Some(constructor) = entity.constructor {
                return Ok(self.constructors.get(constructor).nothrow);
            }
        } else {
            let pending = match entity.exceptions {
                ExceptionState::Pending(id) => !matches!(
                    self.specifications.get(id).qualifier,
                    Some(Qualifier { kind: QualifierKind::Throw(types), .. }) if !types.is_empty()
                ),
                ExceptionState::Template(_) => self.function_templates.primary(function).is_some(),
                _ => false,
            };
            if !pending {
                return Ok(matches!(
                    entity.exceptions,
                    ExceptionState::Value(ExceptionSet::None)
                ));
            }
        }
        let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(self))?;
        analyzer.set_lowering();
        analyzer.stack_base = stack_base;
        let result = analyzer.function_may_throw(function).map(|throws| !throws);
        *self = analyzer.graph;
        result
    }
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
        let mut substitution = self.substitution(self.graph.templates.depth(head), &arguments);
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
            if matches!(self.graph.specifications.get(id).qualifier, Some(Qualifier { kind: QualifierKind::Noexcept(Some(_)), .. })))
    }

    pub(super) fn virtual_nothrow(&mut self, function: EntityId) -> Result<bool, Error> {
        self.function_may_throw(function)
            .map(|may_throw| !may_throw)
    }

    pub(super) fn pending_dynamic_throw(&self, function: EntityId) -> bool {
        matches!(self.graph.entity(function).exceptions, ExceptionState::Pending(id)
            if matches!(self.graph.specifications.get(id).qualifier,
                Some(Qualifier { kind: QualifierKind::Throw(types), .. }) if !types.is_empty()))
    }

    pub(super) fn function_may_throw(&mut self, entity: EntityId) -> Result<bool, Error> {
        if self.graph.entity(entity).builtin.is_some() {
            return Ok(false);
        }
        if self.pending_dynamic_throw(entity) {
            // The written nonempty list already establishes the boolean.
            // Class completion still owns publication of its adjusted types.
            return Ok(true);
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
    fn function_exception_qualifier(&self, info: &DeclaratorInfo) -> Option<Qualifier> {
        let mut qualifier =
            info.qualifiers.and_then(|list| {
                self.tree.qualifiers.get(list).iter().copied().find(|q| {
                    matches!(q.kind, QualifierKind::Throw(_) | QualifierKind::Noexcept(_))
                })
            })?;
        // NoexceptOperand excludes parenthesized literal true. It needs
        // neither a prototype scope nor a pending constant evaluation.
        if let QualifierKind::Noexcept(Some(operand)) = qualifier.kind
            && self.true_operand(operand)
        {
            qualifier.kind = QualifierKind::Noexcept(None);
        }
        Some(qualifier)
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
        let mut qualifier = self.function_exception_qualifier(info);
        let first = matches!(self.graph.entity(entity).exceptions, ExceptionState::Absent);
        if first {
            self.graph.entity_mut(entity).exception_origin = if qualifier.is_some() {
                SpecificationOrigin::Written
            } else {
                SpecificationOrigin::Omitted
            };
        }
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
            && (operand.is_some() || self.pending_virtual_noexcept(entity))
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
        let id = self.graph.specifications.push(PendingSpecification {
            entity,
            qualifier,
            scope,
            limit,
            location,
            first,
            done: false,
            schedule: if self.graph.entity(entity).template_head.is_none()
                && self.graph.class_templates.definitions.active_depth() == 0
            {
                SpecificationSchedule::Unit
            } else {
                SpecificationSchedule::Demand
            },
            mode: if implicit_destructor && self.semantic() {
                SpecificationMode::ImplicitDestructor
            } else if self.template_depth == 0 && self.graph.entity(entity).template_head.is_none()
            {
                SpecificationMode::Ordinary
            } else {
                SpecificationMode::TemplateCompatibility
            },
        })?;
        if first {
            self.graph.entity_mut(entity).exceptions = ExceptionState::Pending(id);
        }
        let member_throw = self.defers_member_throw(entity, info)
            && qualifier.is_some_and(
                |q| matches!(q.kind, QualifierKind::Throw(types) if !types.is_empty()),
            );
        if member_throw {
            self.graph.analysis.pending_throws.push(id);
        }
        let literal_true = operand.is_some_and(|expr| self.true_operand(expr));
        if !first || (!member_throw && (operand.is_none() || literal_true)) {
            self.resolve_specification(id)?;
        }
        Ok(())
    }
    fn defers_member_throw(&self, entity: EntityId, info: &DeclaratorInfo) -> bool {
        let declaration = self.graph.entity(entity);
        self.class_depth != 0
            && declaration.home == Some(info.lookup)
            && declaration
                .home
                .is_some_and(|home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
            && declaration.template_head.is_none()
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
        let pending = self.graph.specifications.get(id);
        if pending.done {
            return Ok(());
        }
        if matches!(pending.mode, SpecificationMode::TemplateExpression) {
            let ExceptionState::Template(pattern) = self.graph.entity(pending.entity).exceptions
            else {
                return Err(Error::Invariant("missing template exception pattern"));
            };
            self.check_template_exception(pattern, pending)?;
            self.graph.specifications.complete(id);
            return Ok(());
        }
        if pending.first {
            self.graph.entity_mut(pending.entity).exceptions = ExceptionState::Evaluating;
        }
        let saved_limit = self.graph.lookup_limit;
        self.graph.lookup_limit = Some(pending.limit);
        // A specification has its declaration's implicit object, independent
        // of any body whose analysis happened to demand the specification.
        let saved_body = self.body_context.take();
        let saved_default = self.default_argument_scope.take();
        let saved_member = self.member_initializer_class.take();
        let saved_allocation_context = self.global_allocation_context;
        self.global_allocation_context = if pending.schedule == SpecificationSchedule::Unit {
            super::allocation::GlobalAllocationContext::Declaration
        } else {
            super::allocation::GlobalAllocationContext::Instantiation
        };
        let result = (|| {
            if !matches!(
                pending.qualifier,
                Some(Qualifier {
                    kind: QualifierKind::Noexcept(Some(_)),
                    ..
                })
            ) {
                self.body_context = self.specification_body(&pending)?;
            }
            self.evaluate_specification(pending)
        })();
        self.body_context = saved_body;
        self.default_argument_scope = saved_default;
        self.member_initializer_class = saved_member;
        self.graph.lookup_limit = saved_limit;
        self.global_allocation_context = saved_allocation_context;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                if pending.first {
                    self.graph.entity_mut(pending.entity).exceptions = ExceptionState::Pending(id);
                }
                return Err(error);
            }
        };
        if let Some(value) = result {
            self.graph.entity_mut(pending.entity).exceptions = ExceptionState::Value(value);
        }
        self.graph.specifications.complete(id);
        Ok(())
    }
    fn specification_nonthrowing(&mut self, pending: PendingSpecification) -> Result<bool, Error> {
        if matches!(pending.mode, SpecificationMode::ImplicitDestructor) {
            let member =
                self.graph
                    .entity(pending.entity)
                    .member_function
                    .ok_or(Error::Invariant(
                        "implicit destructor specification without member",
                    ))?;
            return self.implicit_destructor_nothrow(member.class);
        }
        match pending.qualifier.map(|qualifier| qualifier.kind) {
            Some(QualifierKind::Noexcept(Some(operand))) => {
                self.noexcept_value(Some(pending.entity), operand, pending.scope)
            }
            Some(QualifierKind::Noexcept(None)) => Ok(true),
            Some(QualifierKind::Throw(types)) => Ok(types.is_empty()),
            None => Ok(false),
            _ => Err(Error::Invariant("invalid exception qualifier")),
        }
    }
    fn specification_set(
        &mut self,
        pending: PendingSpecification,
        nonthrowing: bool,
    ) -> Result<ExceptionSet, Error> {
        if let Some(Qualifier {
            kind: QualifierKind::Throw(types),
            ..
        }) = pending.qualifier
        {
            self.dynamic_exception_set(
                types,
                pending.scope,
                true,
                if pending.first {
                    ThrowListUse::Declaration(Some(pending.entity))
                } else {
                    ThrowListUse::Comparison
                },
            )
        } else {
            Ok(if nonthrowing {
                ExceptionSet::None
            } else {
                ExceptionSet::All
            })
        }
    }
    fn evaluate_specification(
        &mut self,
        pending: PendingSpecification,
    ) -> Result<Option<ExceptionSet>, Error> {
        // Redeclarations evaluate their new boolean before demanding the first
        // specification. Later finite-set names are checked only afterward.
        let nonthrowing = self.specification_nonthrowing(pending)?;
        if pending.first {
            return self.specification_set(pending, nonthrowing).map(Some);
        }
        let first_nonthrowing = !self.function_may_throw(pending.entity)?;
        if first_nonthrowing != nonthrowing {
            return Err(Error::at(
                pending.location,
                "redeclaration with an incompatible exception-specification",
            ));
        }
        if nonthrowing
            || !matches!(pending.mode, SpecificationMode::Ordinary)
            || self.graph.entity(pending.entity).template_head.is_some()
            || self
                .graph
                .function_templates
                .primary(pending.entity)
                .is_some()
        {
            return Ok(None);
        }
        let first = match self.graph.source_exceptions.specification(pending.entity) {
            Some(types) => {
                let mut types = types.to_vec();
                types.sort_unstable();
                types.dedup();
                ExceptionSet::Dynamic(types)
            }
            // A pending dynamic list is unpublished until class completion.
            None => ExceptionSet::All,
        };
        let later = self.specification_set(pending, nonthrowing)?;
        if first != later && !self.bad_alloc_redeclaration(pending.entity, &later) {
            return Err(Error::at(
                pending.location,
                "redeclaration with an incompatible exception-specification",
            ));
        }
        Ok(None)
    }
    fn bad_alloc_redeclaration(&self, entity: EntityId, later: &ExceptionSet) -> bool {
        if self.graph.source_exceptions.specification(entity).is_some()
            || !self
                .graph
                .entity(entity)
                .function
                .filter(|function| function.allocation.is_some())
                .and_then(|function| function.special)
                .is_some_and(|special| {
                    matches!(special,
                    super::function_names::SpecialName::Operator { token, .. }
                        if token.as_keyword() == Some(Keyword::New))
                })
        {
            return false;
        }
        let ExceptionSet::Dynamic(types) = later else {
            return false;
        };
        let [ty] = types.as_slice() else { return false };
        let Some(class) = self.graph.class_of(*ty) else {
            return false;
        };
        let Some(name) = self.graph.entity(class).name else {
            return false;
        };
        self.identifiers.bytes(name) == b"bad_alloc"
            && self
                .identifiers
                .find(b"std")
                .is_some_and(|std| self.in_std_namespace(class, std))
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
        let id = self.graph.specifications.push(PendingSpecification {
            entity,
            qualifier: None,
            scope,
            limit,
            location,
            first: true,
            done: false,
            mode: SpecificationMode::TemplateExpression,
            schedule: SpecificationSchedule::Demand,
        })?;
        // Keep the queue's source order shared with ordinary specifications.
        if self.graph.entity(entity).template_head.is_none() {
            // A class template's ordinary member waits for instantiation;
            // its specification's original typed syntax is retained only.
            self.graph.specifications.complete(id);
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
                    self.noexcept_value(owner, operand, scope)?
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
                self.dynamic_exception_set(types, scope, complete, ThrowListUse::Declaration(owner))
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
        self.exception_body(Some(pending.entity), operand).map(Some)
    }
    fn exception_body(
        &mut self,
        owner: Option<EntityId>,
        implicit_object: bool,
    ) -> Result<super::procedural_statements::BodyContext, Error> {
        let (result, object, cv) = if let Some(owner) = owner {
            let entity = self.graph.entity(owner);
            let ty = entity.ty.ok_or(Error::Invariant(
                "exception specification without function type",
            ))?;
            let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
                return Err(Error::Invariant("exception specification on nonfunction"));
            };
            let home = entity
                .home
                .filter(|&home| self.graph.scope(home).kind == super::graph::ScopeKind::Class);
            let object = home
                .and_then(|home| self.graph.scope(home).owner)
                .filter(|_| {
                    implicit_object && !entity.function.is_some_and(|function| function.is_static)
                });
            let constructor = entity
                .member_function
                .is_some_and(|member| member.kind == super::graph::MemberKind::Constructor);
            // The frozen specification reads the outer Type::cv; constructors
            // have an unqualified object, independently of function_cv.
            let cv = if constructor {
                Cv::default()
            } else {
                self.graph.types.get(ty).cv
            };
            (result, object, cv)
        } else {
            (
                self.graph.types.scalar(super::types::Scalar::Void)?,
                None,
                Cv::default(),
            )
        };
        let mut body = super::procedural_statements::BodyContext::new(result, None);
        if let Some(object) = object {
            let object = self.graph.entity(object).ty.ok_or(Error::Invariant(
                "exception specification class without type",
            ))?;
            let object = self.graph.types.qualify(object, cv)?;
            body.object_pointer = Some(self.graph.types.pointer(object)?);
        }
        Ok(body)
    }
    // A required specification has its declaration's object and root demand
    // context, independent of a body or substitution that demanded it.
    pub(super) fn noexcept_value(
        &mut self,
        owner: Option<EntityId>,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let body = self.body_context.take();
        let default = self.default_argument_scope.take();
        let member = self.member_initializer_class.take();
        let enclosing = std::mem::take(&mut self.enclosing_bodies);
        let capture = self.capture_scan.take();
        let unevaluated = std::mem::replace(&mut self.unevaluated, 0);
        let deferring = std::mem::replace(&mut self.type_queries.deferring, 0);
        let evaluating = std::mem::replace(&mut self.evaluating_noexcept, true);
        let lookup = self.lookup_location;
        let result = self.without_soft_substitution(|analyzer| {
            analyzer.body_context = Some(analyzer.exception_body(owner, true)?);
            analyzer.exception_operand_type(operand, scope)?;
            analyzer.check_resolved_function(operand)?;
            analyzer.convert_bool(operand)?;
            if analyzer
                .graph
                .analysis
                .substitution
                .current()
                .failure
                .is_some()
            {
                return Ok(false);
            }
            analyzer.with_semantics(|analyzer| analyzer.converted_constant_bool(operand, scope))
        });
        self.body_context = body;
        self.default_argument_scope = default;
        self.member_initializer_class = member;
        self.enclosing_bodies = enclosing;
        self.capture_scan = capture;
        self.unevaluated = unevaluated;
        self.type_queries.deferring = deferring;
        self.evaluating_noexcept = evaluating;
        self.lookup_location = lookup;
        result.map_err(Error::required_instantiation)
    }
    pub(super) fn closure_noexcept_value(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        // A lambda's declarator is evaluated immediately in its surrounding
        // body, before the call operator's body or its invoker exists.
        let installed = self.semantic() && self.body_context.is_none();
        if installed {
            self.body_context = Some(self.exception_body(None, false)?);
        }
        let checked = (|| {
            if self.semantic() {
                self.typed_expression(operand, scope)?;
                self.check_resolved_function(operand)?;
                self.convert_bool(operand)?;
            }
            Ok(())
        })();
        if installed {
            self.body_context = None;
        }
        checked.map_err(Error::required_instantiation)?;
        if self.semantic() {
            self.converted_constant_bool(operand, scope)
        } else {
            self.constant_value(operand, scope, true)?
                .map(|value| value.bits != 0)
                .ok_or(Error::Invariant("closure noexcept without constant"))
        }
        .map_err(Error::required_instantiation)
    }

    fn dynamic_exception_set(
        &mut self,
        types: crate::syntax::storage::List<ast::TypeId>,
        scope: ScopeId,
        complete: bool,
        usage: ThrowListUse,
    ) -> Result<ExceptionSet, Error> {
        let mut adjusted = Vec::new();
        for syntax_index in 0..self.tree.type_lists.get(types).len() {
            let syntax = self.tree.type_lists.get(types)[syntax_index];
            let ty = self.type_syntax(syntax, scope)?;
            if matches!(usage, ThrowListUse::Declaration(_))
                && let TypeKind::Pack(elements) = &self.graph.types.get(ty).kind
            {
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
            if matches!(usage, ThrowListUse::Declaration(_))
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Reference {
                        kind: RefKind::Rvalue,
                        ..
                    }
                )
            {
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
        if let ThrowListUse::Declaration(Some(owner)) = usage {
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
        Ok(if types.is_empty() {
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
    pub(super) fn finish_class_exceptions(
        &mut self,
        first_throw: usize,
        first_override: usize,
    ) -> Result<(), Error> {
        let mut index = first_throw;
        while index < self.graph.analysis.pending_throws.len() {
            self.resolve_specification(self.graph.analysis.pending_throws[index])?;
            index += 1;
        }
        self.graph.analysis.pending_throws.truncate(first_throw);
        self.check_exception_overrides(first_override, false)
    }
    pub(super) fn finish_unit_exceptions(&mut self) -> Result<(), Error> {
        if self.graph.specifications.unit_closed() {
            return Ok(());
        }
        self.check_pending_nonthrowing_overrides()?;
        self.check_exception_overrides(0, true)?;
        // Resolving a specification can append further declarations. Ordinary
        // members of class-template instances remain demand-owned throughout.
        while let Some((id, pending)) = self.graph.specifications.unit_head()? {
            if pending.schedule == SpecificationSchedule::Unit
                || matches!(pending.mode, SpecificationMode::TemplateExpression)
            {
                self.resolve_specification(id)?;
            }
            self.graph.specifications.advance_unit();
        }
        self.graph.specifications.close_unit();
        Ok(())
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
            self.function_may_throw(source)?
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
    fn resumed_analysis_keeps_demand_owned_specifications_and_their_scope() {
        use super::super::lookup::Filter;
        let source = SourceBuffer::new(
            b"template<class T>struct Lazy{void f()noexcept(sizeof(T)>0);};Lazy<int>value;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"exception-store-owner").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let value = cursor.intern_name(b"value").unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let mut graph = analyzer.analyze(declarations).unwrap();
        let binding = graph.own(graph.global, value, Filter::Ordinary).unwrap();
        let class = graph.class_of(graph.binding(binding).ty.unwrap()).unwrap();
        let scope = graph.entity(class).scope.unwrap();
        let function = graph
            .binding(graph.own(scope, f, Filter::Ordinary).unwrap())
            .entity;
        assert!(matches!(
            graph.entity(function).exceptions,
            ExceptionState::Pending(_)
        ));
        let mut resumed =
            Analyzer::with_graph(tree, cursor, StackBudget::default(), Some(&mut graph)).unwrap();
        resumed.set_lowering();
        let marker = 0u8;
        resumed.stack_base = std::ptr::from_ref(&marker).addr();
        assert!(!resumed.function_may_throw(function).unwrap());
        let work = resumed.graph.work().constant_evaluations;
        assert!(!resumed.function_may_throw(function).unwrap());
        assert_eq!(resumed.graph.work().constant_evaluations, work);
        assert!(matches!(
            resumed.graph.entity(function).exceptions,
            ExceptionState::Value(ExceptionSet::None)
        ));
    }

    #[test]
    fn lowering_specification_failure_restores_the_graph_and_closed_unit_pass() {
        use super::super::lookup::Filter;
        let source = SourceBuffer::new(
            b"struct K{};template<class T>struct Lazy{void bad()noexcept(T::missing);\
              void good()noexcept(sizeof(T)>0);};Lazy<K>value;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"lower-exception-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let value = cursor.intern_name(b"value").unwrap();
        let bad_name = cursor.intern_name(b"bad").unwrap();
        let good_name = cursor.intern_name(b"good").unwrap();
        let budget = StackBudget::default();
        let mut analyzer = Analyzer::new(tree, cursor, budget).unwrap();
        analyzer.set_lowering();
        let mut graph = analyzer.analyze(declarations).unwrap();
        let binding = graph.own(graph.global, value, Filter::Ordinary).unwrap();
        let class = graph.class_of(graph.binding(binding).ty.unwrap()).unwrap();
        let scope = graph.entity(class).scope.unwrap();
        let bad = graph
            .binding(graph.own(scope, bad_name, Filter::Ordinary).unwrap())
            .entity;
        let good = graph
            .binding(graph.own(scope, good_name, Filter::Ordinary).unwrap())
            .entity;
        let marker = 0u8;
        let stack_base = std::ptr::from_ref(&marker).addr();
        let error = graph
            .lower_demand_function_nothrow(tree, cursor, budget, stack_base, bad)
            .unwrap_err();
        assert!(matches!(error, Error::InstantiationDiagnostic { .. }));
        assert!(graph.specifications.unit_closed());
        assert!(matches!(
            graph.entity(bad).exceptions,
            ExceptionState::Pending(_)
        ));
        assert!(
            graph
                .lower_demand_function_nothrow(tree, cursor, budget, stack_base, good)
                .unwrap()
        );
        let evaluations = graph.work().constant_evaluations;
        assert!(
            graph
                .lower_demand_function_nothrow(tree, cursor, budget, stack_base, good)
                .unwrap()
        );
        assert_eq!(graph.work().constant_evaluations, evaluations);
        assert!(graph.specifications.unit_closed());
    }

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

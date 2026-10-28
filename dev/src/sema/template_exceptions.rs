// Translated from analyzer_exceptions.cpp::check_fixed_template_noexcept and
// its OwnNames probes. See NOTICE.
//! Retain a template specification's typed syntax and declaration environment.
//! Fixed operands are checked once; dependent operands wait for instantiation.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, EntityId, ScopeId, ScopeKind},
    types::TypeId,
};
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    syntax::{
        ast::{self, ExprKind, NameKind, SpecifierKind},
        storage::List,
    },
};

#[derive(Clone, Copy, Debug)]
pub(super) struct TemplateException {
    pub operand: ast::ExprId,
    pub scope: ScopeId,
    pub parameters: Option<ast::ParameterClause>,
    pub ty: TypeId,
    pub limit: usize,
}

#[derive(Clone, Copy)]
enum OwnProbe {
    Expression(ast::ExprId),
    Name(ast::Name),
    Type(ast::TypeId),
    Specifiers(List<ast::Specifier>, bool),
}
impl Analyzer<'_> {
    pub(super) fn resolve_specialization_exception(
        &mut self,
        entity: EntityId,
    ) -> Result<(), Error> {
        use super::exceptions::{ExceptionSet, ExceptionState};
        let ExceptionState::Template(specification) = self.graph.entity(entity).exceptions else {
            return Ok(());
        };
        // Like C++ noexcept_pending, recursive queries see the conservative
        // throwing value while this specialization's operand is evaluated.
        self.graph.entity_mut(entity).exceptions = ExceptionState::Evaluating;
        let result = (|analyzer: &mut Self| {
            let prototype_type =
                analyzer.specialization_object_signature(entity, specification.ty)?;
            let primary = analyzer
                .graph
                .function_templates
                .primary(entity)
                .ok_or(Error::Invariant("exception without function template"))?;
            let head = analyzer
                .graph
                .entity(analyzer.graph.binding(primary).entity)
                .template_head
                .ok_or(Error::Invariant("exception pattern without head"))?;
            let parent = analyzer.template_body_frame(entity, head, specification.scope)?;
            let mut packs =
                super::pack_frames::FramePacks::new(&analyzer.graph, parent, analyzer.identifiers);
            let expands = packs.enabled();
            let copied = {
                let mut cloner =
                    packs.cloner(analyzer.tree, analyzer.stack_budget, analyzer.stack_base);
                (|| {
                    let parameters = if expands {
                        specification
                            .parameters
                            .map(|clause| cloner.parameters(clause))
                            .transpose()?
                    } else {
                        specification.parameters
                    };
                    let operand = cloner.expression(specification.operand)?;
                    Ok((parameters, operand, cloner.parameter_packs().to_vec()))
                })()
            };
            let (parameters, operand, parameter_packs) = packs.finish(copied)?;
            let parent = if let Some(clause) = specification.parameters {
                analyzer.function_pack_scope(parent, clause, &parameter_packs, prototype_type)?
            } else {
                parent
            };
            let prototype = analyzer.exception_prototype(TemplateException {
                operand,
                scope: parent,
                parameters,
                ty: prototype_type,
                ..specification
            })?;
            let saved_limit = analyzer.graph.lookup_limit.replace(specification.limit);
            let saved_allocation = std::mem::replace(
                &mut analyzer.global_allocation_context,
                super::allocation::GlobalAllocationContext::Instantiation,
            );
            let result = analyzer.noexcept_value(Some(entity), operand, prototype);
            analyzer.graph.lookup_limit = saved_limit;
            analyzer.global_allocation_context = saved_allocation;
            result.map(|value| {
                if value {
                    ExceptionSet::None
                } else {
                    ExceptionSet::All
                }
            })
        })(self);
        match result {
            Ok(value) => {
                self.graph.entity_mut(entity).exceptions = ExceptionState::Value(value);
                Ok(())
            }
            Err(error) => {
                self.graph.entity_mut(entity).exceptions = ExceptionState::Template(specification);
                Err(error)
            }
        }
    }

    pub(super) fn check_template_exception(
        &mut self,
        pattern: TemplateException,
        pending: super::exceptions::PendingSpecification,
    ) -> Result<(), Error> {
        let mut own = Vec::with_capacity(32);
        let mut scope = Some(pattern.scope);
        while let Some(id) = scope {
            let current = self.graph.scope(id);
            if current.kind == ScopeKind::Class
                && current
                    .owner
                    .is_some_and(|id| self.graph.entity(id).template_head.is_some())
            {
                return Ok(());
            }
            if current.kind == ScopeKind::TemplateParameters {
                for &binding in current.bindings.iter().take(32 - own.len()) {
                    own.push(self.graph.binding(binding).name);
                }
            }
            scope = current.parent;
        }
        let count = self.graph.types.function_parameter_count(pattern.ty)?;
        if let Some(clause) = pattern.parameters {
            let parameters = self.tree.parameters.get(clause.parameters);
            if parameters.len() == count {
                for (index, parameter) in parameters.iter().enumerate() {
                    if own.len() == 32 {
                        break;
                    }
                    let ty = self.graph.types.function_parameter(pattern.ty, index)?;
                    if self.exception_dependent_type(ty)
                        && let Some(name) = self.simple_declarator_name_opt(parameter.declarator)?
                    {
                        own.push(Some(name));
                    }
                }
            }
        }
        if self.exception_names_own(pattern.operand, &own) {
            return Ok(());
        }
        if pattern
            .parameters
            .is_some_and(|clause| self.tree.parameters.get(clause.parameters).len() != count)
        {
            return Ok(());
        }
        let scope = self.exception_prototype(pattern)?;
        let saved_limit = self.graph.lookup_limit;
        let saved_noexcept = self.evaluating_noexcept;
        let saved_depth = self.template_depth;
        self.graph.lookup_limit = Some(pattern.limit);
        self.evaluating_noexcept = true;
        self.template_depth = saved_depth.max(1);
        let saved_body = self.body_context.take();
        let saved_default = self.default_argument_scope.take();
        let saved_member = self.member_initializer_class.take();
        let result = (|| {
            if !self.fixed_template_names(pattern.operand, scope)? {
                return Ok(());
            }
            self.template_depth = saved_depth;
            self.noexcept_value(Some(pending.owner()), pattern.operand, scope)
                .map(|_| ())
        })();
        self.graph.lookup_limit = saved_limit;
        self.evaluating_noexcept = saved_noexcept;
        self.template_depth = saved_depth;
        self.body_context = saved_body;
        self.default_argument_scope = saved_default;
        self.member_initializer_class = saved_member;
        result
    }

    pub(super) fn exception_prototype(
        &mut self,
        pattern: TemplateException,
    ) -> Result<ScopeId, Error> {
        let mut scope = pattern.scope;
        if let Some(clause) = pattern.parameters {
            let parameters = self.tree.parameters.get(clause.parameters);
            let count = self.graph.types.function_parameter_count(pattern.ty)?;
            scope = self
                .graph
                .add_scope(ScopeKind::Function, scope, None, None)?;
            for (index, &parameter) in parameters.iter().enumerate().take(count) {
                let ty = self.graph.types.function_parameter(pattern.ty, index)?;
                let name = self.declarator_name(parameter.declarator)?;
                let object_type = self.graph.types.parameter_object(ty)?;
                let entity = self
                    .graph
                    .add_entity(Some(object_type), name, Keyword::Int)?;
                self.graph
                    .record_prototype_parameter(entity, index, object_type);
                self.graph.add_binding(
                    scope,
                    None,
                    Binding {
                        kind: BindingKind::Parameter,
                        name,
                        entity,
                        ty: Some(ty),
                        key: Keyword::Int,
                    },
                )?;
            }
        }
        Ok(scope)
    }

    fn fixed_template_names(&mut self, root: ast::ExprId, scope: ScopeId) -> Result<bool, Error> {
        let Some(key) = self.operand_key(root, scope)? else {
            return Ok(false);
        };
        Ok(!key.dependent && !self.type_dependent_expression(root, scope)?)
    }

    fn simple_declarator_name_opt(
        &self,
        mut id: Option<ast::DeclaratorId>,
    ) -> Result<Option<NameId>, Error> {
        // OwnNames walks through abstract declarators too: an unnamed
        // dependent parameter contributes no name to the fixed-operand probe.
        while let Some(current) = id {
            let node = self.tree.declarators.get(current);
            if node.name.is_some() {
                return self.simple_declarator_name(current);
            }
            id = node.nested;
        }
        Ok(None)
    }

    pub(super) fn exception_dependent_type(&self, ty: TypeId) -> bool {
        self.graph.dependent_type(ty)
    }

    fn own_name_probe(
        &self,
        name: ast::Name,
        depth: u32,
        own: &[Option<NameId>],
        work: &mut Vec<(OwnProbe, u32)>,
    ) -> bool {
        let next = depth.wrapping_add(1);
        let name = self.tree.names.get(name);
        for (index, component) in self.tree.name_lists.get(name.components).iter().enumerate() {
            if let NameKind::Identifier(name_id) = component.kind
                && index == 0
                && !name.global
                && own.contains(&Some(name_id))
            {
                return true;
            }
            if let NameKind::Decltype(expr) = component.kind {
                work.push((OwnProbe::Expression(expr), next));
            }
            if let Some(arguments) = component.arguments {
                for argument in self.tree.template_arguments.get(arguments) {
                    match argument.kind {
                        ast::TemplateArgumentKind::Expression(expr) => {
                            work.push((OwnProbe::Expression(expr), next));
                        }
                        ast::TemplateArgumentKind::Type(ty) => {
                            work.push((OwnProbe::Type(ty), depth));
                        }
                    }
                }
            }
        }
        false
    }
    fn exception_names_own(&self, root: ast::ExprId, own: &[Option<NameId>]) -> bool {
        let mut work = vec![(OwnProbe::Expression(root), 0u32)];
        while let Some((probe, depth)) = work.pop() {
            let next = depth.wrapping_add(1);
            match probe {
                OwnProbe::Specifiers(list, named_only) => {
                    for spec in self.tree.specs.get(list) {
                        match spec.kind {
                            SpecifierKind::Named(name) => work.push((OwnProbe::Name(name), next)),
                            SpecifierKind::Decltype(expr) if !named_only => {
                                work.push((OwnProbe::Expression(expr), next));
                            }
                            _ => (),
                        }
                    }
                }
                OwnProbe::Type(ty) => work.push((
                    OwnProbe::Specifiers(self.tree.types.get(ty).specifiers, false),
                    depth,
                )),
                OwnProbe::Name(name) => {
                    if self.own_name_probe(name, depth, own, &mut work) {
                        return true;
                    }
                }
                OwnProbe::Expression(expr) => {
                    if depth > 64 {
                        continue;
                    }
                    match self.tree.expressions.get(expr).kind {
                        ExprKind::Id(name) => work.push((OwnProbe::Name(name), depth)),
                        ExprKind::IntegerPack(inner)
                        | ExprKind::Paren(inner)
                        | ExprKind::ComplexPart { operand: inner, .. }
                        | ExprKind::Unary { operand: inner, .. }
                        | ExprKind::Postfix { operand: inner, .. }
                        | ExprKind::PackExpansion(inner) => {
                            work.push((OwnProbe::Expression(inner), next));
                        }
                        ExprKind::Binary { left, right, .. } => work.extend([
                            (OwnProbe::Expression(left), next),
                            (OwnProbe::Expression(right), next),
                        ]),
                        ExprKind::Conditional {
                            condition,
                            then_expr,
                            else_expr,
                        } => work.extend([
                            (OwnProbe::Expression(condition), next),
                            (OwnProbe::Expression(then_expr), next),
                            (OwnProbe::Expression(else_expr), next),
                        ]),
                        ExprKind::Call { callee, arguments } => {
                            work.push((OwnProbe::Expression(callee), next));
                            work.extend(
                                self.tree
                                    .expr_lists
                                    .get(arguments)
                                    .iter()
                                    .map(|&id| (OwnProbe::Expression(id), next)),
                            );
                        }
                        ExprKind::Member { base, .. } => {
                            work.push((OwnProbe::Expression(base), next));
                        }
                        ExprKind::Subscript { base, index } => work.extend([
                            (OwnProbe::Expression(base), next),
                            (OwnProbe::Expression(index), next),
                        ]),
                        ExprKind::TypeTransform { types, .. }
                        | ExprKind::TypePredicate { types, .. } => work.extend(
                            self.tree
                                .type_lists
                                .get(types)
                                .iter()
                                .map(|&ty| (OwnProbe::Type(ty), depth)),
                        ),
                        ExprKind::VaArg { ty, .. }
                        | ExprKind::BuiltinValue { ty, .. }
                        | ExprKind::OffsetOfBase(ty)
                        | ExprKind::OffsetOf { ty, .. } => {
                            work.push((OwnProbe::Type(ty), depth));
                        }
                        ExprKind::Cast { ty, operand, .. } => {
                            work.extend([
                                (OwnProbe::Type(ty), depth),
                                (OwnProbe::Expression(operand), next),
                            ]);
                        }
                        ExprKind::Typeof { ty, operand }
                        | ExprKind::Sizeof { ty, operand, .. }
                        | ExprKind::TypeTrait { ty, operand, .. } => {
                            if let Some(ty) = ty {
                                work.push((OwnProbe::Type(ty), depth));
                            }
                            if let Some(operand) = operand {
                                work.push((OwnProbe::Expression(operand), next));
                            }
                        }
                        ExprKind::FunctionalCast {
                            specifiers,
                            arguments,
                            ..
                        } => {
                            work.push((OwnProbe::Specifiers(specifiers, true), depth));
                            work.extend(
                                self.tree
                                    .expr_lists
                                    .get(arguments)
                                    .iter()
                                    .map(|&id| (OwnProbe::Expression(id), next)),
                            );
                        }
                        ExprKind::SizeofPack(name) if own.contains(&Some(name)) => return true,
                        _ => (),
                    }
                }
            }
        }
        false
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
    fn specialization_query_adds_no_instantiation_level_and_restores_failed_queries() {
        use super::super::{
            allocation::GlobalAllocationContext, exceptions::ExceptionState, lookup::Filter,
            template_context::MAX_INSTANTIATION_DEPTH, types::Scalar,
        };
        let source = SourceBuffer::new(
            b"template<class T>void good()noexcept(sizeof(T)==4);\
              template<class T>void bad()noexcept(T::missing);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"exception-query-depth").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        analyzer.instantiation_depth = MAX_INSTANTIATION_DEPTH;
        let int = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(crate::types::FundamentalType::Int))
            .unwrap();
        for (name, succeeds) in [(b"good".as_slice(), true), (b"bad".as_slice(), false)] {
            let name = analyzer.identifiers.intern(name).unwrap();
            let primary = analyzer
                .graph
                .own(analyzer.graph.global, name, Filter::Ordinary)
                .unwrap();
            let binding = analyzer
                .complete_signature(primary, &mut [Some(int)])
                .unwrap()
                .unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            assert_eq!(
                analyzer.graph.instantiation_levels.get(entity).unwrap(),
                MAX_INSTANTIATION_DEPTH
            );
            assert!(matches!(
                analyzer.graph.entity(entity).exceptions,
                ExceptionState::Template(_)
            ));
            let limit = analyzer.graph.binding_limit();
            analyzer.graph.lookup_limit = Some(limit);
            analyzer.global_allocation_context = GlobalAllocationContext::Declaration;
            for _ in 0..2 {
                let result = analyzer.function_may_throw(entity);
                if succeeds {
                    assert!(!result.unwrap());
                } else {
                    assert!(matches!(result, Err(Error::InstantiationDiagnostic { .. })));
                    assert!(matches!(
                        analyzer.graph.entity(entity).exceptions,
                        ExceptionState::Template(_)
                    ));
                }
                assert_eq!(analyzer.instantiation_depth, MAX_INSTANTIATION_DEPTH);
                assert_eq!(analyzer.graph.lookup_limit, Some(limit));
                assert!(matches!(
                    analyzer.global_allocation_context,
                    GlobalAllocationContext::Declaration
                ));
                assert!(analyzer.body_context.is_none());
            }
        }
    }

    #[test]
    fn specialization_operand_keeps_the_callers_instantiation_level() {
        let source = SourceBuffer::new(
            b"template<class T>struct H{static constexpr int value=1;};\
              template<class T>void f()noexcept(H<T>::value);\
              int q(){return noexcept(f<int>());}using R=H<int>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"exception-formation-level").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"R").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let graph = analyzer.analyze(declarations).unwrap();
        let binding = graph
            .own(graph.global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let ty = graph.binding(binding).ty.unwrap();
        let class = graph.class_of(ty).unwrap();
        assert_eq!(graph.instantiation_levels.get(class).unwrap(), 0);
    }

    #[test]
    fn specification_queries_memoize_without_instantiating_bodies() {
        let source = SourceBuffer::new(
            b"template<class T>int f(const T x)noexcept(sizeof(x)==4){return T::bad;}\
              int main(){return noexcept(f(1))+noexcept(f('x'));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"specialization-exception-demand").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let expressions = analyzer.tree.expressions.mark();
        let instances: Vec<_> = analyzer
            .graph
            .function_templates
            .arguments
            .keys()
            .copied()
            .collect();
        assert_eq!(instances.len(), 2);
        for instance in instances {
            let primary = analyzer.graph.function_templates.primary(instance).unwrap();
            let primary = analyzer.graph.binding(primary).entity;
            assert!(!analyzer.graph.function_templates.body_demanded(primary));
            let super::super::exceptions::ExceptionState::Template(pattern) =
                analyzer.graph.entity(primary).exceptions
            else {
                panic!("primary exception pattern was overwritten");
            };
            assert!(
                analyzer
                    .graph
                    .expression_facts
                    .get(pattern.operand)
                    .is_err()
            );
            assert!(matches!(
                analyzer.graph.entity(instance).exceptions,
                super::super::exceptions::ExceptionState::Value(_)
            ));
            analyzer.resolve_specialization_exception(instance).unwrap();
            analyzer.resolve_specialization_exception(instance).unwrap();
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
    }
}

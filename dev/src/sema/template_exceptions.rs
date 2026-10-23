// Translated from analyzer_exceptions.cpp::check_fixed_template_noexcept and
// its OwnNames probes. See NOTICE.
//! Retain a template specification's typed syntax and declaration environment.
//! Fixed operands are checked once; dependent operands wait for instantiation.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::Filter,
    types::{TypeId, TypeKind},
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
        let result = self.instantiation_context(
            self.graph.instantiation_levels.get(entity)?,
            self.tree.expressions.get(specification.operand).location,
            specification.limit,
            |analyzer| {
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
                let mut packs = super::pack_frames::FramePacks::new(
                    &analyzer.graph,
                    parent,
                    analyzer.identifiers,
                );
                let mut cloner = crate::syntax::cloning::Cloner::new(
                    analyzer.tree,
                    analyzer.stack_budget,
                    analyzer.stack_base,
                )
                .with_expander(&mut packs);
                let parameters = specification
                    .parameters
                    .map(|clause| cloner.parameters(clause))
                    .transpose()
                    .map_err(|error| super::template_bodies::copy_error(&error))?;
                let operand = cloner
                    .expression(specification.operand)
                    .map_err(|error| super::template_bodies::copy_error(&error))?;
                let parameter_packs = cloner.parameter_packs().to_vec();
                let parent = if let Some(clause) = specification.parameters {
                    analyzer.function_pack_scope(
                        parent,
                        clause,
                        &parameter_packs,
                        prototype_type,
                    )?
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
                let saved = analyzer.evaluating_noexcept;
                analyzer.evaluating_noexcept = true;
                let result = (|| {
                    let ty = analyzer.exception_operand_type(operand, prototype)?;
                    analyzer.exception_bool(ty, analyzer.tree.expressions.get(operand).location)?;
                    analyzer.constant_value(operand, prototype, true)
                })();
                analyzer.evaluating_noexcept = saved;
                let value = result?.ok_or(Error::Invariant(
                    "constant exception specification without value",
                ))?;
                Ok(if value.bits == 0 {
                    ExceptionSet::All
                } else {
                    ExceptionSet::None
                })
            },
        );
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
        self.template_depth = 0;
        let result = (|| {
            if !self.fixed_template_names(pattern.operand, scope)? {
                return Ok(());
            }
            let ty = self.exception_operand_type(pattern.operand, scope)?;
            self.exception_bool(ty, self.tree.expressions.get(pattern.operand).location)?;
            self.constant_value(pattern.operand, scope, true)
                .map(|_| ())
        })();
        self.graph.lookup_limit = saved_limit;
        self.evaluating_noexcept = saved_noexcept;
        self.template_depth = saved_depth;
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
        let mut work = vec![root];
        while let Some(id) = work.pop() {
            match self.tree.expressions.get(id).kind {
                ExprKind::Id(name) => {
                    let node = self.tree.names.get(name);
                    let components = self.tree.name_lists.get(node.components);
                    if !node.global
                        && components.len() == 1
                        && let NameKind::Identifier(key) = components[0].kind
                        && components[0].arguments.is_none()
                    {
                        let Ok(Some(binding)) = self.unqualified(scope, key, Filter::Ordinary)
                        else {
                            return Ok(false);
                        };
                        let binding = self.graph.binding(binding);
                        if binding
                            .ty
                            .is_some_and(|ty| self.exception_dependent_type(ty))
                            || !matches!(
                                binding.kind,
                                BindingKind::Variable
                                    | BindingKind::Parameter
                                    | BindingKind::Function
                                    | BindingKind::Enumerator
                            )
                        {
                            return Ok(false);
                        }
                    } else {
                        let binding = self.lookup(name, scope, Filter::Ordinary)?;
                        let binding = self.graph.binding(binding);
                        if binding
                            .ty
                            .is_some_and(|ty| self.exception_dependent_type(ty))
                            || !matches!(
                                binding.kind,
                                BindingKind::Variable
                                    | BindingKind::Parameter
                                    | BindingKind::Function
                                    | BindingKind::Enumerator
                            )
                        {
                            return Ok(false);
                        }
                    }
                }
                ExprKind::Paren(inner)
                | ExprKind::Unary { operand: inner, .. }
                | ExprKind::Postfix { operand: inner, .. }
                | ExprKind::Delete { operand: inner, .. } => work.push(inner),
                ExprKind::Binary { left, right, .. } => work.extend([right, left]),
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => work.extend([else_expr, then_expr, condition]),
                ExprKind::Call { callee, arguments } => {
                    work.extend(self.tree.expr_lists.get(arguments).iter().rev().copied());
                    work.push(callee);
                }
                ExprKind::Sizeof { ty, operand, .. } | ExprKind::TypeTrait { ty, operand, .. } => {
                    if let Some(ty) = ty {
                        let ty = self.type_syntax(ty, scope)?;
                        if self.exception_dependent_type(ty) {
                            return Ok(false);
                        }
                    }
                    if let Some(operand) = operand {
                        if ty.is_none()
                            && matches!(self.tree.expressions.get(id).kind, ExprKind::Sizeof { .. })
                        {
                            self.check_template_size_key(
                                operand,
                                scope,
                                self.tree.expressions.get(id).location,
                            )?;
                        }
                        work.push(operand);
                    }
                }
                ExprKind::TypeTransform { types, .. } | ExprKind::TypePredicate { types, .. } => {
                    for index in 0..self.tree.type_lists.get(types).len() {
                        let ty = self.type_syntax(self.tree.type_lists.get(types)[index], scope)?;
                        if self.exception_dependent_type(ty) {
                            return Ok(false);
                        }
                    }
                }
                ExprKind::VaArg { ty, operand } | ExprKind::Cast { ty, operand, .. } => {
                    let ty = self.type_syntax(ty, scope)?;
                    if self.exception_dependent_type(ty) {
                        return Ok(false);
                    }
                    work.push(operand);
                }
                ExprKind::Member { base, .. } => work.push(base),
                ExprKind::Subscript { base, index } => work.extend([index, base]),
                ExprKind::Braced { elements, .. } => {
                    work.extend(self.tree.expr_lists.get(elements).iter().rev().copied());
                }
                ExprKind::Statement(_)
                | ExprKind::FunctionalCast { .. }
                | ExprKind::New { .. }
                | ExprKind::Lambda(_)
                | ExprKind::PackExpansion(_)
                | ExprKind::SizeofPack(_) => return Ok(false),
                ExprKind::Throw(operand) => work.extend(operand),
                ExprKind::Literal(_) | ExprKind::Keyword(_) => (),
            }
        }
        Ok(true)
    }

    fn check_template_size_key(
        &mut self,
        mut operand: ast::ExprId,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(operand).kind {
            operand = inner;
        }
        let ExprKind::Id(name) = self.tree.expressions.get(operand).kind else {
            return Ok(());
        };
        let node = self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if node.global || components.len() != 1 || components[0].arguments.is_some() {
            return Ok(());
        }
        let NameKind::Identifier(key) = components[0].kind else {
            return Ok(());
        };
        let Ok(Some(binding)) = self.unqualified(scope, key, Filter::Ordinary) else {
            return Ok(());
        };
        let binding = self.graph.binding(binding);
        if matches!(binding.kind, BindingKind::Parameter | BindingKind::Function)
            && let Some(mut ty) = binding.ty
        {
            if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            if !matches!(self.graph.types.get(ty).kind, TypeKind::Function { .. }) {
                return Ok(());
            }
            // C++'s equivalence key first tries the source type's constant
            // sizeof, before the later operand typing adjusts parameters.
            self.type_size(ty, location)?;
        }
        Ok(())
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
                        ExprKind::Paren(inner)
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
                        ExprKind::VaArg { ty, .. } => {
                            work.push((OwnProbe::Type(ty), depth));
                        }
                        ExprKind::Cast { ty, operand, .. } => {
                            work.extend([
                                (OwnProbe::Type(ty), depth),
                                (OwnProbe::Expression(operand), next),
                            ]);
                        }
                        ExprKind::Sizeof { ty, operand, .. }
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

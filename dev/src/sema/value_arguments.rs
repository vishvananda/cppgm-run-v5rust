// Translated from sema/analyzer.cpp::non_type_parameter_type and class_templates.cpp.
// See NOTICE.
//! Value arguments retain canonical types and target values, never text.
use super::{
    Error,
    analyzer::Analyzer,
    constants::{Constant, ConstantState},
    graph::{Binding, BindingKind, ScopeId},
    templates::{DefaultArgument, HeadId},
    types::{Cv, TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::{ast, cloning::Cloner},
};

impl Analyzer<'_> {
    pub(super) fn non_type_parameter_type(
        &mut self,
        specifiers: crate::syntax::storage::List<ast::Specifier>,
        declarator: Option<ast::DeclaratorId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let specs = self.specifiers(specifiers, scope, false)?;
        if matches!(self.graph.types.get(specs.ty).kind, TypeKind::Auto) {
            return Err(Error::at(
                location,
                "non-type template parameter without a type",
            ));
        }
        let ty = self.declarator(declarator, specs.ty, scope)?.ty;
        let ty = self.bare_type(ty)?;
        let ty = self.graph.types.parameter(ty)?;
        if self.graph.dependent_type(ty)
            || self.integral_kind(ty).is_some()
            || matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Enum(_)
                    | TypeKind::Pointer(_)
                    | TypeKind::MemberPointer { .. }
                    | TypeKind::Reference {
                        kind: super::types::RefKind::Lvalue,
                        ..
                    }
                    | TypeKind::Scalar(super::types::Scalar::Nullptr)
            )
        {
            return Ok(ty);
        }
        Err(Error::at(
            location,
            "a non-type template parameter of a type 14.1p4 does not allow",
        ))
    }

    pub(super) fn template_argument(
        &mut self,
        argument: ast::TemplateArgument,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        self.template_argument_for(argument, scope, location, None)
    }

    pub(super) fn template_argument_for(
        &mut self,
        argument: ast::TemplateArgument,
        scope: ScopeId,
        location: SourceOffset,
        target: Option<TypeId>,
    ) -> Result<TypeId, Error> {
        if let Some(identity) = self.template_argument_identity(argument, scope, location)? {
            return Ok(identity);
        }
        let ast::TemplateArgumentKind::Expression(expression) = argument.kind else {
            return Err(Error::Invariant("type argument without identity"));
        };
        self.template_value_for(expression, scope, target)
    }

    pub(super) fn template_value(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        self.template_value_for(expression, scope, None)
    }

    fn template_value_for(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        target: Option<TypeId>,
    ) -> Result<TypeId, Error> {
        if self.semantic() && self.template_depth == 0 {
            let value_uses = self.value_parameter_uses.get();
            self.template_arguments += 1;
            let result = self
                .typed_expression(expression, scope)
                .and_then(|_| self.check_resolved_function(expression));
            self.template_arguments -= 1;
            result?;
            // A frame-bound value keeps its originating template-id dependent
            // for instantiation-context ADL even when today's value is concrete.
            if self.value_parameter_uses.get() != value_uses {
                self.note_dependent_use();
            }
            if target.is_some_and(|ty| self.graph.address_parameter_type(ty))
                || target.is_none() && self.is_address_expression(expression)
            {
                return self.null_template_argument(expression, scope, target);
            }
        }
        if let Some(value) = self.constant_value(expression, scope, self.template_depth == 0)? {
            if self.integral_kind(value.ty).is_none() {
                return Err(Error::at(
                    self.tree.expressions.get(expression).location,
                    "a non-type template argument of non-integral type is not supported here",
                ));
            }
            let ty = self.bare_type(value.ty)?;
            return self.graph.types.intern(
                TypeKind::Value {
                    ty,
                    bits: value.bits,
                },
                Cv::default(),
            );
        }
        if self.template_depth == 0 {
            return Err(Error::at(
                self.tree.expressions.get(expression).location,
                "a template argument that is not a constant expression",
            ));
        }
        self.dependent_value_operand(expression, scope)
    }

    pub(super) fn match_template_argument(
        &mut self,
        head: HeadId,
        index: usize,
        argument: TypeId,
        earlier: &[TypeId],
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let parameter = self.graph.templates.parameters(head)[index];
        let info = self
            .graph
            .entity(parameter)
            .template_parameter
            .ok_or(Error::Invariant("argument without parameter"))?;
        let argument = self.graph.types.canonical_function(argument)?;
        let value = self.value_argument(argument);
        if value != info.value_type.is_some() {
            return Err(Error::at(
                location,
                if value {
                    "a value given for a type template parameter"
                } else {
                    "a type given for a non-type template parameter"
                },
            ));
        }
        let TypeKind::Value { ty: source, bits } = self.graph.types.get(argument).kind else {
            return Ok(argument);
        };
        let target = match self.argument_target(head, index, earlier)? {
            super::address_arguments::ArgumentTarget::Type(ty) => ty,
            super::address_arguments::ArgumentTarget::Unknown => return Ok(argument),
            super::address_arguments::ArgumentTarget::Invalid => {
                return Err(Error::at(
                    location,
                    "a non-type template parameter of a type 14.1p4 does not allow",
                ));
            }
        };
        let target = self.bare_type(target)?;
        if self.graph.address_parameter_type(target) || self.graph.address_parameter_type(source) {
            return self.converted_null_argument(target, argument, location);
        }
        let failure =
            if matches!(self.graph.types.get(target).kind, TypeKind::Enum(_)) && source != target {
                Some("a template argument does not convert to the parameter's enumeration type")
            } else if self.scoped_enum(source) && source != target {
                Some("a scoped enumeration template argument does not convert to an integer")
            } else if let Some(kind) = self.integral_kind(target) {
                let from = self
                    .integral_kind(source)
                    .ok_or(Error::Invariant("nonintegral value argument"))?;
                if kind.represents(bits, from) {
                    return self.graph.types.intern(
                        TypeKind::Value {
                            ty: target,
                            bits: kind.convert(bits),
                        },
                        Cv::default(),
                    );
                }
                Some("a narrowing conversion of a template argument")
            } else if matches!(self.graph.types.get(target).kind, TypeKind::Enum(_)) {
                Some("a non-type template parameter of incomplete enumeration type")
            } else {
                return Err(Error::Missing("address non-type template arguments"));
            };
        Err(Error::at(
            location,
            failure.ok_or(Error::Invariant("missing conversion failure"))?,
        ))
    }

    pub(super) fn match_explicit_template_arguments(
        &mut self,
        head: HeadId,
        given: &[TypeId],
        location: SourceOffset,
    ) -> Result<Option<Vec<TypeId>>, Error> {
        let mut converted = Vec::with_capacity(given.len());
        for (index, &argument) in given.iter().enumerate() {
            match self.match_template_argument(head, index, argument, &converted, location) {
                Ok(argument) => converted.push(argument),
                Err(Error::Diagnostic { .. }) => return Ok(None),
                Err(error) => return Err(error),
            }
        }
        Ok(Some(converted))
    }

    /// Bind one argument in a fresh frame. A value never overwrites the pattern entity.
    pub(super) fn bind_template_argument(
        &mut self,
        scope: ScopeId,
        binding: Binding,
        argument: TypeId,
    ) -> Result<(), Error> {
        if matches!(self.graph.types.get(argument).kind, TypeKind::Pack(_)) {
            return self.bind_argument_pack(scope, binding, argument);
        }
        let (kind, entity, ty) = if let TypeKind::Value { ty, bits } =
            self.graph.types.get(argument).kind
        {
            let entity = self.graph.add_entity(Some(ty), binding.name, binding.key)?;
            self.graph.entity_mut(entity).constant = ConstantState::Value(Constant { bits, ty });
            (BindingKind::Enumerator, entity, ty)
        } else if self
            .graph
            .entity(binding.entity)
            .template_parameter
            .is_some_and(|info| info.value_type.is_some())
        {
            (
                BindingKind::TemplateValue,
                match self.graph.types.get(argument).kind {
                    TypeKind::TemplateParameter { entity, .. } => entity,
                    _ => binding.entity,
                },
                binding
                    .ty
                    .ok_or(Error::Invariant("value parameter without type"))?,
            )
        } else {
            (BindingKind::Alias, binding.entity, argument)
        };
        let id = self.graph.add_binding(
            scope,
            None,
            Binding {
                kind,
                entity,
                ty: Some(ty),
                ..binding
            },
        )?;
        if kind == BindingKind::Enumerator {
            let declared = self
                .graph
                .entity(binding.entity)
                .template_parameter
                .and_then(|info| info.value_type);
            self.graph.set_binding_dependence(
                id,
                declared.is_none_or(|ty| self.graph.dependent_type(ty)),
            );
        }
        Ok(())
    }

    // function_templates.cpp::deduced_values_typed / BoundNames: explicit
    // arguments convert, but a value deduced from a template-id keeps its type.
    pub(super) fn deduced_values_typed(
        &self,
        head: HeadId,
        signature: TypeId,
        arguments: &[Option<TypeId>],
        given: &[Option<TypeId>],
    ) -> Result<bool, Error> {
        for (index, argument) in arguments.iter().enumerate() {
            if given.get(index).is_some_and(Option::is_some) {
                continue;
            }
            let Some(argument) = argument else {
                continue;
            };
            let parameter = self.graph.templates.parameters(head)[index];
            let declared = self
                .graph
                .entity(parameter)
                .template_parameter
                .ok_or(Error::Invariant("deduction without parameter"))?
                .value_type;
            let Some(declared) = declared.filter(|&ty| !self.graph.dependent_type(ty)) else {
                continue;
            };
            let TypeKind::Value { ty, .. } = self.graph.types.get(*argument).kind else {
                continue;
            };
            if self.graph.types.canonical_view(ty) != self.graph.types.canonical_view(declared)
                && !self.bound_names_value(signature, self.graph.templates.depth(head), index)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn bound_operand_names(&self, operand: TypeId, depth: u32, index: usize) -> bool {
        self.named_bound_parameter(operand, depth) == Some(index)
    }
    pub(super) fn named_bound_parameter(&self, operand: TypeId, depth: u32) -> Option<usize> {
        let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(operand).kind else {
            return None;
        };
        let pattern = self.graph.template_operands.pattern(entity)?;
        let mut expression = pattern.expression;
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(expression).kind {
            expression = inner;
        }
        let ast::ExprKind::Id(name) = self.tree.expressions.get(expression).kind else {
            return None;
        };
        let name = self.tree.names.get(name);
        let components = self.tree.name_lists.get(name.components);
        if name.global || components.len() != 1 || components[0].arguments.is_some() {
            return None;
        }
        self.graph
            .template_operands
            .words(entity)
            .and_then(|words| {
                words.iter().find_map(|word| {
                    let super::expression_keys::KeyWord::Type(ty) = word else {
                        return None;
                    };
                    let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(*ty).kind
                    else {
                        return None;
                    };
                    self.graph
                        .entity(entity)
                        .template_parameter
                        .filter(|info| info.value_type.is_some())
                        .and_then(|info| info.index(depth))
                })
            })
    }
    pub(super) fn same_template_value(&self, first: TypeId, second: TypeId) -> bool {
        if first == second {
            return true;
        }
        let (
            TypeKind::Value {
                ty: from,
                bits: first,
            },
            TypeKind::Value {
                ty: to,
                bits: second,
            },
        ) = (
            &self.graph.types.get(first).kind,
            &self.graph.types.get(second).kind,
        )
        else {
            return false;
        };
        match (self.integral_kind(*from), self.integral_kind(*to)) {
            (Some(from), Some(to)) => to.represents(*first, from) && to.convert(*first) == *second,
            _ => false,
        }
    }
    fn bound_names_value(&self, root: TypeId, depth: u32, index: usize) -> bool {
        let mut work = vec![(root, 0u8)];
        let mut seen = std::collections::HashSet::new();
        while let Some((ty, level)) = work.pop() {
            // The frozen owner conservatively accepts deeper type shapes.
            if level > 16 {
                return true;
            }
            if !seen.insert((ty, level)) {
                continue;
            }
            let next = level + 1;
            match &self.graph.types.get(ty).kind {
                TypeKind::Pointer(element) | TypeKind::Reference { element, .. } => {
                    work.push((*element, next));
                }
                TypeKind::MemberPointer { owner, element } => {
                    work.extend([(*owner, next), (*element, next)]);
                }
                TypeKind::Pack(elements) => work.extend(elements.iter().map(|&ty| (ty, next))),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push((*result, next));
                    work.extend(parameters.iter().map(|&ty| (ty, next)));
                }
                TypeKind::Class(entity) => {
                    if let Some(instance) = self.graph.class_templates.instance(*entity) {
                        work.extend(instance.arguments.iter().map(|&ty| (ty, next)));
                    }
                }
                TypeKind::Array {
                    element, operand, ..
                } => {
                    if operand.is_some_and(|ty| self.bound_operand_names(ty, depth, index)) {
                        return true;
                    }
                    work.push((*element, next));
                }
                _ => (),
            }
        }
        false
    }

    pub(super) fn copy_template_argument(
        &mut self,
        syntax: ast::TemplateArgumentKind,
    ) -> Result<ast::TemplateArgument, Error> {
        let mut cloner = Cloner::new(self.tree, self.stack_budget, self.stack_base);
        let kind = match syntax {
            ast::TemplateArgumentKind::Type(ty) => {
                cloner.ty(ty).map(ast::TemplateArgumentKind::Type)
            }
            ast::TemplateArgumentKind::Expression(expression) => cloner
                .expression(expression)
                .map(ast::TemplateArgumentKind::Expression),
        }
        .map_err(|error| super::template_bodies::copy_error(&error))?;
        Ok(ast::TemplateArgument { kind, pack: false })
    }
    pub(super) fn default_location(&self, default: DefaultArgument) -> SourceOffset {
        match default.syntax {
            ast::TemplateArgumentKind::Type(ty) => self.tree.types.get(ty).location,
            ast::TemplateArgumentKind::Expression(expression) => {
                self.tree.expressions.get(expression).location
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::lookup::Filter;
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn converted_arguments_share_shells_but_values_and_types_stay_distinct() {
        let source = SourceBuffer::new(b"template<int N>struct A;using X=A<3>;using Y=A<3L>;using Z=A<-1>;template<class T>struct B;using W=B<int>;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"value-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = Analyzer::new(tree, cursor, StackBudget::default())
            .unwrap()
            .analyze(declarations)
            .unwrap();
        let mut lookup = |name| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
                .ty
                .unwrap()
        };
        assert_eq!(lookup(b"X"), lookup(b"Y"));
        assert_ne!(lookup(b"X"), lookup(b"Z"));
        assert_ne!(lookup(b"X"), lookup(b"W"));
        for name in [b"X", b"Z", b"W"] {
            let TypeKind::Class(entity) = graph.types.get(lookup(name)).kind else {
                panic!("class shell")
            };
            assert!(graph.entity(entity).scope.is_none());
        }
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn substitution_owns_value_bindings_and_keeps_pattern_facts_unpublished() {
        let source = SourceBuffer::new(b"template<int N>int f(){static_assert(N>0,\"positive\");return N;}int main(){return f<3>()+f<4>();}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"value-frames").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let name = cursor.intern_name(b"f").unwrap();
        let primary = graph
            .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
            .entity;
        let head = graph.entity(primary).template_head.unwrap();
        let parameter = graph.templates.parameters(head)[0];
        assert!(matches!(
            graph.entity(parameter).constant,
            ConstantState::None
        ));
        let name = cursor.intern_name(b"N").unwrap();
        let source = graph.templates.parameter_scope(head);
        let pattern = graph.binding(graph.own(source, name, Filter::Ordinary).unwrap());
        assert_eq!(pattern.entity, parameter);
        assert!(graph.entity(pattern.entity).template_parameter.is_some());
        let mut values = Vec::new();
        for &entity in graph.function_templates.arguments.keys() {
            let Some(scope) = graph.entity(entity).scope else {
                continue;
            };
            let Some(binding) = ({
                let mut current = Some(scope);
                let mut found = None;
                while let Some(id) = current {
                    if let Some(binding) = graph.own(id, name, Filter::Ordinary) {
                        found = Some(binding);
                        break;
                    }
                    current = graph.scope(id).parent;
                }
                found
            }) else {
                continue;
            };
            let binding = graph.binding(binding);
            if let ConstantState::Value(value) = graph.entity(binding.entity).constant {
                assert_ne!(binding.entity, parameter);
                assert!(graph.entity(binding.entity).template_parameter.is_none());
                values.push(value.bits);
            }
        }
        values.sort_unstable();
        values.dedup();
        assert_eq!(values, vec![3, 4]);
    }
}

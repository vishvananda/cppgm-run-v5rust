// Translated from sema/analyzer.cpp::non_type_parameter_type and class_templates.cpp.
// See NOTICE.
//! Value arguments retain canonical types and target values, never text.
use super::{
    Error,
    analyzer::Analyzer,
    constants::ConstantState,
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
        let (specifiers, declarator) = Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .parameter_type(specifiers, declarator)
            .map_err(|error| super::template_bodies::copy_error(&error))?;
        let specs = self.specifiers(specifiers, scope, false)?;
        if matches!(self.graph.types.get(specs.ty).kind, TypeKind::Auto(_)) {
            return Err(Error::at(
                location,
                "non-type template parameter without a type",
            ));
        }
        // This ellipsis declares a template parameter pack. It does not
        // expand the parameter's type (an inner function clause still may).
        let mut environment = self.declarator_environment(declarator, scope)?;
        environment.expand_pack = false;
        let ty = self.declarator_in(declarator, specs.ty, environment)?.ty;
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
        self.template_argument_at(argument, scope, location, target, None, false)
    }

    pub(super) fn template_argument_at(
        &mut self,
        argument: ast::TemplateArgument,
        scope: ScopeId,
        location: SourceOffset,
        target: Option<TypeId>,
        template: Option<bool>,
        unknown: bool,
    ) -> Result<TypeId, Error> {
        let plain = if template == Some(false) {
            None
        } else {
            self.plain_template_name(argument)
        };
        let named = if let Some((name, _)) = plain {
            self.template_name_argument(name, scope, template.is_some())?
        } else {
            None
        };
        if let Some(failure) = self.graph.analysis.substitution.current().failure {
            return Err(Error::SubstitutionRejected(failure));
        }
        if let Some(ty) = named {
            return if plain.is_some_and(|(_, expansion)| expansion) {
                self.pack_expansion_type(ty, location)
            } else {
                Ok(ty)
            };
        }
        if template == Some(true) && !plain.is_some_and(|(_, expansion)| expansion) {
            return Err(Error::UnconditionalDiagnostic {
                location,
                message: "a template template-argument that names no template",
            });
        }
        if let Some(identity) = self.template_argument_identity(argument, scope, location)? {
            return Ok(identity);
        }
        let ast::TemplateArgumentKind::Expression(expression) = argument.kind else {
            return Err(Error::Invariant("type argument without identity"));
        };
        self.template_value(expression, scope, target, unknown)
    }

    pub(super) fn template_value(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        target: Option<TypeId>,
        unknown: bool,
    ) -> Result<TypeId, Error> {
        if self.semantic() && self.template_depth == 0 {
            let value_uses = self.value_parameter_uses.get();
            self.template_arguments += 1;
            let result = self.typed_expression(expression, scope).and_then(|facts| {
                if let Some(target) = target.filter(|&ty| self.graph.address_parameter_type(ty))
                    && facts.functions(&self.graph.expression_facts).is_some()
                {
                    let binding = self.matching_function(facts, target)?.ok_or_else(|| {
                        Error::at(
                            self.tree.expressions.get(expression).location,
                            "no overloaded function matches the template parameter's type",
                        )
                    })?;
                    self.select_function_designation(expression, binding)
                } else {
                    self.check_resolved_function(expression)
                }
            });
            self.template_arguments -= 1;
            result?;
            if let Some(failure) = self.graph.analysis.substitution.current().failure {
                return Err(Error::SubstitutionRejected(failure));
            }
            // A frame-bound value keeps its originating template-id dependent
            // for instantiation-context ADL even when today's value is concrete.
            if self.value_parameter_uses.get() != value_uses {
                self.note_dependent_use();
            }
            if target.is_some_and(|ty| self.graph.address_parameter_type(ty))
                || target.is_none()
                    && (self.is_address_expression(expression)
                        || self.graph.address_parameter_type(
                            self.graph.expression_facts.get(expression)?.ty,
                        ))
            {
                return self.address_template_argument(expression, scope, target);
            }
            // An integral object named before its parameter is known may bind
            // a reference or be read as a value. Keep its designation until
            // candidate conversion chooses between those two operations.
            if unknown && self.names_linked_integral_object(expression)? {
                return self.address_template_argument(expression, scope, None);
            }
            if let Some(class) = self
                .graph
                .class_of(self.graph.expression_facts.get(expression)?.ty)
            {
                let mut target = target;
                if target.is_none()
                    && let Some(binding) = self.unique_conversion_function(
                        class,
                        super::conversion_functions::ConversionTarget::Integral,
                    )?
                {
                    let signature = self
                        .graph
                        .binding(binding)
                        .ty
                        .ok_or(Error::Invariant("template conversion without signature"))?;
                    let result = self.call_result_view(signature)?.ty;
                    let result = match self.graph.types.get(result).kind {
                        TypeKind::Reference { element, .. } => element,
                        _ => result,
                    };
                    target = Some(self.bare_type(result)?);
                }
                if let Some(target) = target {
                    let inserted_body = self.body_context.is_none();
                    if inserted_body {
                        let result = self.graph.types.scalar(super::types::Scalar::Void)?;
                        self.body_context =
                            Some(super::procedural_statements::BodyContext::new(result, None));
                    }
                    let converted = self.convert_scalar(expression, target, false);
                    if inserted_body {
                        self.body_context = None;
                    }
                    converted?;
                    // convert's soft failure does not skip converted_constant.
                    let value = self.converted_constant_value(
                        expression,
                        scope,
                        self.graph.analysis.substitution.current().soft == 0,
                    )?;
                    let Some(value) = value else {
                        return self.reject_substitution(
                            self.tree.expressions.get(expression).location,
                            "a template argument that is not a constant expression",
                            false,
                        );
                    };
                    let ty = self.bare_type(value.ty)?;
                    return self.graph.types.intern(
                        TypeKind::Value {
                            ty,
                            bits: value.bits,
                        },
                        Cv::default(),
                    );
                }
            }
        }
        let required =
            self.template_depth == 0 && self.graph.analysis.substitution.current().soft == 0;
        if let Some(value) = self.constant_value(expression, scope, required)? {
            if self.integral_kind(value.ty).is_none() {
                return Err(self.graph.analysis.substitution.hard_error(
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
            let failure = super::substitution_state::Failure {
                location: self.tree.expressions.get(expression).location,
                message: "a template argument that is not a constant expression",
                transient: false,
            };
            let failure = self.graph.analysis.substitution.emit(failure)?;
            return Err(Error::SubstitutionRejected(failure));
        }
        self.dependent_value_operand(expression, scope, target, unknown)
    }

    pub(super) fn match_template_argument(
        &mut self,
        head: HeadId,
        index: usize,
        argument: TypeId,
        earlier: &[TypeId],
        context: super::pack_arguments::MatchContext,
    ) -> Result<Option<TypeId>, Error> {
        self.match_template_argument_to(head, index, argument, earlier, context, None)
    }

    pub(super) fn match_template_argument_to(
        &mut self,
        head: HeadId,
        index: usize,
        argument: TypeId,
        earlier: &[TypeId],
        context: super::pack_arguments::MatchContext,
        element: Option<TypeId>,
    ) -> Result<Option<TypeId>, Error> {
        let parameter = self.graph.templates.parameters(head)[index];
        let info = self
            .graph
            .entity(parameter)
            .template_parameter
            .ok_or(Error::Invariant("argument without parameter"))?;
        let argument = self.graph.types.canonical_function(argument)?;
        let value = self.value_argument(argument);
        let template = self.template_argument_kind(argument);
        if value != info.value_type.is_some() || template != info.inner.is_some() {
            return context.mismatch(self,
                if template {
                    "a template given for a template parameter that is no template template-parameter"
                } else if info.inner.is_some() {
                    "a template template-parameter's argument that is no template"
                } else if value {
                    "a value given for a type template parameter"
                } else {
                    "a type given for a non-type template parameter"
                },
            );
        }
        if matches!(self.graph.types.get(argument).kind, TypeKind::Expansion(_)) {
            return Ok(Some(argument));
        }
        if template && self.match_template_name(info, argument, context)?.is_none() {
            return Ok(None);
        }
        if !matches!(
            self.graph.types.get(argument).kind,
            TypeKind::Value { .. } | TypeKind::Address { .. }
        ) {
            return Ok(Some(argument));
        }
        // Value conversion substitutes its declared target even if unrelated
        // earlier arguments remain dependent. argument_target's conservative
        // probe is only for typing the expression before its value is formed.
        let mut target = element
            .or(info.value_type)
            .ok_or(Error::Invariant("value argument without target"))?;
        if self.graph.dependent_type(target) {
            let given = earlier.iter().copied().map(Some).collect::<Vec<_>>();
            let mut environment = self.substitution(self.graph.templates.depth(head), &given);
            let Some(substituted) = self.substitute_signature_type(target, &mut environment)?
            else {
                return Ok(None);
            };
            target = substituted;
        }
        let target = self.bare_type(target)?;
        if self.graph.dependent_type(target) {
            // A bound expansion can retain a dependent pattern at positions
            // beyond its bound elements. C++ leaves that value unconverted.
            return Ok(Some(argument));
        }
        let Some(argument) = self.read_integral_argument(argument, target, context)? else {
            return Ok(None);
        };
        let (TypeKind::Value { ty: source, .. } | TypeKind::Address { ty: source, .. }) =
            self.graph.types.get(argument).kind
        else {
            return Err(Error::Invariant("converted argument without value"));
        };
        if self.graph.address_parameter_type(target) || self.graph.address_parameter_type(source) {
            return self.converted_address_argument(target, argument, context);
        }
        let TypeKind::Value { bits, .. } = self.graph.types.get(argument).kind else {
            return Err(Error::Invariant(
                "address conversion did not consume argument",
            ));
        };
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
                    if !kind.holds_u64_constant(bits, from) {
                        return context.mismatch(
                            self,
                            "a 128-bit template argument beyond the 64-bit constants evaluated",
                        );
                    }
                    return self
                        .graph
                        .types
                        .intern(
                            TypeKind::Value {
                                ty: target,
                                bits: kind.convert(bits),
                            },
                            Cv::default(),
                        )
                        .map(Some);
                }
                Some("a narrowing conversion of a template argument")
            } else {
                // A concrete target without an integral representation fails
                // conversion. Still-dependent synthesized targets returned above.
                Some("a non-type template parameter of incomplete enumeration type")
            };
        context.mismatch(
            self,
            failure.ok_or(Error::Invariant("missing conversion failure"))?,
        )
    }

    pub(super) fn match_explicit_template_arguments(
        &mut self,
        head: HeadId,
        given: &[TypeId],
        location: SourceOffset,
    ) -> Result<Option<Vec<TypeId>>, Error> {
        let context = super::pack_arguments::MatchContext {
            mode: super::pack_arguments::MatchMode::Nonfatal,
            location,
        };
        let mut converted = Vec::with_capacity(self.graph.templates.parameters(head).len());
        match self.match_pack_arguments(head, given, &mut converted, context, true)? {
            Some(()) => Ok(Some(converted)),
            None => Ok(None),
        }
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
        let (kind, entity, ty) =
            if let TypeKind::Address { ty, .. } = self.graph.types.get(argument).kind {
                let entity = self.graph.add_entity(Some(ty), binding.name, binding.key)?;
                self.graph
                    .constant_addresses
                    .bind_argument(entity, argument);
                (BindingKind::Enumerator, entity, ty)
            } else if let TypeKind::Value { ty, bits } = self.graph.types.get(argument).kind {
                let entity = self.graph.add_entity(Some(ty), binding.name, binding.key)?;
                let value = self.integral_value(ty, bits);
                self.graph.entity_mut(entity).constant = ConstantState::Value(value);
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
    ) -> Result<bool, Error> {
        for (index, argument) in arguments.iter().enumerate() {
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
            let values = match &self.graph.types.get(*argument).kind {
                TypeKind::Pack(elements) => elements.as_slice(),
                _ => std::slice::from_ref(argument),
            };
            if values.iter().any(|&value| {
                matches!(self.graph.types.get(value).kind, TypeKind::Value { ty, .. } | TypeKind::Address { ty, .. }
                if self.graph.types.canonical_view(ty) != self.graph.types.canonical_view(declared))
            }) && !self.bound_names_value(signature, self.graph.templates.depth(head), index)
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
        let first_address = self.graph.argument_address(first);
        let second_address = self.graph.argument_address(second);
        if first_address.is_some() || second_address.is_some() {
            return first_address == second_address;
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
        let mut seen = crate::support::hash::HashSet::default();
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
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Expansion(element) => {
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
                }
                | TypeKind::Vector {
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
        scope: ScopeId,
    ) -> Result<ast::TemplateArgument, Error> {
        let mut packs = super::pack_frames::FramePacks::new(&self.graph, scope, self.identifiers);
        let mut cloner = packs.cloner(self.tree, self.stack_budget, self.stack_base);
        let result = cloner.argument(ast::TemplateArgument {
            kind: syntax,
            pack: false,
        });
        packs.finish(result)
    }

    /// Dependent default formation retains expansions until substitution knows
    /// all participating packs. Only fact-bearing syntax is copied here.
    pub(super) fn copy_template_pattern(
        &mut self,
        syntax: ast::TemplateArgumentKind,
    ) -> Result<ast::TemplateArgument, Error> {
        Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .argument(ast::TemplateArgument {
                kind: syntax,
                pack: false,
            })
            .map_err(|error| super::template_bodies::copy_error(&error))
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
    fn class_template_values_retain_the_full_requested_conversion() {
        let source = SourceBuffer::new(b"struct X{constexpr operator long()const{return 257;}};template<unsigned char N>struct A{static const unsigned char value=N;};static_assert(A<(X())>::value==1,\"converted argument\");".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"converted-template-value").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.analyze(declarations).unwrap();
    }

    #[test]
    fn synthesized_targets_remain_dependent_without_completing_classes() {
        let source = SourceBuffer::new(
            b"template<class T,T V>struct A;template<class T>using Pattern=A<T,0>;".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"value-ordering").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"Pattern").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
        let alias = analyzer.graph.binding(binding).entity;
        let head = analyzer.graph.entity(alias).template_head.unwrap();
        let pattern = analyzer.graph.entity(alias).ty.unwrap();
        let unique = analyzer.unique_partial_arguments(head).unwrap();
        let mut substitution = analyzer.substitution(analyzer.graph.templates.depth(head), &unique);
        let substituted = analyzer
            .substitute_signature_type(pattern, &mut substitution)
            .unwrap()
            .unwrap();
        assert!(substitution.failure.is_none());
        let TypeKind::TemplateParameter {
            entity: invented,
            template: false,
        } = analyzer.graph.types.get(unique[0].unwrap()).kind
        else {
            panic!("unique dependent type argument")
        };
        assert_eq!(
            analyzer
                .graph
                .entity(invented)
                .template_parameter
                .unwrap()
                .depth(),
            0x7fff
        );
        assert!(analyzer.graph.entity(invented).scope.is_none());
        assert!(analyzer.graph.dependent_type(substituted));
        let TypeKind::Class(shell) = analyzer.graph.types.get(substituted).kind else {
            panic!("substituted class shell")
        };
        assert!(analyzer.graph.entity(shell).scope.is_none());
        assert_eq!(analyzer.graph.entity(alias).ty, Some(pattern));
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

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

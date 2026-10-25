// Translated from sema/analyzer_expressions.cpp and analyzer_literals.cpp. See NOTICE.
//! Procedural expression typing on original syntax IDs, with bounded native recursion.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{BindingKind, ScopeId},
    lookup::Filter,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::ast::{self, ExprKind},
};
impl Analyzer<'_> {
    pub(super) fn typed_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        if matches!(self.tree.expressions.get(id).kind, ExprKind::IntegerPack(_)) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "__integer_pack outside a template argument pack expansion",
            ));
        }
        let uses = self.dependent_uses.get();
        let result = self.typed_expression_form(id, scope);
        if matches!(
            self.tree.expressions.get(id).kind,
            ExprKind::Sizeof { .. }
                | ExprKind::SizeofPack(_)
                | ExprKind::TypeTrait { .. }
                | ExprKind::TypePredicate { .. }
                | ExprKind::OffsetOf { .. }
                | ExprKind::Delete { .. }
                | ExprKind::Throw(_)
        ) {
            self.dependent_uses.set(uses);
        }
        result?;
        let mut facts = self.graph.expression_facts.get(id)?;
        // new_facts completes a class-template object's definition, except
        // the designated final call in decltype (5.2.2p11). A pointer or a
        // function designation still needs only its declared type.
        if self.decltype_call != Some(id)
            && !matches!(self.tree.expressions.get(id).kind, ExprKind::Paren(_))
            && let TypeKind::Class(class) = self.graph.types.get(facts.ty).kind
            && self.graph.class_templates.instance(class).is_some()
        {
            self.complete_class(class)?;
        }
        facts.dependence.note(self.dependent_uses.get() != uses);
        self.graph.expression_facts.set(id, facts);
        Ok(facts)
    }
    fn expression_already_typed(&mut self, id: ast::ExprId) -> Result<bool, Error> {
        if let Some(facts) = self.graph.expression_facts.known(id)
            && facts.origin == super::expression_facts::ExpressionOrigin::Implicit
        {
            if self.deduced_initializer == Some(id) {
                self.deduced_initializer = None;
            }
            if facts.dependence.is_dependent() {
                self.note_dependent_use();
            }
            return Ok(true);
        }
        if self.pretyped_callable == Some(id) || self.pretyped_object == Some(id) {
            return self.graph.expression_facts.get(id).map(|_| true);
        }
        if self.deduced_initializer == Some(id) {
            self.deduced_initializer = None;
            return self.graph.expression_facts.get(id).map(|_| true);
        }
        Ok(false)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One semantic source-expression dispatch retains parse-once facts"
    )]
    fn typed_expression_form(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<(), Error> {
        if self.expression_already_typed(id)? {
            return Ok(());
        }
        super::record(&mut self.graph.expression_scopes, id.index(), scope);
        let node = *self.tree.expressions.get(id);
        self.check_stack().map_err(|error| match error {
            Error::Limit("constructs nested too deeply") => {
                Error::at(node.location, "constructs nested too deeply")
            }
            other => other,
        })?;
        let mut facts = match node.kind {
            ExprKind::Statement(body) => self.statement_expression(id, body, scope)?,
            ExprKind::Lambda(lambda) => self.lambda_expression(id, lambda, scope)?,
            ExprKind::Paren(inner) => {
                self.typed_expression(inner, scope)?;
                self.graph.expression_facts.share(id, inner);
                return Ok(());
            }
            ExprKind::Id(name) => {
                let found = self.lookup_result(name, scope, Filter::Ordinary)?;
                self.require_named_result(id, found.functions.as_slice())?;
                self.named_expression(id, &found, scope)?
            }
            ExprKind::Literal(literal) => self.literal_expression(id, literal, scope)?,
            ExprKind::Keyword(keyword @ (Keyword::True | Keyword::False | Keyword::Nullptr)) => {
                let scalar = if keyword == Keyword::Nullptr {
                    Scalar::Nullptr
                } else {
                    Scalar::Bool
                };
                let ty = self.graph.types.scalar(scalar)?;
                self.new_expression_facts(ty, V::Prvalue)?
            }
            ExprKind::Keyword(Keyword::This) => self.this_expression(node.location)?,
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand } => {
                if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::BitAnd)
                    && let Some(facts) = self.qualified_function_address(id, operand, scope)?
                {
                    self.graph.expression_facts.set(id, facts);
                    super::record(&mut self.graph.expression_types, id.index(), facts.ty);
                    return Ok(());
                }
                self.typed_expression(operand, scope)?;
                let Some(op) = op.token.as_punctuator() else {
                    return Err(Error::Invariant("nonpunctuator unary"));
                };
                let postfix = matches!(node.kind, ExprKind::Postfix { .. });
                if let Some(facts) = self.overloaded_operator(id, op, &[operand], postfix, scope)? {
                    facts
                } else {
                    self.scalar_unary(id, op, operand, postfix)?
                }
            }
            ExprKind::Binary {
                assignment: false, ..
            } => return self.scalar_binary_chain(id, scope),
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: true,
            } => {
                self.typed_expression(left, scope)?;
                self.check_resolved_function(left)?;
                let facts = self.graph.expression_facts.get(left)?;
                let Some(op) = op.token.as_punctuator() else {
                    return Err(Error::Invariant("nonpunctuator assignment"));
                };
                if matches!(
                    self.tree.expressions.get(right).kind,
                    ExprKind::Braced { .. }
                ) {
                    if op == crate::lex::pp_lexer::Punctuator::Assign
                        && self.graph.class_of(facts.ty).is_some()
                    {
                        self.type_call_argument(right, scope)?;
                        self.overloaded_operator(id, op, &[left, right], false, scope)?
                            .ok_or_else(|| {
                                Error::at(
                                    node.location,
                                    "no viable assignment operator for the braced-init-list",
                                )
                            })?
                    } else {
                        self.scalar_list_assignment(id, op, facts, right, scope)?
                    }
                } else {
                    self.typed_expression(right, scope)?;
                    if let Some(facts) =
                        self.overloaded_operator(id, op, &[left, right], false, scope)?
                    {
                        facts
                    } else if op == crate::lex::pp_lexer::Punctuator::Assign
                        && self.graph.class_of(facts.ty).is_some()
                    {
                        return Err(Error::at(
                            node.location,
                            "no viable assignment operator for the class object",
                        ));
                    } else {
                        self.scalar_assignment(id, op, left, right)?
                    }
                }
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.typed_expression(condition, scope)?;
                self.convert_bool(condition)?;
                self.typed_expression(then_expr, scope)?;
                self.check_resolved_function(then_expr)?;
                self.typed_expression(else_expr, scope)?;
                self.check_resolved_function(else_expr)?;
                let a = self.graph.expression_facts.get(then_expr)?;
                let b = self.graph.expression_facts.get(else_expr)?;
                self.scalar_conditional(id, [then_expr, else_expr], &[a, b], scope)?
            }
            ExprKind::Member { base, op, member } => {
                self.data_member_expression(id, base, op, member, scope)?
            }
            ExprKind::Subscript { base, index } => {
                self.typed_expression(base, scope)?;
                self.check_resolved_function(base)?;
                let braced = matches!(
                    self.tree.expressions.get(index).kind,
                    ExprKind::Braced { .. }
                );
                if braced
                    && self
                        .graph
                        .class_of(self.graph.expression_facts.get(base)?.ty)
                        .is_none()
                {
                    return Err(Error::at(
                        self.tree.expressions.get(index).location,
                        "a braced-init-list subscript of no class object",
                    ));
                }
                self.type_call_argument(index, scope)?;
                if let Some(facts) = self.overloaded_operator(
                    id,
                    crate::lex::pp_lexer::Punctuator::LeftBracket,
                    &[base, index],
                    false,
                    scope,
                )? {
                    facts
                } else if braced {
                    return Err(Error::at(
                        node.location,
                        "no viable subscript operator for the braced-init-list",
                    ));
                } else {
                    self.scalar_subscript(id, base, index)?
                }
            }
            ExprKind::TypeTrait {
                keyword: Keyword::Typeid,
                ty,
                operand,
            } => self.typeid_expression(id, ty, operand, scope)?,
            ExprKind::TypeTrait {
                keyword: Keyword::Noexcept,
                operand: Some(operand),
                ..
            } => self.noexcept_expression(id, operand, scope)?,
            ExprKind::TypeTrait {
                keyword: Keyword::Alignof,
                ty,
                operand,
            } => self.semantic_alignment(ty, operand, scope, node.location)?,
            ExprKind::Typeof { ty, operand } => self.typeof_expression(id, ty, operand, scope)?,
            ExprKind::TypeTransform { transform, types } => {
                self.transform_expression(id, transform, types, scope)?
            }
            ExprKind::TypePredicate { predicate, types } => {
                self.predicate_expression(id, predicate, types, scope)?
            }
            ExprKind::OffsetOf { designator, .. } => {
                self.offsetof_expression(id, designator, scope)?
            }
            ExprKind::OffsetOfBase(ty) => {
                let ty = self.type_syntax(ty, scope)?;
                self.new_expression_facts(ty, V::Lvalue)?
            }
            ExprKind::ComplexPart { imaginary, operand } => {
                self.complex_component(id, operand, imaginary, scope)?
            }
            ExprKind::BuiltinValue {
                builtin,
                operand,
                ty,
            } => self.builtin_value_expression(id, builtin, operand, ty, scope)?,
            ExprKind::VaArg { operand, ty } => self.va_arg_expression(id, operand, ty, scope)?,
            ExprKind::SizeofPack(name) => self.sizeof_pack_expression(id, name, scope)?,
            ExprKind::Sizeof {
                ty,
                operand,
                parenthesized,
            } => self.semantic_size(ty, operand, parenthesized, scope, node.location)?,
            ExprKind::Call { callee, arguments } => {
                self.procedural_call(id, callee, arguments, scope)?
            }
            ExprKind::Cast {
                keyword,
                ty,
                operand,
            } => {
                let target = self.type_syntax(ty, scope)?;
                self.explicit_scalar_conversion(id, target, operand, keyword, scope)?
            }
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                let target = self.specifiers(specifiers, scope, false)?.ty;
                self.scalar_functional_cast(id, target, arguments, braced, scope)?
            }
            ExprKind::Braced { .. } => {
                return Err(Error::at(
                    node.location,
                    "a braced-init-list is not an expression here",
                ));
            }
            ExprKind::New { .. } => self.allocation_expression(id, scope)?,
            ExprKind::Delete { .. } => self.deletion_expression(id, scope)?,
            ExprKind::Throw(operand) => self.throw_expression(id, operand, scope)?,
            // A concrete list copy expands every bound pattern. An ellipsis
            // still present here (for example, `(sizeof...(N))...`, which
            // names no unexpanded pack) reaches the frozen expression
            // dispatcher's ordinary unsupported-expression diagnostic.
            ExprKind::Fold { .. } => {
                return Err(Error::UnconditionalDiagnostic {
                    location: node.location,
                    message: "a fold-expression without a pack to expand",
                });
            }
            ExprKind::PackExpansion(_) => {
                return Err(Error::at(node.location, "unsupported expression"));
            }
            _ => return Err(Error::Missing("extended semantic expressions")),
        };
        // A constexpr initializer's printed cv type is applied by its declaration view,
        // never by mutating the operand's canonical semantic type.
        facts.ty = self.graph.types.canonical_function(facts.ty)?;
        self.graph.expression_facts.set(id, facts);
        super::record(&mut self.graph.expression_types, id.index(), facts.ty);
        Ok(())
    }
    fn this_expression(
        &mut self,
        location: crate::support::source::SourceOffset,
    ) -> Result<ExprFacts, Error> {
        self.note_dependent_use();
        let object = self
            .current_object_type()?
            .ok_or_else(|| Error::at(location, "`this` outside a non-static member function"))?;
        let pointer = self.graph.types.pointer(object)?;
        self.capture_this(location)?;
        self.new_expression_facts(pointer, V::Prvalue)
    }
    fn sizeof_pack_expression(
        &mut self,
        id: ast::ExprId,
        name: crate::lex::identifiers::NameId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let count = self.pack_size(name, scope)?.ok_or_else(|| {
            Error::at(
                self.tree.expressions.get(id).location,
                "sizeof... of a name that is no parameter pack",
            )
        })?;
        let ty = self.graph.types.scalar(Scalar::Basic(
            crate::types::FundamentalType::UnsignedLongInt,
        ))?;
        let mut facts = self.new_expression_facts(ty, V::Prvalue)?;
        facts.query_value = Some(count);
        Ok(facts)
    }
    pub(super) fn named_expression(
        &mut self,
        id: ast::ExprId,
        found: &super::lookup::LookupResult,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let binding = found
            .binding
            .ok_or_else(|| Error::at(self.tree.expressions.get(id).location, "undeclared name"))?;
        if self.graph.binding(binding).kind == BindingKind::Function {
            if found.functions.as_slice().len() == 1 && self.callee_name != Some(id) {
                self.check_intrinsic_designation(binding, self.tree.expressions.get(id).location)?;
            }
            if found.functions.as_slice().len() == 1
                && self.callee_name != Some(id)
                && self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .availability
                    == super::graph::FunctionAvailability::Deleted
                && self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .template_head
                    .is_none()
            {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "use of a deleted function",
                ));
            }
            let mut facts = self.function_designation(binding, found.functions.as_slice())?;
            // Static functions still name a member through the written class.
            // Keep that access identity even though they need no object.
            facts.member_naming = found
                .qualifier
                .filter(|&home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
                .and_then(|home| self.graph.scope(home).owner);
            if found
                .functions
                .as_slice()
                .iter()
                .any(|&function| self.member_function_owner(function).is_some())
            {
                facts.object = self.implicit_member_object(binding)?;
                if let Some(object) = facts.object {
                    facts.member_naming = found
                        .qualifier
                        .filter(|&home| {
                            self.graph.scope(home).kind == super::graph::ScopeKind::Class
                        })
                        .and_then(|home| self.graph.scope(home).owner)
                        .or_else(|| self.graph.class_of(object.ty));
                    // template_callee does not bind an implicit receiver until
                    // template_id_call resolves it. A static specialization can
                    // win even if lookup's first declaration is nonstatic.
                    let template_callee = self.callee_name == Some(id)
                        && matches!(self.tree.expressions.get(id).kind, ExprKind::Id(name)
                            if self.tree.name_lists.get(self.tree.names.get(name).components)
                                .last().is_some_and(|component| component.arguments.is_some()));
                    if let (Some(class), Some(naming)) =
                        (self.graph.class_of(object.ty), facts.member_naming)
                        && self.member_function_owner(binding).is_some()
                        && !template_callee
                    {
                        self.check_member_subobject(
                            binding,
                            class,
                            naming,
                            scope,
                            self.tree.expressions.get(id).location,
                        )?;
                    }
                    facts.category = V::Prvalue;
                }
            }
            return Ok(facts);
        }
        // Qualified class lookup already checked access through its naming
        // class. A second direct check would lose that path and its frontier.
        if found
            .qualifier
            .is_none_or(|home| self.graph.scope(home).kind != super::graph::ScopeKind::Class)
        {
            self.check_declaration_direct_access(
                binding,
                scope,
                self.tree.expressions.get(id).location,
            )?;
        }
        let binding = self.graph.binding(binding);
        if matches!(
            binding.kind,
            BindingKind::Namespace | BindingKind::NamespaceAlias
        ) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "namespace name used as a value",
            ));
        }
        if matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "type name used as a value",
            ));
        }
        if let Some(argument) = self.graph.constant_addresses.argument(binding.entity) {
            let TypeKind::Address { ty, entity } = self.graph.types.get(argument).kind else {
                return Err(Error::Invariant("address binding without argument"));
            };
            return if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
                let mut facts = self.new_expression_facts(element, V::Lvalue)?;
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) {
                    facts.function = Some(self.graph.function_origin(entity)?);
                }
                Ok(facts)
            } else {
                self.new_expression_facts(ty, V::Prvalue)
            };
        }
        let mut ty = binding
            .ty
            .ok_or(Error::Invariant("value binding without type"))?;
        if binding.kind == BindingKind::Variable {
            if self.graph.entity(binding.entity).template_head.is_some() {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "a variable template named without template arguments",
                ));
            }
            self.use_instance_variable(binding.entity)?;
            ty = self
                .graph
                .entity(binding.entity)
                .ty
                .ok_or(Error::Invariant("variable without type"))?;
        }
        if binding.kind == BindingKind::Parameter {
            ty = self.graph.types.parameter_object(ty)?;
        }
        if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        // new_facts requires a class-template object's definition, including
        // an object designated through a reference parameter. Pointer shells
        // and decltype's uncalled function result remain independent demands.
        if let TypeKind::Class(class) = self.graph.types.get(ty).kind {
            self.complete_class(class)?;
        }
        let category = if matches!(
            binding.kind,
            BindingKind::Enumerator | BindingKind::TemplateValue
        ) {
            V::Prvalue
        } else {
            V::Lvalue
        };
        let mut facts = self.new_expression_facts(ty, category)?;
        if binding.kind == BindingKind::Variable {
            self.static_member_constant(&mut facts, binding.entity);
        }
        if binding.kind == BindingKind::Variable
            && let Some(storage) = self.graph.injected_member_storage(binding.entity)?
        {
            facts.anonymous_member = Some(storage);
            facts.member = Some(
                found
                    .binding
                    .ok_or(Error::Invariant("injected member without binding"))?,
            );
        }
        if facts.anonymous_member.is_none()
            && self.graph.entity(binding.entity).data_member
                == Some(super::graph::DataMemberKind::Instance)
        {
            let selected = found
                .binding
                .ok_or(Error::Invariant("implicit member without binding"))?;
            self.implicit_data_member(id, &mut facts, selected, found.qualifier, scope)?;
        }
        if facts.object.is_some() || facts.anonymous_member.is_some() {
            facts.field_identity = self.graph.entity(binding.entity).bit_field;
            facts.set_bit_field(facts.field_identity.is_some());
        }
        if let Some(alias) = self.graph.lower_binding_alias(binding.entity)
            && let super::bindings::BindingTarget::Member { member, .. } = alias.target
        {
            let bit_field = self.graph.entity(member).bit_field.is_some();
            facts.set_bit_field(bit_field);
            if bit_field {
                facts.member = found.binding;
            }
        }
        self.local_name(
            id,
            &mut facts,
            found
                .binding
                .ok_or(Error::Invariant("named expression without binding"))?,
            scope,
        )?;
        if matches!(
            binding.kind,
            BindingKind::Enumerator | BindingKind::TemplateValue
        ) {
            let super::constants::ConstantState::Value(value) =
                self.graph.entity(binding.entity).constant
            else {
                return Err(Error::Invariant("enumerator without value"));
            };
            facts.ty = value.ty;
            facts.folded = self.held_integral_constant(value).then_some(value.bits);
        }
        Ok(facts)
    }
    pub(super) fn new_expression_facts(
        &mut self,
        ty: TypeId,
        category: V,
    ) -> Result<ExprFacts, Error> {
        let ty = self.graph.types.canonical_function(ty)?;
        Ok(ExprFacts::new(ty, category))
    }
    pub(super) fn scalar_binary_chain(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let base = self.binary_spine.len();
        let uses = self.dependent_uses.get();
        let result = (|| {
            let mut current = id;
            while let ExprKind::Binary {
                left,
                assignment: false,
                ..
            } = self.tree.expressions.get(current).kind
            {
                if self.pretyped_callable == Some(current) || self.pretyped_object == Some(current)
                {
                    break;
                }
                self.binary_spine.push(current);
                current = left;
            }
            self.typed_expression(current, scope)?;
            for index in (base..self.binary_spine.len()).rev() {
                let current = self.binary_spine[index];
                super::record(&mut self.graph.expression_scopes, current.index(), scope);
                let ExprKind::Binary {
                    op, left, right, ..
                } = self.tree.expressions.get(current).kind
                else {
                    return Err(Error::Invariant("binary spine changed"));
                };
                self.typed_expression(right, scope)?;
                let Some(op) = op.token.as_punctuator() else {
                    return Err(Error::Invariant("nonpunctuator binary"));
                };
                let mut facts = if let Some(facts) =
                    self.overloaded_operator(current, op, &[left, right], false, scope)?
                {
                    facts
                } else {
                    self.scalar_binary(current, op, left, right, scope)?
                };
                facts.dependence.note(self.dependent_uses.get() != uses);
                self.graph.expression_facts.set(current, facts);
                super::record(&mut self.graph.expression_types, current.index(), facts.ty);
            }
            Ok(())
        })();
        self.binary_spine.truncate(base);
        result
    }
}

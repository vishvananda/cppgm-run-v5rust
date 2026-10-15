// Translated from sema/analyzer_expressions.cpp and analyzer_literals.cpp. See NOTICE.
//! Procedural expression typing on original syntax IDs, with bounded native recursion.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{BindingKind, ScopeId},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
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
        if self.deduced_initializer == Some(id) {
            self.deduced_initializer = None;
            return self.graph.expression_facts.get(id);
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
            ExprKind::Paren(inner) => {
                let facts = self.typed_expression(inner, scope)?;
                self.graph.expression_facts.share(id, inner);
                return Ok(facts);
            }
            ExprKind::Id(name) => {
                let found = self.lookup_result(name, scope, Filter::Ordinary)?;
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
            ExprKind::Keyword(Keyword::This) => {
                let object = self.current_object_type()?.ok_or_else(|| {
                    Error::at(node.location, "`this` outside a non-static member function")
                })?;
                let pointer = self.graph.types.pointer(object)?;
                self.new_expression_facts(pointer, V::Prvalue)?
            }
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand } => {
                if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::BitAnd)
                    && let Some(facts) = self.qualified_function_address(id, operand, scope)?
                {
                    self.graph.expression_facts.set(id, facts);
                    super::record(&mut self.graph.expression_types, id.index(), facts.ty);
                    return Ok(facts);
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
                keyword: Keyword::Noexcept,
                operand: Some(operand),
                ..
            } => self.noexcept_expression(id, operand, scope)?,
            ExprKind::TypeTrait {
                keyword: Keyword::Alignof,
                ty,
                operand,
            } => self.semantic_alignment(ty, operand, scope, node.location)?,
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
            _ => return Err(Error::Missing("extended semantic expressions")),
        };
        // A constexpr initializer's printed cv type is applied by its declaration view,
        // never by mutating the operand's canonical semantic type.
        facts.ty = self.graph.types.canonical_function(facts.ty)?;
        self.graph.expression_facts.set(id, facts);
        super::record(&mut self.graph.expression_types, id.index(), facts.ty);
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
            if found.functions.as_slice().len() == 1
                && matches!(
                    self.graph
                        .entity(self.graph.binding(binding).entity)
                        .builtin,
                    Some(super::builtin_calls::BuiltinFunction::Floating(_))
                )
            {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "a builtin expanded where it is called named other than in a call",
                ));
            }
            if found.functions.as_slice().len() == 1
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
            if found
                .functions
                .as_slice()
                .iter()
                .any(|&function| self.member_function_owner(function).is_some())
            {
                facts.object = self.implicit_member_object(binding)?;
                if let Some(object) = facts.object {
                    facts.member_naming = self.graph.class_of(object.ty);
                    facts.category = V::Prvalue;
                }
            }
            return Ok(facts);
        }
        self.check_declaration_direct_access(
            binding,
            scope,
            self.tree.expressions.get(id).location,
        )?;
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
        let mut ty = binding
            .ty
            .ok_or(Error::Invariant("value binding without type"))?;
        if binding.kind == BindingKind::Variable {
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
        let category = if binding.kind == BindingKind::Enumerator {
            V::Prvalue
        } else {
            V::Lvalue
        };
        let mut facts = self.new_expression_facts(ty, category)?;
        if binding.kind == BindingKind::Variable {
            self.static_member_constant(&mut facts, binding.entity);
        }
        if binding.kind == BindingKind::Variable
            && let Some(home) = self.graph.entity(binding.entity).home
            && let Some(owner) = self.graph.scope(home).owner
            && let Some(storage) = self.graph.entity(owner).anonymous_storage
            && !matches!(
                self.graph.scope(self.graph.binding_scope(storage)).kind,
                super::graph::ScopeKind::Class
            )
        {
            facts.anonymous_member = Some(storage);
        }
        if facts.anonymous_member.is_none()
            && self.graph.entity(binding.entity).data_member
                == Some(super::graph::DataMemberKind::Instance)
        {
            let selected = found
                .binding
                .ok_or(Error::Invariant("implicit member without binding"))?;
            if let Some(object) = self.implicit_member_object(selected)? {
                facts.object = Some(object);
                facts.member = Some(selected);
                facts.member_naming = self.graph.class_of(object.ty);
                if let Some(class) = facts.member_naming {
                    facts.member_path = self.data_member_path(
                        selected,
                        class,
                        self.tree.expressions.get(id).location,
                    )?;
                }
                if !matches!(
                    self.graph
                        .types
                        .get(
                            self.graph
                                .entity(binding.entity)
                                .ty
                                .ok_or(Error::Invariant("member without type"))?
                        )
                        .kind,
                    TypeKind::Reference { .. }
                ) {
                    let cv = self.graph.types.get(object.ty).cv;
                    let cv = Cv(cv.0
                        & if self.graph.entity(binding.entity).mutability
                            == super::graph::Mutability::Mutable
                        {
                            !Cv::CONST.0
                        } else {
                            u8::MAX
                        });
                    facts.ty = self.graph.types.qualify(ty, cv)?;
                }
            } else if self.unevaluated == 0 {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "non-static data member used without an object",
                ));
            }
        }
        if facts.object.is_some() || facts.anonymous_member.is_some() {
            facts.field_identity = self.graph.entity(binding.entity).bit_field;
            facts.set_bit_field(facts.field_identity.is_some());
        }
        self.local_name(
            id,
            &mut facts,
            found
                .binding
                .ok_or(Error::Invariant("named expression without binding"))?,
            scope,
        )?;
        if binding.kind == BindingKind::Enumerator {
            let super::constants::ConstantState::Value(value) =
                self.graph.entity(binding.entity).constant
            else {
                return Err(Error::Invariant("enumerator without value"));
            };
            facts.ty = value.ty;
            facts.folded = Some(value.bits);
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
    ) -> Result<ExprFacts, Error> {
        let base = self.binary_spine.len();
        let result = (|| {
            let mut current = id;
            while let ExprKind::Binary {
                left,
                assignment: false,
                ..
            } = self.tree.expressions.get(current).kind
            {
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
                let facts = if let Some(facts) =
                    self.overloaded_operator(current, op, &[left, right], false, scope)?
                {
                    facts
                } else {
                    self.scalar_binary(current, op, left, right)?
                };
                self.graph.expression_facts.set(current, facts);
                super::record(&mut self.graph.expression_types, current.index(), facts.ty);
            }
            self.graph.expression_facts.get(id)
        })();
        self.binary_spine.truncate(base);
        result
    }
}

// Translated from constant_evaluator.cpp::constant_conversion and
// lower/expression_lowering.cpp's evaluated constexpr pointer query. See NOTICE.
//! Read completed constant and body facts without evaluating or replaying syntax.
use super::{
    Error,
    constant_addresses::Root,
    expression_facts::{FoldStyle, ValueCategory},
    graph::{BindingKind, DataMemberKind, Graph, ScopeKind},
    specifiers::Explicitness,
    static_initialization::Value,
    types::{Scalar, TypeKind},
};
use crate::syntax::ast::{self, ExprKind, StmtKind};

fn without_parentheses(tree: &ast::Tree, mut id: ast::ExprId) -> ast::ExprId {
    while let ExprKind::Paren(inner) = tree.expressions.get(id).kind {
        id = inner;
    }
    id
}

impl Graph {
    #[must_use]
    pub fn lower_extended_temporary(
        &self,
        entity: super::graph::EntityId,
    ) -> Option<super::constexpr_values::TemporaryId> {
        let address = self.constant_addresses.value(entity)?;
        match address.root {
            Root::Published(id) if address.offset == 0 => Some(id),
            _ => None,
        }
    }
    #[inline]
    #[must_use]
    pub fn lower_evaluated_reference_temporary(&self, entity: super::graph::EntityId) -> bool {
        self.constant_addresses.reference_evaluated(entity)
    }
    pub fn lower_builtin_floating(&self, id: ast::ExprId) -> Result<u128, Error> {
        let value = self.expression_facts.floating_value(id)?;
        let value = self.floating_constants.get(value);
        let super::types::TypeKind::Scalar(super::types::Scalar::Basic(ty)) =
            self.types.get(value.ty).kind
        else {
            return Err(Error::Invariant("floating builtin type"));
        };
        value
            .value
            .bits(
                crate::types::float_value::Format::of(ty)
                    .ok_or(Error::Invariant("floating builtin format"))?,
            )
            .ok_or(Error::Invariant("floating builtin encoding"))
    }
    #[must_use]
    pub fn lower_member_argument(
        &self,
        tree: &ast::Tree,
        id: ast::ExprId,
    ) -> Option<super::symbolic_members::MemberPointer> {
        let id = without_parentheses(tree, id);
        let ExprKind::Id(name) = tree.expressions.get(id).kind else {
            return None;
        };
        let binding = self.resolved_name(name)?;
        self.argument_member_pointer(self.binding(binding).entity)
    }

    pub fn lower_evaluated_member_pointer(
        &self,
        tree: &ast::Tree,
        id: ast::ExprId,
    ) -> Result<Option<super::symbolic_members::MemberPointer>, Error> {
        let id = without_parentheses(tree, id);
        let facts = self.expression_facts.get(id)?;
        if let Some(value) = self.lower_member_argument(tree, id) {
            return Ok(Some(value));
        }
        if !matches!(
            self.types.get(facts.ty).kind,
            TypeKind::MemberPointer { .. }
        ) || facts.category == ValueCategory::Prvalue
            || self.types.get(facts.ty).cv.0 & 2 != 0
        {
            return Ok(None);
        }
        let entity = match tree.expressions.get(id).kind {
            ExprKind::Id(name) => self.lower_name(name)?,
            ExprKind::Member { .. } => self.lower_member(id)?.0,
            _ => return Ok(None),
        };
        let variable = self.entity(entity);
        if !variable.constexpr
            || !self.lower_static_member(entity)
            || matches!(
                self.types.get(self.lower_entity_type(entity)?).kind,
                TypeKind::Reference { .. }
            )
        {
            return Ok(None);
        }
        Ok(match variable.object().constant {
            super::constants::ConstantState::MemberPointer(value) => Some(value),
            _ => None,
        })
    }
    pub fn lower_evaluated_wide(
        &self,
        tree: &ast::Tree,
        id: ast::ExprId,
    ) -> Result<Option<u128>, Error> {
        let id = without_parentheses(tree, id);
        let facts = self.expression_facts.get(id)?;
        let Some(kind) = self.integral_kind(facts.ty) else {
            return Ok(None);
        };
        if kind.bits <= 64
            || facts.category == ValueCategory::Prvalue
            || self.types.get(facts.ty).cv.0 & 2 != 0
        {
            return Ok(None);
        }
        let entity = match tree.expressions.get(id).kind {
            ExprKind::Id(name) => self.lower_name(name)?,
            ExprKind::Member { .. } => self.lower_member(id)?.0,
            _ => return Ok(None),
        };
        if !self.entity(entity).constexpr {
            return Ok(None);
        }
        let super::constants::ConstantState::Value(value) = self.entity(entity).object().constant
        else {
            return Ok(None);
        };
        let extended = if kind.signed && value.bits >> 63 != 0 {
            u64::MAX
        } else {
            0
        };
        Ok((value.high != extended).then_some(value.wide()))
    }
    pub fn lower_evaluated_floating(
        &self,
        tree: &ast::Tree,
        id: ast::ExprId,
    ) -> Result<Option<u128>, Error> {
        let id = without_parentheses(tree, id);
        let facts = self.expression_facts.get(id)?;
        if facts.category == ValueCategory::Prvalue || self.types.get(facts.ty).cv.0 & 2 != 0 {
            return Ok(None);
        }
        let TypeKind::Scalar(Scalar::Basic(kind)) = self.types.get(facts.ty).kind else {
            return Ok(None);
        };
        let Some(format) = crate::types::float_value::Format::of(kind) else {
            return Ok(None);
        };
        let entity = match tree.expressions.get(id).kind {
            ExprKind::Id(name) => self.lower_name(name)?,
            ExprKind::Member { .. } => self.lower_member(id)?.0,
            _ => return Ok(None),
        };
        let variable = self.entity(entity);
        if !variable.constexpr
            || !self.lower_static_member(entity)
            || matches!(
                self.types.get(self.lower_entity_type(entity)?).kind,
                TypeKind::Reference { .. }
            )
        {
            return Ok(None);
        }
        let super::constants::ConstantState::Floating(value) = variable.object().constant else {
            return Ok(None);
        };
        self.floating_constants
            .get(value)
            .value
            .bits(format)
            .map(Some)
            .ok_or(Error::Invariant("completed floating constant encoding"))
    }
    pub fn lower_evaluated_reference(
        &self,
        entity: super::graph::EntityId,
    ) -> Result<Option<Value>, Error> {
        let variable = self.entity(entity);
        if !variable.constexpr
            || !matches!(
                self.types.get(self.lower_entity_type(entity)?).kind,
                TypeKind::Reference { .. }
            )
        {
            return Ok(None);
        }
        if variable.initialization().reference_temporary.is_some()
            && matches!(self.types.get(self.lower_entity_type(entity)?).kind,
                TypeKind::Reference { element, .. } if matches!(self.types.get(element).kind, TypeKind::MemberPointer { .. }))
        {
            return Ok(None);
        }
        if self.constant_addresses.read_through_storage(entity) {
            return Ok(None);
        }
        Ok(self
            .constant_addresses
            .value(entity)
            .and_then(|address| match address.root {
                Root::Variable(root) | Root::Function(root) => {
                    Some(Value::Address(root, address.offset))
                }
                Root::Published(root) => self.static_temporary_value(root, address.offset),
                Root::Literal(root) => Some(Value::Literal(root, address.offset)),
                Root::TypeInformation(ty) => Some(Value::TypeInformation(ty, address.offset)),
                _ => None,
            }))
    }
    pub fn lower_evaluated_pointer(
        &self,
        tree: &ast::Tree,
        id: ast::ExprId,
    ) -> Result<Option<Value>, Error> {
        let id = without_parentheses(tree, id);
        let facts = self.expression_facts.get(id)?;
        if let ExprKind::Id(name) = tree.expressions.get(id).kind
            && let Some(binding) = self.resolved_name(name)
            && let Some(argument) = self
                .constant_addresses
                .argument(self.binding(binding).entity)
            && let TypeKind::Address { ty, entity } = self.types.get(argument).kind
            && matches!(self.types.get(ty).kind, TypeKind::Pointer(_))
        {
            return Ok(Some(Value::Address(entity, 0)));
        }
        if facts.category == ValueCategory::Prvalue
            || self.types.get(facts.ty).cv.0 & 2 != 0
            || !matches!(
                self.types.get(facts.ty).kind,
                TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr)
            )
        {
            return Ok(None);
        }
        let entity = match tree.expressions.get(id).kind {
            ExprKind::Id(name) => self.lower_name(name)?,
            ExprKind::Member { .. } => self.lower_member(id)?.0,
            _ => return Ok(None),
        };
        let variable = self.entity(entity);
        if variable.object().variable.is_none()
            || !variable.constexpr
            || variable
                .ty
                .is_some_and(|ty| matches!(self.types.get(ty).kind, TypeKind::Reference { .. }))
            || matches!(tree.expressions.get(id).kind, ExprKind::Member { .. })
                && variable.object().data_member != Some(DataMemberKind::Static)
        {
            return Ok(None);
        }
        let Some(address) = self.constant_addresses.value(entity) else {
            return Ok(None);
        };
        Ok(match address.root {
            Root::Null => Some(Value::Integer(0)),
            Root::Published(root) => self.static_temporary_value(root, address.offset),
            Root::Literal(root) => Some(Value::Literal(root, address.offset)),
            Root::TypeInformation(ty) => Some(Value::TypeInformation(ty, address.offset)),
            Root::Function(root) => Some(Value::Address(root, address.offset)),
            Root::Variable(root)
                if self.entity(root).home.is_some_and(|scope| {
                    matches!(
                        self.scope(scope).kind,
                        ScopeKind::Namespace | ScopeKind::Class
                    )
                }) && self.entity(root).object().variable.is_some_and(|variable| {
                    variable.thread_storage != super::specifiers::ThreadStorage::ThreadLocal
                }) =>
            {
                Some(Value::Address(root, address.offset))
            }
            _ => None,
        })
    }

    /// The frozen course's narrow empty-class conversion rule. This is a view
    /// of an already analyzed conversion body, independent of constexpr calls.
    pub fn lower_constant_conversion(
        &self,
        tree: &ast::Tree,
        condition: ast::ExprId,
    ) -> Result<Option<bool>, Error> {
        let id = without_parentheses(tree, condition);
        let Some(step) = self.expression_facts.function_conversion(id) else {
            return Ok(None);
        };
        let facts = self.expression_facts.get(id)?;
        let Some(class) = self.class_of(facts.ty) else {
            return Ok(None);
        };
        let temporary =
            match tree.expressions.get(id).kind {
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => tree.expr_lists.get(arguments).is_empty() && braced.is_none_or(|braced| {
                    matches!(tree.expressions.get(braced).kind, ExprKind::Braced { elements, .. }
                            if tree.expr_lists.get(elements).is_empty())
                })
                    && facts
                        .constructor(&self.expression_facts)
                        .is_none_or(|id| self.lower_constructor_trivial(id))
                    && !self.has_nontrivial_destructor(class),
                _ => false,
            };
        let function = self.binding(step.binding).entity;
        let TypeKind::Function { result, .. } =
            self.types.get(self.lower_function(function)?.ty).kind
        else {
            return Err(Error::Invariant("conversion function without signature"));
        };
        if (!matches!(tree.expressions.get(id).kind, ExprKind::Id(_)) && !temporary)
            || !self.lower_class_empty(class)?
            || self.entity(function).callable().explicitness == Explicitness::Explicit
            || self.types.get(result).kind != TypeKind::Scalar(Scalar::Bool)
            || self.types.get(result).cv.0 != 0
        {
            return Ok(None);
        }
        let Some(body) = self.lower_function_body(function) else {
            return Ok(None);
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            return Ok(None);
        };
        if body.handlers.is_some() {
            return Ok(None);
        }
        let StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            return Ok(None);
        };
        let [statement] = tree.stmt_lists.get(items) else {
            return Ok(None);
        };
        let StmtKind::Return(Some(returned)) = tree.statements.get(*statement).kind else {
            return Ok(None);
        };
        let returned = without_parentheses(tree, returned);
        let ExprKind::Id(name) = tree.expressions.get(returned).kind else {
            return Ok(None);
        };
        let Ok(constant) = self.expression_facts.get(returned) else {
            return Ok(None);
        };
        let parameter = self.resolved_name(name).is_some_and(|binding| {
            self.binding(binding).kind == BindingKind::Enumerator
                && self
                    .entity(self.binding(binding).entity)
                    .enumerated()
                    .enumeration
                    .is_none()
        });
        Ok(constant
            .folded()
            .filter(|_| {
                (constant.fold_style() == FoldStyle::StaticMember || parameter)
                    && self.types.get(constant.ty).kind == TypeKind::Scalar(Scalar::Bool)
            })
            .map(|bits| bits != 0))
    }
}

// Shared by branch emission and call-table argument-effect demand. A literal
// shortcut is narrower than general constant evaluation in the frozen course.
impl Graph {
    pub fn lower_literal_truth(
        &self,
        literals: &crate::syntax::storage::TextStore,
        tree: &ast::Tree,
        mut id: ast::ExprId,
    ) -> Result<Option<bool>, Error> {
        use super::lowering::{LiteralCall, StaticValue, TypeView};
        use crate::lex::{PostTokenKind, vocabulary::Keyword};
        loop {
            id = without_parentheses(tree, id);
            let facts = self.lower_expression(id)?;
            let ty = self.lower_type(facts.ty)?;
            let floating = matches!(
                ty,
                TypeView::Scalar(Scalar::Basic(f)) if f.floating_rank() != 0
            );
            if let Some(shape) = self.lower_literal_call(id)? {
                return Ok((!floating && !matches!(ty, TypeView::Array { .. }))
                    .then_some(!matches!(shape, LiteralCall::String { .. })));
            }
            match tree.expressions.get(id).kind {
                ExprKind::Keyword(Keyword::True) => return Ok(Some(true)),
                ExprKind::Keyword(Keyword::False) => return Ok(Some(false)),
                ExprKind::Id(_) if facts.folded.is_some() => {
                    return Ok(facts.folded.map(|v| v != 0));
                }
                ExprKind::Id(_)
                    if matches!(
                        self.lower_evaluated_pointer(tree, id)?,
                        Some(StaticValue::Integer(0))
                    ) =>
                {
                    return Ok(Some(false));
                }
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } if !matches!(ty, TypeView::Class(_)) => {
                    let Some(selected) = self.lower_constant_conversion(tree, condition)? else {
                        return Ok(None);
                    };
                    id = if selected { then_expr } else { else_expr };
                }
                ExprKind::Literal(literal) if !floating => {
                    return Ok(match literal.kind(literals) {
                        PostTokenKind::Literal { data, .. }
                        | PostTokenKind::Character { data, .. } => {
                            Some(data.as_slice().iter().any(|&b| b != 0))
                        }
                        _ => None,
                    });
                }
                _ => return Ok(None),
            }
        }
    }
}

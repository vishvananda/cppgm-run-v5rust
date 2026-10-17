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
    pub(crate) fn lower_evaluated_pointer(
        &self,
        tree: &ast::Tree,
        id: ast::ExprId,
    ) -> Result<Option<Value>, Error> {
        let id = without_parentheses(tree, id);
        let facts = self.expression_facts.get(id)?;
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
        if variable.variable.is_none()
            || !variable.constexpr
            || variable
                .ty
                .is_some_and(|ty| matches!(self.types.get(ty).kind, TypeKind::Reference { .. }))
            || matches!(tree.expressions.get(id).kind, ExprKind::Member { .. })
                && variable.data_member != Some(DataMemberKind::Static)
        {
            return Ok(None);
        }
        let Some(address) = self.constant_addresses.value(entity) else {
            return Ok(None);
        };
        Ok(match address.root {
            Root::Null => Some(Value::Integer(0)),
            Root::Literal(root) => Some(Value::Literal(root, address.offset)),
            Root::Function(root) => Some(Value::Address(root, address.offset)),
            Root::Temporary(root) if self.entity(root).reference_temporary.is_some() => {
                Some(Value::Address(root, address.offset))
            }
            Root::Variable(root)
                if self.entity(root).home.is_some_and(|scope| {
                    matches!(
                        self.scope(scope).kind,
                        ScopeKind::Namespace | ScopeKind::Class
                    )
                }) && self.entity(root).variable.is_some_and(|variable| {
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
    pub(crate) fn lower_constant_conversion(
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
                        .constructor
                        .is_none_or(|id| self.lower_constructor_trivial(id))
                    && !self.lower_nontrivial_destructor(class)?,
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
            || self.entity(function).explicitness == Explicitness::Explicit
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
                    .enumeration
                    .is_none()
        });
        Ok(constant
            .folded
            .filter(|_| {
                (constant.fold_style == FoldStyle::StaticMember || parameter)
                    && self.types.get(constant.ty).kind == TypeKind::Scalar(Scalar::Bool)
            })
            .map(|bits| bits != 0))
    }
}

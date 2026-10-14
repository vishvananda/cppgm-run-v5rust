// Translated from lower/global_lowering.cpp's static_element/static_aggregate. See NOTICE.
//! Analysis-owned array images retain converted values and symbolic addresses in source order.
use super::{
    Error,
    analyzer::Analyzer,
    graph::EntityId,
    lowering::ScopeId,
    static_initialization::Value,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::PostTokenKind,
    syntax::ast::{self, ExprKind},
    types::FundamentalType as F,
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(crate) enum Item {
    Scalar(TypeId, Value),
    Zero(u64),
}
#[derive(Default)]
pub(super) struct Images {
    values: HashMap<EntityId, Vec<Item>>,
}
impl Images {
    pub(super) fn get(&self, entity: EntityId) -> Option<&[Item]> {
        self.values.get(&entity).map(Vec::as_slice)
    }
}
impl Analyzer<'_> {
    pub(super) fn record_array_image(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let (ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr)) =
            initializer.kind
        else {
            return Ok(());
        };
        let mut image = Vec::new();
        if self.static_array_element(target, Some(expr), scope, &mut image)? {
            self.graph.array_images.values.insert(entity, image);
        }
        Ok(())
    }
    fn static_array_element(
        &mut self,
        target: TypeId,
        expr: Option<ast::ExprId>,
        scope: ScopeId,
        out: &mut Vec<Item>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        if let TypeKind::Array { element, bound } = self.graph.types.get(target).kind {
            if let Some(expr) = expr {
                let root = self.graph.expression_facts.root(expr);
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(root).kind {
                    let list = self.tree.expr_lists.get(elements);
                    if list.len() == 1
                        && self.character_element(element)
                        && self.string_literal(list[0])
                    {
                        return self.static_array_string(target, list[0], out);
                    }
                    let mut next = 0;
                    return self.static_array_list(target, list, &mut next, scope, out);
                }
                return self.static_array_string(target, root, out);
            }
            for _ in 0..bound {
                if !self.static_array_element(element, None, scope, out)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        let value = if let Some(expr) = expr {
            let root = self.graph.expression_facts.root(expr);
            if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(root).kind {
                return self.static_array_element(
                    target,
                    self.tree.expr_lists.get(elements).first().copied(),
                    scope,
                    out,
                );
            }
            let Some(value) = self.static_scalar(expr, target, scope)? else {
                return Ok(false);
            };
            value
        } else {
            match self.graph.types.get(target).kind {
                TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr) => {
                    append_zero(out, 8)?;
                    return Ok(true);
                }
                TypeKind::Scalar(Scalar::Basic(ty @ (F::Float | F::Double | F::LongDouble))) => {
                    Value::Floating(ty, 0)
                }
                _ => Value::Integer(0),
            }
        };
        if matches!(value, Value::Integer(0))
            && matches!(
                self.graph.types.get(target).kind,
                TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr)
            )
        {
            append_zero(out, 8)?;
        } else {
            out.push(Item::Scalar(target, value));
        }
        Ok(true)
    }
    fn static_array_list(
        &mut self,
        target: TypeId,
        list: &[ast::ExprId],
        next: &mut usize,
        scope: ScopeId,
        out: &mut Vec<Item>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let TypeKind::Array { element, bound } = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("array image list without array"));
        };
        for _ in 0..bound {
            let expr = list.get(*next).copied();
            if let Some(expr) = expr
                && matches!(self.graph.types.get(element).kind, TypeKind::Array { .. })
                && !matches!(
                    self.tree.expressions.get(expr).kind,
                    ExprKind::Braced { .. }
                )
                && !(self.string_literal(expr)
                    && matches!(self.graph.types.get(element).kind,
                    TypeKind::Array { element, .. } if self.character_element(element)))
            {
                if !self.static_array_list(element, list, next, scope, out)? {
                    return Ok(false);
                }
            } else {
                if expr.is_some() {
                    *next += 1;
                }
                if !self.static_array_element(element, expr, scope, out)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    fn static_array_string(
        &mut self,
        target: TypeId,
        expr: ast::ExprId,
        out: &mut Vec<Item>,
    ) -> Result<bool, Error> {
        let TypeKind::Array { element, bound } = self.graph.types.get(target).kind else {
            return Ok(false);
        };
        let mut root = self.graph.expression_facts.root(expr);
        while let ExprKind::Paren(inner) = self.tree.expressions.get(root).kind {
            root = inner;
        }
        let ExprKind::Literal(literal) = self.tree.expressions.get(root).kind else {
            return Ok(false);
        };
        let PostTokenKind::String { ty, .. } = literal.kind else {
            return Ok(false);
        };
        let width = ty.size();
        let data = self.literal_bytes.get(literal.data);
        for unit in data
            .chunks_exact(width)
            .take(usize::try_from(bound).map_err(|_| Error::Limit("array bound"))?)
        {
            let mut bytes = [0u8; 8];
            bytes[..unit.len()].copy_from_slice(unit);
            out.push(Item::Scalar(
                element,
                Value::Integer(u64::from_le_bytes(bytes)),
            ));
        }
        let units = u64::try_from(data.len() / width).map_err(|_| Error::Limit("string units"))?;
        if bound > units {
            for _ in units..bound {
                out.push(Item::Scalar(element, Value::Integer(0)));
            }
        }
        Ok(true)
    }
}
fn append_zero(out: &mut Vec<Item>, bytes: u64) -> Result<(), Error> {
    if let Some(Item::Zero(prior)) = out.last_mut() {
        *prior = prior
            .checked_add(bytes)
            .ok_or(Error::Limit("array zero bytes"))?;
    } else if bytes != 0 {
        out.push(Item::Zero(bytes));
    }
    Ok(())
}

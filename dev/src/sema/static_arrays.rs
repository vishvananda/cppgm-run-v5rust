// Translated from lower/global_lowering.cpp's static_element/static_aggregate. See NOTICE.
//! Analysis-owned array images retain converted values and symbolic addresses in source order.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::Root,
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
    Table(EntityId),
    Scalar(TypeId, Value),
    Zero(u64),
    BitUnit(u8, u64),
}
#[derive(Default)]
pub(super) struct Images {
    values: HashMap<EntityId, Image>,
}
enum Image {
    Constant(Vec<Item>),
    Dynamic(Vec<Item>),
}
impl Images {
    pub(super) fn set_constant(&mut self, entity: EntityId, items: Vec<Item>) {
        self.values.insert(entity, Image::Constant(items));
    }
    pub(super) fn get(&self, entity: EntityId) -> Option<&[Item]> {
        match self.values.get(&entity) {
            Some(Image::Constant(items)) => Some(items),
            _ => None,
        }
    }
    pub(super) fn prefix(&self, entity: EntityId) -> Option<&[Item]> {
        match self.values.get(&entity) {
            Some(Image::Dynamic(items)) => Some(items),
            _ => None,
        }
    }
}
impl Analyzer<'_> {
    pub(super) fn record_union_default_image(
        &mut self,
        entity: EntityId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let Some(constructor) = self.graph.lower_construction(entity) else {
            return Ok(());
        };
        let plan = self.graph.lower_constructor(constructor);
        let Some((member, initializer)) = plan.members.iter().find_map(|&(member, initializer)| {
            initializer.map(|initializer| (member, initializer))
        }) else {
            return Ok(());
        };
        let ty = self.graph.lower_entity_type(member)?;
        let mut image = Vec::new();
        let expr = match initializer.kind {
            ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr) => Some(expr),
            ast::InitializerKind::Parenthesized(arguments) => {
                let [expr] = self.tree.expr_lists.get(arguments) else {
                    return Ok(());
                };
                Some(*expr)
            }
            _ => return Err(Error::Invariant("default member initializer shape")),
        };
        if !self.static_array_element(ty, expr, scope, &mut image)? {
            return Ok(());
        }
        let (size, _) =
            self.type_size(self.graph.lower_entity_type(entity)?, initializer.location)?;
        let (member_size, _) = self.type_size(ty, initializer.location)?;
        append_zero(
            &mut image,
            size.checked_sub(member_size)
                .ok_or(Error::Invariant("union member larger than storage"))?,
        )?;
        self.graph
            .array_images
            .values
            .insert(entity, Image::Constant(image));
        Ok(())
    }

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
        let empty = matches!(self.tree.expressions.get(expr).kind,
            ExprKind::Braced { elements, .. } if self.tree.expr_lists.get(elements).is_empty());
        let zeroed = empty
            && (self
                .graph
                .expression_facts
                .initializer_array(expr)
                .is_some()
                || self
                    .graph
                    .lower_construction(entity)
                    .is_some_and(|constructor| {
                        self.graph.lower_constructor_trivial(constructor)
                            && self.graph.lower_constructor(constructor).zero_before_value
                    }));
        let complete = if zeroed && self.graph.lower_variable(entity)?.thread_local {
            // The frozen static-global path does not probe evaluated class
            // initialization for TLS; its first-use initializer retains the work.
            false
        } else if zeroed {
            self.static_value_zero(target, &mut image)?
        } else if matches!(self.graph.types.get(target).kind, TypeKind::Class(_))
            && self
                .graph
                .lower_construction(entity)
                .is_some_and(|constructor| !self.graph.lower_constructor_trivial(constructor))
        {
            // A complete object selected for construction takes the evaluated
            // constant-initialization path, not static_element's nested member
            // list spelling (global_lowering.cpp::static_global).
            false
        } else {
            self.static_array_element(target, Some(expr), scope, &mut image)?
        };
        self.graph.array_images.values.insert(
            entity,
            if complete {
                Image::Constant(image)
            } else {
                Image::Dynamic(image)
            },
        );
        Ok(())
    }
    pub(super) fn static_array_element(
        &mut self,
        target: TypeId,
        expr: Option<ast::ExprId>,
        scope: ScopeId,
        out: &mut Vec<Item>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        if let TypeKind::Class(class) = self.graph.types.get(target).kind {
            if expr.is_none() && !self.graph.aggregate_class(class) {
                // An omitted nonaggregate subobject with trivial default
                // construction is zero bytes (missing_element), distinct from
                // the scalar zeros of an evaluated value-initialization.
                if !self
                    .graph
                    .lower_ready_default_constructor(class)
                    .is_some_and(|constructor| self.graph.lower_constructor_trivial(constructor))
                {
                    return Ok(false);
                }
                append_zero(
                    out,
                    self.type_size(target, crate::support::source::SourceOffset(0))?
                        .0,
                )?;
                return Ok(true);
            }
            if let Some(expr) = expr
                && let Some(complete) = self.static_zero_prvalue(target, expr, out)?
            {
                return Ok(complete);
            }
            let list = if let Some(expr) = expr {
                let root = self.graph.expression_facts.root(expr);
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(root).kind else {
                    return Ok(false);
                };
                self.tree.expr_lists.get(elements)
            } else {
                &[]
            };
            let constructor = expr
                .map(|id| self.graph.lower_list_constructor(id))
                .transpose()?
                .flatten();
            let argument_buffer = super::call_arguments::Arguments::new(list);
            return self.static_class_list(
                class,
                argument_buffer.as_slice(),
                &mut 0,
                scope,
                out,
                constructor,
            );
        }
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Reference { .. }
        ) {
            let Some(mut expr) = expr else {
                return Ok(false);
            };
            let root = self.graph.expression_facts.root(expr);
            if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(root).kind
                && self.graph.lower_expression(expr)?.is_place
            {
                expr = *self
                    .tree
                    .expr_lists
                    .get(elements)
                    .first()
                    .ok_or(Error::Invariant("empty static reference list"))?;
            }
            if self
                .graph
                .expression_facts
                .get(expr)?
                .conversion
                .is_some_and(|conversion| conversion.temporary)
            {
                return Ok(false);
            }
            let address = match self.constant_place(expr, scope) {
                Ok(address) => address,
                Err(Error::NonConstant { .. } | Error::Missing(_)) => return Ok(false),
                Err(error) => return Err(error),
            };
            let value = match address.root {
                Root::Variable(entity) if !self.graph.lower_automatic(entity) => {
                    Value::Address(entity, address.offset)
                }
                Root::Temporary(entity) | Root::Function(entity) => {
                    Value::Address(entity, address.offset)
                }
                Root::Published(id) => {
                    let Some(value) = self.graph.static_temporary_value(id, address.offset) else {
                        return Ok(false);
                    };
                    value
                }
                Root::Literal(expr) => Value::Literal(expr, address.offset),
                Root::TypeInformation(ty) => Value::TypeInformation(ty, address.offset),
                _ => return Ok(false),
            };
            out.push(Item::Scalar(target, value));
            return Ok(true);
        }
        if let TypeKind::Array { element, bound, .. } = self.graph.types.get(target).kind {
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
                    let argument_buffer = super::call_arguments::Arguments::new(list);
                    return self.static_array_list(
                        target,
                        argument_buffer.as_slice(),
                        &mut next,
                        scope,
                        out,
                    );
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
                TypeKind::MemberPointer { .. } => Value::MemberPointer(
                    super::symbolic_members::MemberPointer::null(self.bare_type(target)?),
                ),
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
        let TypeKind::Array { element, bound, .. } = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("array image list without array"));
        };
        for _ in 0..bound {
            let expr = list.get(*next).copied();
            if let Some(expr) = expr
                && matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Array { .. } | TypeKind::Class(_)
                )
                && !matches!(
                    self.tree.expressions.get(expr).kind,
                    ExprKind::Braced { .. }
                )
                && !(self.string_literal(expr)
                    && matches!(self.graph.types.get(element).kind,
                    TypeKind::Array { element, .. } if self.character_element(element)))
            {
                let evaluated = if let TypeKind::Class(class) = self.graph.types.get(element).kind {
                    self.static_class_list(class, list, next, scope, out, None)?
                } else {
                    self.static_array_list(element, list, next, scope, out)?
                };
                if !evaluated {
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
    fn static_class_list(
        &mut self,
        class: EntityId,
        list: &[ast::ExprId],
        next: &mut usize,
        scope: ScopeId,
        out: &mut Vec<Item>,
        constructor: Option<super::construction::ConstructorId>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        if !self.graph.aggregate_class(class)
            && !self.static_memberwise_constructor(constructor, list.len() - *next)?
        {
            return Ok(false);
        }
        let ty = self.graph.lower_entity_type(class)?;
        let (size, _) = self.type_size(ty, crate::support::source::SourceOffset(0))?;
        let union = self.graph.entity(class).key == crate::lex::vocabulary::Keyword::Union;
        let selected = union
            .then(|| self.graph.union_list_member(class, *next >= list.len()))
            .flatten();
        let mut position = 0;
        let mut bits = [0u8; 8];
        let mut bit_bytes = 0;
        let mut unit_at = 0;
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if selected.is_some_and(|selected| member != selected) {
                continue;
            }
            let ty = self.graph.lower_entity_type(member)?;
            if let Some(field) = self.graph.entity(member).bit_field {
                if self.graph.unnamed_bit_field(member) {
                    continue;
                }
                let mut expr = list.get(*next).copied();
                if let Some(id) = expr {
                    *next += 1;
                    if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                        expr = self.tree.expr_lists.get(elements).first().copied();
                    }
                }
                let value = if let Some(expr) = expr {
                    let Some(Value::Integer(bits)) = self.static_scalar(expr, ty, scope)? else {
                        return Ok(false);
                    };
                    bits
                } else {
                    0
                };
                let storage = self.graph.bit_fields.storage(field)?;
                if storage.shared || storage.split != 0 || storage.bytes > 8 {
                    return Ok(false);
                }
                if bit_bytes == 0 || storage.offset != unit_at {
                    flush_bits(&mut bits, &mut bit_bytes, &mut position, out)?;
                    if storage.offset < position {
                        return Ok(false);
                    }
                    append_zero(out, storage.offset - position)?;
                    position = storage.offset;
                    unit_at = storage.offset;
                    bit_bytes = storage.bytes;
                }
                for k in 0..self.graph.bit_fields.width(field).min(64) {
                    let at = usize::try_from((storage.bit.wrapping_add(k)) / 8 - unit_at)
                        .map_err(|_| Error::Limit("bit unit offset"))?;
                    if at >= usize::from(bit_bytes) {
                        return Ok(false);
                    }
                    if value >> k & 1 != 0 {
                        bits[at] |= 1 << ((storage.bit.wrapping_add(k)) % 8);
                    }
                }
                if union {
                    break;
                }
                continue;
            }
            flush_bits(&mut bits, &mut bit_bytes, &mut position, out)?;
            let offset = self
                .graph
                .entity(member)
                .member_offset
                .ok_or(Error::Invariant("static member without offset"))?;
            append_zero(
                out,
                offset
                    .checked_sub(position)
                    .ok_or(Error::Invariant("overlapping static members"))?,
            )?;
            let expr = list.get(*next).copied();
            let evaluated = if let Some(expr) = expr
                && !matches!(
                    self.tree.expressions.get(expr).kind,
                    ExprKind::Braced { .. }
                )
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Array { .. } | TypeKind::Class(_)
                )
                && !(self.string_literal(expr)
                    && matches!(self.graph.types.get(ty).kind, TypeKind::Array { element, .. } if self.character_element(element)))
            {
                if let TypeKind::Class(class) = self.graph.types.get(ty).kind {
                    self.static_class_list(class, list, next, scope, out, None)?
                } else {
                    self.static_array_list(ty, list, next, scope, out)?
                }
            } else {
                if expr.is_some() {
                    *next += 1;
                }
                if expr.is_none()
                    && !matches!(
                        self.graph.types.get(ty).kind,
                        TypeKind::Array { .. }
                            | TypeKind::Class(_)
                            | TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
                    )
                {
                    let (size, _) = self.type_size(ty, crate::support::source::SourceOffset(0))?;
                    append_zero(out, size)?;
                    true
                } else {
                    self.static_array_element(ty, expr, scope, out)?
                }
            };
            if !evaluated {
                return Ok(false);
            }
            let (size, _) = self.type_size(ty, crate::support::source::SourceOffset(0))?;
            position = offset
                .checked_add(size)
                .ok_or(Error::Limit("static aggregate size"))?;
            if union {
                break;
            }
        }
        flush_bits(&mut bits, &mut bit_bytes, &mut position, out)?;
        append_zero(
            out,
            size.checked_sub(position)
                .ok_or(Error::Invariant("aggregate size without member bytes"))?,
        )?;
        Ok(true)
    }
    fn static_memberwise_constructor(
        &mut self,
        constructor: Option<super::construction::ConstructorId>,
        count: usize,
    ) -> Result<bool, Error> {
        let Some(constructor) = constructor else {
            return Ok(false);
        };
        let plan = self.graph.constructors.get(constructor);
        let Some(binding) = plan.binding else {
            return Ok(false);
        };
        let entity = self.graph.binding(binding).entity;
        if self.graph.entity(entity).constexpr
            || !plan.bases.is_empty()
            || plan.members.len() != count
        {
            return Ok(false);
        }
        let Some(body) = self.graph.lower_function_body(entity) else {
            return Ok(false);
        };
        let ast::Body::Parsed(body) = self.tree.bodies.get(body) else {
            return Err(Error::Invariant("static constructor body still deferred"));
        };
        let ast::StmtKind::Compound(statements) = self.tree.statements.get(body.compound).kind
        else {
            return Err(Error::Invariant("static constructor body not compound"));
        };
        if body.handlers.is_some() || !self.tree.stmt_lists.get(statements).is_empty() {
            return Ok(false);
        }
        let facts = self.graph.lower_constructor_function(constructor)?;
        let parameters: Vec<_> = self.graph.lower_parameters(facts.scope).collect();
        if parameters.len() != count {
            return Ok(false);
        }
        for (index, &(parameter, _, parameter_ty)) in parameters.iter().enumerate() {
            let (member, initializer) = self.graph.constructors.get(constructor).members[index];
            if self.graph.entity(member).bit_field.is_some() {
                return Ok(false);
            }
            let Some(ast::Initializer {
                kind: ast::InitializerKind::Parenthesized(arguments),
                ..
            }) = initializer
            else {
                return Ok(false);
            };
            let [expr] = self.tree.expr_lists.get(arguments) else {
                return Ok(false);
            };
            let mut expr = *expr;
            while let ExprKind::Paren(inner) = self.tree.expressions.get(expr).kind {
                expr = inner;
            }
            let ExprKind::Id(name) = self.tree.expressions.get(expr).kind else {
                return Ok(false);
            };
            if self.graph.lower_name(name)? != parameter {
                return Ok(false);
            }
            let member_ty = self.graph.lower_entity_type(member)?;
            if self.bare_type(parameter_ty)? != self.bare_type(member_ty)? {
                return Ok(false);
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
        let TypeKind::Array { element, bound, .. } = self.graph.types.get(target).kind else {
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
pub(super) fn append_zero(out: &mut Vec<Item>, bytes: u64) -> Result<(), Error> {
    if let Some(Item::Zero(prior)) = out.last_mut() {
        *prior = prior
            .checked_add(bytes)
            .ok_or(Error::Limit("array zero bytes"))?;
    } else if bytes != 0 {
        out.push(Item::Zero(bytes));
    }
    Ok(())
}

fn flush_bits(
    bits: &mut [u8; 8],
    bytes: &mut u8,
    position: &mut u64,
    out: &mut Vec<Item>,
) -> Result<(), Error> {
    if *bytes != 0 {
        out.push(Item::BitUnit(*bytes, u64::from_le_bytes(*bits)));
        *position = position
            .checked_add(u64::from(*bytes))
            .ok_or(Error::Limit("bit unit image"))?;
        bits.fill(0);
        *bytes = 0;
    }
    Ok(())
}

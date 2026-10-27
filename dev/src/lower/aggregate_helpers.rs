// Translated from lower/aggregate_lowering.cpp and program_lowering.cpp. See NOTICE.
//! Demand-owned aggregate helper shapes; one body is built and released at a time.
use super::{Error, conversions::Context, function::Function, symbols::Symbols, types};
use crate::{
    lowir::{
        builder::FunctionBuilder,
        model::{
            Boundary, Call, Facts, InstructionKind as I, Metadata, Program, SymbolId, Unwind, Value,
        },
        vocabulary::{Binding, Projection, Type},
    },
    sema::{
        Graph,
        lowering::{EntityId, TypeId, TypeView},
    },
    syntax::ast::{self, ExprKind},
};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Key {
    pub class: EntityId,
    pub count: usize,
    pub cv: u8,
}
#[derive(Default)]
pub(super) struct Helpers {
    index: HashMap<Key, SymbolId>,
    shapes: HashMap<EntityId, usize>,
    pending: Vec<(Key, SymbolId)>,
    next: usize,
}
impl Helpers {
    pub(super) fn get(&self, key: Key) -> Option<SymbolId> {
        self.index.get(&key).copied()
    }
    pub(super) fn shapes(&self, class: EntityId) -> usize {
        self.shapes.get(&class).copied().unwrap_or(0)
    }
    pub(super) fn insert(&mut self, key: Key, symbol: SymbolId) {
        self.index.insert(key, symbol);
        *self.shapes.entry(key.class).or_default() += 1;
        self.pending.push((key, symbol));
    }
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn prvalue_helper(
        &mut self,
        ty: TypeId,
        object: Value,
        list: &[ast::ExprId],
    ) -> Result<bool, Error> {
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Ok(false);
        };
        let (members, union) = self.graph.lower_class_members(class)?;
        let members = members.to_vec();
        let members = members.as_slice();
        if union || list.is_empty() {
            return Ok(false);
        }
        for &member in members {
            let ty = self.graph.lower_entity_type(member)?;
            if self.graph.lower_local_name(member).is_none()
                || self.graph.lower_bit_field(member)?.is_some()
                || matches!(
                    self.graph.lower_type(ty)?,
                    TypeView::Array { .. } | TypeView::Class(_)
                )
            {
                return Ok(false);
            }
        }
        let count = list.len().min(members.len());
        if count == 0 {
            return Ok(false);
        }
        self.aggregate_helper_call(
            Key {
                class,
                count,
                cv: self.graph.lower_cv(ty),
            },
            object,
            list,
            &mut 0,
        )?;
        Ok(true)
    }
    /// Return the helper form only when every element supplies all scalar members
    /// or an empty list, and at least one element supplies every member.
    pub(super) fn array_helper_eligible(
        &self,
        class: EntityId,
        bound: u64,
        list: &[ast::ExprId],
        owner: Option<EntityId>,
    ) -> Result<bool, Error> {
        if !self.graph.lower_aggregate_class(class) {
            return Ok(false);
        }
        let (members, union) = self.graph.lower_class_members(class)?;
        let members = members.to_vec();
        let members = members.as_slice();
        if union || members.is_empty() {
            return Ok(false);
        }
        for &member in members {
            if self.graph.lower_local_name(member).is_none() {
                return Ok(false);
            }
            let mut ty = self.graph.lower_entity_type(member)?;
            while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
                ty = element;
            }
            if matches!(self.graph.lower_type(ty)?, TypeView::Class(_)) {
                return Ok(false);
            }
        }
        if let Some(owner) = owner
            && self.initializer_mentions(list, owner)?
        {
            return Ok(false);
        }
        let mut next = 0;
        let mut any_full = false;
        for _ in 0..bound {
            let item = list.get(next).copied();
            if let Some(item) = item
                && let ExprKind::Braced { elements, .. } = self.tree.expressions.get(item).kind
            {
                next += 1;
                let values = self.tree.expr_lists.get(elements);
                let Some(full) = self.helper_members(members, Some(values), &mut 0)? else {
                    return Ok(false);
                };
                any_full |= full;
            } else {
                if let Some(item) = item
                    && matches!(
                        self.graph.lower_type(self.graph.lower_expression(item)?.ty)?,
                        TypeView::Class(source) if source == class
                    )
                {
                    return Ok(false);
                }
                let Some(full) = self.helper_members(members, item.map(|_| list), &mut next)?
                else {
                    return Ok(false);
                };
                any_full |= full;
            }
        }
        Ok(any_full)
    }
    fn helper_members(
        &self,
        members: &[EntityId],
        values: Option<&[ast::ExprId]>,
        next: &mut usize,
    ) -> Result<Option<bool>, Error> {
        let mut full = true;
        for &member in members {
            // C++ helper_members/helper_shape leave bit-fields in place;
            // a helper's ordinary scalar stores cannot address a bit unit.
            if self.graph.lower_bit_field(member)?.is_some() {
                return Ok(None);
            }
            let ty = self.graph.lower_entity_type(member)?;
            let present = values.is_some_and(|values| *next < values.len());
            if let TypeView::Array { element, .. } = self.graph.lower_type(ty)?
                && let Some(id) = values.and_then(|values| values.get(*next)).copied()
                && !matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. })
                && types::character(&*self.graph, element)?
                && matches!(
                    self.graph.lower_type(self.graph.lower_expression(id)?.ty)?,
                    TypeView::Array { .. }
                )
            {
                // A bare string initializes the member at its final address;
                // the helper form would introduce an extra array transfer.
                return Ok(None);
            }
            if !matches!(self.graph.lower_type(ty)?, TypeView::Array { .. }) {
                if !present
                    && (!values.is_some_and(<[ast::ExprId]>::is_empty)
                        || matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }))
                {
                    return Ok(None);
                }
                full &= present;
            }
            if let Some(values) = values {
                self.skip_helper_item(ty, values, next)?;
            }
        }
        Ok(Some(full))
    }
    fn skip_helper_item(
        &self,
        ty: TypeId,
        values: &[ast::ExprId],
        next: &mut usize,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let Some(&id) = values.get(*next) else {
            return Ok(());
        };
        if let TypeView::Array { element, bound } = self.graph.lower_type(ty)? {
            let braced = matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. });
            let string = types::character(&*self.graph, element)?
                && matches!(
                    self.graph.lower_type(self.graph.lower_expression(id)?.ty)?,
                    TypeView::Array { .. }
                );
            if !braced && !string {
                for _ in 0..bound {
                    self.skip_helper_item(element, values, next)?;
                }
                return Ok(());
            }
        }
        *next += 1;
        Ok(())
    }
    fn initializer_mentions(&self, list: &[ast::ExprId], owner: EntityId) -> Result<bool, Error> {
        let mut work = list.to_vec();
        while let Some(id) = work.pop() {
            match self.tree.expressions.get(id).kind {
                ExprKind::Id(name) => {
                    if self.graph.lower_name(name)? == owner {
                        return Ok(true);
                    }
                }
                ExprKind::Binary { left, right, .. }
                | ExprKind::Subscript {
                    base: left,
                    index: right,
                } => work.extend([left, right]),
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => work.extend([condition, then_expr, else_expr]),
                ExprKind::Paren(id)
                | ExprKind::ComplexPart { operand: id, .. }
                | ExprKind::Unary { operand: id, .. }
                | ExprKind::Postfix { operand: id, .. }
                | ExprKind::Cast { operand: id, .. }
                | ExprKind::Member { base: id, .. } => work.push(id),
                ExprKind::Call { callee, arguments } => {
                    work.push(callee);
                    work.extend(self.tree.expr_lists.get(arguments));
                }
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    work.extend(self.tree.expr_lists.get(arguments));
                    work.extend(braced);
                }
                ExprKind::Braced { elements, .. } => {
                    work.extend(self.tree.expr_lists.get(elements));
                }
                ExprKind::New {
                    placement,
                    initializer,
                    ..
                } => {
                    if let Some(placement) = placement {
                        work.extend(self.tree.expr_lists.get(placement));
                    }
                    if let Some(initializer) = initializer {
                        match initializer.kind {
                            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                                work.push(id);
                            }
                            ast::InitializerKind::Parenthesized(arguments) => {
                                work.extend(self.tree.expr_lists.get(arguments));
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }
    pub(super) fn aggregate_array_helpers(
        &mut self,
        class: EntityId,
        bound: u64,
        address: Value,
        list: &[ast::ExprId],
    ) -> Result<(), Error> {
        let mut next = 0;
        let element = self.graph.lower_entity_type(class)?;
        for index in 0..bound {
            let first = self.lifetime.cleanups.len();
            let object = if index == 0 {
                address
            } else {
                let offset = self.builder.integer(u128::from(
                    index
                        .checked_mul(types::layout(&*self.graph, element)?.0)
                        .ok_or(crate::lowir::Error::Limit)?,
                ))?;
                self.emit(I::Index(Type::I8, None, address, offset), element)?
                    .value
            };
            if let Some(&item) = list.get(next)
                && let ExprKind::Braced { elements, .. } = self.tree.expressions.get(item).kind
            {
                next += 1;
                self.aggregate_helper_call(
                    Key {
                        class,
                        count: self.graph.lower_class_member_entities(class).len(),
                        cv: 0,
                    },
                    object,
                    &self.owned_expression_ids(elements),
                    &mut 0,
                )?;
            } else {
                self.aggregate_helper_call(
                    Key {
                        class,
                        count: self.graph.lower_class_member_entities(class).len(),
                        cv: 0,
                    },
                    object,
                    list,
                    &mut next,
                )?;
            }
            if let Some(count) = self.aggregation.count.take() {
                self.aggregation.count = Some(self.aggregate_element_made(count, first)?);
            }
        }
        Ok(())
    }
    fn aggregate_helper_call(
        &mut self,
        key: Key,
        object: Value,
        list: &[ast::ExprId],
        next: &mut usize,
    ) -> Result<(), Error> {
        let (members, _) = self.graph.lower_class_members(key.class)?;
        let members = members.to_vec();
        let members = members.as_slice();
        let mut values = Vec::with_capacity(members.len() + 1);
        values.push(object);
        for &member in members.iter().take(key.count) {
            let ty = self.graph.lower_entity_type(member)?;
            if matches!(self.graph.lower_type(ty)?, TypeView::Array { .. }) {
                values.push(self.aggregate_array_argument(ty, list, next)?);
                continue;
            }
            let mut expr = list.get(*next).copied();
            if let Some(id) = expr {
                *next += 1;
                if !matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. })
                    && let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind
                {
                    expr = self.tree.expr_lists.get(elements).first().copied();
                }
            }
            let value = if matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }) {
                let mut id = expr.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "helper reference without initializer",
                )))?;
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind
                    && self.graph.lower_expression(id)?.is_place
                {
                    id = *self
                        .tree
                        .expr_lists
                        .get(elements)
                        .first()
                        .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                            "empty helper reference list",
                        )))?;
                }
                if let Some(owner) = self.reference_lifetimes.aggregate {
                    self.bind_extended_reference(id, ty, owner)?
                } else {
                    // The helper's arguments have ordinary full-expression
                    // lifetimes, even when its aggregate destination persists.
                    self.bind_reference_with_persistence(id, ty, Context::Initialization, false)?
                }
            } else if let Some(id) = expr {
                let value = self.operand(id, Context::Argument)?;
                self.fit(value, ty)?.value
            } else {
                self.zero(ty)?.value
            };
            values.push(value);
        }
        let symbol = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .aggregate_helper_symbol(self.builder.program(), key.class, key.count, key.cv)?;
        let arguments = self.builder.arguments(&values)?;
        self.effect(I::Call(Call {
            result: Type::Void,
            callee: Value::Symbol(symbol),
            arguments,
            copy_elision: false,
            argument_classes: Vec::new(),
            signature: None,
        }))
    }
}
pub(super) fn emit_pending(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    output: &mut super::definition_order::Definitions,
) -> Result<(), Error> {
    while let Some(&(key, symbol)) = symbols
        .aggregate_helpers
        .pending
        .get(symbols.aggregate_helpers.next)
    {
        symbols.aggregate_helpers.next += 1;
        emit(program, graph, key, symbol)?;
        output.stream(program, None)?;
    }
    Ok(())
}
pub(super) fn declare(
    program: &mut Program,
    symbols: &Symbols<'_, '_>,
    graph: &Graph,
    key: Key,
    symbol: SymbolId,
) -> Result<(), Error> {
    let mut parameters = vec![crate::lowir::model::Parameter {
        name: program.names.intern("this")?,
        ty: Type::Ptr,
        pass: None,
        object_bytes: graph.lower_class_layout(key.class)?.0,
        no_alias: false,
        classes: None,
    }];
    for &member in graph
        .lower_class_member_entities(key.class)
        .iter()
        .take(key.count)
    {
        let name = graph
            .lower_local_name(member)
            .ok_or(crate::sema::Error::Invariant("helper unnamed member"))?;
        let name = program.names.intern(symbols.identifier(name)?)?;
        let ty = graph.lower_entity_type(member)?;
        let (pass, object_bytes) = super::function::reference_boundary(graph, ty)?;
        parameters.push(crate::lowir::model::Parameter {
            name,
            ty: if matches!(
                graph.lower_type(ty)?,
                TypeView::Array { .. } | TypeView::Reference { .. }
            ) {
                Type::Ptr
            } else {
                types::storage(graph, ty)?
            },
            pass,
            object_bytes,
            no_alias: false,
            classes: None,
        });
    }
    let signature = program.add_signature(crate::lowir::model::Signature {
        parameters,
        result: Type::Void,
        boundary: Boundary {
            unwind: Unwind::No,
            ..Boundary::default()
        },
    })?;
    program.set_forward_boundary(
        symbol,
        signature,
        Metadata {
            binding: Some(Binding::Internal),
            ..Metadata::default()
        },
    );
    Ok(())
}
fn emit(program: &mut Program, graph: &Graph, key: Key, symbol: SymbolId) -> Result<(), Error> {
    let (members, _) = graph.lower_class_members(key.class)?;
    let signature = program
        .forward_boundary(symbol)
        .ok_or(crate::sema::Error::Invariant(
            "aggregate helper without boundary",
        ))?;
    let count = program.signature(signature).parameters.len();
    let mut builder = FunctionBuilder::from_forward_boundary(program, symbol)?;
    builder.block("entry")?;
    let mut slots = Vec::with_capacity(count);
    for index in 0..count {
        let parameter = &builder.program().signature(signature).parameters[index];
        let (name, ty) = (parameter.name, parameter.ty);
        let slot = builder.named_slot(name, ty)?;
        let incoming = builder.parameter(index)?;
        builder.effect(I::Store(
            Facts::memory(
                false,
                !matches!(ty, Type::I8 | Type::U8 | Type::Object { .. }),
            ),
            ty,
            incoming,
            slot,
        ))?;
        slots.push(slot);
    }
    for (index, &member) in members.iter().enumerate() {
        let ty = graph.lower_entity_type(member)?;
        let storage = types::storage(graph, ty)?;
        let array = matches!(graph.lower_type(ty)?, TypeView::Array { .. });
        let value = if let Some(&slot) = slots.get(index + 1) {
            // Array members load their source address after addressing the destination.
            if array {
                None
            } else {
                Some(builder.generated(I::Load(
                    Facts::memory(
                        false,
                        !matches!(storage, Type::I8 | Type::U8 | Type::Object { .. }),
                    ),
                    storage,
                    slot,
                ))?)
            }
        } else {
            Some(
                builder.integer(if super::member_pointers::is_data_pointer(graph, ty) {
                    u128::MAX
                } else {
                    0
                })?,
            )
        };
        let root = builder.generated(I::Load(Facts::memory(false, true), Type::Ptr, slots[0]))?;
        let offset = builder.integer(u128::from(graph.lower_member_offset(member)?))?;
        let address =
            builder.generated(I::Index(Type::I8, Some(Projection::Field), root, offset))?;
        if array {
            let slot = *slots
                .get(index + 1)
                .ok_or(Error::Missing("omitted aggregate helper array"))?;
            let source = builder.generated(I::Load(Facts::memory(false, true), Type::Ptr, slot))?;
            let (bytes, align) = types::layout(graph, ty)?;
            builder.effect(I::CopyObject(bytes, align, source, address))?;
        } else {
            let volatile = graph.lower_cv(ty) & 2 != 0
                && !matches!(graph.lower_type(ty)?, TypeView::Reference { .. });
            builder.effect(I::Store(
                Facts::memory(
                    volatile,
                    !volatile && !matches!(storage, Type::I8 | Type::U8 | Type::Object { .. }),
                ),
                storage,
                value.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "helper value missing",
                )))?,
                address,
            ))?;
        }
    }
    builder.effect(I::Return(Type::Void, None))?;
    builder.finish()?;
    Ok(())
}

// Translated from lower/aggregate_lowering.cpp::aggregate_object/class/item/path_address. See NOTICE.
//! Named plain aggregates store source values before recomputing each member path.
use super::{Error, conversions::Context, function::Function, types};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{EntityId, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};
#[derive(Clone, Copy)]
pub(super) struct Step {
    ty: Type,
    projection: Projection,
    offset: u64,
    untyped: bool,
}
#[derive(Clone, Copy)]
enum Root {
    Slot(Value),
    Address(Value),
}
struct Path {
    root: Root,
    steps: Vec<Step>,
    volatile: bool,
}
#[derive(Clone, Copy)]
enum Item {
    NamedMember,
    StructuredMember,
    ArrayElement,
}
struct Input<'a> {
    values: &'a [ast::ExprId],
    next: usize,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn local_aggregate(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        slot: Value,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        self.symbols.require_trivial_storage(ty)?;
        let Some(initializer) = initializer else {
            if !self.graph.lower_trivial_default_construction(entity)? {
                return Err(Error::Missing("default class construction lowering"));
            }
            self.emit(I::Addr(slot), ty)?;
            return Ok(());
        };
        let (ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id)) =
            initializer.kind
        else {
            return Err(Error::Missing("class constructor/copy lowering"));
        };
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Missing("class constructor/copy lowering"));
        };
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "aggregate without class",
            )));
        };
        if self.aggregate_stores(class)? {
            self.emit(I::Addr(slot), ty)?;
        }
        let mut input = Input {
            values: self.tree.expr_lists.get(elements),
            next: 0,
        };
        let mut path = Path {
            root: Root::Slot(slot),
            steps: std::mem::take(&mut self.named_path),
            volatile: self.graph.lower_cv(ty) & 2 != 0,
        };
        let result = self.named_members(class, &mut input, &mut path);
        path.steps.clear();
        self.named_path = path.steps;
        result
    }
    pub(super) fn aggregate_array(
        &mut self,
        ty: TypeId,
        address: Value,
        values: &[ast::ExprId],
    ) -> Result<(), Error> {
        self.symbols.require_trivial_storage(ty)?;
        let mut input = Input { values, next: 0 };
        let mut path = Path {
            root: Root::Address(address),
            steps: Vec::new(),
            volatile: self.graph.lower_cv(ty) & 2 != 0,
        };
        self.named_contents(ty, &mut input, &mut path, false)
    }
    fn aggregate_stores(&self, class: EntityId) -> Result<bool, Error> {
        self.check_stack()?;
        let (members, _) = self.graph.lower_class_members(class)?;
        for &member in members {
            let mut ty = self.graph.lower_entity_type(member)?;
            while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
                ty = element;
            }
            if let TypeView::Class(class) = self.graph.lower_type(ty)? {
                if self.aggregate_stores(class)? {
                    return Ok(true);
                }
            } else {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn named_members(
        &mut self,
        class: EntityId,
        input: &mut Input<'_>,
        path: &mut Path,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if matches!(path.root, Root::Address(_)) {
            return self.structured_members(class, input, path);
        }
        let (members, union) = self.graph.lower_class_members(class)?;
        for &member in members {
            let ty = self.graph.lower_entity_type(member)?;
            path.steps.push(Step {
                ty: Type::I8,
                projection: Projection::Field,
                offset: self.graph.lower_member_offset(member)?,
                untyped: union,
            });
            let saved_volatile = path.volatile;
            path.volatile |= self.graph.lower_cv(ty) & 2 != 0;
            self.named_item(ty, input, path, Item::NamedMember)?;
            path.volatile = saved_volatile;
            path.steps.pop();
            if union {
                break;
            }
        }
        Ok(())
    }
    fn named_item(
        &mut self,
        ty: TypeId,
        input: &mut Input<'_>,
        path: &mut Path,
        item: Item,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let value_first = matches!(item, Item::NamedMember);
        let mut expr = input.values.get(input.next).copied();
        if let TypeView::Array { element, bound } = self.graph.lower_type(ty)?
            && types::character(self.graph, element)?
            && let Some(id) = expr
            && let ExprKind::Literal(literal) =
                self.tree.expressions.get(self.without_parentheses(id)).kind
            && let crate::lex::PostTokenKind::String { ty: literal_ty, .. } = literal.kind
        {
            input.next += 1;
            return self.named_string(
                element,
                bound,
                literal,
                literal_ty.size(),
                path,
                matches!(item, Item::StructuredMember),
            );
        }
        if matches!(
            self.graph.lower_type(ty)?,
            TypeView::Class(_) | TypeView::Array { .. }
        ) {
            if let Some(id) = expr
                && let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind
            {
                input.next += 1;
                let mut nested = Input {
                    values: self.tree.expr_lists.get(elements),
                    next: 0,
                };
                return self.named_contents(
                    ty,
                    &mut nested,
                    path,
                    matches!(item, Item::StructuredMember),
                );
            }
            return self.named_contents(ty, input, path, matches!(item, Item::StructuredMember));
        }
        if let Some(id) = expr {
            input.next += 1;
            if !matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. })
                && let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind
            {
                expr = self.tree.expr_lists.get(elements).first().copied();
            }
        }
        let before = if value_first {
            None
        } else {
            Some(self.named_address(path, ty)?)
        };
        let value = if matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }) {
            let mut id = expr.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "aggregate reference without initializer",
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
                        "empty aggregate reference list",
                    )))?;
            }
            self.bind_reference(id, ty, Context::Initialization)?
        } else if let Some(id) = expr {
            let context = if matches!(item, Item::ArrayElement) {
                Context::Initialization
            } else {
                Context::MemberInitialization
            };
            let value = self.operand(id, context)?;
            self.fit(value, ty)?.value
        } else {
            self.zero(ty)?.value
        };
        let address = if let Some(address) = before {
            address
        } else {
            self.named_address(path, ty)?
        };
        self.named_store(ty, path, address, value)
    }
    fn named_store(
        &mut self,
        ty: TypeId,
        path: &Path,
        address: Value,
        value: Value,
    ) -> Result<(), Error> {
        let storage = types::storage(self.graph, ty)?;
        let volatile = path.volatile
            && !matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. })
            || self.graph.lower_cv(ty) & 2 != 0;
        self.effect(I::Store(
            Facts::memory(
                volatile,
                !volatile
                    && !self.untyped_places.contains(&address)
                    && !matches!(storage, Type::I8 | Type::U8),
            ),
            storage,
            value,
            address,
        ))
    }
    fn named_string(
        &mut self,
        element: TypeId,
        bound: u64,
        literal: ast::Literal,
        width: usize,
        path: &mut Path,
        field: bool,
    ) -> Result<(), Error> {
        let ty = types::storage(self.graph, element)?;
        for index in 0..bound {
            path.steps.push(Step {
                ty,
                projection: Projection::ArrayElement,
                offset: index,
                untyped: false,
            });
            let address = if let Root::Address(root) = path.root {
                self.structured_element(root, element, index, field)?
            } else {
                self.named_address(path, element)?
            };
            let unit = self.literal_array_unit(literal, index, width)?;
            let value = self.builder.integer(u128::from(unit))?;
            self.named_store(element, path, address, value)?;
            path.steps.pop();
        }
        Ok(())
    }
    fn named_contents(
        &mut self,
        ty: TypeId,
        input: &mut Input<'_>,
        path: &mut Path,
        field: bool,
    ) -> Result<(), Error> {
        self.check_stack()?;
        match self.graph.lower_type(ty)? {
            TypeView::Class(class) => self.named_members(class, input, path),
            TypeView::Array { element, bound } => {
                if matches!(path.root, Root::Address(_))
                    || self.member_class_array(element, path)?
                {
                    return self.structured_elements(ty, element, bound, input, path, field);
                }
                for index in 0..bound {
                    let (ty_ir, offset) =
                        if matches!(self.graph.lower_type(element)?, TypeView::Class(_)) {
                            (
                                Type::I8,
                                index
                                    .checked_mul(types::layout(self.graph, element)?.0)
                                    .ok_or(crate::lowir::Error::Limit)?,
                            )
                        } else {
                            (types::storage(self.graph, element)?, index)
                        };
                    path.steps.push(Step {
                        ty: ty_ir,
                        projection: Projection::ArrayElement,
                        offset,
                        untyped: false,
                    });
                    self.named_item(element, input, path, Item::ArrayElement)?;
                    path.steps.pop();
                }
                Ok(())
            }
            _ => Err(Error::Semantic(crate::sema::Error::Invariant(
                "aggregate contents without aggregate",
            ))),
        }
    }
    fn member_class_array(&self, mut element: TypeId, path: &Path) -> Result<bool, Error> {
        if !path
            .steps
            .last()
            .is_some_and(|step| step.projection == Projection::Field)
        {
            return Ok(false);
        }
        while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
            element = inner;
        }
        Ok(matches!(
            self.graph.lower_type(element)?,
            TypeView::Class(_)
        ))
    }
    fn structured_members(
        &mut self,
        class: EntityId,
        input: &mut Input<'_>,
        path: &Path,
    ) -> Result<(), Error> {
        let Root::Address(root) = path.root else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "structured class without address",
            )));
        };
        let (members, union) = self.graph.lower_class_members(class)?;
        for &member in members {
            let ty = self.graph.lower_entity_type(member)?;
            let offset = self
                .builder
                .integer(u128::from(self.graph.lower_member_offset(member)?))?;
            let address = self
                .emit(
                    I::Index(Type::I8, Some(Projection::Field), root, offset),
                    ty,
                )?
                .value;
            if union || self.untyped_places.contains(&root) {
                self.untyped_places.insert(address);
            }
            let mut child = Path {
                root: Root::Address(address),
                steps: Vec::new(),
                volatile: path.volatile || self.graph.lower_cv(ty) & 2 != 0,
            };
            self.named_item(ty, input, &mut child, Item::StructuredMember)?;
            if union {
                break;
            }
        }
        Ok(())
    }
    fn structured_elements(
        &mut self,
        ty: TypeId,
        element: TypeId,
        bound: u64,
        input: &mut Input<'_>,
        path: &Path,
        field: bool,
    ) -> Result<(), Error> {
        let root = self.named_address(path, ty)?;
        for index in 0..bound {
            let address = self.structured_element(root, element, index, field)?;
            let mut child = Path {
                root: Root::Address(address),
                steps: Vec::new(),
                volatile: path.volatile || self.graph.lower_cv(element) & 2 != 0,
            };
            self.named_item(element, input, &mut child, Item::ArrayElement)?;
        }
        Ok(())
    }
    fn structured_element(
        &mut self,
        root: Value,
        element: TypeId,
        index: u64,
        field: bool,
    ) -> Result<Value, Error> {
        let object = matches!(
            self.graph.lower_type(element)?,
            TypeView::Class(_) | TypeView::Array { .. }
        );
        let (storage, projection, offset) = if field && !object {
            (
                types::storage(self.graph, element)?,
                Some(Projection::ArrayElement),
                index,
            )
        } else {
            if index == 0 {
                return Ok(root);
            }
            (
                Type::I8,
                (!object).then_some(Projection::ArrayElement),
                index
                    .checked_mul(types::layout(self.graph, element)?.0)
                    .ok_or(crate::lowir::Error::Limit)?,
            )
        };
        let offset = self.builder.integer(u128::from(offset))?;
        let address = self
            .emit(I::Index(storage, projection, root, offset), element)?
            .value;
        if self.untyped_places.contains(&root) {
            self.untyped_places.insert(address);
        }
        Ok(address)
    }
    fn named_address(&mut self, path: &Path, source: TypeId) -> Result<Value, Error> {
        let mut address = match path.root {
            Root::Slot(slot) => self.emit(I::Addr(slot), source)?.value,
            Root::Address(address) => address,
        };
        let mut untyped = false;
        for step in &path.steps {
            let offset = self.builder.integer(u128::from(step.offset))?;
            address = self
                .emit(
                    I::Index(step.ty, Some(step.projection), address, offset),
                    source,
                )?
                .value;
            untyped |= step.untyped;
            if untyped {
                self.untyped_places.insert(address);
            }
        }
        Ok(address)
    }
}

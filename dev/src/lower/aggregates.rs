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
    scale: Option<u64>,
    untyped: bool,
}
#[derive(Clone, Copy)]
enum Root {
    Slot(Value),
    Address(Value),
    This(TypeId),
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
    pub(super) fn constructor_aggregate_base(
        &mut self,
        ty: TypeId,
        object: Value,
        initializer: ast::Initializer,
    ) -> Result<(), Error> {
        let ast::InitializerKind::Braced(id) = initializer.kind else {
            return Err(Error::Missing("base class copy materialization"));
        };
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "base aggregate list shape",
            )));
        };
        self.symbols.require_trivial_storage(ty)?;
        let mut input = Input {
            values: self.tree.expr_lists.get(elements),
            next: 0,
        };
        let mut path = Path {
            root: Root::Address(object),
            steps: std::mem::take(&mut self.named_path),
            volatile: self.graph.lower_cv(ty) & 2 != 0,
        };
        let result = self.named_contents(ty, &mut input, &mut path, false);
        path.steps.clear();
        self.named_path = path.steps;
        result
    }
    pub(super) fn local_aggregate(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        slot: Value,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        self.initialize_aggregate(entity, ty, slot, initializer, true)
    }
    pub(super) fn initialize_aggregate(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        slot: Value,
        initializer: Option<ast::Initializer>,
        root_address: bool,
    ) -> Result<(), Error> {
        self.symbols.require_trivial_storage(ty)?;
        if initializer.is_some() && self.graph.lower_constructor_arguments(entity).is_some() {
            return self.construct_local(entity, ty, slot);
        }
        let Some(initializer) = initializer else {
            if !self.graph.lower_trivial_default_construction(entity)? {
                return self.construct_local(entity, ty, slot);
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
        if root_address && self.aggregate_stores(class)? {
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
    pub(super) fn constructor_aggregate_member(
        &mut self,
        member: EntityId,
        pointer: TypeId,
        initializer: ast::Initializer,
        union: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(member)?;
        let values = match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                    self.tree.expr_lists.get(elements)
                } else if matches!(self.graph.lower_type(ty)?, TypeView::Array { .. }) {
                    return Err(Error::Semantic(crate::sema::Error::Diagnostic {
                        location,
                        message: "an array member initialized from an expression",
                    }));
                } else {
                    return Err(Error::Missing(
                        "constructor class expression member lowering",
                    ));
                }
            }
            ast::InitializerKind::Parenthesized(arguments)
                if self.tree.expr_lists.get(arguments).is_empty() =>
            {
                &[]
            }
            ast::InitializerKind::Parenthesized(_)
                if matches!(self.graph.lower_type(ty)?, TypeView::Array { .. }) =>
            {
                return Err(Error::Semantic(crate::sema::Error::Diagnostic {
                    location,
                    message: "an array member initialized from an expression",
                }));
            }
            _ => {
                return Err(Error::Missing(
                    "constructor class expression member lowering",
                ));
            }
        };
        self.symbols.require_trivial_storage(ty)?;
        let mut input = Input { values, next: 0 };
        let mut path = Path {
            root: Root::This(pointer),
            steps: std::mem::take(&mut self.named_path),
            volatile: self.graph.lower_cv(ty) & 2 != 0,
        };
        path.steps.push(Step {
            scale: None,
            ty: Type::I8,
            projection: Projection::Field,
            offset: self.graph.lower_member_offset(member)?,
            untyped: union,
        });
        let string = if let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            types::character(self.graph, element)?
                && values.first().is_some_and(|&id| {
                    matches!(
                        self.tree.expressions.get(self.without_parentheses(id)).kind,
                        ExprKind::Literal(ast::Literal {
                            kind: crate::lex::PostTokenKind::String { .. },
                            ..
                        })
                    )
                })
        } else {
            false
        };
        let result = if string {
            self.named_item(ty, &mut input, &mut path, Item::ArrayElement)
        } else {
            self.named_contents(ty, &mut input, &mut path, false)
        };
        path.steps.clear();
        self.named_path = path.steps;
        result
    }
    pub(super) fn global_class_array(
        &mut self,
        ty: TypeId,
        storage: Value,
        initializer: ast::Initializer,
    ) -> Result<(), Error> {
        let (ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id)) =
            initializer.kind
        else {
            return Err(Error::Missing("class array non-list initialization"));
        };
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Missing("class array non-list initialization"));
        };
        let mut input = Input {
            values: self.tree.expr_lists.get(elements),
            next: 0,
        };
        let mut path = Path {
            root: Root::Slot(storage),
            steps: std::mem::take(&mut self.named_path),
            volatile: self.graph.lower_cv(ty) & 2 != 0,
        };
        let result = self.named_contents(ty, &mut input, &mut path, false);
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
            if self.graph.lower_unnamed_bit_field(member) {
                continue;
            }
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
        let saved = self.bit_initialization;
        self.bit_initialization = super::bit_fields::Initialization::default();
        let result = self.named_members_inner(class, input, path);
        self.bit_initialization = saved;
        result
    }
    fn named_field(
        &mut self,
        member: EntityId,
        input: &mut Input<'_>,
        path: &mut Path,
        value_first: bool,
    ) -> Result<(), Error> {
        let field = self.field(member)?;
        let mut expr = input.values.get(input.next).copied();
        if let Some(id) = expr {
            input.next += 1;
            if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                expr = self.tree.expr_lists.get(elements).first().copied();
            }
        }
        let before = if value_first {
            None
        } else {
            Some(self.field_path_address(field, path)?)
        };
        let value = if let Some(id) = expr {
            let value = self.operand(id, Context::MemberInitialization)?;
            self.fit(value, field.source)?
        } else {
            self.zero(field.source)?
        };
        let address = if let Some(address) = before {
            address
        } else {
            self.field_path_address(field, path)?
        };
        self.initialize_field_value(
            field,
            super::bit_fields::Unit {
                address,
                volatile: path.volatile || self.graph.lower_cv(field.source) & 2 != 0,
            },
            value,
        )?;
        let layout = self
            .graph
            .lower_bit_field(member)?
            .ok_or(Error::Missing("field layout"))?;
        self.bit_initialization.high = layout.bit.wrapping_add(layout.width);
        Ok(())
    }
    fn field_path_address(
        &mut self,
        field: super::bit_fields::Field,
        path: &mut Path,
    ) -> Result<Value, Error> {
        path.steps.push(Step {
            scale: None,
            ty: Type::I8,
            projection: Projection::Field,
            offset: field.offset,
            untyped: true,
        });
        let result = self.named_address(path, field.source);
        path.steps.pop();
        result
    }
    fn named_members_inner(
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
            if self.graph.lower_unnamed_bit_field(member) {
                continue;
            }
            if self.graph.lower_bit_field(member)?.is_some() {
                self.named_field(member, input, path, !matches!(path.root, Root::This(_)))?;
                if union {
                    break;
                }
                continue;
            }
            self.bit_initialization.unit = 0;
            let ty = self.graph.lower_entity_type(member)?;
            path.steps.push(Step {
                scale: None,
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
            self.bit_initialization.unit = 0;
            self.bit_initialization.high = self
                .graph
                .lower_member_offset(member)?
                .wrapping_add(types::layout(self.graph, ty)?.0)
                .wrapping_mul(8);
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
        let value_first = matches!(item, Item::NamedMember) && !matches!(path.root, Root::This(_));
        let mut expr = input.values.get(input.next).copied();
        if let TypeView::Class(class) = self.graph.lower_type(ty)?
            && !self.graph.lower_aggregate_class(class)
        {
            self.symbols.require_trivial_storage(ty)?;
            let object = self.named_address(path, ty)?;
            let (constructor, arguments) = if let Some(id) = expr {
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
                    return Err(Error::Missing("class subobject expression materialization"));
                };
                input.next += 1;
                let constructor = self
                    .graph
                    .lower_list_constructor(id)?
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "class list without selected constructor",
                    )))?;
                let arguments = if self.tree.expr_lists.get(elements).is_empty() {
                    crate::sema::lowering::ConstructorArguments::Value
                } else {
                    crate::sema::lowering::ConstructorArguments::Values(elements)
                };
                (constructor, arguments)
            } else {
                (
                    self.graph.lower_default_constructor(class)?,
                    crate::sema::lowering::ConstructorArguments::Value,
                )
            };
            return self.construct_at(constructor, ty, object, Some(arguments));
        }
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
                scale: None,
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
                    let (ty_ir, scale) =
                        if matches!(self.graph.lower_type(element)?, TypeView::Class(_)) {
                            (Type::I8, Some(types::layout(self.graph, element)?.0))
                        } else {
                            (types::storage(self.graph, element)?, None)
                        };
                    path.steps.push(Step {
                        scale,
                        ty: ty_ir,
                        projection: Projection::ArrayElement,
                        offset: index,
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
            if self.graph.lower_unnamed_bit_field(member) {
                continue;
            }
            if self.graph.lower_bit_field(member)?.is_some() {
                let mut field_path = Path {
                    root: Root::Address(root),
                    steps: Vec::new(),
                    volatile: path.volatile,
                };
                self.named_field(member, input, &mut field_path, false)?;
                if union {
                    break;
                }
                continue;
            }
            self.bit_initialization.unit = 0;
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
            self.bit_initialization.unit = 0;
            self.bit_initialization.high = self
                .graph
                .lower_member_offset(member)?
                .wrapping_add(types::layout(self.graph, ty)?.0)
                .wrapping_mul(8);
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
            Root::This(pointer) => self.object_pointer(pointer)?.value,
        };
        let mut untyped = false;
        for step in &path.steps {
            let mut offset = self.builder.integer(u128::from(step.offset))?;
            if let Some(bytes) = step.scale {
                let size = self.builder.integer(u128::from(bytes))?;
                offset = self
                    .emit(
                        I::Binary(
                            crate::lowir::vocabulary::Binary::Mul,
                            Facts::default(),
                            Type::I64,
                            offset,
                            size,
                        ),
                        source,
                    )?
                    .value;
            }
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

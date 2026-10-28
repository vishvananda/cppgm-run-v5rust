// Translated from constant_interpreter.cpp and constant_construction.cpp. See NOTICE.
//! Flat evaluation storage for class and array skeletons, with canonical subobject keys.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constexpr_calls::Returned,
    constexpr_frames::CellId,
    expression_facts::{ObjectSource, ValueCategory},
    graph::{EntityId, ScopeId},
    inheritance::BasePathId,
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

#[derive(Default)]
pub(super) struct ProjectionQuery {
    work: Vec<(TypeId, i64)>,
    visited: std::collections::HashSet<(TypeId, i64)>,
}

#[derive(Clone, Copy)]
enum SkeletonOrigin {
    Fresh,
    Copy,
}
impl SkeletonOrigin {
    fn copied(self, copy: bool) -> Self {
        if copy { Self::Copy } else { self }
    }
}

impl Analyzer<'_> {
    pub(super) fn constant_string_array(
        &mut self,
        target: Address,
        expr: ast::ExprId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let expr = self.graph.expression_facts.root(expr);
        let ast::ExprKind::Literal(literal) = self.tree.expressions.get(expr).kind else {
            return Ok(false);
        };
        let crate::lex::PostTokenKind::String { ty, .. } = literal.kind else {
            return Ok(false);
        };
        let TypeKind::Array { element, bound, .. } = self.graph.types.get(target.designated).kind
        else {
            return Err(Error::Invariant("constant string without array target"));
        };
        let element = self.bare_type(element)?;
        let width = ty.size();
        for index in 0..bound {
            let start = usize::try_from(index)
                .ok()
                .and_then(|i| i.checked_mul(width));
            let mut bytes = [0; 8];
            if let Some((start, end)) =
                start.and_then(|start| start.checked_add(width).map(|end| (start, end)))
                && let Some(unit) = self.literal_bytes.get(literal.data).get(start..end)
            {
                bytes[..width].copy_from_slice(unit);
            }
            let kind = self
                .integral_kind(element)
                .ok_or(Error::Invariant("string array element type"))?;
            let place = Address {
                designated: element,
                offset: target
                    .offset
                    .wrapping_add(super::integral::signed(index.wrapping_mul(
                        u64::try_from(width).map_err(|_| Error::Limit("string code unit size"))?,
                    ))),
                ..target
            };
            self.store_constant_value(
                place,
                Value::Integer(super::constants::Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    high: 0,
                    bits: kind.convert(u64::from_le_bytes(bytes)),
                    ty: element,
                }),
                location,
            )?;
        }
        Ok(true)
    }
    pub(super) fn allocate_constant_object(
        &mut self,
        ty: TypeId,
        value: Option<Value>,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let mut element = ty;
        while let TypeKind::Array { element: child, .. } = self.graph.types.get(element).kind {
            element = child;
        }
        if let Some(class) = self.graph.class_of(element) {
            self.complete_class(class)?;
            if self.graph.has_nontrivial_destructor(class) {
                return Err(nonconstant(
                    location,
                    "an object with a non-trivial destructor in a constant expression",
                ));
            }
        }
        self.allocate_constant_storage(ty, value, location)
    }
    // Declaration storage outlives initialization: its destructor does not run
    // in this evaluation. Temporaries and parameters use the checked entry above.
    pub(super) fn allocate_constant_storage(
        &mut self,
        ty: TypeId,
        value: Option<Value>,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        self.allocate_constant_cells(ty, value, location, SkeletonOrigin::Fresh)
    }
    // Scalar lane values are held in addressable cells for this interpreter.
    // Materializing an already computed value does not construct an object
    // skeleton in the frozen interpreter.
    pub(super) fn allocate_constant_value_storage(
        &mut self,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        self.allocate_constant_cells(ty, None, location, SkeletonOrigin::Copy)
    }
    fn allocate_constant_cells(
        &mut self,
        ty: TypeId,
        value: Option<Value>,
        location: SourceOffset,
        origin: SkeletonOrigin,
    ) -> Result<Address, Error> {
        let cell_type = self.bare_type(ty)?;
        self.constant_object_leaves(cell_type, 0, location)?;
        let place = self
            .constexpr_frames
            .allocate((ty, cell_type), value, location)?;
        let root =
            self.constexpr_frames
                .locate(&self.graph.constant_member_spans, place, location)?;
        self.populate_constant_skeleton(place, root, location, origin)?;
        self.constexpr_frames.check_storage_budget(location)?;
        Ok(Address {
            designated: ty,
            ..place
        })
    }
    fn constant_object_leaves(
        &mut self,
        ty: TypeId,
        depth: u32,
        location: SourceOffset,
    ) -> Result<u64, Error> {
        self.check_stack()?;
        let count = match self.graph.types.get(ty).kind {
            TypeKind::Complex(_) => 3,
            TypeKind::Vector { bound, .. } if self.constant_lane_vector(ty).is_some() => {
                bound.saturating_add(1)
            }
            TypeKind::Array { element, bound, .. } => bound
                .max(1)
                .saturating_mul(self.constant_object_leaves(element, depth, location)?),
            TypeKind::Class(class) if depth <= 64 => {
                self.complete_class(class)?;
                let mut count = 1u64;
                for index in 0..self.graph.entity(class).bases.len() {
                    let child = self.graph.entity(class).bases[index].class;
                    let ty = self.graph.lower_entity_type(child)?;
                    count = count.saturating_add(self.constant_object_leaves(
                        ty,
                        depth + 1,
                        location,
                    )?);
                }
                for index in 0..self.graph.entity(class).members.len() {
                    let child = self.graph.entity(class).members[index];
                    let ty = self.graph.lower_entity_type(child)?;
                    count = count.saturating_add(self.constant_object_leaves(
                        ty,
                        depth + 1,
                        location,
                    )?);
                }
                count
            }
            _ => 1,
        };
        if count > 1 << 22 {
            return Err(nonconstant(
                location,
                "constant evaluation exceeds its storage budget",
            ));
        }
        Ok(count)
    }
    fn populate_constant_skeleton(
        &mut self,
        place: Address,
        cell: CellId,
        location: SourceOffset,
        origin: SkeletonOrigin,
    ) -> Result<(), Error> {
        if matches!(origin, SkeletonOrigin::Fresh) {
            self.constexpr_frames.skeleton_step();
        }
        self.check_stack()?;
        match self.graph.types.get(place.designated).kind {
            TypeKind::Complex(part) => {
                let size = self.type_size(part, location)?.0;
                for (index, offset) in [0, size].into_iter().enumerate() {
                    let child = Address {
                        designated: part,
                        offset: place.offset.wrapping_add(super::integral::signed(offset)),
                        ..place
                    };
                    let child_cell = self
                        .constexpr_frames
                        .add_child(cell, child, None, location)?;
                    // C++ constructs one part/lane skeleton and copies it.
                    self.populate_constant_skeleton(
                        child,
                        child_cell,
                        location,
                        origin.copied(index != 0),
                    )?;
                }
            }
            TypeKind::Array { element, bound, .. } | TypeKind::Vector { element, bound, .. } => {
                if self.vector_shape(place.designated).is_some()
                    && self.constant_lane_vector(place.designated).is_none()
                {
                    self.constexpr_frames.finish_cell(cell);
                    return Ok(());
                }
                if matches!(origin, SkeletonOrigin::Fresh)
                    && matches!(
                        self.graph.types.get(place.designated).kind,
                        TypeKind::Array { .. }
                    )
                    && !self.constexpr_frames.array_skeleton_fits(bound)
                {
                    self.constexpr_frames.finish_cell(cell);
                    return Ok(());
                }
                let size = self.type_size(element, location)?.0;
                for index in 0..bound {
                    let ty = self.bare_type(element)?;
                    let child = Address {
                        designated: ty,
                        offset: place
                            .offset
                            .wrapping_add(super::integral::signed(index.wrapping_mul(size))),
                        ..place
                    };
                    let child_cell = self
                        .constexpr_frames
                        .add_child(cell, child, None, location)?;
                    let copy = matches!(
                        self.graph.types.get(place.designated).kind,
                        TypeKind::Vector { .. }
                    ) && index != 0;
                    self.populate_constant_skeleton(
                        child,
                        child_cell,
                        location,
                        origin.copied(copy),
                    )?;
                }
            }
            TypeKind::Class(class) => {
                self.class_layout(class, location)?;
                if self.graph.entity(class).key == Keyword::Union {
                    self.constexpr_frames.mark_union(cell);
                } else {
                    self.prepare_constant_member_spans(class, location)?;
                    self.constexpr_frames.mark_class(cell, class);
                }
                for index in 0..self.graph.entity(class).bases.len() {
                    let base = self.graph.entity(class).bases[index];
                    let ty = self.bare_type(self.graph.lower_entity_type(base.class)?)?;
                    let offset = base
                        .offset
                        .ok_or(Error::Invariant("constant base without layout"))?;
                    let child = Address {
                        designated: ty,
                        offset: place.offset.wrapping_add(super::integral::signed(offset)),
                        ..place
                    };
                    let child_cell = self
                        .constexpr_frames
                        .add_child(cell, child, None, location)?;
                    self.populate_constant_skeleton(child, child_cell, location, origin)?;
                }
                for index in 0..self.graph.entity(class).members.len() {
                    let member = self.graph.entity(class).members[index];
                    if self.graph.unnamed_bit_field(member) {
                        continue;
                    }
                    let mut child = self.constant_member_slot(place, member, location)?;
                    child.designated = self.bare_type(child.designated)?;
                    let child_cell =
                        self.constexpr_frames
                            .add_child(cell, child, Some(member), location)?;
                    self.populate_constant_skeleton(child, child_cell, location, origin)?;
                }
            }
            _ => (),
        }
        self.constexpr_frames.finish_cell(cell);
        Ok(())
    }
    pub(super) fn constant_member_slot(
        &mut self,
        place: Address,
        member: EntityId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if place.past_end {
            return Err(nonconstant(
                location,
                "a member of an object one past the end of an array",
            ));
        }
        let Some(class) = self.graph.class_of(place.designated) else {
            return Err(nonconstant(
                location,
                "a conversion to a base class outside constant evaluation",
            ));
        };
        let mut home = self
            .graph
            .entity(member)
            .home
            .and_then(|scope| self.graph.scope(scope).owner)
            .ok_or(Error::Invariant("constant member without owner"))?;
        let mut chain = Vec::new();
        while home != class {
            let Some(storage) = self.graph.anonymous_member_storage(home) else {
                break;
            };
            chain.push(storage);
            home = self
                .graph
                .entity(storage)
                .home
                .and_then(|scope| self.graph.scope(scope).owner)
                .ok_or(Error::Invariant("anonymous constant member without owner"))?;
        }
        let mut place = place;
        if home != class {
            let path = self.graph.retain_base_path(class, home)?;
            place = self.constant_project_path(place, Some(path), location)?;
        }
        for storage in chain.into_iter().rev() {
            place = self.constant_direct_member_slot(place, storage, location)?;
        }
        self.constant_direct_member_slot(place, member, location)
    }
    fn constant_direct_member_slot(
        &mut self,
        place: Address,
        member: EntityId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let class = self
            .graph
            .class_of(place.designated)
            .ok_or(Error::Invariant("constant member owner type"))?;
        self.class_layout(class, location)?;
        let entity = self.graph.entity(member);
        let offset = if let Some(field) = entity.bit_field {
            self.graph.bit_fields.storage(field)?.bit / 8
        } else {
            entity
                .member_offset
                .ok_or(Error::Invariant("constant member without layout"))?
        };
        let field = entity.bit_field.map(|_| member);
        let declared = self.graph.lower_entity_type(member)?;
        let ty = if matches!(
            self.graph.types.get(declared).kind,
            TypeKind::Reference { .. }
        ) {
            declared
        } else {
            let cv = if self.graph.lower_mutable_member(member) {
                super::types::Cv::default()
            } else {
                self.type_cv(place.designated)
            };
            self.graph.types.qualify(declared, cv)?
        };
        Ok(Address {
            designated: ty,
            offset: place.offset.wrapping_add(super::integral::signed(offset)),
            selection: if let Some(field) = field {
                super::constant_addresses::Selection::BitField(field)
            } else if self.graph.entity(class).key == Keyword::Union {
                let owner = self.bare_type(place.designated)?;
                super::constant_addresses::Selection::Variant(self.constexpr_frames.designate(
                    member,
                    owner,
                    place.offset,
                )?)
            } else {
                place.selection
            },
            ..place
        })
    }
    pub(super) fn constant_member_designation(
        &mut self,
        object: Address,
        member: EntityId,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let mut place = self.constant_member_slot(object, member, location)?;
        if matches!(
            self.graph.types.get(place.designated).kind,
            TypeKind::Reference { .. }
        ) {
            place = self.read_constant_address(place, location)?;
        }
        place.designated = ty;
        Ok(place)
    }
    pub(super) fn store_constant_value(
        &mut self,
        mut place: Address,
        mut value: Value,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if let Some(member) = place.bit_field()
            && let Value::Integer(integer) = value
        {
            let field = self
                .graph
                .entity(member)
                .bit_field
                .ok_or(Error::Invariant("constant bit field without identity"))?;
            let width = self.graph.bit_fields.width(field);
            let kind = self
                .integral_kind(place.designated)
                .ok_or(Error::Invariant("nonintegral constant bit field"))?;
            // Preserve the frozen interpreter's low-half-only field rule:
            // fields at least 64 bits wide retain the complete assigned value.
            if width < 64
                && !matches!(
                    self.graph.types.get(place.designated).kind,
                    TypeKind::Scalar(super::types::Scalar::Bool)
                )
            {
                let mask = (1u64 << width) - 1;
                let mut bits = integer.bits & mask;
                if kind.signed && width != 0 && bits >> (width - 1) != 0 {
                    bits |= !mask;
                }
                value = Value::Integer(super::constants::Constant {
                    bits: kind.convert(bits),
                    ..integer
                });
            }
        }

        place.designated = self.bare_type(place.designated)?;
        self.constexpr_frames
            .write(&self.graph.constant_member_spans, place, value, location)
    }
    pub(super) fn constant_zero_object(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        place.designated = self.bare_type(place.designated)?;
        let cells =
            self.constexpr_frames
                .zero_cells(&self.graph.constant_member_spans, place, location)?;
        for index in cells {
            let child = self.constexpr_frames.cell_place(place, index);
            if matches!(
                self.graph.types.get(child.designated).kind,
                TypeKind::Class(_) | TypeKind::Complex(_) | TypeKind::Array { .. }
            ) || self.constant_lane_vector(child.designated).is_some()
            {
                continue;
            }
            if matches!(
                self.graph.types.get(child.designated).kind,
                TypeKind::Reference { .. }
            ) {
                continue;
            }
            let value = if self.vector_shape(child.designated).is_some() {
                Value::Integer(super::constants::Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    ty: child.designated,
                    bits: 0,
                    high: 0,
                })
            } else {
                self.zero_constant_scalar(child.designated)?
            };
            self.constexpr_frames.zero_value(index, value);
        }
        Ok(())
    }
    pub(super) fn activate_constant_place(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        place.designated = self.bare_type(place.designated)?;
        self.constexpr_frames
            .activate(&self.graph.constant_member_spans, place, location)?;
        Ok(())
    }
    pub(super) fn constant_object_place(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let id = self.graph.expression_facts.root(id);
        let facts = self.graph.expression_facts.get(id)?;
        if facts.category != ValueCategory::Prvalue {
            return self.constant_place(id, scope);
        }
        let location = self.tree.expressions.get(id).location;
        let place = self.allocate_constant_object(facts.ty, None, location)?;
        self.materialize_constant_object(id, place, scope)?;
        Ok(place)
    }
    pub(super) fn constant_receiver_place(
        &mut self,
        source: ObjectSource,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        match source {
            ObjectSource::Implicit => self
                .constexpr_frames
                .current()
                .ok()
                .and_then(|frame| frame.object)
                .ok_or_else(|| {
                    nonconstant(location, "`this` outside a constexpr member function call")
                }),
            ObjectSource::Explicit {
                expression,
                arrow: false,
            } => self.constant_object_place(expression, scope),
            ObjectSource::Explicit {
                expression,
                arrow: true,
            } => {
                let place = self.constant_arrow_operand(expression, scope)?;
                if place.root == Root::Null {
                    return Err(nonconstant(
                        location,
                        "a member function called through a null pointer",
                    ));
                }
                Ok(place)
            }
        }
    }
    pub(super) fn constant_arrow_operand(
        &mut self,
        base: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let count = self.graph.expression_facts.arrow_calls(base).len();
        if count == 0 {
            return self.address_operand(base, scope);
        }
        let location = self.tree.expressions.get(base).location;
        let mut object = self.constant_object_place(base, scope)?;
        for index in 0..count {
            let step = self.graph.expression_facts.arrow_calls(base)[index];
            let function = self.graph.binding(step.binding).entity;
            if self.graph.virtuals.function(function).is_virtual {
                return Err(nonconstant(
                    location,
                    "a virtual function call in a constant expression",
                ));
            }
            object = self.constant_project_path(object, step.path, location)?;
            let result =
                self.invoke_constant_function(function, &[], Some(object), scope, location)?;
            if index + 1 == count {
                return match self.returned_scalar(result, location)? {
                    Value::Pointer(pointer) => Ok(pointer),
                    _ => Err(Error::Invariant("arrow chain without pointer result")),
                };
            }
            let Returned::Place(place) = result else {
                return Err(Error::Invariant("intermediate arrow without object result"));
            };
            object = place;
        }
        Err(Error::Invariant("empty arrow chain"))
    }
    pub(super) fn constant_project_path(
        &mut self,
        place: Address,
        path: Option<BasePathId>,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let Some(path) = path else { return Ok(place) };
        let &(last, index) = self
            .graph
            .base_paths
            .get(path)
            .last()
            .ok_or(Error::Invariant("empty constant base path"))?;
        let base = self.graph.entity(last).bases[index].class;
        self.constant_base_subobject(place, base, location)
    }
    pub(super) fn constant_base_subobject(
        &mut self,
        mut place: Address,
        base: EntityId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if place.past_end {
            return Err(nonconstant(
                location,
                "a base class of an object one past the end of an array",
            ));
        }
        let class = self
            .graph
            .class_of(place.designated)
            .ok_or(Error::Invariant("constant base path without class"))?;
        let offset = self
            .constant_base_offset(class, base, location)?
            .ok_or_else(|| {
                nonconstant(
                    location,
                    "a conversion to a base class outside constant evaluation",
                )
            })?;
        place.offset = place.offset.wrapping_add(offset);
        place.designated = self.graph.types.qualify(
            self.graph.lower_entity_type(base)?,
            self.type_cv(place.designated),
        )?;
        Ok(place)
    }
    pub(super) fn constant_base_pointer(
        &mut self,
        place: Address,
        element: TypeId,
        path: BasePathId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if place.root == Root::Null {
            return Ok(Address {
                designated: element,
                ..place
            });
        }
        self.constant_cast_subobject(
            place,
            element,
            super::expression_facts::BaseCast {
                path,
                downcast: false,
                project_zero: false,
            },
            location,
        )
    }
    /// A cast owns its route independently of any conversion on its result.
    /// Downcasts inspect storage types without reading an object's value.
    pub(super) fn constant_cast_subobject(
        &mut self,
        mut place: Address,
        target: TypeId,
        cast: super::expression_facts::BaseCast,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let &(derived, _) = self
            .graph
            .base_paths
            .get(cast.path)
            .first()
            .ok_or(Error::Invariant("empty constant cast route"))?;
        if !cast.downcast && place.past_end {
            return Err(nonconstant(
                location,
                "a base class of an object one past the end of an array",
            ));
        }
        let edges = self.graph.base_paths.get(cast.path);
        let &(last, index) = edges
            .last()
            .ok_or(Error::Invariant("empty constant cast route"))?;
        let base = self.graph.entity(last).bases[index].class;
        let offset = self
            .constant_base_offset(derived, base, location)?
            .ok_or_else(|| {
                nonconstant(
                    location,
                    if cast.downcast {
                        "a conversion to a derived class outside constant evaluation"
                    } else {
                        "a conversion to a base class outside constant evaluation"
                    },
                )
            })?;
        if cast.downcast {
            place.offset = place.offset.wrapping_sub(offset);
            let root = self.address_root_type(place, location)?;
            if !self.constant_type_holds(root, place.offset, target, location)? {
                return Err(nonconstant(
                    location,
                    "a cast to a derived class of an object that is not one",
                ));
            }
        } else {
            place.offset = place.offset.wrapping_add(offset);
        }
        place.designated = target;
        Ok(place)
    }
    pub(super) fn constant_type_holds(
        &mut self,
        root: TypeId,
        offset: i64,
        wanted: TypeId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        // Like ConstantInterpreter::holds, this is a type/layout query, including
        // inactive union members. Array indices are selected arithmetically;
        // repeated base graphs visit each (type, offset) only once per query.
        if offset == 0 && self.same_array_shape(root, wanted) {
            return Ok(true);
        }
        let mut query = self
            .constant_evaluation
            .as_mut()
            .ok_or(Error::Invariant("projection outside constant evaluation"))?
            .projections
            .take()
            .unwrap_or_default();
        query.work.push((root, offset));
        let result = self.constant_subobject_search(wanted, location, &mut query);
        query.work.clear();
        query.visited.clear();
        self.constant_evaluation
            .as_mut()
            .ok_or(Error::Invariant("projection lost evaluation storage"))?
            .projections = Some(query);
        result
    }
    fn constant_subobject_search(
        &mut self,
        wanted: TypeId,
        location: SourceOffset,
        query: &mut ProjectionQuery,
    ) -> Result<bool, Error> {
        while let Some((ty, offset)) = query.work.pop() {
            if offset == 0 && self.same_array_shape(ty, wanted) {
                return Ok(true);
            }
            if offset < 0 || !query.visited.insert((ty, offset)) {
                continue;
            }
            match self.graph.types.get(ty).kind {
                TypeKind::Complex(element) => {
                    let size = self.type_size(element, location)?.0;
                    for start in [0, super::integral::signed(size)] {
                        self.queue_constant_subobject(
                            &mut query.work,
                            element,
                            start,
                            offset,
                            location,
                        )?;
                    }
                }
                TypeKind::Array { element, bound, .. }
                | TypeKind::Vector { element, bound, .. } => {
                    if self.vector_shape(ty).is_some() && self.constant_lane_vector(ty).is_none() {
                        continue;
                    }
                    let size = self.type_size(element, location)?.0;
                    let offset = offset.cast_unsigned();
                    if size != 0 && offset / size < bound {
                        query
                            .work
                            .push((element, super::integral::signed(offset % size)));
                    }
                }
                TypeKind::Class(class) => {
                    self.class_layout(class, location)?;
                    for index in 0..self.graph.entity(class).bases.len() {
                        let edge = self.graph.entity(class).bases[index];
                        let ty = self.graph.lower_entity_type(edge.class)?;
                        let start = super::integral::signed(
                            edge.offset.ok_or(Error::Invariant("base without layout"))?,
                        );
                        self.queue_constant_subobject(
                            &mut query.work,
                            ty,
                            start,
                            offset,
                            location,
                        )?;
                    }
                    for index in 0..self.graph.entity(class).members.len() {
                        let member = self.graph.entity(class).members[index];
                        let entity = self.graph.entity(member);
                        if entity.bit_field.is_some() {
                            continue;
                        }
                        let ty = self.graph.lower_entity_type(member)?;
                        let start = super::integral::signed(
                            entity
                                .member_offset
                                .ok_or(Error::Invariant("member without layout"))?,
                        );
                        self.queue_constant_subobject(
                            &mut query.work,
                            ty,
                            start,
                            offset,
                            location,
                        )?;
                    }
                }
                _ => (),
            }
        }
        Ok(false)
    }

    fn queue_constant_subobject(
        &mut self,
        work: &mut Vec<(TypeId, i64)>,
        ty: TypeId,
        start: i64,
        offset: i64,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let size = super::integral::signed(self.type_size(ty, location)?.0.max(1));
        if offset >= start && offset < start.wrapping_add(size) {
            work.push((ty, offset.wrapping_sub(start)));
        }
        Ok(())
    }
    // A recorded conversion function is the operand's value directly.
    // Rvalue queries own a separate source-expression visit; operand queries
    // do not enter that operation before locating the conversion's receiver.
    pub(super) fn constant_conversion_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        let step = self
            .graph
            .expression_facts
            .function_conversion(id)
            .ok_or(Error::Invariant("constant conversion without selection"))?;
        let location = self.tree.expressions.get(id).location;
        let returned = self.constant_conversion_result(id, scope)?;
        let mut value = self.returned_scalar(returned, location)?;
        if let Value::Pointer(pointer) = value
            && let Some(path) = step.after_path
            && let TypeKind::Pointer(element) = self.graph.types.get(step.after.result).kind
        {
            value = Value::Pointer(self.constant_base_pointer(pointer, element, path, location)?);
        }
        if let Value::MemberPointer(member) = value
            && matches!(
                self.graph.types.get(step.after.result).kind,
                TypeKind::MemberPointer { .. }
            )
        {
            value =
                Value::MemberPointer(self.adjusted_member(member, step.after.result, location)?);
        }
        self.convert_constant_scalar(value, step.after.result, location)
    }
    pub(super) fn constant_conversion_result(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Returned, Error> {
        let location = self.tree.expressions.get(id).location;
        let step = self
            .graph
            .expression_facts
            .function_conversion(id)
            .ok_or(Error::Invariant("constant conversion without selection"))?;
        let function = self.graph.binding(step.binding).entity;
        let place = self.constant_object_place(id, scope)?;
        let place = self.constant_project_path(place, step.path, location)?;
        if self.graph.virtuals.function(function).is_virtual {
            return Err(nonconstant(
                location,
                "a virtual conversion function in a constant expression",
            ));
        }
        self.invoke_constant_function(function, &[], Some(place), scope, location)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{analyzer::Analyzer, constants::ConstantState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn conditional_references_keep_returned_storage_and_original_receivers() {
        use crate::sema::{constant_addresses::Root, expression_facts::ValueCategory};

        let source = SourceBuffer::new(
            br"
            struct Pad {int pad; constexpr Pad():pad(0){}};
            struct Value {int n; constexpr Value(int n):n(n){}};
            struct Derived:Pad,Value {constexpr Derived():Pad(),Value(17){}};
            constexpr Derived storage;
            struct Proxy {
                constexpr Proxy(){}
                constexpr operator const Derived&()const {return storage;}
            };
            constexpr Proxy receiver;
            constexpr auto& first=true?Proxy():static_cast<const Value&>(storage);
            constexpr auto& second=false?static_cast<const Value&>(storage):receiver;
            struct Counter {
                int* calls; int* value;
                constexpr Counter(int& c,int& v):calls(&c),value(&v){}
                constexpr operator int&(){++*calls;return *value;}
            };
            constexpr int count(){int calls=0,n=9;Counter c(calls,n);
                const int& r=true?c:n;return 100*calls+r;}
            constexpr int once=count();
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"conditional-storage").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"storage".as_slice(), b"first", b"second", b"once"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        analyzer.finish_static_initializers().unwrap();
        let entities: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
                analyzer.graph.binding(binding).entity
            })
            .collect();
        for reference in &entities[1..3] {
            let place = analyzer.graph.constant_addresses.value(*reference).unwrap();
            assert_eq!(place.root, Root::Variable(entities[0]));
            // The frozen glvalue interpreter keeps the selected result's
            // designation even when the conditional has a base result type.
            assert_eq!(place.offset, 0);
        }
        assert_eq!(
            analyzer
                .graph
                .lower_initializer_constructors(entities[1])
                .len(),
            1
        );
        assert!(
            analyzer
                .graph
                .lower_initializer_constructors(entities[2])
                .is_empty()
        );
        assert!(matches!(analyzer.graph.entity(entities[3]).constant,
            ConstantState::Value(value) if value.bits == 109));
        let expressions = analyzer.tree.expressions.mark();
        for declaration in &analyzer.tree.decl_lists.get(declarations)[6..8].to_vec() {
            let ast::DeclKind::Simple { declarators, .. } =
                analyzer.tree.declarations.get(*declaration).kind
            else {
                panic!("reference declaration");
            };
            let initializer = analyzer.tree.init_declarators.get(declarators)[0]
                .initializer
                .unwrap();
            let expr = analyzer.single_initializer(initializer).unwrap();
            let ast::ExprKind::Conditional {
                then_expr,
                else_expr,
                ..
            } = analyzer.tree.expressions.get(expr).kind
            else {
                panic!("conditional initializer");
            };
            let receiver = [then_expr, else_expr]
                .into_iter()
                .find(|&operand| {
                    analyzer
                        .graph
                        .expression_facts
                        .function_conversion(operand)
                        .is_some()
                })
                .unwrap();
            let original = analyzer.graph.expression_facts.get(receiver).unwrap();
            for _ in 0..2 {
                let place = analyzer.constant_place(expr, global).unwrap();
                assert_eq!(place.root, Root::Variable(entities[0]));
                assert_eq!(place.offset, 0);
                let facts = analyzer.graph.expression_facts.get(receiver).unwrap();
                assert_eq!(facts.ty, original.ty);
                assert_eq!(facts.category, original.category);
                assert_ne!(facts.ty, place.designated);
                assert_eq!(
                    analyzer.graph.expression_facts.get(expr).unwrap().category,
                    ValueCategory::Lvalue
                );
            }
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert!(!analyzer.constexpr_frames.active());
        assert!(analyzer.constant_evaluation.is_none());
        assert!(analyzer.initializing_declarator.is_none());
    }

    #[test]
    fn base_cast_queries_keep_storage_identity_and_release_failed_probes() {
        let source = SourceBuffer::new(
            br"
            struct A{int n;constexpr A(int n):n(n){}};
            struct B{int n;constexpr B(int n):n(n){}};
            struct D:A,B{int n;constexpr D(int n):A(n),B(n+1),n(n+2){}};
            struct E{int pad;D d;constexpr E(int n):pad(1),d(n){}};
            constexpr const D& down(const B& b){return static_cast<const D&>(b);}
            constexpr int read(int n){E e(n);const B* p=&e.d;return down(*p).n;}
            constexpr int first=read(5);
            int fallback=down(B(3)).n;
            constexpr int second=read(9);
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constant-base-storage").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"first".as_slice(), b"second"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        let expressions = analyzer.tree.expressions.mark();
        analyzer.declarations(declarations, global).unwrap();
        analyzer.finish_static_initializers().unwrap();
        for (name, bits) in names.into_iter().zip([7, 11]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            assert!(
                matches!(analyzer.graph.entity(analyzer.graph.binding(binding).entity).constant,
                ConstantState::Value(value) if value.bits == bits)
            );
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert!(!analyzer.constexpr_frames.active());
        assert!(analyzer.constant_evaluation.is_none());
        assert!(
            analyzer
                .constant_scratch
                .projections
                .as_ref()
                .unwrap()
                .work
                .is_empty()
        );
        assert!(
            analyzer
                .constant_scratch
                .projections
                .as_ref()
                .unwrap()
                .visited
                .is_empty()
        );
    }

    #[test]
    fn objects_keep_invocation_values_local_and_reuse_concrete_member_bodies() {
        let source = SourceBuffer::new(
            br"
            template<class T>struct S{
                T n;
                constexpr S(T x):n(x){}
                constexpr operator T()const {int sum=0;for(int i=0;i<n;++i)sum+=i;return sum;}
                void unused(){T::missing;}
            };
            constexpr int a=int(S<int>(3));
            constexpr int b=int(S<int>(5));
            constexpr int f(){S<int> s(4);const S<int>& r=s;return int(r);}
            constexpr int c=f();
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constexpr-objects").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"a".as_slice(), b"b", b"c"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        for (name, bits) in names.into_iter().zip([3, 10, 6]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            assert!(
                matches!(analyzer.graph.entity(analyzer.graph.binding(binding).entity).constant,
                ConstantState::Value(value) if value.bits == bits)
            );
        }
        let declaration = analyzer.tree.decl_lists.get(declarations)[1];
        let ast::DeclKind::Simple { declarators, .. } =
            analyzer.tree.declarations.get(declaration).kind
        else {
            panic!("simple declaration");
        };
        let initializer = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let expr = analyzer.single_initializer(initializer).unwrap();
        let mark = analyzer.tree.expressions.mark();
        assert_eq!(
            analyzer
                .constant_value(expr, global, true)
                .unwrap()
                .unwrap()
                .bits,
            3
        );
        assert_eq!(analyzer.tree.expressions.mark(), mark);
        assert!(!analyzer.constexpr_frames.active());
        assert!(analyzer.constant_evaluation.is_none());
    }
}

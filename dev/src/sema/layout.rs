// Translated from sema/layout.cpp's type_size, lay_out and ordinary member placement. See NOTICE.
//! Demand-only layout memos and an explicit dependency worklist for plain classes/unions.
use super::{Error, analyzer::Analyzer, graph::EntityId, types::TypeKind};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};

#[derive(Clone, Copy, Debug)]
pub(super) enum LayoutState {
    NotStarted,
    InProgress,
    Value {
        size: u64,
        alignment: u64,
        non_virtual_size: u64,
        empty: bool,
        contains_empty: bool,
        contains_volatile_or_union: bool,
    },
    Unsupported,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LayoutCategory {
    Pod,
    NonPod,
}
pub(super) enum StorageDemand {
    Object(super::types::TypeId),
    AnonymousUnion(EntityId),
    CallBoundary(super::types::TypeId),
}
/// Incremental unit-tail reachability. A vtable can demand a template member
/// body; resume only its suspended boundary and newly published edges after
/// that body is analyzed, without rewalking previously completed functions.
#[derive(Default)]
pub(super) struct Boundaries {
    root_count: usize,
    explicit_roots: bool,
    seen_tables: std::collections::HashSet<EntityId>,
    seen_constructors: std::collections::HashSet<super::construction::ConstructorId>,
    seen: std::collections::HashSet<EntityId>,
    signatures: std::collections::HashSet<super::types::TypeId>,
    destruction_edges: std::collections::HashMap<EntityId, Vec<EntityId>>,
    constructor_edges: std::collections::HashMap<EntityId, Vec<super::construction::ConstructorId>>,
    layouts: std::collections::HashMap<EntityId, Vec<(StorageDemand, SourceOffset)>>,
}
pub(super) fn round_up(value: u64, alignment: u64) -> u64 {
    value.wrapping_add(alignment - 1) / alignment * alignment
}
impl Analyzer<'_> {
    pub(super) fn request_union_storage(&mut self, entity: EntityId, location: SourceOffset) {
        self.graph.storage_layouts.push((
            self.body_context.as_ref().map(|body| body.entity),
            StorageDemand::AnonymousUnion(entity),
            location,
        ));
    }
    pub(super) fn request_storage_layout(
        &mut self,
        entity: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        self.request_object_layout(ty, location);
        Ok(())
    }
    pub(super) fn request_object_layout(
        &mut self,
        ty: super::types::TypeId,
        location: SourceOffset,
    ) {
        self.request_object_layout_for(
            ty,
            location,
            self.body_context.as_ref().map(|body| body.entity),
        );
    }
    pub(super) fn request_object_layout_for(
        &mut self,
        ty: super::types::TypeId,
        location: SourceOffset,
        owner: Option<EntityId>,
    ) {
        let mut object = ty;
        while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
            object = element;
        }
        if matches!(self.graph.types.get(object).kind, TypeKind::Class(_)) {
            self.graph
                .storage_layouts
                .push((owner, StorageDemand::Object(ty), location));
        }
    }
    pub(super) fn finish_lowering_boundaries(&mut self) -> Result<(), Error> {
        let mut state = std::mem::take(&mut self.graph.boundaries);
        let result = self.advance_lowering_boundaries(&mut state);
        self.graph.boundaries = state;
        result
    }
    fn constructor_default_boundaries(
        &mut self,
        entity: Option<EntityId>,
        first: usize,
        work: &mut Vec<EntityId>,
        constructors: &mut Vec<super::construction::ConstructorId>,
    ) -> Result<usize, Error> {
        self.prepare_constructor_defaults(entity)?;
        for demand in &self.graph.root_demands.as_slice()[first..] {
            match demand {
                super::function_demands::Demand::Function { binding, .. } => {
                    work.push(self.graph.binding(*binding).entity);
                }
                super::function_demands::Demand::Constructor(id) => constructors.push(*id),
            }
        }
        Ok(self.graph.root_demands.as_slice().len())
    }
    fn constructor_plan_boundaries(
        &mut self,
        constructor: super::construction::ConstructorId,
        root_count: &mut usize,
        seen_constructors: &mut std::collections::HashSet<super::construction::ConstructorId>,
        seen_tables: &mut std::collections::HashSet<EntityId>,
        work: &mut Vec<EntityId>,
        constructors: &mut Vec<super::construction::ConstructorId>,
    ) -> Result<(), Error> {
        if let Some(binding) = self.graph.constructors.get(constructor).binding {
            // Reached storage construction (including arrays) demands
            // the selected definition, separately from its signature.
            self.demand_specialization_body(binding);
            let entity = self.graph.binding(binding).entity;
            if self.graph.function_templates.bodies.pending_member(entity)
                && !self
                    .graph
                    .function_templates
                    .specializations
                    .contains(entity)
            {
                self.demand_instance_member(entity);
                self.graph.boundary_roots.push(entity);
                return Ok(());
            }
        }
        if !seen_constructors.insert(constructor) {
            return Ok(());
        }
        self.request_subobject_constructor_defaults(constructor)?;
        *root_count = self.constructor_default_boundaries(None, *root_count, work, constructors)?;
        let class = self.graph.constructors.get(constructor).class;
        self.virtual_table_boundaries(class, seen_tables, work)?;
        let plan = self.graph.constructors.get(constructor);
        if let Some(binding) = plan.binding {
            work.push(self.graph.binding(binding).entity);
        }
        constructors.extend(plan.bases.iter().filter_map(|base| base.constructor));
        constructors.extend(plan.subobjects.iter().map(|child| child.constructor));
        constructors.extend(
            plan.copy
                .iter()
                .flatten()
                .filter_map(|action| action.constructor)
                .filter(|&id| self.graph.constructors.get(id).nontrivial()),
        );
        Ok(())
    }
    fn advance_lowering_boundaries(&mut self, state: &mut Boundaries) -> Result<(), Error> {
        use super::function_demands::Demand;
        let Boundaries {
            root_count,
            explicit_roots,
            seen_tables,
            seen_constructors,
            seen,
            signatures,
            destruction_edges,
            constructor_edges,
            layouts,
        } = state;
        self.prepare_constructor_defaults(None)?;
        let mut work = std::mem::take(&mut self.graph.boundary_roots);
        // Explicit definitions are emitted even without a calling expression.
        // Prepare only those named roots' ordinary class ABI boundaries.
        if !*explicit_roots {
            work.extend(self.graph.lower_explicit_functions());
            for index in 0..self.graph.lower_explicit_classes().len() {
                let class = self.graph.lower_explicit_classes()[index];
                self.virtual_table_boundaries(class, seen_tables, &mut work)?;
            }
            *explicit_roots = true;
        }
        for (owner, destructor) in std::mem::take(&mut self.graph.storage_destructions) {
            if let Some(owner) = owner {
                destruction_edges.entry(owner).or_default().push(destructor);
            } else {
                work.push(destructor);
            }
        }
        for demand in &self.graph.root_demands.as_slice()[*root_count..] {
            if let Demand::Function { binding, .. } = demand {
                work.push(self.graph.binding(*binding).entity);
            }
        }
        let mut constructors = Vec::new();
        for (owner, constructor) in std::mem::take(&mut self.graph.constructor_boundaries) {
            if let Some(owner) = owner {
                constructor_edges
                    .entry(owner)
                    .or_default()
                    .push(constructor);
            } else {
                constructors.push(constructor);
            }
        }
        for demand in &self.graph.root_demands.as_slice()[*root_count..] {
            if let Demand::Constructor(id) = demand {
                constructors.push(*id);
            }
        }
        *root_count = self.graph.root_demands.as_slice().len();
        while !work.is_empty() || !constructors.is_empty() {
            if let Some(constructor) = constructors.pop() {
                self.constructor_plan_boundaries(
                    constructor,
                    root_count,
                    seen_constructors,
                    seen_tables,
                    &mut work,
                    &mut constructors,
                )?;
                continue;
            }
            let entity = work
                .pop()
                .ok_or(Error::Invariant("missing boundary demand"))?;
            if seen.contains(&entity) {
                continue;
            }
            if self.graph.function_templates.bodies.pending_member(entity)
                && !self
                    .graph
                    .function_templates
                    .specializations
                    .contains(entity)
            {
                self.demand_instance_member(entity);
                self.graph.boundary_roots.push(entity);
                continue;
            }
            if self.graph.function_templates.primary(entity).is_some() {
                self.resolve_specialization_exception(entity)?;
            }
            *root_count = self.constructor_default_boundaries(
                Some(entity),
                *root_count,
                &mut work,
                &mut constructors,
            )?;
            seen.insert(entity);
            self.request_qualifier_signature(entity);
            if let Some(requests) = layouts.remove(&entity) {
                self.graph.storage_layouts.extend(
                    requests
                        .into_iter()
                        .map(|(demand, location)| (None, demand, location)),
                );
            }
            self.assignment_boundaries(entity, &mut work, &mut constructors)?;
            if let Some(constructor) = self.graph.entity(entity).constructor {
                constructors.push(constructor);
            }
            if let Some(children) = constructor_edges.remove(&entity) {
                constructors.extend(children);
            }
            if let Some(destructors) = destruction_edges.remove(&entity) {
                work.extend(destructors);
            }
            for demand in self.graph.entity(entity).demands.as_slice() {
                match demand {
                    Demand::Function { binding, .. } => {
                        work.push(self.graph.binding(*binding).entity);
                    }
                    Demand::Constructor(id) => constructors.push(*id),
                }
            }
            if let Some(class) = self.graph.lower_function(entity)?.object {
                self.class_layout(class, SourceOffset(0))?;
                if self.graph.lower_polymorphic(class)
                    && self.graph.lower_virtual_class(class)?.key == Some(entity)
                {
                    self.virtual_table_boundaries(class, seen_tables, &mut work)?;
                }
                if self
                    .graph
                    .entity(entity)
                    .member_function
                    .is_some_and(|key| key.kind == super::graph::MemberKind::Destructor)
                {
                    self.prepare_destruction(class)?;
                    self.virtual_table_boundaries(class, seen_tables, &mut work)?;
                    // A reached virtual destructor emits a deleting entry. Its
                    // recorded call target has an independent body/signature
                    // boundary, even when it is a copied static class member.
                    if let Some(deallocator) = self.graph.virtual_deallocator(class) {
                        work.push(deallocator);
                    }
                }
            }
            let ty = self.graph.lower_entity_type(entity)?;
            if let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind
                && let TypeKind::Class(class) = self.graph.types.get(result).kind
                && self.graph.entity(class).complete
            {
                self.class_layout(class, SourceOffset(0))?;
                self.prepare_class_parameter(class)?;
            }
            let count = self.graph.types.function_parameter_count(ty)?;
            for index in 0..count {
                let parameter = self.graph.types.function_parameter(ty, index)?;
                if let TypeKind::Class(class) = self.graph.types.get(parameter).kind
                    && self.graph.entity(class).complete
                {
                    self.class_layout(class, SourceOffset(0))?;
                    self.prepare_class_parameter(class)?;
                    // The caller owns a by-value parameter, including one made
                    // by a selected copy constructor's default argument.
                    self.prepare_destruction(class)?;
                    if let Some(binding) = self.graph.destructor_binding(class) {
                        work.push(self.graph.binding(binding).entity);
                    }
                }
                self.prepare_reference_boundary(parameter, SourceOffset(0))?;
            }
            // Destruction preparation can publish containment edges while this
            // demand closure is running. Activate only edges of reached owners.
            for (owner, destructor) in std::mem::take(&mut self.graph.storage_destructions) {
                if let Some(owner) = owner.filter(|owner| !seen.contains(owner)) {
                    destruction_edges.entry(owner).or_default().push(destructor);
                } else {
                    work.push(destructor);
                }
            }
        }
        for (owner, demand, location) in std::mem::take(&mut self.graph.storage_layouts) {
            if owner.is_none_or(|owner| seen.contains(&owner)) {
                match demand {
                    StorageDemand::Object(ty) => {
                        self.type_size(ty, location)?;
                    }
                    StorageDemand::AnonymousUnion(entity) => {
                        self.type_size(self.graph.lower_entity_type(entity)?, location)?;
                        if !self.graph.lower_automatic(entity) {
                            let scope = self
                                .graph
                                .entity(entity)
                                .home
                                .ok_or(Error::Invariant("union object without scope"))?;
                            self.record_union_default_image(entity, scope)?;
                        }
                    }
                    StorageDemand::CallBoundary(ty) => {
                        if signatures.insert(ty) {
                            self.prepare_indirect_boundary(ty, location)?;
                        }
                    }
                }
            } else if let Some(owner) = owner {
                layouts.entry(owner).or_default().push((demand, location));
            }
        }
        Ok(())
    }
    fn prepare_indirect_boundary(
        &mut self,
        ty: super::types::TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("indirect boundary without function type"));
        };
        let count = self.graph.types.function_parameter_count(ty)?;
        for index in 0..=count {
            let object = if index == count {
                result
            } else {
                self.graph.types.function_parameter(ty, index)?
            };
            if let TypeKind::Class(class) = self.graph.types.get(object).kind
                && self.graph.entity(class).complete
            {
                self.class_layout(class, location)?;
                self.prepare_class_parameter(class)?;
            } else if index < count {
                self.prepare_reference_boundary(object, location)?;
            }
        }
        Ok(())
    }
    fn prepare_reference_boundary(
        &mut self,
        ty: super::types::TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind else {
            return Ok(());
        };
        // symbol_lowering.cpp::parameter_for asks object_layout, which may
        // complete a referenced specialization. This is signature layout demand,
        // independent of object construction, member bodies and emission.
        match self.graph.types.get(element).kind {
            TypeKind::Function { .. }
            | TypeKind::Scalar(super::types::Scalar::Void)
            | TypeKind::Array { bound: 0, .. } => return Ok(()),
            _ => (),
        }
        if let TypeKind::Class(class) = self.graph.types.get(element).kind
            && !self.complete_class(class)?
        {
            return Ok(());
        }
        self.type_size(element, location)?;
        Ok(())
    }
    pub(super) fn class_layout(
        &mut self,
        root: EntityId,
        location: SourceOffset,
    ) -> Result<(u64, u64), Error> {
        let base = self.layout_work.len();
        let result = self.finish_layout(root, location);
        if result.is_err() {
            for &(entity, _) in &self.layout_work[base..] {
                self.graph.entity_mut(entity).layout = LayoutState::NotStarted;
            }
        }
        self.layout_work.truncate(base);
        result
    }
    fn start_layout(&mut self, entity: EntityId, location: SourceOffset) -> Result<bool, Error> {
        if !self.complete_class(entity)? {
            return Err(Error::at(location, "size of an incomplete class"));
        }
        // Virtual-base formation and unevaluated path queries do not demand
        // storage. Its ABI placement/construction tables are a separate group.
        if self
            .graph
            .entity(entity)
            .bases
            .iter()
            .any(|edge| edge.is_virtual)
        {
            return Err(Error::Missing("virtual base class layout"));
        }
        match self.graph.entity(entity).layout {
            LayoutState::Value { .. } => Ok(false),
            LayoutState::InProgress => Err(Error::at(location, "class contains itself")),
            LayoutState::Unsupported => Err(Error::Missing(
                "class layout attributes and virtual members",
            )),
            LayoutState::NotStarted => {
                self.graph.entity_mut(entity).layout = LayoutState::InProgress;
                self.layout_work.push((entity, 0));
                Ok(true)
            }
        }
    }
    fn finish_layout(
        &mut self,
        root: EntityId,
        location: SourceOffset,
    ) -> Result<(u64, u64), Error> {
        let base = self.layout_work.len();
        self.start_layout(root, location)?;
        while self.layout_work.len() > base {
            let &(entity, index) = self
                .layout_work
                .last()
                .ok_or(Error::Invariant("missing layout frame"))?;
            let bases = self.graph.entity(entity).bases.len();
            if let Some(dependency) = self
                .graph
                .entity(entity)
                .bases
                .get(index)
                .map(|edge| edge.class)
            {
                self.layout_work
                    .last_mut()
                    .ok_or(Error::Invariant("missing layout frame"))?
                    .1 += 1;
                self.start_layout(dependency, location)?;
            } else if let Some(&member) = self.graph.entity(entity).members.get(index - bases) {
                self.layout_work
                    .last_mut()
                    .ok_or(Error::Invariant("missing layout frame"))?
                    .1 += 1;
                let mut ty = self
                    .graph
                    .entity(member)
                    .ty
                    .ok_or(Error::Invariant("member without type"))?;
                while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                    ty = element;
                }
                if let TypeKind::Class(dependency) = self.graph.types.get(ty).kind {
                    self.start_layout(dependency, location)?;
                }
            } else {
                self.place_members(entity, location)?;
                self.layout_work.pop();
            }
        }
        let LayoutState::Value {
            size, alignment, ..
        } = self.graph.entity(root).layout
        else {
            return Err(Error::Invariant("uncompleted layout"));
        };
        Ok((size, alignment))
    }
    fn place_members(&mut self, entity: EntityId, location: SourceOffset) -> Result<(), Error> {
        let mut bits = std::mem::take(&mut self.bit_layout);
        let result = self.place_class_members(entity, location, &mut bits);
        bits.clear();
        self.bit_layout = bits;
        result
    }
    fn place_class_members(
        &mut self,
        entity: EntityId,
        location: SourceOffset,
        bits: &mut super::bit_layout::BitLayout,
    ) -> Result<(), Error> {
        let union = self.graph.entity(entity).key == Keyword::Union;
        let mut size = 0u64;
        let mut data_size = 0u64;
        let mut contains_volatile_or_union = union;
        let mut empty = self.graph.entity(entity).members.iter().all(|&member| {
            let member = self.graph.entity(member);
            member.name.is_none()
                && member
                    .bit_field
                    .is_some_and(|id| self.graph.bit_fields.width(id) == 0)
        });
        let polymorphic = self.graph.entity(entity).polymorphism.is_dynamic();
        let primary = self
            .graph
            .entity(entity)
            .bases
            .iter()
            .position(|edge| self.graph.entity(edge.class).polymorphism.is_dynamic());
        if polymorphic {
            empty = false;
            if primary.is_none() {
                size = 8;
                data_size = 8;
            }
        }
        let mut contains_empty = false;
        let mut alignment = if polymorphic && primary.is_none() {
            8
        } else {
            1
        };
        let attributes = self.graph.entity(entity).storage_attributes;
        let pack = u64::from(self.graph.entity(entity).class_packing);
        let mut non_pod = polymorphic
            || self.graph.entity(entity).layout_category == LayoutCategory::NonPod
            || !self.graph.entity(entity).bases.is_empty();
        let mut packs_all = true;
        let mut placement = super::empty_layout::EmptyPlacement::new(&self.graph, entity)?;
        for index in primary.into_iter().chain(
            (0..self.graph.entity(entity).bases.len()).filter(|index| Some(*index) != primary),
        ) {
            let base = self.graph.entity(entity).bases[index].class;
            let LayoutState::Value {
                size: bytes,
                alignment: natural,
                non_virtual_size,
                empty: base_empty,
                contains_empty: child_empty,
                contains_volatile_or_union: child_volatile,
            } = self.graph.entity(base).layout
            else {
                return Err(Error::Invariant("base without completed layout"));
            };
            empty &= base_empty;
            contains_empty |= child_empty;
            contains_volatile_or_union |= child_volatile;
            let base_alignment = if base_empty || pack == 0 {
                natural
            } else {
                natural.min(pack)
            };
            let mut offset = if base_empty {
                0
            } else {
                round_up(data_size, base_alignment)
            };
            if placement.conflict(&self.graph, base, offset, location)? {
                if base_empty {
                    offset = round_up(data_size, natural);
                }
                while placement.conflict(&self.graph, base, offset, location)? {
                    offset = offset.wrapping_add(natural);
                }
            }
            if base_empty {
                size = size.max(offset.wrapping_add(bytes));
            } else {
                data_size = offset.wrapping_add(non_virtual_size);
                size = size.max(data_size);
            }
            if size >> 60 != 0 {
                return Err(Error::at(
                    location,
                    "a class too large (its size reaches 2^60 bytes)",
                ));
            }
            alignment = alignment.max(base_alignment);
            self.graph.entity_mut(entity).bases[index].offset = Some(offset);
            placement.record_base(&self.graph, base, offset, data_size, base_empty, location)?;
        }
        contains_empty |= empty;
        let mut data_bits = data_size * 8;
        for index in 0..self.graph.entity(entity).members.len() {
            let member = self.graph.entity(entity).members[index];
            let ty = self
                .graph
                .entity(member)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            let (bytes, natural) = self.type_size(ty, location)?;
            let mut scalar = ty;
            while let TypeKind::Array { element, .. } = self.graph.types.get(scalar).kind {
                scalar = element;
            }
            contains_volatile_or_union |= self.graph.types.get(scalar).cv.0 & 2 != 0;
            let mut packable = true;
            if let TypeKind::Class(child) = self.graph.types.get(scalar).kind {
                let child = self.graph.entity(child);
                non_pod |= child.layout_category == LayoutCategory::NonPod;
                packable =
                    child.layout_category == LayoutCategory::Pod || child.storage_attributes.packed;
                if let LayoutState::Value {
                    contains_volatile_or_union: child,
                    contains_empty: child_empty,
                    ..
                } = child.layout
                {
                    contains_volatile_or_union |= child;
                    contains_empty |= child_empty
                        && super::empty_layout::class_elements(&self.graph, ty)
                            .is_some_and(|(_, count)| count != 0);
                }
            }
            packs_all &= packable;
            let member_attributes = self.graph.entity(member).storage_attributes;
            let requested = u64::from(member_attributes.alignment);
            let mut member_alignment = if member_attributes.packed || attributes.packed && packable
            {
                requested.max(1)
            } else {
                natural.max(requested)
            };
            if pack != 0 {
                member_alignment = member_alignment.min(pack);
            }
            if let Some(id) = self.graph.entity(member).bit_field {
                let named = self.graph.entity(member).name.is_some();
                let placed = bits.place_member(
                    &mut self.graph.bit_fields,
                    id,
                    super::bit_layout::Placement {
                        data_bits,
                        natural: bytes,
                        requested,
                        packed: member_attributes.packed || attributes.packed,
                        pack,
                        union,
                        named,
                        location,
                    },
                )?;
                data_bits = placed.data_bits;
                size = size.max(placed.size);
                alignment = alignment.max(placed.alignment);
                self.graph.entity_mut(member).member_offset = None;
                continue;
            }
            data_size = round_up(data_bits, 8) / 8;
            let mut offset = if union {
                0
            } else {
                round_up(data_size, member_alignment)
            };
            if !union {
                while placement.member_conflict(&self.graph, ty, offset, location)? {
                    offset = round_up(offset.wrapping_add(natural), member_alignment);
                }
                data_size = offset.wrapping_add(bytes);
                placement.record_member(&self.graph, ty, offset, data_size, location)?;
            }
            self.graph.entity_mut(member).member_offset = Some(offset);
            bits.end_run(offset);
            data_bits = data_size * 8;
            size = if union {
                size.max(bytes)
            } else {
                size.max(data_size)
            };
            alignment = alignment.max(member_alignment);
            if (bytes | size) >> 60 != 0 {
                return Err(Error::at(
                    location,
                    "a class too large (its size reaches 2^60 bytes)",
                ));
            }
        }
        let requested = u64::from(attributes.alignment);
        if requested != 0 {
            if requested >= alignment {
                alignment = requested;
            } else if attributes.standard_alignment {
                return Err(Error::at(
                    location,
                    "a class alignment-specifier weaker than its natural alignment",
                ));
            }
        }
        self.graph.entity_mut(entity).layout_category = if non_pod {
            LayoutCategory::NonPod
        } else {
            LayoutCategory::Pod
        };
        self.graph.entity_mut(entity).storage_attributes.packed &= packs_all;
        let full_size = round_up(size.max(1), alignment);
        bits.finish(
            &mut self.graph.bit_fields,
            if non_pod {
                round_up(data_bits, 8) / 8
            } else {
                full_size
            },
            location,
        )?;
        self.graph.entity_mut(entity).layout = LayoutState::Value {
            contains_volatile_or_union,
            empty,
            contains_empty,
            non_virtual_size: if non_pod { size } else { full_size },
            size: full_size,
            alignment,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::LayoutState;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn result_boundaries_prepare_only_demanded_complete_class_facts() {
        let source = SourceBuffer::new(b"struct S{int n;};using Alias=S;S f(){return S{1};}Alias g(){return Alias{2};}struct Unused{int n;};struct Incomplete;Incomplete named();int h(){auto p=&named;return p==0;}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"result-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        let mut entity = |spelling| {
            let name = cursor.intern_name(spelling).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity
        };
        let result = entity(b"S");
        assert!(graph.lower_direct_class_parameter(result).unwrap());
        assert!(matches!(
            graph.entity(result).layout,
            LayoutState::Value { size: 4, .. }
        ));
        for class in [entity(b"Unused"), entity(b"Incomplete")] {
            assert!(matches!(
                graph.entity(class).layout,
                LayoutState::NotStarted
            ));
            assert!(graph.entity(class).implicit_copies.is_none());
        }
    }

    #[test]
    fn reference_boundaries_complete_only_reached_parameter_specializations() {
        let source = SourceBuffer::new(
            br"
            template<class T> struct Box { T n; void unused(){T::missing();} };
            template<class T> struct Missing;
            using Named = Box<int>;
            using Indirect = Box<long>;
            using Dormant = Box<char>;
            using Result = Box<short>;
            using Incomplete = Missing<int>;
            void named(const Named&) {}
            void indirect(void(*p)(const Indirect&), Indirect* q) {p(*q);p(*q);}
            inline void dormant(Dormant&) {}
            void result(Result&(*p)()) {p();}
            void incomplete(Incomplete&) {}
            "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"reference-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [
            b"Named".as_slice(),
            b"Indirect",
            b"Dormant",
            b"Result",
            b"Incomplete",
        ]
        .into_iter()
        .map(|spelling| cursor.intern_name(spelling).unwrap())
        .collect();
        let unused = cursor.intern_name(b"unused").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let classes: Vec<_> = names
            .into_iter()
            .map(|name| {
                let alias = analyzer
                    .graph
                    .own(analyzer.graph.global, name, Filter::Prefix)
                    .unwrap();
                analyzer
                    .graph
                    .class_of(analyzer.graph.binding(alias).ty.unwrap())
                    .unwrap()
            })
            .collect();
        for &class in &classes {
            assert!(matches!(
                analyzer.graph.entity(class).layout,
                LayoutState::NotStarted
            ));
        }
        let complete: Vec<_> = classes
            .iter()
            .map(|&class| analyzer.graph.entity(class).complete)
            .collect();
        analyzer.finish_lowering_boundaries().unwrap();
        for ((&class, &was_complete), size) in
            classes
                .iter()
                .zip(&complete)
                .zip([Some(4), Some(8), None, None, None])
        {
            if let Some(expected) = size {
                assert!(matches!(analyzer.graph.entity(class).layout,
                    LayoutState::Value { size, .. } if size == expected));
                assert!(analyzer.graph.entity(class).complete);
            } else {
                assert_eq!(analyzer.graph.entity(class).complete, was_complete);
                assert!(matches!(
                    analyzer.graph.entity(class).layout,
                    LayoutState::NotStarted
                ));
            }
            assert!(analyzer.graph.entity(class).implicit_copies.is_none());
        }
        // Repeating the closure neither copies instantiated syntax nor starts
        // an unused member body, and retains the canonical layout facts.
        let expressions = analyzer.tree.expressions.mark();
        let nodes = analyzer.tree.declarations.mark();
        analyzer.finish_lowering_boundaries().unwrap();
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.declarations.mark(), nodes);
        for &class in &classes[..2] {
            let scope = analyzer.graph.entity(class).scope.unwrap();
            let binding = analyzer.graph.own(scope, unused, Filter::Ordinary).unwrap();
            let member = analyzer.graph.binding(binding).entity;
            assert!(!analyzer.graph.function_templates.bodies.started(member));
            assert!(!analyzer.graph.function_templates.body_demanded(member));
        }
    }
}

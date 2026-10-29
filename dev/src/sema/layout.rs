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
        non_virtual_alignment: u64,
        non_virtual_size: u64,
        empty: bool,
        contains_empty: bool,
        contains_volatile_or_union: bool,
        contains_data_member_pointer: bool,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LayoutCategory {
    Pod,
    NonPod,
}
pub(super) enum StorageDemand {
    Object(super::types::TypeId),
    VariadicClass(super::types::TypeId),
    AnonymousUnion(EntityId),
    CallBoundary(super::types::TypeId),
    TypeInformation(super::types::TypeId),
}
/// Incremental unit-tail reachability. A vtable can demand a template member
/// body; resume only its suspended boundary and newly published edges after
/// that body is analyzed, without rewalking previously completed functions.
#[derive(Default)]
pub(super) struct Boundaries {
    root_count: usize,
    explicit_roots: bool,
    definition_groups: Option<usize>,
    seen_tables: std::collections::HashSet<EntityId>,
    seen_constructors: std::collections::HashSet<super::construction::ConstructorId>,
    seen: std::collections::HashSet<EntityId>,
    signatures: std::collections::HashSet<super::types::TypeId>,
    destruction_edges: std::collections::HashMap<EntityId, Vec<EntityId>>,
    constructor_edges: std::collections::HashMap<EntityId, Vec<super::construction::ConstructorId>>,
    layouts: std::collections::HashMap<EntityId, Vec<(StorageDemand, SourceOffset)>>,
    class_informations: std::collections::HashMap<EntityId, Vec<EntityId>>,
}
pub(super) fn round_up(value: u64, alignment: u64) -> u64 {
    value.wrapping_add(alignment - 1) / alignment * alignment
}
fn check_class_size(size: u64, location: SourceOffset) -> Result<(), Error> {
    if size >> 60 != 0 {
        return Err(Error::at(
            location,
            "a class too large (its size reaches 2^60 bytes)",
        ));
    }
    Ok(())
}
impl Analyzer<'_> {
    pub(super) fn demand_storage_layout(
        &mut self,
        entity: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        // A typeid operand can become evaluated only after it is typed.
        // Keep its receiver/conversion layout with the other ordered uses;
        // replay in an enclosing query must defer it again rather than emit it.
        if self.deferring_type_query() {
            self.type_queries
                .uses
                .push(super::type_queries::DeferredUse::Layout(entity, location));
        }
        if self.unevaluated == 0 {
            self.request_storage_layout(entity, location)?;
        }
        Ok(())
    }
    pub(super) fn request_union_storage(&mut self, entity: EntityId, location: SourceOffset) {
        self.graph.storage_layouts.push((
            self.body_context.as_ref().and_then(|body| body.function),
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
            self.body_context.as_ref().and_then(|body| body.function),
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
        let result = self.advance_lowering_boundaries(&mut state).and_then(|()| {
            if let Some(next) = state.definition_groups.as_mut() {
                self.prepare_definition_group_layouts(next)?;
            }
            Ok(())
        });
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
        self.graph.demand_constructor_object_pointer(constructor)?;
        let plan = self.graph.constructors.get(constructor);
        if plan.binding.is_none()
            && let Some(function) = plan.declaration
        {
            self.graph.bind_implicit_constructor(function)?;
        }
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
                // Signature facts are needed by the caller before the selected
                // member body is instantiated or emitted.
                self.prepare_function_boundary(entity, work)?;
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
        self.constructor_cleanup_boundaries(class, work)?;
        self.virtual_table_boundaries(class, seen_tables, work)?;
        let plan = self.graph.constructors.get(constructor);
        if let Some(binding) = plan.binding {
            work.push(self.graph.binding(binding).entity);
        }
        constructors.extend(
            plan.bases
                .iter()
                .chain(&plan.indirect_bases)
                .filter(|base| !base.forwards)
                .filter_map(|base| base.constructor),
        );
        constructors.extend(
            plan.subobjects
                .iter()
                .flatten()
                .map(|child| child.constructor),
        );
        constructors.extend(
            plan.copy
                .iter()
                .flatten()
                .filter_map(|action| action.constructor)
                .filter(|&id| self.graph.constructors.get(id).nontrivial()),
        );
        Ok(())
    }
    fn prepare_definition_group_layouts(&mut self, next: &mut usize) -> Result<(), Error> {
        // The unit's completed table groups are the ABI inventory for an
        // emitted external virtual definition, even with external table keys.
        // Visit each group once; this requests layouts, never member bodies.
        while let Some(&class) = self.graph.lower_grouped_classes().get(*next) {
            let segments = self
                .graph
                .lower_vtable_group(class)
                .ok_or(Error::Invariant("completed group missing recipe"))?
                .segments
                .len();
            for segment in 0..segments {
                let through = self
                    .graph
                    .lower_vtable_group(class)
                    .ok_or(Error::Invariant("completed group missing recipe"))?
                    .segments[segment]
                    .class;
                // Lost-primary anchors and definition-owned virtual variants
                // read the primary chain even when its table key is external.
                self.graph.demand_virtual_chain(through, true)?;
            }
            let count = self
                .graph
                .lower_vtable_group(class)
                .ok_or(Error::Invariant("completed group missing recipe"))?
                .entries
                .len();
            if count != 0 {
                self.class_layout(class, SourceOffset(0))?;
            }
            for index in 0..count {
                let function = self
                    .graph
                    .lower_vtable_group(class)
                    .ok_or(Error::Invariant("completed group missing recipe"))?
                    .entries[index]
                    .function;
                if !self.graph.lower_virtual_pure(function) {
                    self.prepare_covariant_result(function)?;
                }
            }
            *next += 1;
        }
        Ok(())
    }
    fn prepare_virtual_definition_layouts(
        &mut self,
        entity: EntityId,
        groups: &mut Option<usize>,
    ) -> Result<(), Error> {
        // A virtual definition owns its adjusting thunks even when another
        // unit owns the table. Prepare its checked result and the unit's
        // completed group inventory without requesting unrelated bodies.
        if self.graph.lower_function_defined(entity)
            && !self.graph.lower_virtual_pure(entity)
            && !self.graph.lower_function(entity)?.internal
        {
            self.prepare_covariant_result(entity)?;
            if self.graph.lower_virtual_slot(entity).is_some()
                && let Some(class) = self.graph.lower_function(entity)?.object
            {
                self.graph.demand_virtual_chain(class, true)?;
                self.prepare_definition_group_layouts(groups.get_or_insert(0))?;
            }
        }
        Ok(())
    }
    fn advance_lowering_boundaries(&mut self, state: &mut Boundaries) -> Result<(), Error> {
        use super::function_demands::Demand;
        let Boundaries {
            root_count,
            explicit_roots,
            definition_groups,
            seen_tables,
            seen_constructors,
            seen,
            signatures,
            destruction_edges,
            constructor_edges,
            layouts,
            class_informations,
        } = state;
        self.prepare_constructor_defaults(None)?;
        let mut work = std::mem::take(&mut self.graph.boundary_roots);
        for (owner, class) in std::mem::take(&mut self.graph.class_information_uses) {
            if owner.is_none_or(|owner| seen.contains(&owner)) {
                self.class_layout(class, SourceOffset(0))?;
                self.virtual_table_boundaries(class, seen_tables, &mut work)?;
            } else if let Some(owner) = owner {
                class_informations.entry(owner).or_default().push(class);
            }
        }
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
                // Signature facts are needed by the caller before the selected
                // member body is instantiated or emitted.
                self.prepare_function_boundary(entity, &mut work)?;
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
            self.prepare_throw_specification_layouts(entity)?;
            self.prepare_virtual_definition_layouts(entity, definition_groups)?;
            if let Some(classes) = class_informations.remove(&entity) {
                for class in classes {
                    self.class_layout(class, SourceOffset(0))?;
                    self.virtual_table_boundaries(class, seen_tables, &mut work)?;
                }
            }
            if let Some(invoker) = self.graph.lower_closure_converter(entity) {
                work.push(invoker);
            }
            // A captureless invoker runs the call operator's analyzed body.
            // Its locals and selected calls retain that body's demand owner.
            let body_owner = self.graph.lower_closure_body_owner(entity);
            if body_owner != entity {
                work.push(body_owner);
            }
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
                // Entry anchors read this definition's primary chain even when
                // another unit owns its table; unrelated declarations do not.
                if self.graph.lower_virtual_slot(entity).is_some()
                    && self.graph.lower_function_defined(entity)
                    && !self.graph.lower_virtual_pure(entity)
                    && !self.graph.lower_function(entity)?.internal
                {
                    self.graph.demand_virtual_chain(class, true)?;
                }
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
            self.prepare_function_boundary(entity, &mut work)?;
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
                    StorageDemand::TypeInformation(ty) => {
                        self.prepare_rtti_layouts(ty, location)?;
                    }
                    StorageDemand::Object(ty) => {
                        self.type_size(ty, location)?;
                    }
                    StorageDemand::VariadicClass(ty) => {
                        self.type_size(ty, location)?;
                        let class = self
                            .graph
                            .class_of(ty)
                            .ok_or(Error::Invariant("variadic class boundary without class"))?;
                        self.prepare_class_parameter(class)?;
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
    fn prepare_function_boundary(
        &mut self,
        entity: EntityId,
        work: &mut Vec<EntityId>,
    ) -> Result<(), Error> {
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
        match self.graph.entity(entity).layout {
            LayoutState::Value { .. } => Ok(false),
            LayoutState::InProgress => Err(Error::at(location, "class contains itself")),
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
    pub(super) fn member_size(
        &mut self,
        ty: super::types::TypeId,
        location: SourceOffset,
    ) -> Result<(u64, u64), Error> {
        if self.hosted()
            && self.template_depth == 0
            && let TypeKind::Array {
                element,
                bound: 0,
                operand: None,
            } = self.graph.types.get(ty).kind
        {
            return Ok((0, self.type_size(element, location)?.1));
        }
        self.type_size(ty, location)
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
        let mut bases = self.place_class_bases(entity, location)?;
        let mut size = bases.size;
        let mut data_size = bases.data_size;
        let mut alignment = bases.alignment;
        let mut contains_volatile_or_union = union || bases.contains_volatile_or_union;
        let mut contains_data_member_pointer = bases.contains_data_member_pointer;
        let mut contains_empty = bases.contains_empty;
        let attributes = self.graph.entity(entity).storage_attributes;
        let pack = u64::from(self.graph.entity(entity).class_packing);
        let mut non_pod = bases.dynamic
            || self.graph.entity(entity).layout_category == LayoutCategory::NonPod
            || !self.graph.entity(entity).bases.is_empty();
        let mut packs_all = true;
        let mut data_bits = data_size.wrapping_mul(8);
        for index in 0..self.graph.entity(entity).members.len() {
            let member = self.graph.entity(entity).members[index];
            let ty = self
                .graph
                .entity(member)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            let (bytes, natural) = self.member_size(ty, location)?;
            let mut scalar = ty;
            while let TypeKind::Array { element, .. } = self.graph.types.get(scalar).kind {
                scalar = element;
            }
            contains_volatile_or_union |= self.graph.types.get(scalar).cv.0 & 2 != 0;
            if (!union || self.graph.entity(entity).members.first() == Some(&member))
                && self.graph.entity(member).bit_field.is_none()
            {
                contains_data_member_pointer |= self.graph.lower_holds_data_member_pointer(ty)?;
            }
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
                let declared = if self.graph.entity(member).bit_field.is_none() {
                    self.graph.alias_alignments.object(member, natural)
                } else {
                    natural
                };
                declared.max(requested)
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
            if !union
                && member_attributes.overlapping
                && let TypeKind::Class(child) = self.graph.types.get(ty).kind
            {
                let empty = self.graph.lower_class_empty(child)?;
                if empty && requested == 0 {
                    member_alignment = natural;
                }
                let (offset, tail) = bases.placement.place_overlapping_member(
                    &self.graph,
                    child,
                    data_bits,
                    member_alignment,
                    natural,
                    location,
                )?;
                self.graph.entity_mut(member).member_offset = Some(offset);
                if !empty {
                    data_bits = tail.wrapping_mul(8);
                    bits.end_run(offset);
                }
                size = size.max(offset.wrapping_add(bytes));
                alignment = alignment.max(member_alignment);
                continue;
            }
            data_size = round_up(data_bits, 8) / 8;
            let mut offset = if union {
                0
            } else {
                round_up(data_size, member_alignment)
            };
            if !union {
                while bases
                    .placement
                    .member_conflict(&self.graph, ty, offset, location)?
                {
                    offset = round_up(offset.wrapping_add(natural), member_alignment);
                }
                data_size = offset.wrapping_add(bytes);
                bases
                    .placement
                    .record_member(&self.graph, ty, offset, data_size, location)?;
            }
            self.graph.entity_mut(member).member_offset = Some(offset);
            bits.end_run(offset);
            data_bits = data_size.wrapping_mul(8);
            size = if union {
                size.max(bytes)
            } else {
                size.max(data_size)
            };
            alignment = alignment.max(member_alignment);
        }
        check_class_size(size, location)?;
        let non_virtual_size = size;
        let non_virtual_alignment = alignment;
        let member_data = round_up(data_bits, 8) / 8;
        bases.size = size;
        bases.data_size = member_data;
        bases.alignment = alignment;
        self.finish_virtual_layout(entity, &mut bases, location)?;
        size = bases.size;
        check_class_size(size, location)?;
        alignment = bases.alignment;
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
            if non_pod { member_data } else { full_size },
            location,
        )?;
        self.graph.entity_mut(entity).layout = LayoutState::Value {
            contains_data_member_pointer,
            contains_volatile_or_union,
            empty: bases.empty,
            contains_empty,
            non_virtual_alignment: if bases.layout.bases.is_empty() {
                alignment
            } else {
                non_virtual_alignment
            },
            non_virtual_size: if non_pod { non_virtual_size } else { full_size },
            size: full_size,
            alignment,
        };
        self.publish_virtual_layout(entity, bases);
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
    fn query_receiver_layouts_follow_evaluated_reachable_uses() {
        let source = SourceBuffer::new(
            br"namespace std { struct type_info {}; }
            struct Result { virtual int f(); };
            struct A { int pad; Result& ref; Result& get(); };
            struct L : A {}; struct R : A {};
            struct Reached : L, R {};
            struct Unevaluated : L, R {};
            struct Dormant : L, R {};
            struct Plain { int pad; Result& ref; };
            void query(Reached* p, Unevaluated* q, Plain* r) {
                typeid((typeid(p->R::get()), p->R::ref));
                sizeof(q->R::get());
                typeid((q->R::get(), 0));
                typeid(r->ref);
            }
            inline void dormant(Dormant* p) { typeid(p->R::get()); }"
                .to_vec(),
        )
        .unwrap();
        let input = Preprocessor::with_name(&source, b"query-layouts").unwrap();
        let mut unit = SyntaxUnit::new(input);
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), count);
        for (spelling, evaluated) in [
            (b"Reached".as_slice(), true),
            (b"Plain", true),
            (b"Unevaluated", false),
            (b"Dormant", false),
        ] {
            let name = cursor.intern_name(spelling).unwrap();
            let binding = graph.own(graph.global, name, Filter::Prefix).unwrap();
            assert_eq!(
                matches!(
                    graph.entity(graph.binding(binding).entity).layout,
                    LayoutState::NotStarted
                ),
                !evaluated,
            );
        }
    }

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

// Translated from sema/analyzer_virtual.cpp's declaration and override analysis. See NOTICE.
//! Indexed virtual identities and declaration-owned override facts. No body or layout demand.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    function_names::ValueKey,
    graph::{BindingId, EntityId, MemberKind, ScopeId, ScopeKind},
    specifiers::{Friendship, Specifiers, Virtuality},
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Default, Debug)]
pub(super) struct VirtualFunction {
    pub is_virtual: bool,
    pub is_final: bool,
    pub pure: bool,
    pub overridden: Option<EntityId>,
}

#[derive(Default)]
pub(super) struct Virtuals {
    functions: HashMap<EntityId, VirtualFunction>,
    declarations: HashMap<(EntityId, ValueKey, TypeId), EntityId>,
    signatures: HashSet<(ValueKey, TypeId)>,
    destructors: HashMap<EntityId, EntityId>,
    pure_counts: HashMap<EntityId, u32>,
    pub(super) classes: HashMap<EntityId, ClassVirtuals>,
    slots: HashMap<EntityId, SlotId>,
    pub(super) vcall_orders: HashMap<EntityId, u64>,
    projection_chains: HashSet<EntityId>,
    primary_overrides: HashMap<EntityId, EntityId>,
    pub(super) slot_levels: HashMap<EntityId, Vec<Option<EntityId>>>,
    function_signatures: HashMap<EntityId, (ValueKey, TypeId)>,
    covariant_results: HashMap<EntityId, EntityId>,
    pub(super) groups: HashMap<EntityId, super::vtable_groups::VtableGroup>,
    pub(super) grouped_classes: Vec<EntityId>,
    pub(super) base_memberships: HashSet<(EntityId, EntityId)>,
    pub(super) chain_overriders: HashMap<(EntityId, EntityId, EntityId), Option<EntityId>>,
    deallocators: HashMap<EntityId, EntityId>,
    assignment_targets: HashSet<EntityId>,
    implicit_assignments: HashMap<EntityId, Vec<EntityId>>,
    layouts: HashMap<EntityId, Vec<Option<VirtualEntry>>>,
    pub(super) storage_layouts: HashMap<EntityId, super::virtual_layout::VirtualLayout>,
    pub(super) final_overriders: super::final_overriders::Groups,
}
impl Virtuals {
    pub(super) fn abstract_class(&self, class: EntityId) -> bool {
        self.final_overriders
            .abstract_class(class)
            .unwrap_or_else(|| self.pure_counts.get(&class).copied().unwrap_or(0) != 0)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Polymorphism {
    #[default]
    Static,
    Dynamic,
}
impl Polymorphism {
    pub(super) fn is_dynamic(self) -> bool {
        self == Self::Dynamic
    }
}
/// The ABI slot identity survives overrides without copying inherited tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SlotId(u32);
impl SlotId {
    pub(crate) fn deleting(self) -> Result<Self, Error> {
        Ok(Self(
            self.0
                .checked_add(1)
                .ok_or(Error::Limit("virtual slot limit"))?,
        ))
    }
    pub(crate) fn byte_offset(self) -> u64 {
        u64::from(self.0) * 8
    }
    pub(crate) fn index(self) -> Result<usize, Error> {
        usize::try_from(self.0).map_err(|_| Error::Limit("virtual slot limit"))
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SlotKind {
    Function,
    CompleteDestructor,
    DeletingDestructor,
}
#[derive(Clone, Copy)]
pub(crate) struct VirtualEntry {
    pub slot: SlotId,
    pub function: EntityId,
    pub kind: SlotKind,
    pub introducer: EntityId,
    pub adjusted: EntityId,
}
/// Class completion records only its changes. Lowering builds a demanded table
/// from this primary-chain recipe, never all the tables of an unused hierarchy.
pub(crate) struct ClassVirtuals {
    pub primary: Option<EntityId>,
    pub has_virtual_bases: bool,
    pub has_virtual_primary: bool,
    pub bases: Vec<EntityId>,
    pub size: u32,
    pub key: Option<EntityId>,
    pub entries: Vec<VirtualEntry>,
}
impl Virtuals {
    pub(super) fn function(&self, entity: EntityId) -> VirtualFunction {
        self.functions.get(&entity).copied().unwrap_or_default()
    }
    pub(super) fn destructor(&self, class: EntityId) -> Option<EntityId> {
        self.destructors.get(&class).copied()
    }
    pub(super) fn signature(&self, function: EntityId) -> Result<(ValueKey, TypeId), Error> {
        self.function_signatures
            .get(&function)
            .copied()
            .ok_or(Error::Invariant("virtual without signature identity"))
    }
}

impl Analyzer<'_> {
    pub(super) fn has_virt_specifier(&self, info: &DeclaratorInfo) -> bool {
        info.qualifiers.is_some_and(|list| {
            self.tree
                .qualifiers
                .get(list)
                .iter()
                .any(|qualifier| matches!(qualifier.kind, ast::QualifierKind::Virt(_)))
        })
    }

    pub(super) fn check_virtual_alias(
        &self,
        info: &DeclaratorInfo,
        specs: Specifiers,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if specs.virtuality == Virtuality::Virtual || self.has_virt_specifier(info) {
            return Err(Error::at(
                location,
                "a virt-specifier or virtual in an alias-declaration",
            ));
        }
        Ok(())
    }

    pub(super) fn virtual_declaration(
        &mut self,
        binding: BindingId,
        info: &DeclaratorInfo,
        specs: Specifiers,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() || self.template_depth != 0 {
            return Ok(());
        }
        let mut is_override = false;
        let mut is_final = false;
        if let Some(list) = info.qualifiers {
            for qualifier in self.tree.qualifiers.get(list) {
                if let ast::QualifierKind::Virt(name) = qualifier.kind {
                    let seen = if name == self.override_name {
                        &mut is_override
                    } else {
                        &mut is_final
                    };
                    if *seen {
                        return Err(Error::at(qualifier.location, "a repeated virt-specifier"));
                    }
                    *seen = true;
                }
            }
        }
        let function = self.graph.binding(binding).entity;
        let home = self
            .graph
            .entity(function)
            .home
            .ok_or(Error::Invariant("function without home"))?;
        let first = self.graph.function_origin(function)? == binding
            && !self
                .graph
                .function_templates
                .specializations
                .contains(function);
        if !first
            || self.graph.scope(home).kind != ScopeKind::Class
            || specs.friendship == Friendship::Friend
        {
            if specs.virtuality == Virtuality::Virtual {
                return Err(Error::at(
                    location,
                    "virtual outside the initial declaration of a class member function",
                ));
            }
            if is_override || is_final {
                return Err(Error::at(
                    location,
                    "a virt-specifier outside the declaration of a class member function",
                ));
            }
            return self.check_abstract_function(info.ty, location);
        }
        let class = self
            .graph
            .scope(home)
            .owner
            .ok_or(Error::Invariant("member without class"))?;
        let special = self
            .graph
            .entity(function)
            .member_function
            .map(|key| key.kind);
        let is_static = self
            .graph
            .entity(function)
            .function
            .is_some_and(|props| props.is_static);
        let mut facts = VirtualFunction {
            is_virtual: specs.virtuality == Virtuality::Virtual,
            is_final,
            ..VirtualFunction::default()
        };
        if facts.is_virtual {
            let message = if is_static {
                Some("a static member function declared virtual")
            } else if special == Some(MemberKind::Constructor) {
                Some("a constructor declared virtual")
            } else if self.graph.entity(class).key == Keyword::Union {
                Some("a union with a virtual function")
            } else {
                None
            };
            if let Some(message) = message {
                return Err(Error::at(location, message));
            }
        }
        let key = if special == Some(MemberKind::Conversion) {
            ValueKey::Special(super::function_names::SpecialName::Conversion)
        } else {
            self.declaration_key(info)?
        };
        let signature = if special == Some(MemberKind::Conversion) {
            self.graph.types.canonical_function(info.ty)?
        } else {
            self.graph.types.declaration_signature(info.ty)?
        };
        if special != Some(MemberKind::Constructor)
            && self.graph.entity(function).template_head.is_none()
        {
            let destructor = special == Some(MemberKind::Destructor);
            if destructor || self.graph.virtuals.signatures.contains(&(key, signature)) {
                let mut work: Vec<_> = self
                    .graph
                    .entity(class)
                    .bases
                    .iter()
                    .rev()
                    .map(|edge| edge.class)
                    .collect();
                let mut seen = HashSet::new();
                while let Some(base) = work.pop() {
                    if !seen.insert(base) {
                        continue;
                    }
                    let found = if destructor {
                        self.graph.virtuals.destructor(base)
                    } else {
                        self.graph
                            .virtuals
                            .declarations
                            .get(&(base, key, signature))
                            .copied()
                    };
                    if let Some(found) = found {
                        self.check_virtual_override(function, found, class, location)?;
                        self.check_override_nonthrowing(
                            function,
                            found,
                            destructor.then_some(base),
                            location,
                        )?;
                        facts.is_virtual = true;
                        facts.overridden = facts.overridden.or(Some(found));
                        self.note_exception_override(
                            function,
                            found,
                            destructor.then_some(base),
                            location,
                        );
                    } else {
                        work.extend(
                            self.graph
                                .entity(base)
                                .bases
                                .iter()
                                .rev()
                                .map(|edge| edge.class),
                        );
                    }
                }
            }
        }
        if is_override && facts.overridden.is_none() {
            return Err(Error::at(
                location,
                "a function marked override overrides no base class member function",
            ));
        }
        if is_final && !facts.is_virtual {
            return Err(Error::at(
                location,
                "a function marked final is not virtual",
            ));
        }
        if facts.is_virtual && specs.constexpr {
            return Err(Error::at(location, "a constexpr virtual function"));
        }
        if facts.is_virtual {
            self.graph.entity_mut(class).polymorphism = Polymorphism::Dynamic;
            self.graph.virtuals.functions.insert(function, facts);
            if special == Some(MemberKind::Destructor) {
                self.graph.virtuals.destructors.insert(class, function);
            } else {
                self.note_virtual_assignment_target(key, info.ty)?;
                self.graph.virtuals.signatures.insert((key, signature));
                self.graph
                    .virtuals
                    .declarations
                    .insert((class, key, signature), function);
                self.graph
                    .virtuals
                    .function_signatures
                    .insert(function, (key, signature));
            }
            self.graph.entity_mut(class).aggregation =
                super::aggregate_initialization::Aggregation::Ineligible;
            self.graph.entity_mut(class).layout_category = super::layout::LayoutCategory::NonPod;
        }
        self.check_abstract_function(info.ty, location)
    }

    pub(super) fn check_abstract_object(
        &self,
        mut ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() || self.template_depth != 0 {
            return Ok(());
        }
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        if let TypeKind::Class(class) = self.graph.types.get(ty).kind
            && self.graph.entity(class).complete
            && self.graph.virtuals.abstract_class(class)
        {
            return Err(Error::at(location, "an object of an abstract class type"));
        }
        Ok(())
    }

    fn check_abstract_function(&self, ty: TypeId, location: SourceOffset) -> Result<(), Error> {
        let TypeKind::Function {
            result, parameters, ..
        } = &self.graph.types.get(ty).kind
        else {
            return Err(Error::Invariant(
                "abstract function check without function type",
            ));
        };
        self.check_abstract_object(*result, location)?;
        for &parameter in parameters {
            self.check_abstract_object(parameter, location)?;
        }
        Ok(())
    }

    fn note_virtual_assignment_target(&mut self, key: ValueKey, ty: TypeId) -> Result<(), Error> {
        if key != super::assignment::key()? {
            return Ok(());
        }
        let TypeKind::Function {
            parameters,
            variadic: false,
            cv: super::types::Cv(0),
            reference: None,
            ..
        } = &self.graph.types.get(ty).kind
        else {
            return Ok(());
        };
        if let [parameter] = parameters.as_slice()
            && let TypeKind::Reference { element, .. } = self.graph.types.get(*parameter).kind
            && let TypeKind::Class(class) = self.graph.types.get(element).kind
        {
            self.graph.virtuals.assignment_targets.insert(class);
        }
        Ok(())
    }

    pub(super) fn complete_virtual_assignments(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.graph.virtuals.assignment_targets.contains(&class) {
            return Ok(());
        }
        // The inherited virtual parameter names this class. Its implicit
        // candidates and selected subobject actions are now actually needed.
        self.prepare_copy_properties(class)?;
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("assignment scope"))?;
        let key = super::assignment::key()?;
        let functions: Vec<_> = self
            .graph
            .functions_at(scope, key)
            .map(|binding| self.graph.binding(binding).entity)
            .filter(|&function| {
                self.graph.entity(function).assignment_origin == super::assignment::Origin::Implicit
            })
            .collect();
        for function in functions {
            let ty = self.graph.lower_entity_type(function)?;
            let signature = self.graph.types.declaration_signature(ty)?;
            let mut work: Vec<_> = self
                .graph
                .entity(class)
                .bases
                .iter()
                .rev()
                .map(|base| base.class)
                .collect();
            let mut seen = HashSet::new();
            let mut overridden = None;
            while let Some(base) = work.pop() {
                if !seen.insert(base) {
                    continue;
                }
                if let Some(&found) = self
                    .graph
                    .virtuals
                    .declarations
                    .get(&(base, key, signature))
                {
                    self.check_virtual_override(function, found, class, location)?;
                    self.check_override_nonthrowing(function, found, None, location)?;
                    self.note_exception_override(function, found, None, location);
                    overridden = overridden.or(Some(found));
                } else {
                    work.extend(
                        self.graph
                            .entity(base)
                            .bases
                            .iter()
                            .rev()
                            .map(|edge| edge.class),
                    );
                }
            }
            if let Some(found) = overridden {
                self.graph.virtuals.functions.insert(
                    function,
                    VirtualFunction {
                        is_virtual: true,
                        overridden: Some(found),
                        ..VirtualFunction::default()
                    },
                );
                self.graph
                    .virtuals
                    .declarations
                    .insert((class, key, signature), function);
                self.graph
                    .virtuals
                    .function_signatures
                    .insert(function, (key, signature));
                self.graph
                    .virtuals
                    .implicit_assignments
                    .entry(class)
                    .or_default()
                    .push(function);
            }
        }
        Ok(())
    }

    pub(super) fn complete_virtual_members(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.needs_final_overriders(class) {
            return self.graph.resolve_final_overriders(class, location);
        }
        // Primary-chain abstractness remains a compact delta. A preceding
        // non-dynamic base contributes no virtual functions to that chain.
        let primary = self
            .graph
            .entity(class)
            .bases
            .iter()
            .find(|edge| {
                !edge.is_virtual && self.graph.entity(edge.class).polymorphism.is_dynamic()
            })
            .map(|edge| edge.class);
        let mut pure = primary
            .and_then(|base| self.graph.virtuals.pure_counts.get(&base))
            .copied()
            .unwrap_or(0);
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("virtual class without scope"))?;
        for index in 0..self.graph.virtual_member_count(class)? {
            let Some(function) = self.graph.virtual_member_at(class, index)? else {
                continue;
            };
            if self.graph.entity(function).home != Some(scope) {
                continue;
            }
            let facts = self.graph.virtuals.function(function);
            if !facts.is_virtual
                || self
                    .graph
                    .entity(function)
                    .member_function
                    .is_some_and(|key| key.kind == MemberKind::Destructor)
            {
                continue;
            }
            if facts
                .overridden
                .is_some_and(|base| self.graph.virtuals.function(base).pure)
            {
                pure = pure
                    .checked_sub(1)
                    .ok_or(Error::Invariant("virtual pure count underflow"))?;
            }
            pure = pure
                .checked_add(u32::from(facts.pure))
                .ok_or(Error::Limit("virtual pure count limit"))?;
        }
        let destructor_pure = |class| {
            self.graph
                .entity(class)
                .declared_destructor
                .is_some_and(|binding| {
                    self.graph
                        .virtuals
                        .function(self.graph.binding(binding).entity)
                        .pure
                })
        };
        if primary.is_some_and(destructor_pure) {
            pure = pure
                .checked_sub(1)
                .ok_or(Error::Invariant("destructor pure count underflow"))?;
        }
        pure = pure
            .checked_add(u32::from(destructor_pure(class)))
            .ok_or(Error::Limit("virtual pure count limit"))?;
        if pure != 0 {
            self.graph.virtuals.pure_counts.insert(class, pure);
        }
        Ok(())
    }

    pub(super) fn complete_virtual_layout(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let has_virtual_bases = self.graph.entity(class).bases.iter().any(|edge| {
            edge.is_virtual
                || self
                    .graph
                    .virtuals
                    .classes
                    .get(&edge.class)
                    .is_some_and(|facts| facts.has_virtual_bases)
        });
        if !self.graph.entity(class).polymorphism.is_dynamic() && !has_virtual_bases {
            return Ok(());
        }
        let primary = self.virtual_table_primary(class, location)?;
        let mut size = if let Some(base) = primary {
            self.graph.lower_virtual_class(base)?.size
        } else {
            0
        };
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("virtual class scope"))?;
        let mut entries = Vec::new();
        let mut key = None;
        let mut seen = HashSet::new();
        for index in 0..self.graph.virtual_member_count(class)? {
            let Some(function) = self.graph.virtual_member_at(class, index)? else {
                continue;
            };
            if self.graph.entity(function).home != Some(scope) || !seen.insert(function) {
                continue;
            }
            let facts = self.graph.virtuals.function(function);
            if !facts.is_virtual {
                continue;
            }
            let destructor = self
                .graph
                .entity(function)
                .member_function
                .is_some_and(|member| member.kind == MemberKind::Destructor);
            let mut next = primary;
            let mut overridden = None;
            while let Some(base) = next {
                if let Some(found) = self.graph.declared_virtual_overrider(base, function)? {
                    overridden = Some(found);
                    break;
                }
                next = self.graph.lower_virtual_class(base)?.primary;
            }
            let inherited = overridden
                .map(|base| {
                    self.graph
                        .virtuals
                        .slots
                        .get(&base)
                        .copied()
                        .ok_or(Error::Invariant("override without completed virtual slot"))
                })
                .transpose()?;
            let adjusted = if let Some(base) = overridden {
                self.graph.virtuals.primary_overrides.insert(function, base);
                self.covariant_slot_needs_adjustment(function, base, location)?
            } else {
                false
            };
            let slot = if let Some(slot) = inherited.filter(|_| !adjusted) {
                slot
            } else {
                let slot = SlotId(size);
                size = size
                    .checked_add(if destructor { 2 } else { 1 })
                    .ok_or(Error::Limit("virtual slot limit"))?;
                slot
            };
            self.graph.virtuals.slots.insert(function, slot);
            let order = overridden
                .and_then(|base| self.graph.virtuals.vcall_orders.get(&base).copied())
                .map_or(slot.byte_offset(), |inherited| {
                    inherited.min(slot.byte_offset())
                });
            self.graph.virtuals.vcall_orders.insert(function, order);
            entries.push(VirtualEntry {
                slot,
                function,
                introducer: function,
                adjusted: function,
                kind: if destructor {
                    SlotKind::CompleteDestructor
                } else {
                    SlotKind::Function
                },
            });
            if destructor {
                entries.push(VirtualEntry {
                    slot: SlotId(
                        slot.0
                            .checked_add(1)
                            .ok_or(Error::Limit("virtual slot limit"))?,
                    ),
                    function,
                    introducer: function,
                    adjusted: function,
                    kind: SlotKind::DeletingDestructor,
                });
            }
            let entity = self.graph.entity(function);
            if key.is_none()
                && !facts.pure
                && entity.inline != super::specifiers::InlinePreference::Inline
                && entity.special_definition.is_none()
                && entity.availability != super::graph::FunctionAvailability::Deleted
            {
                key = Some(function);
            }
        }
        // analyzer_virtual.cpp::complete_virtual: instantiated members have
        // vague linkage. Each unit needing the table defines its slot targets;
        // an explicit specialization owns its members and keeps an ordinary key.
        if self.graph.instance_class(scope) {
            key = None;
        }
        let bases = if has_virtual_bases {
            self.graph.virtual_base_order(class)
        } else {
            Vec::new()
        };
        let own_virtual_primary = primary.is_some_and(|primary| {
            !self
                .graph
                .entity(class)
                .bases
                .iter()
                .any(|edge| !edge.is_virtual && edge.class == primary)
        });
        let has_virtual_primary = own_virtual_primary
            || self.graph.entity(class).bases.iter().any(|edge| {
                self.graph
                    .virtuals
                    .classes
                    .get(&edge.class)
                    .is_some_and(|facts| facts.has_virtual_primary)
            });
        for &base in &bases {
            self.graph.virtuals.base_memberships.insert((class, base));
        }
        self.graph.virtuals.classes.insert(
            class,
            ClassVirtuals {
                primary,
                has_virtual_bases,
                has_virtual_primary,
                bases,
                size,
                key,
                entries,
            },
        );
        if has_virtual_bases {
            // Sharing is a relation between actual subobject placements. Layout
            // is demanded only when a virtual primary can share a pointer.
            if self.graph.has_virtual_primary_below(class) {
                self.class_layout(class, location)?;
            }
            self.graph.complete_virtual_group(class)?;
        } else {
            self.graph.complete_nonvirtual_group(class)?;
        }
        Ok(())
    }

    fn covariant_slot_needs_adjustment(
        &mut self,
        function: EntityId,
        overridden: EntityId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let Some((derived, base, _)) = self.graph.covariant_classes(function, overridden)? else {
            return Ok(false);
        };
        self.class_layout(derived, location)?;
        Ok(self
            .graph
            .nonvirtual_result_offset(derived, base)?
            .is_none_or(|offset| offset != 0))
    }

    fn check_virtual_override(
        &mut self,
        function: EntityId,
        found: EntityId,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self
            .graph
            .entity(function)
            .function
            .is_some_and(|props| props.is_static)
        {
            return Err(Error::at(
                location,
                "a static member function overrides a virtual function",
            ));
        }
        if self.graph.virtuals.function(found).is_final {
            return Err(Error::at(location, "overriding a final virtual function"));
        }
        if !self
            .graph
            .entity(function)
            .member_function
            .is_some_and(|key| key.kind == MemberKind::Destructor)
        {
            if self.graph.entity(function).availability != self.graph.entity(found).availability {
                return Err(Error::at(
                    location,
                    "a deleted function and a function not deleted override one another",
                ));
            }
            self.check_override_result(function, found, class, location)?;
        }
        Ok(())
    }

    fn check_override_result(
        &mut self,
        function: EntityId,
        found: EntityId,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let result = |entity| -> Result<TypeId, Error> {
            let ty = self.graph.lower_entity_type(entity)?;
            if let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind {
                Ok(result)
            } else {
                Err(Error::Invariant("virtual without function type"))
            }
        };
        let mine = result(function)?;
        let theirs = result(found)?;
        if mine == theirs {
            return Ok(());
        }
        let m = self.graph.types.get(mine);
        let t = self.graph.types.get(theirs);
        let elements = match (&m.kind, &t.kind) {
            (TypeKind::Pointer(a), TypeKind::Pointer(b)) => Some((*a, *b)),
            (
                TypeKind::Reference {
                    element: a,
                    kind: ar,
                },
                TypeKind::Reference {
                    element: b,
                    kind: br,
                },
            ) if ar == br => Some((*a, *b)),
            _ => None,
        }
        .filter(|_| m.cv == t.cv);
        let classes = elements
            .and_then(|(a, b)| Some((a, b, self.graph.class_of(a)?, self.graph.class_of(b)?)));
        let Some((a, b, derived, base)) = classes else {
            return Err(Error::at(
                location,
                "an overrider's return type is neither the overridden function's nor covariant with it",
            ));
        };
        if self.graph.types.get(a).cv.0 & !self.graph.types.get(b).cv.0 != 0 {
            return Err(Error::at(
                location,
                "a covariant return type more cv-qualified than the overridden function's",
            ));
        }
        if derived == base {
            return Ok(());
        }
        if !self.graph.entity(derived).complete && derived != class {
            return Err(Error::at(
                location,
                "a covariant return type of an incomplete class",
            ));
        }
        if !self.graph.is_base_of(base, derived) {
            return Err(Error::at(
                location,
                "a covariant return type whose class is not derived from the overridden function's",
            ));
        }
        if self.graph.base_count(derived, base) > 1 {
            return Err(Error::at(
                location,
                "a covariant return type whose class has the overridden function's as an ambiguous base",
            ));
        }
        let path = self.graph.retain_base_path(derived, base)?;
        self.check_base_path_access(
            path,
            self.graph
                .entity(class)
                .scope
                .ok_or(Error::Invariant("class without scope"))?,
            location,
        )?;
        // Keep the checked result identity for thunk demand. A secondary-only
        // override does not run the primary slot's adjustment/layout query.
        self.graph
            .virtuals
            .covariant_results
            .insert(function, derived);
        Ok(())
    }

    pub(super) fn pure_specifier(
        &mut self,
        function: EntityId,
        initializer: Option<ast::Initializer>,
        info: &DeclaratorInfo,
        specs: Specifiers,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() || self.template_depth != 0 {
            return Ok(());
        }
        let Some(ast::Initializer {
            kind: ast::InitializerKind::Assign(expr),
            ..
        }) = initializer
        else {
            return Ok(());
        };
        let zero = matches!(self.tree.expressions.get(expr).kind, ast::ExprKind::Literal(lit) if self.literal_bytes.get(lit.spelling) == b"0");
        if !zero
            || !self.graph.virtuals.function(function).is_virtual
            || self.graph.scope(self.declaration_scope(scope)).kind != ScopeKind::Class
            || info.home.is_some()
            || specs.friendship == Friendship::Friend
        {
            return Err(Error::at(
                location,
                "a pure-specifier on a declaration that is not of a virtual function",
            ));
        }
        self.graph
            .virtuals
            .functions
            .get_mut(&function)
            .ok_or(Error::Invariant("pure nonvirtual function"))?
            .pure = true;
        Ok(())
    }

    pub(super) fn virtual_table_boundaries(
        &mut self,
        class: EntityId,
        seen: &mut HashSet<EntityId>,
        work: &mut Vec<EntityId>,
    ) -> Result<(), Error> {
        if !self.graph.lower_dynamic_class(class) || seen.contains(&class) {
            return Ok(());
        }
        if !self.graph.lower_emits_vtable(class)? {
            return Ok(());
        }
        seen.insert(class);
        self.class_layout(class, SourceOffset(0))?;
        self.prepare_destruction(class)?;
        self.graph.demand_virtual_chain(class, true)?;
        for index in 0..self.graph.lower_virtual_slots(class)?.len() {
            let function = self.graph.lower_virtual_slots(class)?[index]
                .ok_or(Error::Invariant("unfilled virtual slot"))?
                .function;
            self.virtual_target_boundary(function, work)?;
        }
        // Secondary entries may name functions absent from the primary table.
        // Their bodies, argument boundaries and covariant layouts are demanded
        // by the same emitted table, independently of a source call to them.
        let count = self
            .graph
            .lower_vtable_group(class)
            .map_or(0, |group| group.segments.len());
        for segment in 1..count {
            let group = self
                .graph
                .lower_vtable_group(class)
                .ok_or(Error::Invariant("missing demanded vtable group"))?;
            if group.segments[segment].shared {
                continue;
            }
            let entries = group.segments[segment].entries.clone();
            for index in entries {
                let function = self
                    .graph
                    .lower_vtable_group(class)
                    .ok_or(Error::Invariant("missing demanded vtable group"))?
                    .entries[index]
                    .function;
                self.virtual_target_boundary(function, work)?;
            }
        }
        Ok(())
    }

    fn virtual_target_boundary(
        &mut self,
        function: EntityId,
        work: &mut Vec<EntityId>,
    ) -> Result<(), Error> {
        if !self.graph.lower_virtual_pure(function) {
            self.prepare_covariant_result(function)?;
            work.push(function);
        }
        Ok(())
    }

    pub(super) fn prepare_covariant_result(&mut self, function: EntityId) -> Result<(), Error> {
        if let Some(&class) = self.graph.virtuals.covariant_results.get(&function) {
            self.class_layout(class, SourceOffset(0))?;
        }
        Ok(())
    }

    pub(super) fn virtual_definition(
        &mut self,
        function: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if let Some(member) = self.graph.entity(function).member_function
            && member.kind == MemberKind::Destructor
            && self.graph.virtuals.function(function).is_virtual
            && !self.graph.virtuals.deallocators.contains_key(&member.class)
        {
            self.complete_virtual_deallocation(member.class, function, location)?;
        }
        Ok(())
    }

    pub(super) fn complete_virtual_deallocation(
        &mut self,
        class: EntityId,
        destructor: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let entity = self.graph.entity(destructor);
        let defaulted = matches!(
            entity.special_definition,
            Some(super::special_members::SpecialDefinition::Defaulted { first: true, .. })
        );
        if entity.availability == super::graph::FunctionAvailability::Deleted
            || (!entity.defined && !defaulted)
        {
            return Ok(());
        }
        self.lookup_location = location;
        let home = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("virtual deallocation scope"))?;
        let key = super::allocation::key(Keyword::Delete, false);
        let found = self.qualified_result(home, key, super::lookup::Filter::Ordinary)?;
        let (found, naming) = if found.binding.is_some() {
            (found, Some(class))
        } else {
            (
                self.qualified_result(self.graph.global, key, super::lookup::Filter::Ordinary)?,
                None,
            )
        };
        let selected = self.usual_deallocation(found.functions.as_slice())?;
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("destructor scope"))?;
        let failure = if found
            .binding
            .is_none_or(|id| self.graph.binding(id).kind != super::graph::BindingKind::Function)
        {
            Some(Error::at(
                location,
                "no deallocation function for a virtual destructor",
            ))
        } else if let Some(binding) = selected {
            if self
                .graph
                .entity(self.graph.binding(binding).entity)
                .availability
                == super::graph::FunctionAvailability::Deleted
            {
                Some(Error::at(location, "use of a deleted function"))
            } else if let Some(naming) = naming
                && let Some(declaration) =
                    self.selected_function_declaration(binding, found.functions.as_slice())
            {
                self.check_named_member_access(declaration, naming, naming, scope, location)
                    .err()
            } else {
                None
            }
        } else {
            Some(Error::at(
                location,
                "no usual deallocation function for a virtual destructor",
            ))
        };
        if let Some(error) = failure {
            if !defaulted {
                return Err(error);
            }
            self.graph.entity_mut(destructor).availability =
                super::graph::FunctionAvailability::Deleted;
            self.graph
                .entity_mut(class)
                .destruction
                .as_mut()
                .ok_or(Error::Invariant("destructor properties"))?
                .deleted = true;
        } else if let Some(binding) = selected {
            self.graph
                .virtuals
                .deallocators
                .insert(class, self.graph.binding(binding).entity);
        }
        Ok(())
    }

    pub(super) fn complete_virtual_destructor(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let implicit = self.graph.entity(class).declared_destructor.is_none();
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index].class;
            if let Some(found) = self.graph.virtuals.destructor(base) {
                if implicit && self.graph.virtuals.function(found).is_final {
                    return Err(Error::at(location, "overriding a final virtual function"));
                }
                if implicit {
                    self.graph
                        .entity_mut(class)
                        .destruction
                        .as_mut()
                        .ok_or(Error::Invariant("implicit virtual destructor properties"))?
                        .trivial = false;
                    let binding = self
                        .destructor_callable(class)?
                        .ok_or(Error::Invariant("implicit virtual destructor callable"))?;
                    let function = self.graph.binding(binding).entity;
                    self.graph.virtuals.destructors.insert(class, function);
                    self.graph.virtuals.functions.insert(
                        function,
                        VirtualFunction {
                            is_virtual: true,
                            overridden: Some(found),
                            ..VirtualFunction::default()
                        },
                    );
                    self.note_implicit_destructor_override(class, base, location);
                }
            }
        }
        if let Some(destructor) = self.graph.virtuals.destructor(class) {
            self.complete_virtual_deallocation(class, destructor, location)?;
            self.graph
                .entity_mut(class)
                .destruction
                .as_mut()
                .ok_or(Error::Invariant("virtual destructor without properties"))?
                .trivial = false;
        }
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index].class;
            if self.graph.virtuals.destructor(base).is_some() {
                self.check_completed_destructor_override(class, base, location)?;
            }
        }
        Ok(())
    }
}

impl super::graph::Graph {
    fn covariant_classes(
        &self,
        function: EntityId,
        overridden: EntityId,
    ) -> Result<Option<(EntityId, EntityId, bool)>, Error> {
        let result = |function| -> Result<TypeId, Error> {
            let ty = self.lower_entity_type(function)?;
            let TypeKind::Function { result, .. } = self.types.get(ty).kind else {
                return Err(Error::Invariant("virtual result type"));
            };
            Ok(result)
        };
        let mine = result(function)?;
        let theirs = result(overridden)?;
        if mine == theirs {
            return Ok(None);
        }
        let (from, to, reference) = match (&self.types.get(mine).kind, &self.types.get(theirs).kind)
        {
            (TypeKind::Pointer(from), TypeKind::Pointer(to)) => (*from, *to, false),
            (
                TypeKind::Reference { element: from, .. },
                TypeKind::Reference { element: to, .. },
            ) => (*from, *to, true),
            _ => return Ok(None),
        };
        Ok(self
            .class_of(from)
            .zip(self.class_of(to))
            .filter(|(from, to)| from != to)
            .map(|(from, to)| (from, to, reference)))
    }
    pub(crate) fn lower_covariant_result(
        &self,
        function: EntityId,
        adjusted: EntityId,
        through: EntityId,
    ) -> Result<Option<super::base_projections::CovariantResult>, Error> {
        let Some((from, to, reference)) = self.covariant_classes(function, adjusted)? else {
            return Ok(None);
        };
        let through = (through != from
            && through != to
            && self.is_base_of(through, from)
            && self.is_base_of(to, through))
        .then_some(through);
        Ok(Some(super::base_projections::CovariantResult {
            from,
            to,
            through,
            reference,
        }))
    }
    pub(super) fn declared_virtual_overrider(
        &self,
        class: EntityId,
        function: EntityId,
    ) -> Result<Option<EntityId>, Error> {
        if self
            .entity(function)
            .member_function
            .is_some_and(|m| m.kind == MemberKind::Destructor)
        {
            return Ok(self.virtuals.destructor(class));
        }
        let (key, signature) = self.virtuals.signature(function)?;
        Ok(self
            .virtuals
            .declarations
            .get(&(class, key, signature))
            .copied())
    }

    pub(crate) fn lower_emits_vtable(&self, class: EntityId) -> Result<bool, Error> {
        Ok(self
            .lower_virtual_class(class)?
            .key
            .is_none_or(|key| self.entity(key).defined)
            && !self.lower_class_instantiated_elsewhere(class))
    }

    pub(super) fn demand_virtual_slots(&mut self, root: EntityId) -> Result<(), Error> {
        self.demand_virtual_chain(root, false)
    }
    pub(super) fn demand_virtual_chain(
        &mut self,
        root: EntityId,
        retain_chain: bool,
    ) -> Result<(), Error> {
        let mut chain = Vec::new();
        let mut next = Some(root);
        while let Some(class) = next {
            // An already cached head need not have retained an external key
            // owner's table below it. A secondary projection reads every link.
            if self.virtuals.layouts.contains_key(&class)
                && (!retain_chain || self.virtuals.projection_chains.contains(&class))
            {
                break;
            }
            chain.push(class);
            next = self.lower_virtual_class(class)?.primary;
        }
        let mut work = next
            .and_then(|base| self.virtuals.layouts.get(&base))
            .cloned()
            .unwrap_or_default();
        for class in chain.into_iter().rev() {
            let facts = self.lower_virtual_class(class)?;
            work.resize(
                usize::try_from(facts.size).map_err(|_| Error::Limit("virtual slot limit"))?,
                None,
            );
            for entry in &facts.entries {
                if matches!(entry.kind, SlotKind::DeletingDestructor) {
                    continue;
                }
                let function = entry.function;
                let own = entry.slot;
                let mut next = Some(function);
                while let Some(overridden) = next {
                    let slot = self
                        .lower_virtual_slot(overridden)
                        .ok_or(Error::Invariant("overridden virtual without slot"))?;
                    let old = work.get(slot.index()?).copied().flatten();
                    let introducer = if slot == own
                        && self
                            .virtuals
                            .primary_overrides
                            .get(&function)
                            .and_then(|base| self.lower_virtual_slot(*base))
                            != Some(own)
                    {
                        function
                    } else {
                        old.map_or(overridden, |old| old.introducer)
                    };
                    let adjusted = if slot == own {
                        function
                    } else {
                        old.map_or(introducer, |old| old.adjusted)
                    };
                    let placed = VirtualEntry {
                        slot,
                        function,
                        kind: entry.kind,
                        introducer,
                        adjusted,
                    };
                    *work
                        .get_mut(slot.index()?)
                        .ok_or(Error::Invariant("virtual slot outside primary table"))? =
                        Some(placed);
                    if matches!(entry.kind, SlotKind::CompleteDestructor) {
                        let slot = slot.deleting()?;
                        *work
                            .get_mut(slot.index()?)
                            .ok_or(Error::Invariant("destructor slot outside primary table"))? =
                            Some(VirtualEntry {
                                slot,
                                kind: SlotKind::DeletingDestructor,
                                ..placed
                            });
                    }
                    next = self.virtuals.primary_overrides.get(&introducer).copied();
                }
            }
            // Like analyzer_virtual.cpp::virtual_slots, retain the demanded
            // chain's layouts only where this unit can emit their tables.
            if retain_chain
                || class == root
                || facts.key.is_none_or(|key| self.lower_function_defined(key))
            {
                self.virtuals.layouts.insert(class, work.clone());
            }
            if retain_chain {
                self.virtuals.projection_chains.insert(class);
            }
        }
        Ok(())
    }
    pub(crate) fn lower_virtual_slots(
        &self,
        class: EntityId,
    ) -> Result<&[Option<VirtualEntry>], Error> {
        self.virtuals
            .layouts
            .get(&class)
            .map(Vec::as_slice)
            .ok_or(Error::Invariant("undemanded virtual slot layout"))
    }
    pub(super) fn virtual_member_count(&self, class: EntityId) -> Result<usize, Error> {
        let scope = self
            .entity(class)
            .scope
            .ok_or(Error::Invariant("virtual class scope"))?;
        Ok(self.scope(scope).bindings.len()
            + 1
            + self
                .virtuals
                .implicit_assignments
                .get(&class)
                .map_or(0, Vec::len))
    }
    pub(super) fn virtual_member_at(
        &self,
        class: EntityId,
        index: usize,
    ) -> Result<Option<EntityId>, Error> {
        let scope = self
            .entity(class)
            .scope
            .ok_or(Error::Invariant("virtual class scope"))?;
        let declared = self.scope(scope).bindings.len();
        if index < declared {
            return Ok(Some(self.binding(self.scope(scope).bindings[index]).entity));
        }
        if index == declared {
            return Ok(self
                .entity(class)
                .implicit_destructor
                .map(|binding| self.binding(binding).entity));
        }
        Ok(self
            .virtuals
            .implicit_assignments
            .get(&class)
            .and_then(|functions| functions.get(index - declared - 1))
            .copied())
    }
    pub(crate) fn lower_virtual_deallocator(&self, class: EntityId) -> Result<EntityId, Error> {
        self.virtual_deallocator(class).ok_or(Error::Invariant(
            "virtual destructor without deallocation fact",
        ))
    }
    pub(super) fn virtual_deallocator(&self, class: EntityId) -> Option<EntityId> {
        self.virtuals.deallocators.get(&class).copied()
    }
    pub(crate) fn lower_polymorphic(&self, class: EntityId) -> bool {
        self.entity(class).polymorphism.is_dynamic()
    }
    pub(crate) fn lower_dynamic_class(&self, class: EntityId) -> bool {
        self.lower_polymorphic(class)
            || self
                .virtuals
                .classes
                .get(&class)
                .is_some_and(|facts| facts.has_virtual_bases)
    }
    pub(crate) fn lower_virtual_class(&self, class: EntityId) -> Result<&ClassVirtuals, Error> {
        self.virtuals
            .classes
            .get(&class)
            .ok_or(Error::Invariant("missing completed virtual class facts"))
    }
    pub(crate) fn lower_virtual_slot(&self, function: EntityId) -> Option<SlotId> {
        self.virtuals.slots.get(&function).copied()
    }
    pub(crate) fn lower_virtual_pure(&self, function: EntityId) -> bool {
        self.virtuals.function(function).pure
            || self.entity(function).availability == super::graph::FunctionAvailability::Deleted
    }
    pub(crate) fn lower_virtual_dispatch(&self, expression: ast::ExprId) -> Result<bool, Error> {
        Ok(self.expression_facts.get(expression)?.dispatch.is_virtual())
    }
    pub(crate) fn lower_virtual_class_identity(
        &self,
        class: EntityId,
    ) -> Result<(bool, bool, Keyword, ScopeId), Error> {
        let entity = self.entity(class);
        let home = entity.home.ok_or(Error::Invariant("class without home"))?;
        Ok((
            self.lower_entity_unit_local(class)?,
            self.scope(home).kind != ScopeKind::Namespace,
            entity.key,
            entity.scope.unwrap_or(home),
        ))
    }
    pub(crate) fn lower_rtti_bases(
        &self,
        class: EntityId,
    ) -> impl Iterator<Item = Result<(EntityId, bool, bool, u64), Error>> + '_ {
        self.entity(class).bases.iter().map(|base| {
            Ok((
                base.class,
                base.access == super::graph::Access::Public,
                base.is_virtual,
                base.offset
                    .ok_or(Error::Invariant("RTTI base without layout"))?,
            ))
        })
    }
    pub(crate) fn lower_inheritance_edge(
        &self,
        class: EntityId,
        index: usize,
    ) -> Option<(EntityId, bool, Option<u64>)> {
        self.entity(class)
            .bases
            .get(index)
            .map(|edge| (edge.class, edge.is_virtual, edge.offset))
    }
    pub(crate) fn lower_inheritance_edges(
        &self,
        class: EntityId,
    ) -> impl Iterator<Item = (EntityId, bool)> + '_ {
        self.entity(class)
            .bases
            .iter()
            .map(|base| (base.class, base.is_virtual))
    }
    pub(crate) fn lower_virtual_key_owner(&self, function: EntityId) -> Option<EntityId> {
        let home = self.entity(function).home?;
        let class = self.scope(home).owner?;
        (self.virtuals.classes.get(&class)?.key == Some(function)).then_some(class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn pure_declaration_survives_its_definition_and_concrete_override() {
        let source = SourceBuffer::new(
            b"struct B{virtual int f()=0;virtual ~B(){}};int B::f(){return 3;}struct D:B{int f(){int x=0;return x+7;}};int main(){B*b=new D;int v=b->f();delete b;return v==7?0:1;}".to_vec(),
        ).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"pure-declaration-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            cursor.intern_name(b"B").unwrap(),
            cursor.intern_name(b"D").unwrap(),
        ];
        let function_name = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        for (index, name) in names.into_iter().enumerate() {
            let class = graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Elaborated)
                        .unwrap(),
                )
                .entity;
            let scope = graph.entity(class).scope.unwrap();
            let member = graph
                .binding(
                    graph
                        .own(scope, function_name, super::super::lookup::Filter::Ordinary)
                        .unwrap(),
                )
                .entity;
            assert_eq!(graph.virtuals.function(member).pure, index == 0);
            assert_eq!(graph.virtuals.abstract_class(class), index == 0);
        }
    }

    #[test]
    fn deleting_entry_prepares_only_its_selected_deallocator_body() {
        let source = SourceBuffer::new(
            br#"
            template<class T>struct Box{
                virtual ~Box();
                static void operator delete(void*p){::operator delete(p);}
            };
            template<class T>Box<T>::~Box(){}
            using Unused=Box<char>;using Used=Box<int>;
            static_assert(sizeof(Unused)>0,"");
            int main(){Used b;return 0;}
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"deleting-entry-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            cursor.intern_name(b"Unused").unwrap(),
            cursor.intern_name(b"Used").unwrap(),
        ];
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        for (index, name) in names.into_iter().enumerate() {
            let alias = graph
                .own(graph.global, name, super::super::lookup::Filter::Prefix)
                .unwrap();
            let class = graph.class_of(graph.binding(alias).ty.unwrap()).unwrap();
            let scope = graph.entity(class).scope.unwrap();
            let binding = graph
                .functions_at(scope, super::super::allocation::key(Keyword::Delete, false))
                .next()
                .unwrap();
            let deallocator = graph.binding(binding).entity;
            assert_eq!(
                graph.function_templates.bodies.started(deallocator),
                index == 1
            );
            assert_eq!(
                graph.virtual_deallocator(class),
                (index == 1).then_some(deallocator)
            );
            assert_eq!(graph.virtuals.layouts.contains_key(&class), index == 1);
            if index == 1 {
                let parameters: Vec<_> = graph
                    .lower_parameters(graph.entity(deallocator).scope)
                    .collect();
                assert_eq!(parameters.len(), 1);
                assert!(parameters[0].1.is_some());
                assert!(matches!(
                    graph.types.get(parameters[0].2).kind,
                    TypeKind::Pointer(_)
                ));
            }
        }
    }

    #[test]
    fn instantiated_virtual_classes_have_no_key_without_demanding_slot_bodies() {
        let source = SourceBuffer::new(
            br#"
            template<class T>struct Box{virtual int f();void unused(){T::missing();}};
            template<class T>int Box<T>::f(){return sizeof(T);}
            template<>struct Box<double>{virtual int f();};
            struct Ordinary{virtual int f();};
            template<class T>struct Outer{struct Member{virtual int f();};};
            template<class T>int Outer<T>::Member::f(){return 3;}
            template<>struct Outer<double>{struct Member{virtual int f();};};
            using I=Box<int>;using E=Box<double>;
            using M=Outer<int>::Member;using EM=Outer<double>::Member;
            static_assert(sizeof(I)>0,"");static_assert(sizeof(E)>0,"");
            static_assert(sizeof(M)>0,"");static_assert(sizeof(EM)>0,"");
            static_assert(sizeof(Ordinary)>0,"");
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"virtual-instance-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"I".as_slice(), b"M", b"E", b"EM", b"Ordinary"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        for (index, name) in names.into_iter().enumerate() {
            let binding = graph
                .own(graph.global, name, super::super::lookup::Filter::Prefix)
                .unwrap();
            let class = graph.class_of(graph.binding(binding).ty.unwrap()).unwrap();
            let facts = graph.lower_virtual_class(class).unwrap();
            assert_eq!(facts.key.is_none(), index < 2);
            assert_eq!(
                graph.instance_class(graph.entity(class).scope.unwrap()),
                index < 2
            );
            assert_eq!(facts.size, 1);
            for entry in &facts.entries {
                assert!(!graph.function_templates.bodies.started(entry.function));
                assert!(!graph.function_templates.body_demanded(entry.function));
            }
        }
        assert!(graph.root_demands.as_slice().is_empty());
        assert!(graph.virtuals.layouts.is_empty());
    }

    #[test]
    fn implicit_virtual_assignment_owns_selected_actions_without_output_demand() {
        let source = SourceBuffer::new(b"struct B;struct A{virtual A&operator=(const B&){return *this;}virtual ~A(){}};struct B:A{int n;};struct Unused{int m;};".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"implicit-virtual-assignment").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"A".as_slice(), b"B", b"Unused"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let expressions = tree.expressions.mark();
        let syntax = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let classes: Vec<_> = names
            .into_iter()
            .map(|name| {
                graph
                    .binding(
                        graph
                            .own(graph.global, name, super::super::lookup::Filter::Elaborated)
                            .unwrap(),
                    )
                    .entity
            })
            .collect();
        let [base, derived, unused] = classes.as_slice() else {
            panic!("class inventory");
        };
        let functions = &graph.virtuals.implicit_assignments[derived];
        assert_eq!(functions.len(), 1);
        let assignment = functions[0];
        let overridden = graph.virtuals.function(assignment).overridden.unwrap();
        assert_eq!(
            graph.lower_function(overridden).unwrap().object,
            Some(*base)
        );
        assert_eq!(
            graph.lower_virtual_slot(assignment),
            graph.lower_virtual_slot(overridden)
        );
        let actions = &graph.lower_assignment(assignment).unwrap().actions;
        assert_eq!(actions.len(), 2);
        assert!(actions[0].function.is_some());
        assert!(matches!(
            graph.entity(*unused).copy_preparation,
            super::super::copy_properties::Preparation::Pending
        ));
        assert!(graph.root_demands.as_slice().is_empty());
        // Covariant B& -> A& checks the completed result layout, even at
        // offset zero (analyzer_virtual.cpp::result_needs_adjustment).
        for &class in [base, derived] {
            assert!(matches!(
                graph.entity(class).layout,
                super::super::layout::LayoutState::Value { .. }
            ));
        }
        assert!(matches!(
            graph.entity(*unused).layout,
            super::super::layout::LayoutState::NotStarted
        ));
        assert!(graph.virtuals.layouts.is_empty());
        assert_eq!(expressions, tree.expressions.mark());
        assert_eq!(syntax, tree.declarations.mark());
    }

    #[test]
    fn covariant_override_facts_share_entities_and_demand_only_the_result_layout() {
        let source = SourceBuffer::new(b"struct A{virtual A*f()const &;virtual void g()=0;virtual ~A()=0;};struct B:A{B*f()const & override;void g()override;};struct Unused{virtual void h();};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"virtual-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let names: Vec<_> = [b"A".as_slice(), b"B", b"Unused", b"f", b"g"]
            .iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let class = |name| {
            graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Elaborated)
                        .unwrap(),
                )
                .entity
        };
        let a = class(names[0]);
        let b = class(names[1]);
        let unused = class(names[2]);
        let member = |class, name| {
            let home = graph.entity(class).scope.unwrap();
            graph
                .binding(
                    graph
                        .own(home, name, super::super::lookup::Filter::Ordinary)
                        .unwrap(),
                )
                .entity
        };
        let af = member(a, names[3]);
        let bf = member(b, names[3]);
        let bg = member(b, names[4]);
        assert_eq!(graph.virtuals.function(bf).overridden, Some(af));
        assert!(graph.virtuals.function(bf).is_virtual);
        assert!(!graph.virtuals.function(bg).pure);
        assert_eq!(graph.virtuals.pure_counts.get(&a), Some(&2));
        assert!(!graph.virtuals.pure_counts.contains_key(&b));
        assert_eq!(graph.virtuals.functions.len(), 7);
        let base_destructor = graph.virtuals.destructor(a).unwrap();
        let derived_destructor = graph.virtuals.destructor(b).unwrap();
        assert_ne!(base_destructor, derived_destructor);
        assert_eq!(
            graph.virtuals.function(derived_destructor).overridden,
            Some(base_destructor)
        );
        assert_eq!(
            graph.lower_virtual_slot(base_destructor),
            graph.lower_virtual_slot(derived_destructor)
        );
        let table = graph.lower_virtual_class(b).unwrap();
        assert_eq!(table.size, 4);
        assert!(
            table
                .entries
                .iter()
                .filter(|entry| !matches!(entry.kind, SlotKind::Function))
                .all(|entry| entry.function == derived_destructor)
        );
        assert_eq!(
            table
                .entries
                .iter()
                .filter(|entry| !matches!(entry.kind, SlotKind::Function))
                .count(),
            2
        );
        assert!(graph.entity(b).implicit_destructor.is_some());
        assert!(graph.lower_virtual_deallocator(b).is_ok());
        assert!(graph.root_demands.as_slice().is_empty());
        for id in [a, b] {
            assert!(matches!(
                graph.entity(id).layout,
                super::super::layout::LayoutState::Value { .. }
            ));
        }
        assert!(matches!(
            graph.entity(unused).layout,
            super::super::layout::LayoutState::NotStarted
        ));
        assert!(graph.virtuals.layouts.is_empty());
        assert_eq!(expressions, tree.expressions.mark());
    }
}

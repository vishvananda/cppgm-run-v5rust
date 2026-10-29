// Translated from analyzer_construction.cpp::implicit_default_facts,
// inner_plan_facts and anonymous_default_facts. See NOTICE.
//! Permanent facts follow the structural failure boundary, in source order.
//! Anonymous entries have no independent function exception specification.
use super::{
    Error,
    analyzer::Analyzer,
    construction::{ConstructorId, DefaultFunction},
    graph::{EntityId, FunctionAvailability},
    special_members::SpecialDefinition,
    types::TypeKind,
};
use crate::lex::vocabulary::Keyword;

struct Facts {
    trivial: bool,
    nothrow: bool,
    initialized: bool,
}
impl Analyzer<'_> {
    pub(super) fn complete_implicit_default_facts(
        &mut self,
        id: ConstructorId,
        trivial: bool,
        deleted: bool,
    ) -> Result<(), Error> {
        let class = self.graph.constructors.get(id).class;
        let union = self.graph.entity(class).key == Keyword::Union;
        let mut facts = Facts {
            trivial,
            nothrow: true,
            initialized: false,
        };
        let mut variant_nontrivial = false;
        let mut anonymous_deleted = false;
        for index in 0..self.graph.constructors.get(id).bases.len() {
            if let Some(child) = self.graph.constructors.get(id).bases[index].constructor {
                self.default_selected_facts(child, &mut facts)?;
            }
        }
        for index in 0..self.graph.constructors.get(id).members.len() {
            let (member, _) = self.graph.constructors.get(id).members[index];
            let action = self.graph.constructors.get(id).subobjects[index];
            if let Some(child) = action.filter(|child| {
                self.graph
                    .constructors
                    .get(child.constructor)
                    .declaration
                    .is_none()
            }) {
                self.inner_default_facts(child.constructor, &mut facts)?;
                let (nontrivial, bad) = self.anonymous_default_facts(child.constructor)?;
                anonymous_deleted |= bad;
                variant_nontrivial |= union && nontrivial;
            }
            self.default_member_facts(id, index, &mut facts)?;
            if union
                && let Some(child) = self.default_member_class(member)
                && self.graph.anonymous_member_storage(child).is_none()
            {
                // Selection precedes the OR, even if an earlier variant is nontrivial.
                let selected = self.implicit_constructor_function(child)?;
                variant_nontrivial |= !self.default_function_trivial(selected)?;
            }
        }
        for index in 0..self.graph.constructors.get(id).indirect_bases.len() {
            if let Some(child) = self.graph.constructors.get(id).indirect_bases[index].constructor {
                self.default_selected_facts(child, &mut facts)?;
            }
        }
        self.graph
            .constructors
            .set_default_triviality(id, facts.trivial);
        let function = self
            .graph
            .constructors
            .get(id)
            .declaration
            .ok_or(Error::Invariant("default facts without function"))?;
        if self.graph.constructors.get(id).binding.is_some()
            && !matches!(
                self.graph.entity(function).special_definition,
                Some(SpecialDefinition::Defaulted { first: true, .. })
            )
        {
            facts.nothrow = self.graph.function_nothrow_value(function);
        }
        self.graph.constructors.set_nothrow(id, facts.nothrow);
        let deleted = (variant_nontrivial && !facts.initialized)
            || anonymous_deleted
            || deleted
            || !self.constructor_subobjects_destructible(class)?;
        self.graph
            .constructors
            .finish_default_facts(id, facts.trivial, facts.nothrow, deleted);
        if let Some(function) = self.graph.constructors.get(id).declaration
            && self.graph.constructors.get(id).deleted
        {
            self.graph.entity_mut(function).availability = FunctionAvailability::Deleted;
        }
        let transfer = self
            .graph
            .constructor_copies_storage(self.graph.constructors.get(id));
        self.graph.constructors.set_default_transfer(id, transfer);
        self.finish_inner_default_metadata(id);
        Ok(())
    }

    fn default_selected_facts(
        &mut self,
        id: ConstructorId,
        facts: &mut Facts,
    ) -> Result<(), Error> {
        facts.trivial = facts.trivial && !self.graph.constructors.get(id).nontrivial();
        facts.nothrow = facts.nothrow && self.subobject_constructor_nothrow(id)?;
        Ok(())
    }
    fn default_member_facts(
        &mut self,
        id: ConstructorId,
        index: usize,
        facts: &mut Facts,
    ) -> Result<(), Error> {
        let (member, initializer) = self.graph.constructors.get(id).members[index];
        if initializer.is_some() {
            facts.trivial = false;
            facts.initialized = true;
            facts.nothrow = facts.nothrow && !self.graph.member_initializers.throwing(member);
        }
        if let Some(child) = self.graph.constructors.get(id).subobjects[index]
            && self
                .graph
                .constructors
                .get(child.constructor)
                .declaration
                .is_some()
        {
            self.default_selected_facts(child.constructor, facts)?;
        }
        Ok(())
    }
    fn inner_default_facts(&mut self, root: ConstructorId, facts: &mut Facts) -> Result<(), Error> {
        let mut work = vec![(root, 0usize)];
        while let Some((id, index)) = work.pop() {
            if index == self.graph.constructors.get(id).members.len() {
                continue;
            }
            work.push((id, index + 1));
            if let Some(child) = self.graph.constructors.get(id).subobjects[index]
                && self
                    .graph
                    .constructors
                    .get(child.constructor)
                    .declaration
                    .is_none()
            {
                work.push((child.constructor, 0));
            } else {
                self.default_member_facts(id, index, facts)?;
            }
        }
        Ok(())
    }
    pub(super) fn inner_default_nothrow(&mut self, root: ConstructorId) -> Result<bool, Error> {
        let mut facts = Facts {
            trivial: true,
            nothrow: true,
            initialized: false,
        };
        self.inner_default_facts(root, &mut facts)?;
        Ok(facts.nothrow)
    }
    fn default_member_class(&self, member: EntityId) -> Option<EntityId> {
        let mut ty = self.graph.entity(member).ty?;
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        self.graph.class_of(ty)
    }
    fn default_function_trivial(&self, selected: Option<DefaultFunction>) -> Result<bool, Error> {
        let Some(slot) = selected else {
            return Ok(false);
        };
        let id = self
            .graph
            .entity(slot.function)
            .constructor
            .ok_or(Error::Invariant("default function facts without record"))?;
        Ok(!self.graph.constructors.get(id).nontrivial())
    }
    fn anonymous_default_facts(&mut self, root: ConstructorId) -> Result<(bool, bool), Error> {
        let mut work = vec![(root, 0usize, false)];
        let mut deleted = false;
        let mut result = false;
        while let Some((id, index, nontrivial)) = work.last_mut() {
            if *index == self.graph.constructors.get(*id).members.len() {
                let class = self.graph.constructors.get(*id).class;
                deleted |= self.graph.entity(class).key == Keyword::Union
                    && *nontrivial
                    && self.graph.entity(class).initialized_members == 0;
                let child = *nontrivial;
                work.pop();
                if let Some((_, _, parent)) = work.last_mut() {
                    *parent |= child;
                } else {
                    result = child;
                }
                continue;
            }
            let member = self.graph.constructors.get(*id).members[*index].0;
            let action = self.graph.constructors.get(*id).subobjects[*index];
            *index += 1;
            if let Some(child) = action.filter(|child| {
                self.graph
                    .constructors
                    .get(child.constructor)
                    .declaration
                    .is_none()
            }) {
                work.push((child.constructor, 0, false));
                continue;
            }
            // Source queries the member's function before testing its retained
            // initializer, independently of the accumulated variant result.
            let child_class = self.default_member_class(member);
            let constructor = if let Some(class) = child_class {
                self.implicit_constructor_function(class)?
            } else {
                None
            };
            *nontrivial |= self.graph.entity(member).member_initializer.is_some()
                || child_class.is_some() && !self.default_function_trivial(constructor)?;
        }
        Ok((result, deleted))
    }
    fn finish_inner_default_metadata(&mut self, root: ConstructorId) {
        let mut work = vec![(root, false)];
        while let Some((id, finish)) = work.pop() {
            if !finish {
                work.push((id, true));
                work.extend(
                    self.graph
                        .constructors
                        .get(id)
                        .subobjects
                        .iter()
                        .flatten()
                        .filter(|child| {
                            self.graph
                                .constructors
                                .get(child.constructor)
                                .declaration
                                .is_none()
                        })
                        .map(|child| (child.constructor, false)),
                );
                continue;
            }
            if self.graph.constructors.get(id).declaration.is_some() {
                continue;
            }
            let record = self.graph.constructors.get(id);
            let trivial = record.members.iter().all(|(_, init)| init.is_none())
                && record
                    .subobjects
                    .iter()
                    .flatten()
                    .all(|child| !self.graph.constructors.get(child.constructor).nontrivial());
            // This record is an inline array of entries, not a function. Its
            // exception demands always walk that array with the caller's fold.
            self.graph
                .constructors
                .finish_default_facts(id, trivial, false, false);
        }
    }
}

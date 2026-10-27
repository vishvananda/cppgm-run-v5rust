// Translated from sema/analyzer_construction.cpp::member_plans/member_plan. See NOTICE.
//! Constructor-owned nested actions retain original initializer and member identities.
use super::{
    Error,
    analyzer::Analyzer,
    construction::{Constructor, ConstructorId, MemberConstruction, Triviality},
    graph::{EntityId, ScopeId},
    types::TypeKind,
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

type Written = HashMap<EntityId, ast::MemInitializer>;
type Designations<'a> = HashMap<EntityId, Option<&'a ast::MemInitializer>>;
pub(super) struct MemberActions {
    pub members: Vec<(EntityId, Option<ast::Initializer>)>,
    pub written_members: Vec<bool>,
    pub subobjects: Vec<MemberConstruction>,
    pub deferred: bool,
}

impl Analyzer<'_> {
    // A union designates direct variant objects, even when several written
    // initializers name members of one nested anonymous struct.
    fn member_designation<'a>(
        &self,
        member: EntityId,
        written: &'a Written,
        known: &mut Designations<'a>,
    ) -> Option<&'a ast::MemInitializer> {
        let mut work = vec![(member, false)];
        while let Some((current, finish)) = work.pop() {
            if known.contains_key(&current) {
                continue;
            }
            if let Some(initializer) = written.get(&current) {
                known.insert(current, Some(initializer));
                continue;
            }
            let child = self
                .graph
                .entity(current)
                .ty
                .and_then(|ty| self.graph.class_of(ty))
                .filter(|&child| self.graph.anonymous_member_storage(child).is_some());
            if let Some(child) = child {
                let members = &self.graph.entity(child).members;
                if !finish {
                    work.push((current, true));
                    work.extend(members.iter().rev().map(|&member| (member, false)));
                    continue;
                }
                let first = members
                    .iter()
                    .find_map(|member| known.get(member).copied().flatten());
                known.insert(current, first);
            } else {
                known.insert(current, None);
            }
        }
        known.get(&member).copied().flatten()
    }
    pub(super) fn written_member_plans(
        &mut self,
        class: EntityId,
        written: &Written,
        scope: ScopeId,
        location: SourceOffset,
        deferred: bool,
    ) -> Result<MemberActions, Error> {
        self.member_actions(
            class,
            written,
            scope,
            location,
            &mut Designations::new(),
            deferred,
        )
    }
    fn member_actions<'a>(
        &mut self,
        class: EntityId,
        written: &'a Written,
        scope: ScopeId,
        location: SourceOffset,
        known: &mut Designations<'a>,
        mut deferred: bool,
    ) -> Result<MemberActions, Error> {
        self.check_stack()?;
        let variant = self.graph.entity(class).key == Keyword::Union;
        let mut designated = false;
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if let Some(initializer) = self.member_designation(member, written, known) {
                if variant && designated {
                    return Err(if self.graph.anonymous_member_storage(class).is_some() {
                        Error::at(
                            location,
                            "a constructor initializes several members of an anonymous union",
                        )
                    } else {
                        Error::at(
                            initializer.location,
                            "a union's constructor initializes several members",
                        )
                    });
                }
                designated = true;
            }
        }
        let mut plans = Vec::with_capacity(self.graph.entity(class).members.len());
        let mut subobjects = Vec::new();
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            let ty = self
                .graph
                .entity(member)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            let designation = self.member_designation(member, written, known);
            if variant && designated && designation.is_none() {
                plans.push((member, None));
                continue;
            }
            if let TypeKind::Class(child) = self.graph.types.get(ty).kind
                && self.graph.anonymous_member_storage(child).is_some()
            {
                let (constructor, pending) =
                    self.anonymous_member_plan(child, written, scope, location, known, deferred)?;
                deferred |= pending;
                subobjects.push(MemberConstruction {
                    member,
                    constructor,
                    arguments: None,
                });
                plans.push((member, None));
                continue;
            }
            if !(written.contains_key(&member) || variant && designated) {
                self.request_member_initializer(member)?;
            }
            let source = written
                .get(&member)
                .map(|written| written.initializer)
                .or_else(|| {
                    if variant && designated {
                        None
                    } else {
                        self.graph.entity(member).member_initializer
                    }
                });
            if let Some(written) = written.get(&member) {
                let initializer = ast::Initializer {
                    location: written.location,
                    ..written.initializer
                };
                let selected = match self.written_member_construction(ty, initializer, scope) {
                    Err(Error::DeferredBuiltin) if self.reserved_context.deferring => {
                        deferred = true;
                        self.note_deferred_initializer(initializer);
                        None
                    }
                    result => result?,
                };
                if let Some((constructor, arguments)) = selected {
                    subobjects.push(MemberConstruction {
                        member,
                        constructor,
                        arguments: Some(arguments),
                    });
                }
            } else if source.is_none() && !variant {
                let mut object = ty;
                while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
                    object = element;
                }
                if let TypeKind::Class(child) = self.graph.types.get(object).kind {
                    let constructor = self.default_constructor(child, location)?;
                    self.validate_constructor_use(constructor, scope, location)?;
                    if self.graph.types.get(object).cv.0 & 1 != 0
                        && !self.graph.constructors.get(constructor).const_default
                    {
                        return Err(Error::at(
                            location,
                            "a const object leaves a member uninitialized",
                        ));
                    }
                    if !matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. }) {
                        self.demand_constructor_use(constructor)?;
                    }
                    subobjects.push(MemberConstruction {
                        member,
                        constructor,
                        arguments: None,
                    });
                    plans.push((member, source));
                    continue;
                }
                if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                    return Err(Error::at(location, "a reference member is not initialized"));
                }
                if self.graph.types.get(object).cv.0 & 1 != 0 {
                    return Err(Error::at(location, "a const member is not initialized"));
                }
            }
            if !written.contains_key(&member)
                && source.is_some()
                && let Some(constructor) = self.graph.entity(member).constructor
            {
                subobjects.push(MemberConstruction {
                    member,
                    constructor,
                    arguments: self.graph.entity(member).constructor_arguments,
                });
            }
            plans.push((member, source));
        }
        Ok(MemberActions {
            deferred,
            written_members: plans
                .iter()
                .map(|(member, _)| written.contains_key(member))
                .collect(),
            members: plans,
            subobjects,
        })
    }
    fn anonymous_member_plan<'a>(
        &mut self,
        class: EntityId,
        written: &'a Written,
        scope: ScopeId,
        location: SourceOffset,
        known: &mut Designations<'a>,
        deferred: bool,
    ) -> Result<(ConstructorId, bool), Error> {
        let MemberActions {
            members,
            written_members,
            subobjects,
            deferred,
        } = self.member_actions(class, written, scope, location, known, deferred)?;
        let mut nothrow = true;
        for &(_, initializer) in &members {
            if !deferred && let Some(initializer) = initializer {
                nothrow &= !self.initializer_may_throw(initializer)?;
            }
        }
        for child in &subobjects {
            nothrow &= self.graph.constructors.get(child.constructor).nothrow;
        }
        let callable_type = self.constructor_signature(None)?;
        let constructor = self.publish_constructor(Constructor {
            facts: super::construction::ConstructorFacts::Complete,
            copy: None,
            copy_transfer: super::construction::CopyTransfer::Call,
            class,
            binding: None,
            declaration: None,
            object_pointer: None,
            callable_type,
            nothrow,
            deleted: false,
            const_default: false,
            members,
            written_members,
            bases: Vec::new(),
            indirect_bases: Vec::new(),
            subobjects,
            delegation: None,
            triviality: Triviality::Trivial,
        })?;
        Ok((constructor, deferred))
    }
}

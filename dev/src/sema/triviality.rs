// Translated from sema/analyzer_traits.cpp::trivially_copyable,
// copy_members/copy_member_trivial/subobject_members_select. See NOTICE.
//! Triviality queries reuse canonical special-member facts without demanding bodies.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{EntityId, FunctionAvailability},
    types::{Cv, RefKind, TypeId, TypeKind},
};
use std::collections::HashMap;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Kind {
    reference: RefKind,
    assignment: bool,
    nothrow: bool,
}
#[derive(Clone, Copy)]
enum Answer {
    Active,
    Ready(bool),
}
#[derive(Default)]
pub(super) struct Triviality {
    classes: HashMap<EntityId, Answer>,
    subobjects: HashMap<(EntityId, Kind), Answer>,
}
#[derive(Clone, Copy)]
struct Member {
    kind: Kind,
    parameter: TypeId,
    count: usize,
    cv: Cv,
    deleted: bool,
    trivial: bool,
    exceptions: MemberExceptions,
}
#[derive(Clone, Copy)]
enum MemberExceptions {
    Declared(EntityId),
    Implicit(bool),
}
impl Analyzer<'_> {
    pub(super) fn class_trivially_copyable(&mut self, class: EntityId) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(answer) = self.graph.triviality.classes.get(&class) {
            return match *answer {
                Answer::Ready(value) => Ok(value),
                Answer::Active => Err(Error::Invariant("cyclic copy trait")),
            };
        }
        self.prepare_copy_properties(class)?;
        self.graph.triviality.classes.insert(class, Answer::Active);
        let result = (|| {
            if !self.graph.destructor_trivial(class)? {
                return Ok(false);
            }
            for assignment in [false, true] {
                for member in self.trait_copy_members(class, assignment)? {
                    if !self.trait_member_trivial(class, member)? {
                        return Ok(false);
                    }
                }
            }
            Ok(true)
        })();
        if let Ok(value) = result {
            self.graph
                .triviality
                .classes
                .insert(class, Answer::Ready(value));
        } else {
            self.graph.triviality.classes.remove(&class);
        }
        result
    }
    pub(super) fn legacy_copy_trait(
        &mut self,
        class: EntityId,
        assignment: bool,
        nothrow: bool,
    ) -> Result<bool, Error> {
        self.prepare_copy_properties(class)?;
        let mut found = false;
        for member in self.trait_copy_members(class, assignment)? {
            if member.kind.reference != RefKind::Lvalue {
                continue;
            }
            found = true;
            if !(self.trait_member_trivial(class, member)?
                || nothrow && self.trait_member_nothrow(class, member)?)
            {
                return Ok(false);
            }
        }
        Ok(found)
    }
    fn trait_member_nothrow(&mut self, class: EntityId, member: Member) -> Result<bool, Error> {
        match member.exceptions {
            MemberExceptions::Implicit(_) if member.deleted => self.trait_subobjects(
                class,
                Kind {
                    nothrow: true,
                    ..member.kind
                },
            ),
            MemberExceptions::Implicit(nothrow) => Ok(nothrow),
            MemberExceptions::Declared(function) => {
                self.function_may_throw(function).map(|throws| !throws)
            }
        }
    }

    fn trait_copy_members(
        &mut self,
        class: EntityId,
        assignment: bool,
    ) -> Result<Vec<Member>, Error> {
        let entity = self.graph.entity(class);
        let home = entity
            .scope
            .ok_or(Error::Invariant("copy trait class scope"))?;
        let bindings = if assignment {
            self.graph
                .functions_at(home, super::assignment::key()?)
                .collect::<Vec<_>>()
        } else {
            entity.declared_constructors.clone()
        };
        let mut members = Vec::new();
        // C++'s name index prepends declarations. Selection of a move's
        // fallback copy observes that order, before the implicit members.
        for binding in bindings.into_iter().rev() {
            let entity = self.graph.binding(binding).entity;
            let function = self.graph.entity(entity);
            let kind = if assignment {
                function.assignment_kind
            } else {
                function.copy_kind
            };
            let Some(reference) = kind else {
                continue;
            };
            let signature = function
                .ty
                .ok_or(Error::Invariant("copy trait signature"))?;
            let TypeKind::Function { cv, parameters, .. } = &self.graph.types.get(signature).kind
            else {
                return Err(Error::Invariant("copy trait function type"));
            };
            members.push(Member {
                kind: Kind {
                    reference,
                    assignment,
                    nothrow: false,
                },
                parameter: parameters[0],
                count: parameters.len(),
                cv: *cv,
                deleted: function.availability == FunctionAvailability::Deleted,
                exceptions: if if assignment {
                    function.assignment_origin == super::assignment::Origin::Implicit
                } else {
                    function.copy_origin == super::copy_construction::Origin::Implicit
                } {
                    MemberExceptions::Implicit(if assignment {
                        function.assignment.as_ref().is_some_and(|p| p.nothrow)
                    } else {
                        function
                            .copy_properties
                            .is_some_and(|id| self.graph.copy_properties.get(id).nothrow)
                    })
                } else {
                    MemberExceptions::Declared(entity)
                },
                trivial: if assignment {
                    function.assignment.as_ref().is_some_and(|p| p.trivial)
                } else {
                    function
                        .copy_properties
                        .is_some_and(|id| self.graph.copy_properties.get(id).trivial)
                },
            });
        }
        let copies = self
            .graph
            .entity(class)
            .implicit_copies
            .ok_or(Error::Invariant("copy trait candidates"))?;
        if !assignment {
            for (reference, pair) in [
                (
                    RefKind::Lvalue,
                    copies
                        .copy
                        .zip(copies.copy_properties)
                        .map(|((ty, _), id)| (ty, id)),
                ),
                (RefKind::Rvalue, copies.moving.zip(copies.move_properties)),
            ] {
                if let Some((parameter, id)) = pair {
                    let facts = self.graph.copy_properties.get(id);
                    members.push(Member {
                        kind: Kind {
                            reference,
                            assignment,
                            nothrow: false,
                        },
                        parameter,
                        count: 1,
                        cv: Cv::default(),
                        deleted: facts.deleted,
                        trivial: facts.trivial,
                        exceptions: MemberExceptions::Implicit(facts.nothrow),
                    });
                }
            }
        }
        // implicit_copy_member omits deleted implicit moves entirely. The
        // explicit deleted members above still participate in this query.
        Ok(members)
    }
    fn trait_member_trivial(&mut self, class: EntityId, member: Member) -> Result<bool, Error> {
        if !member.deleted {
            return Ok(member.trivial);
        }
        if member.count != 1
            || !matches!(
                self.graph.types.get(member.parameter).kind,
                TypeKind::Reference { .. }
            )
        {
            return Ok(false);
        }
        self.trait_subobjects(class, member.kind)
    }
    fn trait_subobjects(&mut self, class: EntityId, kind: Kind) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(answer) = self.graph.triviality.subobjects.get(&(class, kind)) {
            return match *answer {
                Answer::Ready(value) => Ok(value),
                Answer::Active => Err(Error::Invariant("cyclic subobject copy trait")),
            };
        }
        self.graph
            .triviality
            .subobjects
            .insert((class, kind), Answer::Active);
        let result = self.trait_subobject_selection(class, kind);
        if let Ok(value) = result {
            self.graph
                .triviality
                .subobjects
                .insert((class, kind), Answer::Ready(value));
        } else {
            self.graph.triviality.subobjects.remove(&(class, kind));
        }
        result
    }
    fn trait_subobject_selection(&mut self, class: EntityId, kind: Kind) -> Result<bool, Error> {
        let entity = self.graph.entity(class);
        if !kind.nothrow
            && (entity.polymorphism.is_dynamic() || entity.bases.iter().any(|edge| edge.is_virtual))
        {
            return Ok(false);
        }
        let count = entity.bases.len() + entity.members.len();
        for index in 0..count {
            let Some((child, _, _)) = self.graph.copy_child(class, index)? else {
                continue;
            };
            if !self.trait_selection_trivial(child, kind)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn trait_selection_trivial(&mut self, child: EntityId, kind: Kind) -> Result<bool, Error> {
        let members = self.trait_copy_members(child, kind.assignment)?;
        let mut selected = members
            .iter()
            .copied()
            .find(|m| m.kind.reference == kind.reference && m.kind.assignment == kind.assignment)
            .or_else(|| {
                members
                    .iter()
                    .copied()
                    .find(|m| m.kind.reference == RefKind::Lvalue)
            });
        if selected.is_none() {
            return Ok(false);
        }
        if kind.reference == RefKind::Lvalue {
            let const_reference = members.iter().any(|m| m.kind.reference == kind.reference && m.kind.assignment == kind.assignment && m.cv.0 & 2 == 0 && matches!(self.graph.types.get(m.parameter).kind, TypeKind::Reference { element, .. } if self.graph.types.get(element).cv == Cv::CONST));
            let best: Vec<_> = members
                .iter()
                .copied()
                .filter(|m| {
                    m.kind.reference == kind.reference
                        && m.kind.assignment == kind.assignment
                        && match self.graph.types.get(m.parameter).kind {
                            TypeKind::Reference { element, .. } => {
                                let cv = self.graph.types.get(element).cv;
                                cv.0 & 1 != 0 && (cv == Cv::CONST || !const_reference)
                            }
                            _ => true,
                        }
                })
                .collect();
            if best.len() > 1 {
                if kind.nothrow {
                    return Ok(true);
                }
                for member in best {
                    if !member.deleted && !self.trait_member_trivial(child, member)? {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
            if let Some(&member) = best.first() {
                selected = Some(member);
            }
        }
        if let Some(member) = selected
            && !(self.trait_member_trivial(child, member)?
                || kind.nothrow && self.trait_member_nothrow(child, member)?)
        {
            return Ok(false);
        }
        Ok(true)
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
    fn repeated_triviality_reuse_complete_queries_without_body_or_syntax_demand() {
        let source = SourceBuffer::new(b"struct L{int n;template<class T>void unused(){T::missing();}};struct A{L a[4];};struct D{D(const D&)=delete;};struct N{N&operator=(const N&);};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"copy-trait-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"L", b"A", b"D", b"N"]
            .into_iter()
            .map(|text| cursor.intern_name(text).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer
                    .graph
                    .own(global, name, super::super::lookup::Filter::Elaborated)
                    .unwrap();
                analyzer.graph.binding(binding).entity
            })
            .collect();
        let syntax = (
            analyzer.tree.expressions.mark(),
            analyzer.tree.bodies.mark(),
        );
        let demands = analyzer.graph.root_demands.as_slice().len();
        for (&class, expected) in classes.iter().zip([true, true, true, false]) {
            assert_eq!(analyzer.class_trivially_copyable(class).unwrap(), expected);
        }
        let queries = (
            analyzer.graph.triviality.classes.len(),
            analyzer.graph.triviality.subobjects.len(),
        );
        for _ in 0..8 {
            for (&class, expected) in classes.iter().zip([true, true, true, false]) {
                assert_eq!(analyzer.class_trivially_copyable(class).unwrap(), expected);
            }
        }
        assert_eq!(
            queries,
            (
                analyzer.graph.triviality.classes.len(),
                analyzer.graph.triviality.subobjects.len()
            )
        );
        assert_eq!(
            syntax,
            (
                analyzer.tree.expressions.mark(),
                analyzer.tree.bodies.mark()
            )
        );
        assert_eq!(demands, analyzer.graph.root_demands.as_slice().len());
    }
}

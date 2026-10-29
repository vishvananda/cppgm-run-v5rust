// Translated from analyzer_initialization.cpp::implicit_constructor and
// analyzer_construction.cpp::implicit_default_facts/constructor_plan. See NOTICE.
//! Function identity precedes its live plan. Frames suspend at the reached
//! subobject; only the owning structural phase quarantines ambient failure.
use super::{
    Error,
    analyzer::Analyzer,
    construction::{
        BaseInitialization, Constructor, ConstructorExceptionSource, ConstructorFacts,
        ConstructorId, ConstructorState, CopyTransfer, DefaultFunction, MemberConstruction,
        Triviality,
    },
    graph::{BindingId, EntityId, FunctionAvailability, ScopeId},
    special_members::SpecialDefinition,
    substitution_state::{Ambient, Failure},
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};

pub(super) enum Lookup {
    Selected(BindingId),
    Absent,
    Ambiguous,
}
#[derive(Clone, Copy)]
enum Entry {
    Base {
        owner: ConstructorId,
        index: usize,
        indirect: bool,
    },
    Member {
        owner: ConstructorId,
        index: usize,
    },
}
#[derive(Clone, Copy)]
struct Waiting {
    entry: Entry,
    class: EntityId,
    object_type: Option<TypeId>,
    selected: Option<DefaultFunction>,
}
struct Frame {
    root: ConstructorId,
    class: EntityId,
    scope: ScopeId,
    entries: Vec<Entry>,
    next: usize,
    waiting: Option<Waiting>,
    pending: Option<Failure>,
    object: Option<EntityId>,
    deleted: bool,
    implicit_facts: bool,
    forwarded: Option<BindingId>,
    location: SourceOffset,
}
impl Analyzer<'_> {
    pub(super) fn implicit_constructor_function(
        &mut self,
        class: EntityId,
    ) -> Result<Option<DefaultFunction>, Error> {
        if let Some(slot) = self.graph.constructors.default_function(class) {
            return Ok(Some(slot));
        }
        self.check_stack()?;
        let (selected, frame) = self.begin_default_function(class)?;
        let Some(frame) = frame else {
            return Ok(selected);
        };
        let mut work = vec![frame];
        let result = self.default_function_frames(&mut work);
        if result.is_err() {
            while let Some(frame) = work.pop() {
                self.restore_default_frame(&frame);
            }
        }
        result?;
        Ok(selected)
    }

    fn begin_default_function(
        &mut self,
        class: EntityId,
    ) -> Result<(Option<DefaultFunction>, Option<Frame>), Error> {
        if let Some(slot) = self.graph.constructors.default_function(class) {
            return Ok((Some(slot), None));
        }
        let slot = match self.find_default_constructor(class)? {
            Lookup::Selected(binding) => DefaultFunction {
                function: self.graph.binding(binding).entity,
                declared: Some(binding),
            },
            Lookup::Ambiguous => return Ok((None, None)),
            Lookup::Absent if self.graph.entity(class).user_constructor.is_some() => {
                return Ok((None, None));
            }
            Lookup::Absent => DefaultFunction {
                function: self.implicit_default_declaration(class)?,
                declared: None,
            },
        };
        self.graph.constructors.retain_default_function(class, slot);
        let implicit = slot.declared.is_none();
        let defaulted = matches!(
            self.graph.entity(slot.function).special_definition,
            Some(SpecialDefinition::Defaulted { first: true, .. })
        );
        let closure = implicit && self.graph.closure_call(class).is_some();
        let id = if implicit || defaulted {
            if let Some(id) = self.graph.entity(slot.function).constructor {
                id
            } else {
                self.default_header(class, Some(slot))?
            }
        } else {
            self.declared_constructor_plan(
                slot.declared
                    .ok_or(Error::Invariant("declared default without binding"))?,
            )?
        };
        self.graph.entity_mut(class).default_construction = ConstructorState::Ready(id);
        if closure {
            self.graph.entity_mut(slot.function).availability = FunctionAvailability::Deleted;
            self.graph.constructors.mark_default_deleted(id);
            return Ok((Some(slot), None));
        }
        if !self.semantic()
            || !(implicit || defaulted)
            || self.graph.constructors.get(id).facts != ConstructorFacts::Declaration
        {
            return Ok((Some(slot), None));
        }
        let frame = self.begin_implicit_default_plan(id, class)?;
        Ok((Some(slot), Some(frame)))
    }

    pub(super) fn implicit_default_plan(
        &mut self,
        function: EntityId,
        class: EntityId,
    ) -> Result<ConstructorId, Error> {
        let id = if let Some(id) = self.graph.entity(function).constructor {
            id
        } else {
            let binding = self.graph.function_origin(function)?;
            self.default_header(
                class,
                Some(DefaultFunction {
                    function,
                    declared: Some(binding),
                }),
            )?
        };
        if self.graph.constructors.get(id).facts != ConstructorFacts::Declaration {
            return Ok(id);
        }
        let frame = self.begin_implicit_default_plan(id, class)?;
        let mut work = vec![frame];
        let result = self.default_function_frames(&mut work);
        if result.is_err() {
            while let Some(frame) = work.pop() {
                self.restore_default_frame(&frame);
            }
        }
        result?;
        Ok(id)
    }

    fn begin_implicit_default_plan(
        &mut self,
        id: ConstructorId,
        class: EntityId,
    ) -> Result<Frame, Error> {
        let function = self
            .graph
            .constructors
            .get(id)
            .declaration
            .ok_or(Error::Invariant("default facts without function"))?;
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("default plan without class scope"))?;
        if self.graph.constructors.get(id).binding.is_none()
            || matches!(
                self.graph.entity(function).special_definition,
                Some(SpecialDefinition::Defaulted { first: true, .. })
            )
        {
            let seed = self.graph.function_nothrow_value(function);
            let pending = self.pending_constructor_noexcept(function);
            self.graph.constructors.set_nothrow(id, seed);
            self.graph.entity_mut(function).constructor_exceptions =
                ConstructorExceptionSource::ComputedDefault {
                    constructor: id,
                    pending,
                };
        }
        let state = self.graph.analysis.substitution.clone();
        let pending = state.current().failure;
        state.replace(Ambient {
            failure: None,
            ..state.current()
        });
        let entries = self.default_skeleton(id);
        let entries = match entries {
            Ok(entries) => entries,
            Err(error) => {
                if pending.is_some() {
                    state.replace(Ambient {
                        failure: pending,
                        ..state.current()
                    });
                }
                return Err(error);
            }
        };
        self.default_function_plan_state(class, id, ConstructorState::Published(id));
        Ok(Frame {
            root: id,
            class,
            scope,
            entries,
            next: 0,
            waiting: None,
            pending,
            object: self.base_object,
            deleted: false,
            implicit_facts: true,
            forwarded: None,
            location: SourceOffset(0),
        })
    }

    /// The inheriting definition owns only its structure here. Its Soft scope
    /// and deletion flag belong to the caller; exception facts wait for demand.
    pub(super) fn inherited_structural_plan(
        &mut self,
        binding: BindingId,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        let function = self.graph.binding(binding).entity;
        let id = if let Some(id) = self.graph.entity(function).constructor {
            id
        } else {
            self.default_header(
                class,
                Some(DefaultFunction {
                    function,
                    declared: Some(binding),
                }),
            )?
        };
        if self.graph.constructors.get(id).facts != ConstructorFacts::Declaration {
            return Ok(id);
        }
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("inherited plan without class scope"))?;
        let forwarded = self
            .graph
            .inherited_constructors
            .source(function)
            .ok_or(Error::Invariant("inherited plan without source"))?;
        let frame = Frame {
            root: id,
            class,
            scope,
            entries: self.default_skeleton(id)?,
            next: 0,
            waiting: None,
            pending: None,
            object: self.base_object,
            deleted: false,
            implicit_facts: false,
            forwarded: Some(forwarded),
            location,
        };
        let mut work = vec![frame];
        let result = self.default_function_frames(&mut work);
        if result.is_err() {
            while let Some(frame) = work.pop() {
                self.restore_default_frame(&frame);
            }
        }
        result?;
        Ok(id)
    }

    fn default_function_plan_state(
        &mut self,
        class: EntityId,
        id: ConstructorId,
        state: ConstructorState,
    ) {
        if self
            .graph
            .constructors
            .default_function(class)
            .is_some_and(|slot| Some(slot.function) == self.graph.constructors.get(id).declaration)
        {
            self.graph.entity_mut(class).default_construction = state;
        }
    }

    pub(super) fn default_header(
        &mut self,
        class: EntityId,
        slot: Option<DefaultFunction>,
    ) -> Result<ConstructorId, Error> {
        let declaration = slot.map(|slot| slot.function);
        let binding = slot.and_then(|slot| slot.declared);
        let nothrow =
            declaration.is_some_and(|function| self.graph.function_nothrow_value(function));
        let trivial = declaration
            .and_then(|function| self.graph.entity(function).copy_properties)
            .is_some_and(|facts| self.graph.copy_properties.get(facts).trivial);
        let record = Constructor {
            facts: ConstructorFacts::Declaration,
            class,
            binding,
            declaration,
            object_pointer: None,
            callable_type: declaration.and_then(|function| self.graph.entity(function).ty),
            nothrow,
            deleted: declaration.is_some_and(|function| {
                self.graph.entity(function).availability == FunctionAvailability::Deleted
            }),
            members: Vec::new(),
            written_members: Vec::new(),
            bases: Vec::new(),
            indirect_bases: Vec::new(),
            subobjects: Vec::new(),
            delegation: None,
            copy: None,
            copy_transfer: CopyTransfer::Call,
            triviality: if trivial {
                Triviality::Trivial
            } else {
                Triviality::Nontrivial
            },
        };
        self.store_constructor(record)
    }

    fn default_skeleton(&mut self, root: ConstructorId) -> Result<Vec<Entry>, Error> {
        let mut records = vec![(root, true)];
        while let Some((id, function)) = records.pop() {
            let class = self.graph.constructors.get(id).class;
            let bases = if function {
                self.graph
                    .entity(class)
                    .bases
                    .iter()
                    .map(|edge| Self::empty_default_base(edge.class))
                    .collect()
            } else {
                Vec::new()
            };
            let indirect = if function {
                self.indirect_constructor_bases(class)
                    .into_iter()
                    .map(Self::empty_default_base)
                    .collect()
            } else {
                Vec::new()
            };
            let members = self
                .graph
                .entity(class)
                .members
                .iter()
                .map(|&member| (member, None))
                .collect();
            self.graph
                .constructors
                .install_default_entries(id, members, bases, indirect);
            for index in 0..self.graph.constructors.get(id).members.len() {
                let member = self.graph.constructors.get(id).members[index].0;
                let Some(child) = self
                    .graph
                    .entity(member)
                    .ty
                    .and_then(|ty| self.graph.class_of(ty))
                    .filter(|&child| self.graph.anonymous_member_storage(child) == Some(member))
                else {
                    continue;
                };
                let inner = self.default_header(child, None)?;
                self.graph.constructors.set_default_member(
                    id,
                    index,
                    None,
                    Some(MemberConstruction {
                        member,
                        constructor: inner,
                        arguments: None,
                    }),
                )?;
                records.push((inner, false));
            }
        }
        // All inner edges and leaf identities exist before the first semantic
        // dependency. Flatten only entry handles, never another mutable plan.
        let mut todo = vec![root];
        let mut entries = Vec::new();
        let mut owners = Vec::new();
        while let Some(id) = todo.pop() {
            owners.push(id);
            let record = self.graph.constructors.get(id);
            todo.extend(
                record
                    .subobjects
                    .iter()
                    .flatten()
                    .map(|child| child.constructor),
            );
        }
        for &id in &owners {
            self.graph.constructors.publish_default_entries(id);
        }
        let mut pending = vec![(root, 0usize)];
        while let Some((id, next)) = pending.pop() {
            let record = self.graph.constructors.get(id);
            if next < record.bases.len() {
                pending.push((id, next + 1));
                entries.push(Entry::Base {
                    owner: id,
                    index: next,
                    indirect: false,
                });
            } else if next < record.bases.len() + record.members.len() {
                let index = next - record.bases.len();
                pending.push((id, next + 1));
                if let Some(inner) = record.subobjects[index] {
                    pending.push((inner.constructor, 0));
                } else {
                    entries.push(Entry::Member { owner: id, index });
                }
            } else {
                let index = next - record.bases.len() - record.members.len();
                if index < record.indirect_bases.len() {
                    pending.push((id, next + 1));
                    entries.push(Entry::Base {
                        owner: id,
                        index,
                        indirect: true,
                    });
                }
            }
        }
        Ok(entries)
    }

    fn empty_default_base(class: EntityId) -> BaseInitialization {
        BaseInitialization {
            class,
            constructor: None,
            arguments: None,
            initializer: None,
            forwards: false,
        }
    }

    fn default_function_frames(&mut self, work: &mut Vec<Frame>) -> Result<(), Error> {
        while !work.is_empty() {
            let at = work.len() - 1;
            if let Some(waiting) = work[at].waiting.take() {
                self.complete_default_entry(&work[at], waiting)?;
                continue;
            }
            if let Some(&entry) = work[at].entries.get(work[at].next) {
                work[at].next += 1;
                let demand = self.prepare_default_entry(&mut work[at], entry)?;
                if let Some(mut waiting) = demand {
                    let (slot, child) = self.begin_default_function(waiting.class)?;
                    waiting.selected = slot;
                    work[at].waiting = Some(waiting);
                    if let Some(child) = child {
                        work.push(child);
                    }
                }
                continue;
            }
            let frame = work.pop().ok_or(Error::Invariant("default frame stack"))?;
            if !frame.implicit_facts {
                self.restore_default_frame(&frame);
                if frame.deleted {
                    self.graph.constructors.mark_default_deleted(frame.root);
                }
                continue;
            }
            let failed = self.graph.analysis.substitution.current().failure.is_some();
            let deleted = frame.deleted || failed;
            let trivial = !self.graph.lower_dynamic_class(frame.class) && !failed;
            self.restore_default_frame(&frame);
            self.complete_implicit_default_facts(frame.root, trivial, deleted)?;
            self.default_function_plan_state(
                frame.class,
                frame.root,
                ConstructorState::Ready(frame.root),
            );
        }
        Ok(())
    }

    fn restore_default_frame(&mut self, frame: &Frame) {
        self.base_object = frame.object;
        if frame.pending.is_some() {
            let state = &self.graph.analysis.substitution;
            state.replace(Ambient {
                failure: frame.pending,
                ..state.current()
            });
        }
    }

    fn prepare_default_entry(
        &mut self,
        frame: &mut Frame,
        entry: Entry,
    ) -> Result<Option<Waiting>, Error> {
        self.base_object = match entry {
            Entry::Base { .. } => Some(frame.class),
            Entry::Member { .. } => None,
        };
        let (class, object_type) = match entry {
            Entry::Base {
                owner,
                index,
                indirect,
            } => {
                let plan = self.graph.constructors.get(owner);
                let class = (if indirect {
                    &plan.indirect_bases
                } else {
                    &plan.bases
                })[index]
                    .class;
                if let Some(source) = frame.forwarded.filter(|&binding| {
                    self.graph
                        .entity(self.graph.binding(binding).entity)
                        .member_function
                        .is_some_and(|member| member.class == class)
                }) {
                    let function = self.graph.binding(source).entity;
                    let constructor = if let Some(id) = self.graph.entity(function).constructor {
                        id
                    } else {
                        self.default_header(
                            class,
                            Some(DefaultFunction {
                                function,
                                declared: Some(source),
                            }),
                        )?
                    };
                    self.graph
                        .constructors
                        .set_forwarded_base(owner, index, indirect, constructor);
                    return Ok(None);
                }
                (class, None)
            }
            Entry::Member { owner, index } => {
                let holder = self.graph.constructors.get(owner).class;
                let member = self.graph.constructors.get(owner).members[index].0;
                self.request_member_initializer(member)?;
                let variable = self.graph.entity(member);
                if let Some(initializer) = variable.member_initializer {
                    let action = variable.constructor.map(|constructor| MemberConstruction {
                        member,
                        constructor,
                        arguments: variable.constructor_arguments,
                    });
                    self.graph.constructors.set_default_member(
                        owner,
                        index,
                        Some(initializer),
                        action,
                    )?;
                    return Ok(None);
                }
                if self.graph.entity(holder).key == Keyword::Union {
                    return Ok(None);
                }
                let mut ty = variable
                    .ty
                    .ok_or(Error::Invariant("default member without type"))?;
                if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                    frame.deleted |= variable.name.is_some();
                    return Ok(None);
                }
                while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                    ty = element;
                }
                let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
                    frame.deleted |= self.graph.types.get(ty).cv.0 & 1 != 0;
                    return Ok(None);
                };
                (class, Some(ty))
            }
        };
        Ok(Some(Waiting {
            entry,
            class,
            object_type,
            selected: None,
        }))
    }

    fn complete_default_entry(&mut self, frame: &Frame, waiting: Waiting) -> Result<(), Error> {
        let selected = if let Some(slot) = waiting.selected {
            self.check_function_not_deleted(slot.function, frame.location)?;
            self.check_constructor_function_access(
                slot.function,
                waiting.class,
                frame.scope,
                frame.location,
                None,
            )?;
            Some(
                self.graph
                    .entity(slot.function)
                    .constructor
                    .ok_or(Error::Invariant("default subobject without record"))?,
            )
        } else {
            self.emit_substitution_failure(
                frame.location,
                "no default constructor for the class",
                false,
            )?;
            None
        };
        match waiting.entry {
            Entry::Base {
                owner,
                index,
                indirect,
            } => self
                .graph
                .constructors
                .set_default_base(owner, index, indirect, selected),
            Entry::Member { owner, index } => {
                let member = self.graph.constructors.get(owner).members[index].0;
                let action = selected.map(|constructor| MemberConstruction {
                    member,
                    constructor,
                    arguments: None,
                });
                self.graph
                    .constructors
                    .set_default_member(owner, index, None, action)?;
                if waiting
                    .object_type
                    .is_some_and(|ty| self.graph.types.get(ty).cv.0 & 1 != 0)
                {
                    self.check_const_default(waiting.class, waiting.selected, frame.location)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::lookup::Filter,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    fn inspect(action: impl FnOnce(&mut Analyzer<'_>, [EntityId; 3])) {
        let source = SourceBuffer::new(b"struct Deleted{Deleted()=delete;};struct Good{int value;};struct Bad{Deleted value;};struct NeedsArg{NeedsArg(int);};struct Hard{NeedsArg value;};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-function-state").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names =
            [b"Good".as_slice(), b"Bad", b"Hard"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes = names.map(|name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                .entity
        });
        action(&mut analyzer, classes);
    }

    #[test]
    fn a_query_keeps_its_deleted_function_and_restores_the_pending_failure() {
        inspect(|analyzer, [good, bad, _]| {
            let charges = analyzer.graph.work.totals.substitution_failures;
            analyzer.base_object = Some(good);
            let demands = analyzer.graph.root_demands.as_slice().len();
            analyzer
                .with_substitution_soft(Some(good), |analyzer| {
                    analyzer.emit_substitution_failure(SourceOffset(7), "earlier operand", true)?;
                    let outer = analyzer.graph.analysis.substitution.current();
                    let good_fn = analyzer.implicit_constructor_function(good)?.unwrap();
                    let good_id = analyzer.graph.entity(good_fn.function).constructor.unwrap();
                    assert!(!analyzer.graph.constructors.get(good_id).deleted);
                    assert!(!analyzer.graph.constructors.get(good_id).nontrivial());
                    assert_eq!(
                        analyzer.graph.analysis.substitution.current().failure,
                        outer.failure
                    );
                    let bad_fn = analyzer.queried_default_constructor_function(bad)?.unwrap();
                    let bad_id = analyzer.graph.entity(bad_fn.function).constructor.unwrap();
                    assert!(analyzer.graph.constructors.get(bad_id).deleted);
                    assert!(analyzer.graph.constructors.get(bad_id).nontrivial());
                    assert_eq!(
                        analyzer.graph.work.totals.substitution_failures,
                        charges + 1
                    );
                    let again = analyzer.queried_default_constructor_function(bad)?.unwrap();
                    assert_eq!(again.function, bad_fn.function);
                    assert_eq!(
                        analyzer.graph.work.totals.substitution_failures,
                        charges + 1
                    );
                    let current = analyzer.graph.analysis.substitution.current();
                    assert_eq!(current.failure, outer.failure);
                    assert_eq!(current.owner, outer.owner);
                    assert_eq!(current.soft, outer.soft);
                    assert_eq!(analyzer.unevaluated, 0);
                    assert_eq!(analyzer.base_object, Some(good));
                    assert!(analyzer.graph.function_origin(good_fn.function).is_err());
                    assert!(analyzer.graph.function_origin(bad_fn.function).is_err());
                    Ok(())
                })
                .unwrap();
            assert_eq!(
                analyzer.graph.work.totals.substitution_failures,
                charges + 2
            );
            assert_eq!(analyzer.graph.root_demands.as_slice().len(), demands);
        });
    }

    #[test]
    fn a_hard_plan_failure_keeps_published_entries_and_restores_context() {
        inspect(|analyzer, [good, _, hard]| {
            analyzer.base_object = Some(good);
            analyzer
                .with_substitution_soft(Some(good), |analyzer| {
                    analyzer.emit_substitution_failure(SourceOffset(9), "outer failure", false)?;
                    let outer = analyzer.graph.analysis.substitution.current();
                    let error = analyzer.without_soft_substitution(|analyzer| {
                        analyzer.implicit_constructor_function(hard)
                    });
                    assert!(matches!(
                        error,
                        Err(Error::UnconditionalDiagnostic {
                            message: "no default constructor for the class",
                            ..
                        })
                    ));
                    let slot = analyzer.graph.constructors.default_function(hard).unwrap();
                    let id = analyzer.graph.entity(slot.function).constructor.unwrap();
                    let record = analyzer.graph.constructors.get(id);
                    assert!(record.facts == ConstructorFacts::PlanPublished);
                    assert_eq!(record.members.len(), 1);
                    assert_eq!(record.subobjects.len(), 1);
                    assert!(record.subobjects[0].is_none());
                    assert!(!analyzer.constant_constructor_eligible(
                        id,
                        &mut std::collections::HashMap::new()
                    )?);
                    assert_eq!(analyzer.base_object, Some(good));
                    let current = analyzer.graph.analysis.substitution.current();
                    assert_eq!(current.failure, outer.failure);
                    assert_eq!(current.owner, outer.owner);
                    assert_eq!(current.soft, outer.soft);
                    assert_eq!(
                        analyzer
                            .implicit_constructor_function(hard)?
                            .unwrap()
                            .function,
                        slot.function
                    );
                    Ok(())
                })
                .unwrap();
        });
    }
}

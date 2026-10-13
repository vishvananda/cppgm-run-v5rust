// Translated from sema/analyzer_initialization.cpp and analyzer_construction.cpp. See NOTICE.
//! Demand-only implicit default construction for classes with scalar subobjects.
//! Records are unit-owned; declarations and expressions retain compact identities.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, EntityId},
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ConstructorId(u32);
#[derive(Clone, Copy, Default)]
pub(super) enum ConstructorState {
    #[default]
    NotStarted,
    InProgress,
    Ready(ConstructorId),
}
pub(super) struct Constructor {
    pub class: EntityId,
    pub object_pointer: TypeId,
    // The dump's function type includes its implicit object parameter.
    pub presentation_type: TypeId,
    deleted: bool,
    const_default: bool,
    initializers: usize,
}
#[derive(Default)]
pub(super) struct Constructors {
    records: Vec<Constructor>,
}
impl Constructors {
    fn add(&mut self, constructor: Constructor) -> Result<ConstructorId, Error> {
        let id = ConstructorId(
            u32::try_from(self.records.len())
                .map_err(|_| Error::Limit("implicit constructor limit"))?,
        );
        self.records.push(constructor);
        Ok(id)
    }
    pub(super) fn get(&self, id: ConstructorId) -> &Constructor {
        &self.records[id.0 as usize]
    }
}
impl Analyzer<'_> {
    pub(super) fn default_initialize_declaration(
        &mut self,
        binding: BindingId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.default_initialize(binding, location)?;
        let variable = self.graph.entity(self.graph.binding(binding).entity);
        // declare_variable counts nontrivial automatic construction after
        // selecting it. anonymous_union's hidden-object path does not count it.
        if variable
            .variable
            .is_some_and(|properties| properties.automatic)
            && variable
                .constructor
                .is_some_and(|id| self.graph.constructors.get(id).initializers != 0)
        {
            self.note_local_initializer();
        }
        Ok(())
    }
    pub(super) fn default_initialize(
        &mut self,
        binding: BindingId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.template_depth != 0 {
            return Ok(());
        }
        let variable = self.graph.binding(binding).entity;
        if self.graph.entity(variable).data_member == Some(super::graph::DataMemberKind::Static)
            && (self.graph.entity(variable).constexpr
                || matches!(
                    self.graph.entity(variable).constant,
                    super::constants::ConstantState::Value(_)
                ))
        {
            return Ok(());
        }
        let ty = self
            .graph
            .entity(variable)
            .ty
            .ok_or(Error::Invariant("default initialization without type"))?;
        let mut object = ty;
        while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
            object = element;
        }
        if let TypeKind::Class(class) = self.graph.types.get(object).kind {
            let constructor = self.implicit_scalar_constructor(class)?;
            let record = self.graph.constructors.get(constructor);
            if record.deleted {
                return Err(Error::at(location, "use of a deleted function"));
            }
            if self.graph.types.get(object).cv.0 & 1 != 0 && !record.const_default {
                return Err(Error::at(
                    location,
                    if self.graph.entity(class).key == Keyword::Union {
                        "a const union object initializes no member"
                    } else {
                        "a const object leaves a member uninitialized"
                    },
                ));
            }
            self.graph.entity_mut(variable).constructor = Some(constructor);
            // An array shows one element action without demanding a definition.
            if !matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. })
                && self.unevaluated == 0
            {
                self.record_demand(super::function_demands::Demand::Constructor(constructor));
            }
        } else if self.graph.types.get(object).cv.0 & 1 != 0
            && !self
                .graph
                .entity(variable)
                .variable
                .is_some_and(|properties| properties.is_extern)
        {
            return Err(Error::at(location, "const object without an initializer"));
        } else if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
            return Err(Error::at(location, "reference without an initializer"));
        }
        Ok(())
    }

    fn implicit_scalar_constructor(&mut self, class: EntityId) -> Result<ConstructorId, Error> {
        match self.graph.entity(class).default_construction {
            ConstructorState::Ready(id) => return Ok(id),
            ConstructorState::InProgress => {
                return Err(Error::Invariant("recursive scalar constructor demand"));
            }
            ConstructorState::NotStarted => (),
        }
        if self.graph.entity(class).user_constructor.is_some() {
            return Err(Error::Missing(
                "user-declared default constructor selection",
            ));
        }
        if !self.graph.entity(class).complete {
            return Err(Error::Invariant("constructor of an incomplete class"));
        }
        if self.graph.entity(class).template_head.is_some() {
            return Err(Error::Missing("class template default construction"));
        }
        self.graph.entity_mut(class).default_construction = ConstructorState::InProgress;
        let result = self.scalar_constructor_facts(class);
        self.graph.entity_mut(class).default_construction =
            result.as_ref().map_or(ConstructorState::NotStarted, |&id| {
                ConstructorState::Ready(id)
            });
        result
    }

    fn scalar_constructor_facts(&mut self, class: EntityId) -> Result<ConstructorId, Error> {
        let union = self.graph.entity(class).key == Keyword::Union;
        let mut deleted = false;
        let initialized = self.graph.entity(class).initialized_members;
        for &member in &self.graph.entity(class).members {
            let member = self.graph.entity(member);
            let mut ty = member
                .ty
                .ok_or(Error::Invariant("constructor member without type"))?;
            if member.member_initializer.is_some() {
                continue;
            }
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                return Err(Error::Missing("class subobject default construction"));
            }
            if !union {
                deleted |= matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                    || self.graph.types.get(ty).cv.0 & 1 != 0;
            }
        }
        let members = self.graph.entity(class).members.len();
        let const_default = members == 0
            || if union {
                initialized == 1
            } else {
                initialized == members
            };
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("constructor class without type"))?;
        let object_pointer = self
            .graph
            .types
            .intern(TypeKind::Pointer(ty), Cv::default())?;
        let void = self.graph.types.scalar(Scalar::Void)?;
        let presentation_type = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters: vec![object_pointer],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        self.graph.constructors.add(Constructor {
            class,
            object_pointer,
            presentation_type,
            deleted,
            const_default,
            initializers: initialized,
        })
    }
}

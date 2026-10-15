// Translated from analyzer_expressions.cpp::destructor_access and
// analyzer_calls.cpp's bound destructor call. See NOTICE.
//! Destructor designation and invocation retain the original object and selected type.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{DestructorTarget, ExprFacts, ObjectArgument, ValueCategory},
    function_resolution::CallTarget,
    graph::{BindingKind, FunctionAvailability, ScopeId},
    lookup::Filter,
    types::{Cv, Scalar, TypeKind},
};
use crate::syntax::{ast, storage::List};

impl Analyzer<'_> {
    pub(super) fn destructor_designation(
        &mut self,
        id: ast::ExprId,
        object: ObjectArgument,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        let last = *components
            .last()
            .ok_or(Error::Invariant("empty destructor name"))?;
        let ast::NameKind::Destructor(key) = last.kind else {
            return Err(Error::at(
                location,
                "a destructor name outside the supported forms",
            ));
        };
        let qualified = node.global || components.len() > 1;
        let object_class = self.graph.class_of(object.ty);
        let complete = object_class.filter(|&class| {
            self.graph.entity(class).complete && self.graph.entity(class).scope.is_some()
        });
        if qualified && complete.is_none() {
            return Err(Error::at(
                location,
                "a qualified destructor name for an object of no complete class type",
            ));
        }
        let naming = if let Some(class) = complete {
            let search = self.data_member_qualifier(name, class, scope)?;
            self.graph.scope(search).owner
        } else {
            object_class
        };
        self.lookup_location = location;
        let found = if let Some(home) = complete
            .and(naming)
            .and_then(|class| self.graph.entity(class).scope)
        {
            self.qualified(home, key, Filter::Elaborated)?
        } else {
            None
        };
        let binding = if let Some(binding) = found {
            Some(binding)
        } else {
            self.unqualified(scope, key, Filter::Elaborated)?
        }
        .filter(|&binding| {
            matches!(
                self.graph.binding(binding).kind,
                BindingKind::Type | BindingKind::Alias
            )
        })
        .ok_or(Error::at(
            location,
            "the destructor name does not name a type",
        ))?;
        if last.arguments.is_some() {
            return Err(Error::Missing("destructor template-id specialization"));
        }
        let named_type = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("destructor type name without type"))?;
        let target = if qualified {
            self.graph.lower_entity_type(
                naming.ok_or(Error::Invariant("qualified destructor without class"))?,
            )?
        } else {
            object.ty
        };
        if self.bare_type(named_type)? != self.bare_type(target)? {
            return Err(Error::at(
                location,
                "the destructor name does not name the object's type",
            ));
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        let target = if let Some(class) = naming {
            if !self.graph.entity(class).complete {
                return Err(Error::at(location, "a class without a destructor"));
            }
            let binding = self.explicit_destructor_binding(class)?;
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.availability == FunctionAvailability::Deleted {
                return Err(Error::at(location, "use of a deleted function"));
            }
            self.check_special_member_access(
                binding,
                object_class.ok_or(Error::Invariant("class destructor without object class"))?,
                scope,
                location,
            )?;
            DestructorTarget::Class(binding)
        } else {
            DestructorTarget::Scalar
        };
        let ty = match target {
            DestructorTarget::Class(binding) => self.graph.types.canonical_function(
                self.graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("destructor without type"))?,
            )?,
            DestructorTarget::Scalar => self.graph.types.intern(
                TypeKind::Function {
                    result: void,
                    parameters: Vec::new(),
                    variadic: false,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )?,
        };
        let mut facts = self.new_expression_facts(ty, ValueCategory::Prvalue)?;
        facts.destructor = Some(target);
        facts.object = Some(object);
        facts.member_naming = naming;
        if let (Some(object), Some(naming)) = (object_class, naming)
            && object != naming
        {
            if self.lowering() {
                self.request_storage_layout(object, location)?;
            }
            facts.member_naming_path = Some(self.graph.retain_base_path(object, naming)?);
        }
        Ok(facts)
    }

    fn explicit_destructor_binding(
        &mut self,
        class: super::graph::EntityId,
    ) -> Result<super::graph::BindingId, Error> {
        self.prepare_destruction(class)?;
        if let Some(binding) = self
            .destructor_callable(class)?
            .or(self.graph.entity(class).implicit_destructor)
        {
            return Ok(binding);
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        let signature = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters: Vec::new(),
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        self.graph.add_implicit_destructor(class, signature)
    }

    pub(super) fn destructor_call(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        target: DestructorTarget,
    ) -> Result<ExprFacts, Error> {
        if !self.tree.expr_lists.get(arguments).is_empty() {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "a destructor call with arguments",
            ));
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        let mut facts = self.new_expression_facts(void, ValueCategory::Prvalue)?;
        facts.destructor = Some(target);
        if let DestructorTarget::Class(binding) = target {
            facts.call = Some(CallTarget::Direct(
                self.graph
                    .function_origin(self.graph.binding(binding).entity)?,
            ));
            self.graph.record_call_binding(id, binding);
            self.demand_function(binding, true);
        }
        Ok(facts)
    }
}

// Translated from analyzer.cpp::using_declaration and
// analyzer_construction.cpp's inheriting constructor family. See NOTICE.
//! Sparse unit-owned inheritance identities. Signatures are published at class
//! completion; subobject plans and parameter storage are demanded at first use.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    function_names::ValueKey,
    graph::{
        Binding, BindingId, BindingKind, EntityId, FunctionProperties, MemberKey, MemberKind,
        ScopeId, ScopeKind,
    },
    types::{Cv, Scalar, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct InstanceKey {
    class: EntityId,
    source: EntityId,
    count: usize,
}
#[derive(Clone, Copy)]
enum Instance {
    InProgress,
    Ready(BindingId),
}
#[derive(Clone, Copy)]
struct Inheritance {
    source: BindingId,
    root: EntityId,
    full: bool,
}
#[derive(Default)]
pub(super) struct InheritedConstructors {
    bases: HashMap<EntityId, EntityId>,
    sources: HashMap<EntityId, Inheritance>,
    #[expect(
        clippy::box_collection,
        reason = "Demand-only cache keeps the by-value semantic graph out of the measured oversized analyze_unit stack frame"
    )]
    specializations: Option<Box<HashMap<InstanceKey, Instance>>>,
}
impl InheritedConstructors {
    fn instance(&self, key: InstanceKey) -> Option<Instance> {
        self.specializations.as_deref()?.get(&key).copied()
    }
    fn set_instance(&mut self, key: InstanceKey, value: Instance) {
        self.specializations
            .get_or_insert_with(Box::default)
            .insert(key, value);
    }
    fn discard_instance(&mut self, key: InstanceKey) {
        if let Some(instances) = &mut self.specializations {
            instances.remove(&key);
        }
    }
    pub(super) fn source(&self, function: EntityId) -> Option<BindingId> {
        self.sources.get(&function).map(|record| record.source)
    }
    pub(super) fn full_root(&self, function: EntityId) -> Option<EntityId> {
        self.sources
            .get(&function)
            .filter(|record| record.full)
            .map(|record| record.root)
    }
}
impl Analyzer<'_> {
    /// Patch only already planned inherited entries after concrete member typing.
    pub(super) fn typed_inheriting_member(&mut self, member: EntityId) {
        let Some(mut class) = self
            .graph
            .entity(member)
            .home
            .and_then(|home| self.graph.scope(home).owner)
        else {
            return;
        };
        while let Some(storage) = self.graph.anonymous_member_storage(class) {
            let Some(owner) = self
                .graph
                .entity(storage)
                .home
                .and_then(|home| self.graph.scope(home).owner)
            else {
                break;
            };
            class = owner;
        }
        if !self.graph.inherited_constructors.bases.contains_key(&class) {
            return;
        }
        if !matches!(
            self.graph.entity(class).literal,
            super::literal_types::LiteralState::InProgress
        ) {
            self.graph.entity_mut(class).literal = super::literal_types::LiteralState::NotStarted;
        }
        let Some(constructor) = self.graph.entity(member).constructor else {
            return;
        };
        let action = super::construction::MemberConstruction {
            member,
            constructor,
            arguments: self.graph.entity(member).constructor_arguments,
        };
        let mut work: Vec<_> = self
            .graph
            .entity(class)
            .declared_constructors
            .iter()
            .filter_map(|&binding| {
                let function = self.graph.binding(binding).entity;
                (self.graph.entity(function).home == self.graph.entity(class).scope
                    && self.graph.inherited_constructors.source(function).is_some())
                .then(|| self.graph.entity(function).constructor)
                .flatten()
            })
            .collect();
        while let Some(id) = work.pop() {
            for child in &self.graph.constructors.get(id).subobjects {
                if self
                    .graph
                    .entity(child.member)
                    .ty
                    .and_then(|ty| self.graph.class_of(ty))
                    .is_some_and(|class| self.graph.anonymous_member_storage(class).is_some())
                {
                    work.push(child.constructor);
                }
            }
            self.graph.constructors.patch_default_member(id, action);
        }
    }
    fn record_inherited_constructor(
        &mut self,
        function: EntityId,
        source: BindingId,
    ) -> Result<(), Error> {
        // C++ record_inherited_constructor: retain the original declaring
        // constructor even through shortened/default-argument forwarding forms.
        let parent = self.graph.binding(source).entity;
        let root = self
            .graph
            .inherited_constructors
            .sources
            .get(&parent)
            .map_or(parent, |record| record.root);
        let own = self.graph.lower_entity_type(function)?;
        let declared = self.graph.lower_entity_type(root)?;
        let (
            TypeKind::Function {
                parameters,
                variadic,
                ..
            },
            TypeKind::Function {
                parameters: root_parameters,
                variadic: root_variadic,
                ..
            },
        ) = (
            &self.graph.types.get(own).kind,
            &self.graph.types.get(declared).kind,
        )
        else {
            return Err(Error::Invariant("inherited constructor signature"));
        };
        let full = parameters.len() == root_parameters.len() && variadic == root_variadic;
        self.graph
            .inherited_constructors
            .sources
            .insert(function, Inheritance { source, root, full });
        Ok(())
    }
    pub(super) fn note_inherited_constructors(
        &mut self,
        scope: ScopeId,
        prefix: ScopeId,
        key: ValueKey,
        target: ast::Name,
    ) -> Result<bool, Error> {
        if self.graph.scope(prefix).kind != ScopeKind::Class {
            return Ok(false);
        }
        let base = self
            .graph
            .scope(prefix)
            .owner
            .ok_or(Error::Invariant("base without class"))?;
        let components = self
            .tree
            .name_lists
            .get(self.tree.names.get(target).components);
        let names_owner = self.graph.scope(scope).kind == ScopeKind::Class
            && components.len() >= 2
            && matches!(components[components.len() - 2].kind, ast::NameKind::Identifier(name) if Some(name) == key.identifier());
        if key.identifier() != self.graph.entity(base).name && !names_owner {
            return Ok(false);
        }
        if self.graph.scope(scope).kind == ScopeKind::Class {
            let class = self
                .graph
                .scope(scope)
                .owner
                .ok_or(Error::Invariant("using without class"))?;
            self.graph.inherited_constructors.bases.insert(class, base);
        }
        Ok(true)
    }
    pub(super) fn declare_inherited_constructors(&mut self, class: EntityId) -> Result<(), Error> {
        let Some(&base) = self.graph.inherited_constructors.bases.get(&class) else {
            return Ok(());
        };
        let owner = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("inheriting class without scope"))?;
        let least = usize::from(self.graph.entity(class).user_constructor.is_none());
        for index in (0..self.graph.entity(base).declared_constructors.len()).rev() {
            let source = self.graph.entity(base).declared_constructors[index];
            let entity = self.graph.binding(source).entity;
            if self.graph.entity(entity).template_head.is_some() {
                // The base template retains its own canonical identity. Only a
                // selected specialization creates a derived forwarding entry.
                let binding = self
                    .graph
                    .add_binding(owner, None, self.graph.binding(source))?;
                self.graph.inherit_binding_access(binding, source);
                self.graph
                    .entity_mut(class)
                    .declared_constructors
                    .push(binding);
                continue;
            }
            let ty = self
                .graph
                .binding(source)
                .ty
                .ok_or(Error::Invariant("inherited constructor without type"))?;
            let count = self.graph.types.function_parameter_count(ty)?;
            let mut first = count;
            while first > 0
                && self
                    .graph
                    .entity(entity)
                    .defaults
                    .get(first - 1)
                    .is_some_and(Option::is_some)
            {
                first -= 1;
            }
            for count in (first.max(least)..=count).rev() {
                if count == 1 {
                    let parameter = self.graph.types.function_parameter(ty, 0)?;
                    if matches!(self.graph.types.get(parameter).kind, TypeKind::Reference {element, ..} if self.graph.class_of(element) == Some(base))
                    {
                        break;
                    }
                }
                let void = self.graph.types.scalar(Scalar::Void)?;
                let parameters = (0..count)
                    .map(|i| self.graph.types.function_parameter(ty, i))
                    .collect::<Result<Vec<_>, _>>()?;
                let signature = self.graph.types.intern(
                    TypeKind::Function {
                        result: void,
                        parameters,
                        variadic: false,
                        cv: Cv::default(),
                        reference: None,
                    },
                    Cv::default(),
                )?;
                if self.graph.constructor(class, signature).is_some() {
                    continue;
                }
                let function = self.graph.add_entity(
                    Some(signature),
                    self.graph.entity(class).name,
                    Keyword::Int,
                )?;
                let explicitness = self.graph.entity(entity).explicitness;
                let availability = self.graph.entity(entity).availability;
                let node = self.graph.entity_mut(function);
                node.defined = true;
                node.inline = super::specifiers::InlinePreference::Inline;
                node.definition_emission = super::function_demands::DefinitionEmission::Demanded;
                node.explicitness = explicitness;
                node.availability = availability;
                node.constructor_exceptions =
                    super::construction::ConstructorExceptionSource::SubobjectPlan;
                node.function = Some(FunctionProperties {
                    effects: super::graph::FunctionEffect::Default,
                    no_inline: false,
                    returns: super::graph::FunctionReturn::MayReturn,
                    internal_linkage: false,
                    language: super::linkage::Language::Cpp,
                    denotes: None,
                    is_static: false,
                    special: None,
                    allocation: None,
                });
                let binding = self.graph.add_binding(
                    owner,
                    None,
                    Binding {
                        kind: BindingKind::Function,
                        name: None,
                        entity: function,
                        ty: Some(signature),
                        key: Keyword::Int,
                    },
                )?;
                self.graph.inherit_binding_access(binding, source);
                self.graph.record_member_function(
                    MemberKey {
                        class,
                        kind: MemberKind::Constructor,
                        signature,
                        shape: None,
                    },
                    binding,
                )?;
                self.record_inherited_constructor(function, source)?;
            }
        }
        Ok(())
    }
    pub(super) fn inherited_constructor_specialization(
        &mut self,
        class: EntityId,
        source: BindingId,
        arguments: usize,
    ) -> Result<BindingId, Error> {
        let function = self.graph.binding(source).entity;
        let Some(member) = self.graph.entity(function).member_function else {
            return Ok(source);
        };
        if member.class == class || self.graph.function_templates.primary(function).is_none() {
            return Ok(source);
        }
        self.check_stack()?;
        let base = *self
            .graph
            .inherited_constructors
            .bases
            .get(&class)
            .ok_or(Error::Invariant("inherited template without named base"))?;
        let forwarded = if member.class == base {
            source
        } else {
            self.inherited_constructor_specialization(base, source, arguments)?
        };
        let ty = self
            .graph
            .lower_entity_type(self.graph.binding(forwarded).entity)?;
        let count = arguments.min(self.graph.types.function_parameter_count(ty)?);
        let key = InstanceKey {
            class,
            source: function,
            count,
        };
        match self.graph.inherited_constructors.instance(key) {
            Some(Instance::Ready(binding)) => {
                self.demand_inherited_definition(binding)?;
                return Ok(binding);
            }
            Some(Instance::InProgress) => {
                return Err(Error::at(
                    SourceOffset(0),
                    "template instantiation nested too deeply",
                ));
            }
            None => (),
        }
        let TypeKind::Function { variadic, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("inherited template without function type"));
        };
        let parameters = (0..count)
            .map(|index| self.graph.types.function_parameter(ty, index))
            .collect::<Result<Vec<_>, _>>()?;
        let void = self.graph.types.scalar(Scalar::Void)?;
        let signature = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters,
                variadic,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let owner = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("inherited template without owner"))?;
        let inherited =
            self.graph
                .add_entity(Some(signature), self.graph.entity(class).name, Keyword::Int)?;
        let source_entity = self.graph.entity(self.graph.binding(forwarded).entity);
        let properties = (
            source_entity.function,
            source_entity.availability,
            source_entity.explicitness,
        );
        let node = self.graph.entity_mut(inherited);
        node.home = Some(owner);
        node.defined = true;
        node.inline = super::specifiers::InlinePreference::Inline;
        node.definition_emission = super::function_demands::DefinitionEmission::Demanded;
        node.function = properties.0;
        node.availability = properties.1;
        node.explicitness = properties.2;
        node.constructor_exceptions =
            super::construction::ConstructorExceptionSource::SubobjectPlan;
        node.member_function = Some(MemberKey {
            class,
            kind: MemberKind::Constructor,
            signature,
            shape: None,
        });
        let binding = self.graph.add_binding(
            owner,
            None,
            Binding {
                kind: BindingKind::Function,
                name: None,
                entity: inherited,
                ty: Some(signature),
                key: Keyword::Int,
            },
        )?;
        self.graph.inherit_binding_access(binding, forwarded);
        self.record_inherited_constructor(inherited, forwarded)?;
        self.graph
            .inherited_constructors
            .set_instance(key, Instance::InProgress);
        if let Err(error) = self.build_inherited_constructor(binding, class) {
            if !matches!(error, Error::Diagnostic { .. } | Error::NonConstant { .. }) {
                self.graph.inherited_constructors.discard_instance(key);
                return Err(error);
            }
            self.graph.work.totals.substitution_failures += 1;
            // The implicit definition's invalid subobject plan deletes this
            // selected forwarding entry; the use owns its final diagnostic.
            self.graph.entity_mut(inherited).availability =
                super::graph::FunctionAvailability::Deleted;
        }
        self.graph
            .inherited_constructors
            .set_instance(key, Instance::Ready(binding));
        // Exception facts precede definition demand, as in the frozen plan.
        self.demand_inherited_definition(binding)?;
        Ok(binding)
    }
    fn demand_inherited_definition(&mut self, mut binding: BindingId) -> Result<(), Error> {
        if self.unevaluated != 0 || self.template_depth != 0 {
            return Ok(());
        }
        while let Some(source) = self
            .graph
            .inherited_constructors
            .source(self.graph.binding(binding).entity)
        {
            let function = self.graph.entity(self.graph.binding(binding).entity);
            if function.availability == super::graph::FunctionAvailability::Deleted {
                break;
            }
            let count = self.graph.types.function_parameter_count(
                function
                    .ty
                    .ok_or(Error::Invariant("inherited definition without type"))?,
            )?;
            self.instantiate_call_defaults(source, count)?;
            self.demand_specialization_body(source);
            binding = source;
        }
        Ok(())
    }
    pub(super) fn selected_constructor_plan(
        &mut self,
        binding: BindingId,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        let function = self.graph.binding(binding).entity;
        if self.graph.entity(function).availability == super::graph::FunctionAvailability::Deleted {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.declared_constructor_plan(binding).map_err(|error| {
            if self.graph.inherited_constructors.source(function).is_some()
                && matches!(error, Error::Diagnostic { .. })
            {
                self.graph.entity_mut(function).availability =
                    super::graph::FunctionAvailability::Deleted;
                Error::at(location, "use of a deleted function")
            } else {
                error
            }
        })
    }
    pub(super) fn inherited_constructor_plan(
        &mut self,
        binding: BindingId,
    ) -> Result<ConstructorId, Error> {
        let mut pending = Vec::new();
        let mut current = binding;
        while self
            .graph
            .entity(self.graph.binding(current).entity)
            .constructor
            .is_none()
        {
            let function = self.graph.binding(current).entity;
            let Some(source) = self.graph.inherited_constructors.source(function) else {
                break;
            };
            pending.push(current);
            current = source;
        }
        for binding in pending.into_iter().rev() {
            let class = self
                .graph
                .entity(self.graph.binding(binding).entity)
                .member_function
                .ok_or(Error::Invariant("inheriting constructor without class"))?
                .class;
            self.build_inherited_constructor(binding, class)?;
        }
        self.graph
            .entity(self.graph.binding(binding).entity)
            .constructor
            .ok_or(Error::Invariant("inherited plan was not published"))
    }
    fn build_inherited_constructor(
        &mut self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<(), Error> {
        self.check_stack()?;
        // The inherited base is already selected. Other direct bases and
        // uninitialized class members ask for their own default construction.
        let source = self
            .graph
            .inherited_constructors
            .source(self.graph.binding(binding).entity)
            .ok_or(Error::Invariant("inheriting constructor without source"))?;
        let base = self
            .graph
            .entity(self.graph.binding(source).entity)
            .member_function
            .ok_or(Error::Invariant("forwarded constructor without class"))?
            .class;
        if !self
            .graph
            .entity(class)
            .bases
            .iter()
            .any(|edge| edge.class == base)
        {
            return Err(Error::Missing("inherited constructor indirect base plans"));
        }
        for index in 0..self.graph.entity(class).bases.len() {
            let child = self.graph.entity(class).bases[index].class;
            if child != base {
                self.default_constructor(child, SourceOffset(0))?;
            }
        }
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if let Some(child) = self.default_member_dependency(member)? {
                self.default_member_constructor(child, SourceOffset(0))?;
            }
        }
        let id = self.scalar_constructor_facts(class, Some(binding))?;
        let function = self.graph.binding(binding).entity;
        self.graph.entity_mut(function).exceptions =
            super::exceptions::ExceptionState::Value(if self.graph.constructors.get(id).nothrow {
                super::exceptions::ExceptionSet::None
            } else {
                super::exceptions::ExceptionSet::All
            });
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("inheriting signature missing"))?;
        let owner = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("inheriting class without scope"))?;
        let scope = self.graph.add_scope(
            ScopeKind::Function,
            owner,
            Some(owner),
            self.graph.entity(class).name,
        )?;
        self.graph.scope_mut(scope).owner = Some(function);
        self.graph.entity_mut(function).scope = Some(scope);
        for index in 0..self.graph.types.function_parameter_count(ty)? {
            let ty = self.graph.types.function_parameter(ty, index)?;
            let parameter = self.graph.add_entity(Some(ty), None, Keyword::Int)?;
            self.graph.add_binding(
                scope,
                Some(scope),
                Binding {
                    kind: BindingKind::Parameter,
                    name: None,
                    entity: parameter,
                    ty: Some(ty),
                    key: Keyword::Int,
                },
            )?;
            self.graph
                .entity_mut(function)
                .parameter_entities
                .push(parameter);
        }
        self.check_subobject_destruction(class, owner, SourceOffset(0))?;
        self.prepare_destruction(class)?;
        self.graph.constructor_boundaries.push((
            self.body_context.as_ref().and_then(|body| body.function),
            id,
        ));
        Ok(())
    }
    pub(super) fn check_inherited_constructor_access(
        &self,
        binding: BindingId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let function = self.graph.binding(binding).entity;
        let Some(source) = self.graph.inherited_constructors.source(function) else {
            return Ok(());
        };
        let class = self
            .graph
            .entity(function)
            .member_function
            .ok_or(Error::Invariant("inheriting constructor without owner"))?
            .class;
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("inheriting class without scope"))?;
        self.check_special_member_access(source, class, scope, location)
    }
}

impl super::graph::Graph {
    pub(crate) fn lower_positional_parameters(&self, function: EntityId) -> bool {
        self.inherited_constructors.source(function).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::Instance;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, construction::ConstructorState, lookup::Filter, types::TypeKind},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };
    #[test]
    fn inheritance_keeps_signatures_sources_and_demanded_storage_separate() {
        let source = SourceBuffer::new(b"struct B{B(int,int=3);B(const B&);};typedef B Alias;struct D:Alias{using Alias::Alias;int n=7;};struct E:D{using D::D;};struct Unused:B{using B::B;};void f(){E e(1);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"inherited-identities").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(expressions, tree.expressions.mark());
        let mut class = |name: &[u8]| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity
        };
        let base = class(b"B");
        let derived = class(b"D");
        let end = class(b"E");
        let unused = class(b"Unused");
        for owner in [derived, end, unused] {
            let declarations = &graph.entity(owner).declared_constructors;
            assert_eq!(declarations.len(), 2); // The base copy constructor is excluded.
            let mut counts: Vec<_> = declarations
                .iter()
                .map(|&id| {
                    graph
                        .types
                        .function_parameter_count(graph.binding(id).ty.unwrap())
                        .unwrap()
                })
                .collect();
            counts.sort_unstable();
            assert_eq!(counts, [1, 2]);
            for &binding in declarations {
                let function = graph.binding(binding).entity;
                let record = graph.inherited_constructors.sources[&function];
                assert_eq!(
                    graph.entity(record.root).member_function.unwrap().class,
                    base
                );
                let count = graph
                    .types
                    .function_parameter_count(graph.binding(binding).ty.unwrap())
                    .unwrap();
                assert_eq!(record.full, count == 2);
                assert_eq!(
                    graph
                        .lower_function_in_mode(function, true)
                        .unwrap()
                        .inherited_constructor,
                    (count == 2).then_some(record.root)
                );
                assert!(
                    graph
                        .lower_function(function)
                        .unwrap()
                        .inherited_constructor
                        .is_none()
                );
            }
            assert!(matches!(
                graph.entity(owner).default_construction,
                ConstructorState::NotStarted
            ));
        }
        let one_argument = |owner| {
            graph
                .entity(owner)
                .declared_constructors
                .iter()
                .copied()
                .find(|&binding| {
                    graph
                        .types
                        .function_parameter_count(graph.binding(binding).ty.unwrap())
                        .unwrap()
                        == 1
                })
                .unwrap()
        };
        let binding = one_argument(end);
        let function = graph.binding(binding).entity;
        let plan = graph
            .constructors
            .get(graph.entity(function).constructor.unwrap());
        assert_eq!(plan.class, end);
        assert!(plan.bases[0].forwards);
        let source = graph.inherited_constructors.source(function).unwrap();
        assert_eq!(source, one_argument(derived));
        let middle = graph.constructors.get(plan.bases[0].constructor.unwrap());
        assert_eq!(middle.class, derived);
        let crate::syntax::ast::InitializerKind::Assign(expression) =
            middle.members[0].1.unwrap().kind
        else {
            panic!("member initializer shape changed");
        };
        let crate::syntax::ast::InitializerKind::Assign(original_expression) = graph
            .entity(graph.entity(derived).members[0])
            .member_initializer
            .unwrap()
            .kind
        else {
            panic!("original initializer shape changed");
        };
        assert_eq!(expression, original_expression);
        let original = graph
            .inherited_constructors
            .source(graph.binding(source).entity)
            .unwrap();
        assert_eq!(
            graph
                .entity(graph.binding(original).entity)
                .member_function
                .unwrap()
                .class,
            base
        );
        let parameters: Vec<_> = graph
            .lower_parameters(graph.entity(function).scope)
            .collect();
        assert_eq!(parameters.len(), 1);
        assert!(parameters[0].1.is_none());
        assert!(matches!(
            graph.types.get(parameters[0].2).kind,
            TypeKind::Scalar(_)
        ));
        assert!(
            graph
                .entity(unused)
                .declared_constructors
                .iter()
                .all(|&binding| graph
                    .entity(graph.binding(binding).entity)
                    .constructor
                    .is_none())
        );
        assert!(graph.lower_class_layout(unused).is_err());
    }

    #[test]
    fn inherited_instance_keys_retain_class_source_and_forwarded_count() {
        let source = SourceBuffer::new(
            br#"
            struct A { int n; template<class T>constexpr A(T x,int y=5):n(x+y){} };
            struct B:A { using A::A; };
            struct C:A { using A::A; };
            constexpr B b1(3),b2(4),b3(5,6);
            constexpr C c1(7);
            static_assert(b1.n==8 && b2.n==9 && b3.n==11 && c1.n==12, "values");
            int main(){return 0;}
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"inherited-keys").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let entries: Vec<_> = analyzer
            .graph
            .inherited_constructors
            .specializations
            .as_ref()
            .unwrap()
            .iter()
            .map(|(&key, &value)| (key, value))
            .collect();
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries
                .iter()
                .map(|(key, _)| key.source)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            1
        );
        assert_eq!(
            entries
                .iter()
                .map(|(key, _)| key.class)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            2
        );
        let mark = analyzer.tree.expressions.mark();
        for (key, value) in entries {
            let Instance::Ready(binding) = value else {
                panic!("completed forwarding entry");
            };
            let source = analyzer.graph.function_origin(key.source).unwrap();
            assert_eq!(
                analyzer
                    .inherited_constructor_specialization(key.class, source, key.count)
                    .unwrap(),
                binding
            );
            let entity = analyzer
                .graph
                .entity(analyzer.graph.binding(binding).entity);
            assert_eq!(entity.parameter_entities.len(), key.count);
            assert!(entity.constructor.is_some());
            let record = analyzer.graph.inherited_constructors.sources
                [&analyzer.graph.binding(binding).entity];
            assert_eq!(record.root, key.source);
            assert_eq!(record.full, key.count == 2);
        }
        assert_eq!(analyzer.tree.expressions.mark(), mark);
        assert_eq!(analyzer.constexpr_frames.depth(), 0);
    }
}
